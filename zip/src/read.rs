pub(crate) mod poll;

use {
    crate::{
        datetime::ZipDateTime,
        path::{Sanitize, ZipPath},
        specs::{
            compression::Compression, extra_field::ExtraField, GeneralPurposeFlag,
            Zip32CentralDirectoryEndRecord, Zip64CentralDirectoryEndLocator,
            Zip64CentralDirectoryEndRecord, ZipCentralDirectoryEndRecord, ZipEntry, ZipSpecs,
            SIGNATURE_LENGTH,
        },
        ZipError, ZipFile, ZipResult,
    },
    fastsearch::FastSearch,
    indexmap::IndexMap,
    smol::{
        io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, SeekFrom},
        stream::{iter, StreamExt},
    },
};

pub(crate) trait ZipAsyncReadExt {
    async fn read_u32_le(&mut self) -> ZipResult<u32>
    where
        Self: AsyncRead + Unpin,
    {
        let mut buffer = [0; 4];
        self.read_exact(&mut buffer).await?;
        Ok(u32::from_le_bytes(buffer))
    }

    async fn read_zip32_record(&mut self) -> ZipResult<(u64, Zip32CentralDirectoryEndRecord)>
    where
        Self: AsyncRead + AsyncSeek + Unpin,
    {
        let length = self.seek(SeekFrom::End(0)).await? as usize;
        let signature = Zip32CentralDirectoryEndRecord::SIGNATURE.to_le_bytes();
        let position = length.saturating_sub(Zip32CentralDirectoryEndRecord::MAX_SIZE);
        let max = length - position;

        let mut buffer = Vec::with_capacity(max);

        self.seek(SeekFrom::Start(position as u64)).await?;
        self.read_to_end(&mut buffer).await?;
        if let Some(idx) = buffer.rsearch(&signature) {
            let offset = (position + idx) as u64;
            self.seek(SeekFrom::Start(offset + 4)).await?;
            let mut buffer = [0; Zip32CentralDirectoryEndRecord::SIZE];
            self.read_exact(&mut buffer).await?;
            let record = Zip32CentralDirectoryEndRecord::try_from(buffer)?;
            Ok((offset, record))
        } else {
            Err(ZipError::SignatureNotFound(
                "Central Directory End Record Signature Not Found".into(),
            ))
        }
    }

    async fn read_to_zip_path(&mut self, path: &mut ZipPath) -> ZipResult<usize>
    where
        Self: AsyncRead + Unpin,
    {
        let mut string = String::new();
        let read = self.read_to_string(&mut string).await?;
        path.append(string);
        path.sanitize();
        Ok(read)
    }

    async fn read_zip64_locator(&mut self) -> ZipResult<Zip64CentralDirectoryEndLocator>
    where
        Self: AsyncRead + Unpin,
    {
        let signature = self.read_u32_le().await?;
        if signature != Zip64CentralDirectoryEndLocator::SIGNATURE {
            Err(ZipError::SignatureNotFound(
                "Invalid Central Directory End Locator Signature".into(),
            ))
        } else {
            let mut buffer = [0; Zip64CentralDirectoryEndLocator::SIZE];
            self.read_exact(&mut buffer).await?;
            Ok(Zip64CentralDirectoryEndLocator::try_from(buffer)?)
        }
    }

    async fn read_zip64_record(&mut self) -> ZipResult<Zip64CentralDirectoryEndRecord>
    where
        Self: AsyncRead + AsyncSeek + Unpin,
    {
        let signature = self.read_u32_le().await?;
        if signature != Zip64CentralDirectoryEndRecord::SIGNATURE {
            Err(ZipError::SignatureNotFound(
                "Invalid Signature for Zip64 Central Directory End Record".into(),
            ))
        } else {
            let mut buffer = [0; Zip64CentralDirectoryEndRecord::SIZE];
            self.read_exact(&mut buffer).await?;
            Ok(Zip64CentralDirectoryEndRecord::try_from(buffer)?)
        }
    }

