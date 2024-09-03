#![warn(dead_code)]

pub(crate) mod datetime;
pub mod error;
pub mod path;
pub mod read;
pub mod specs;

pub use specs::compression;
use {
    async_compression::futures::bufread::*,
    datetime::ZipDateTime,
    error::{ZipError, ZipResult},
    indexmap::IndexMap,
    path::ZipPath,
    read::ZipAsyncReadExt,
    smol::{io::{AsyncRead, AsyncSeek, AsyncSeekExt, SeekFrom}, stream::Stream},
    specs::{
        compression::{Compression, Decode},
        extra_field::ExtraField,
        GeneralPurposeFlag, ZipEntry,
    },
    std::{ffi::OsStr, ops::Deref, pin::Pin},
};

pub struct ZipArchive<R> {
    comment: Option<String>,
    pub(crate) entries: IndexMap<ZipPath, ZipEntry>,
    pub(crate) reader: R,
}

#[derive(Debug)]
pub struct ZipFile {
    pub(crate) version_needed: u16,
    pub(crate) flags: GeneralPurposeFlag,
    pub compression: Compression,
    pub last_mod_datetime: ZipDateTime,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name: ZipPath,
    pub extra_field: Option<Vec<ExtraField>>,
    pub(crate) data: Vec<u8>,
}

impl Deref for ZipFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.as_slice()
    }
}

impl<R> ZipArchive<R>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    pub async fn new(mut reader: R) -> ZipResult<Self> {
        let eocdr = reader.read_zip_cd_end().await?;
        let entries = reader.read_zip_entry(&eocdr).await?;

        let comment = eocdr.comment;
        Ok(Self {
            reader,
            entries,
            comment,
        })
    }

    pub async fn file_by_name<S>(&mut self, path: S) -> ZipResult<ZipFile>
    where
        S: AsRef<OsStr>,
    {
        let key = ZipPath::from(path.as_ref());
        let entry = match self.entries.get(&key) {
            Some(value) => value,
            None => Err(ZipError::InvalidArchive("Invalid Key".into()))?,
        };

        let offset = entry.file_header_offset as u64;
        self.reader.seek(SeekFrom::Start(offset)).await?;
        let mut file = self.reader.read_zipfile().await?;
        file.file_name.metadata = entry.file_name.metadata.clone();
        Ok(file)
    }

    pub async fn file_by_index(&mut self, index: usize) -> ZipResult<ZipFile> {
        let entry = match self.entries.get_index(index) {
            Some((_name, value)) => value,
            None => Err(ZipError::InvalidArchive("Invalid Index".into()))?,
        };
        let offset = entry.file_header_offset as u64;
        self.reader.seek(SeekFrom::Start(offset)).await?;
        let mut file = self.reader.read_zipfile().await?;
        file.file_name.metadata = entry.file_name.metadata.clone();
        Ok(file)
    }

    pub fn file_names(&self) -> Vec<ZipPath> {
        self.entries
            .iter()
            .map(|(name, _value)| name.clone())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn comment(&self) -> &Option<String> {
        &self.comment
    }

    pub fn stream(&mut self) -> Pin<Box<dyn Stream<Item = ZipResult<ZipFile>> + '_>> {
        Box::pin(async_fn_stream::try_fn_stream(|emitter| async move {
            for (_, entry) in &self.entries {
                let offset = entry.file_header_offset as u64;
                self.reader.seek(SeekFrom::Start(offset)).await?;
                let mut file = self.reader.read_zipfile().await?;
                file.file_name.metadata = entry.file_name.metadata.clone();
                let _ = emitter.emit(file).await;
            }
            Ok(())
        }))
    }
}

impl ZipFile {
    pub async fn extract(self) -> ZipResult<Vec<u8>> {
        match self.compression {
            Compression::Stored => Ok(self.data),
            Compression::Deflate => {
                DeflateDecoder::decode(&*self.data, self.uncompressed_size as usize).await
            }
            Compression::Deflate64 => {
                Deflate64Decoder::decode(&*self.data, self.uncompressed_size as usize).await
            }
            Compression::Bzip2 => {
                BzDecoder::decode(&*self.data, self.uncompressed_size as usize).await
            }
            Compression::Lzma => {
                LzmaDecoder::decode(&*self.data, self.uncompressed_size as usize).await
            }
            Compression::Zstd => {
                ZstdDecoder::decode(&*self.data, self.uncompressed_size as usize).await
            }
            Compression::Xz => {
                XzDecoder::decode(&*self.data, self.uncompressed_size as usize).await
            }
        }
    }

    pub fn is_dir(&self) -> bool {
        self.file_name.is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.file_name.is_file()
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::{compression::Compression, error::ZipResult, ZipArchive},
        smol::{
            fs::{create_dir_all, read_dir, write, File},
            stream::StreamExt,
        },
        std::path::{Path, PathBuf},
    };

    async fn recursive_read<P>(path: P) -> ZipResult<Vec<PathBuf>>
    where
        P: AsRef<Path>,
    {
        let root_path = path.as_ref();
        let mut paths = Vec::new();
        if root_path.is_dir() {
            let mut dir = read_dir(root_path).await?;

            while let Some(try_entry) = dir.next().await {
                let entry = try_entry?.path();
                let mut entries = if entry.is_dir() {
                    Box::pin(recursive_read(entry)).await?
                } else {
                    Vec::from([entry])
                };
                paths.append(&mut entries);
            }
        } else {
            paths.push(root_path.into())
        }
        Ok(paths)
    }

    #[test]
    fn read_zip_archive() {
        smol::block_on(async {
            for path in recursive_read("./tests").await.unwrap() {
                let mut file = File::open(path).await.unwrap();

                let _zip = ZipArchive::new(&mut file).await.unwrap();
            }
        })
    }

    #[test]
    fn decompress_zip_archive_first_entry() {
        smol::block_on(async {
            let test_path = "./tests";
            let paths = recursive_read(&test_path).await.unwrap();
            for path in paths {
                let mut file = File::open(&path).await.unwrap();

                let mut zip = ZipArchive::new(&mut file).await.unwrap();
                let mut zip = zip.stream();
                while let Some(zip_file) = zip.next().await {
                    let zip_file = zip_file.unwrap();
                    let path = Path::new(test_path).join(&*zip_file.file_name);
                    dbg!(&path);
                    let parent = path.parent().unwrap();
                    if zip_file.is_file() {
                        let compression = zip_file.compression;
                        let compressed_data = Vec::from(&*zip_file);
                        let compressed_size = zip_file.compressed_size as usize;
                        let uncompressed_size = zip_file.uncompressed_size as usize;
                        let uncompressed_data = zip_file.extract().await.unwrap();

                        if !parent.exists() {
                            create_dir_all(&parent).await.unwrap();
                        }

                        write(path, &uncompressed_data).await.unwrap();
                        if compression != Compression::Stored {
                            assert_eq!(compressed_size, compressed_data.len());
                            assert_eq!(uncompressed_size, uncompressed_data.len());
                        }
                        break;
                    }
                }
            }
        })
    }
}