    async fn read_zip_cd_end(&mut self) -> ZipResult<ZipCentralDirectoryEndRecord>
    where
        Self: AsyncRead + AsyncSeek + Unpin,
    {
        let length = self.seek(SeekFrom::End(0)).await?;
        let zip32 = Zip32CentralDirectoryEndRecord::SIGNATURE.to_le_bytes();
        let position = length.saturating_sub(Zip32CentralDirectoryEndRecord::MAX_SIZE as u64);
        let max = length - position;

        let mut buffer = Vec::with_capacity(max as usize);

        self.seek(SeekFrom::Start(position)).await?;
        self.read_to_end(&mut buffer).await?;
        if let Some(idx_32) = buffer.rsearch(&zip32) {
            let idx = idx_32 + 4;
            let zip32_record = Zip32CentralDirectoryEndRecord::try_from(
                TryInto::<[u8; 18]>::try_into(&buffer[idx..idx + 18])?,
            )?;
            let comment = if zip32_record.file_comment_length > 0 {
                let mut string = String::new();
                (&buffer[idx + 18..])
                    .take(zip32_record.file_comment_length as u64)
                    .read_to_string(&mut string)
                    .await?;
                Some(string)
            } else {
                None
            };
            if zip32_record.central_directory_size == u32::MAX
                || zip32_record.number_of_entries == u16::MAX
            {
                let zip64 = Zip64CentralDirectoryEndRecord::SIGNATURE.to_le_bytes();
                let zip64_record = if let Some(idx_64) = (&buffer[..idx_32]).rsearch(&zip64) {
                    let idx = idx_64 + 4;
                    Zip64CentralDirectoryEndRecord::try_from(TryInto::<[u8; 52]>::try_into(
                        &buffer[idx..idx + 52],
                    )?)?
                } else {
                    let locator = Zip64CentralDirectoryEndLocator::SIGNATURE.to_le_bytes();

                    if let Some(lidx) = (&buffer[..idx_32]).rsearch(&locator) {
                        self.seek(SeekFrom::Start((lidx + 4) as u64 + position))
                            .await?;

                        let mut buffer = [0; Zip64CentralDirectoryEndLocator::SIZE];
                        self.read_exact(&mut buffer).await?;
                        let locator = Zip64CentralDirectoryEndLocator::try_from(buffer)?;

                        self.seek(SeekFrom::Start(locator.relative_offset + 4))
                            .await?;
                        let mut buffer = [0; Zip64CentralDirectoryEndRecord::SIZE];
                        self.read_exact(&mut buffer).await?;
                        Zip64CentralDirectoryEndRecord::try_from(buffer)?
                    } else {
                        self.seek(SeekFrom::Start(position + (idx_32 - 20) as u64))
                            .await?;
                        let locator = self.read_zip64_locator().await?;
                        self.seek(SeekFrom::Start(locator.relative_offset)).await?;
                        self.read_zip64_record().await?
                    }
                };
                let _version_made_by = Some(zip64_record.version_made_by);
                let version_needed = Some(zip64_record.version_needed);
                let disk_number = zip64_record.disk_number;
                let central_directory_start_disk = zip64_record.central_directory_start_disk;
                let number_of_entries_in_disk = zip64_record.number_of_entries_in_disk;
                let number_of_entries = zip64_record.number_of_entries;
                let central_directory_size = zip64_record.central_directory_size;
                let central_directory_offset = zip64_record.central_directory_offset;
                Ok(ZipCentralDirectoryEndRecord {
                    _version_made_by,
                    version_needed,
                    disk_number,
                    central_directory_start_disk,
                    number_of_entries_in_disk,
                    number_of_entries,
                    central_directory_size,
                    central_directory_offset,
                    comment,
                })
            } else {
                let _version_made_by = None;
                let version_needed = None;
                let disk_number = zip32_record.disk_number as u32;
                let central_directory_start_disk = zip32_record.central_directory_start_disk as u32;
                let number_of_entries_in_disk = zip32_record.number_of_entries_in_disk as u64;
                let number_of_entries = zip32_record.number_of_entries as u64;
                let central_directory_size = zip32_record.central_directory_size as u64;
                let central_directory_offset = zip32_record.central_directory_offset as u64;

                Ok(ZipCentralDirectoryEndRecord {
                    _version_made_by,
                    version_needed,
                    disk_number,
                    central_directory_start_disk,
                    number_of_entries_in_disk,
                    number_of_entries,
                    central_directory_size,
                    central_directory_offset,
                    comment,
                })
            }
        } else {
            Err(ZipError::SignatureNotFound(
                "Central Directory End Record Signature not Found".into(),
            ))
        }
    }

    async fn read_zip_entry(
        &mut self,
        eocdr: &ZipCentralDirectoryEndRecord,
    ) -> ZipResult<IndexMap<ZipPath, ZipEntry>>
    where
        Self: AsyncRead + AsyncSeek + Unpin,
    {
        let size = eocdr.central_directory_size;
        let offset = eocdr.central_directory_offset;
        let mut buffer = Vec::with_capacity(size as usize);
        let signature = ZipEntry::SIGNATURE.to_le_bytes();
        self.seek(SeekFrom::Start(offset)).await?;
        self.take(size).read_to_end(&mut buffer).await?;
        let multi_idx = buffer.search_all(&signature);
        let mut map = IndexMap::new();
        let mut stream = iter(multi_idx);

        while let Some(idx) = stream.next().await {
            let entry = ZipEntry::try_from(&buffer[idx..])?;
            let name = entry.file_name.clone();
            map.extend([(name, entry)]);
        }
        Ok(map)
    }

    async fn read_zipfile(&mut self) -> ZipResult<ZipFile>
    where
        Self: AsyncRead + Unpin,
    {
        let signature = self.read_u32_le().await?;

        if signature != ZipFile::SIGNATURE {
            Err(ZipError::SignatureNotFound(
                "Local File Header Signature not found".into(),
            ))?
        }
        let mut buffer = [0; ZipFile::SIZE];
        self.read(&mut buffer).await?;
        let datetime: [u8; 4] = buffer[6..10].try_into()?;

        let version_needed = u16::from_le_bytes(buffer[0..2].try_into()?);
        let flags = GeneralPurposeFlag::from(u16::from_le_bytes(buffer[2..4].try_into()?));
        let compression = Compression::try_from(u16::from_le_bytes(buffer[4..6].try_into()?))?;
        let last_mod_datetime = ZipDateTime::try_from(datetime)?;
        let crc32 = u32::from_le_bytes(buffer[10..14].try_into()?);
        let compressed_size = u32::from_le_bytes(buffer[14..18].try_into()?);
        let uncompressed_size = u32::from_le_bytes(buffer[18..22].try_into()?);

        let file_name = {
            let length = u16::from_le_bytes(buffer[22..24].try_into()?) as u64;
            let mut path = ZipPath::new();
            self.take(length).read_to_zip_path(&mut path).await?;
            path
        };
        let extra_field: Option<Vec<ExtraField>> = {
            let length = u16::from_le_bytes(buffer[24..26].try_into()?) as u64;

            if length > 0 {
                let mut buffer = Vec::new();
                self.take(length).read_to_end(&mut buffer).await?;
                None
            } else {
                None
            }
        };
        let mut data = Vec::with_capacity(compressed_size as usize);

        self.take(compressed_size as u64)
            .read_to_end(&mut data)
            .await?;

        Ok(ZipFile {
            version_needed,
            flags,
            compression,
            last_mod_datetime,
            crc32,
            compressed_size,
            uncompressed_size,
            file_name,
            extra_field,
            data,
        })
    }
}

impl<R> ZipAsyncReadExt for R where R: AsyncRead + Unpin {}

impl ZipSpecs for Zip64CentralDirectoryEndRecord {
    const SIZE: usize = 52;
    const SIGNATURE: u32 = 0x06064b50;
}

impl ZipSpecs for Zip32CentralDirectoryEndRecord {
    const SIZE: usize = 18;
    const SIGNATURE: u32 = 0x06054b50;
}

impl ZipSpecs for Zip64CentralDirectoryEndLocator {
    const SIZE: usize = 16;
    const SIGNATURE: u32 = 0x07064b50;
}

impl ZipSpecs for ZipEntry {
    const SIZE: usize = 42;
    const SIGNATURE: u32 = 0x02014b50;
}

impl ZipSpecs for ZipFile {
    const SIZE: usize = 26;
    const SIGNATURE: u32 = 0x04034b50;
}

impl Zip32CentralDirectoryEndRecord {
    const MAX_SIZE: usize = SIGNATURE_LENGTH as usize + (Self::SIZE as usize + u16::MAX as usize);
}
