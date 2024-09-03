use {
    crate::{
        datetime::ZipDateTime,
        error::{ZipError, ZipResult},
        path::ZipPath,
        specs::{compression::Compression, extra_field::ExtraField, GeneralPurposeFlag, ZipSpecs},
        ZipFile,
    },
    smol::{
        io::{AsyncRead, AsyncReadExt, AsyncSeek, SeekFrom},
        ready,
    },
    std::{io, pin, task},
};

pub(crate) trait ZipPollReadExt {
    fn poll_read_u32_le(
        self: pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<u32, io::Error>>
    where
        Self: AsyncRead,
    {
        let mut buffer = [0; 4];
        match ready!(self.poll_read(cx, &mut buffer)) {
            Ok(_) => task::Poll::Ready(Ok(u32::from_le_bytes(buffer))),
            Err(err) => task::Poll::Ready(Err(err)),
        }
    }

    fn poll_read_zipfile(
        mut self: pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<ZipResult<ZipFile>>
    where
        Self: AsyncRead + AsyncSeek,
    {
        match ready!(self.as_mut().poll_read_u32_le(cx)) {
            Ok(signature) => {
                if signature != ZipFile::SIGNATURE {
                    return task::Poll::Ready(Err(ZipError::SignatureNotFound(
                        "Local File Header Signature not found".into(),
                    )));
                }
                let mut buffer = [0; ZipFile::SIZE];
                match ready!(self.as_mut().poll_read(cx, &mut buffer)) {
                    Ok(_) => {
                        let datetime: [u8; 4] = buffer[6..10].try_into().unwrap();

                        let version_needed = u16::from_le_bytes(buffer[0..2].try_into().unwrap());
                        let flags = GeneralPurposeFlag::from(u16::from_le_bytes(
                            buffer[2..4].try_into().unwrap(),
                        ));
                        let compression = Compression::try_from(u16::from_le_bytes(
                            buffer[4..6].try_into().unwrap(),
                        ))
                        .unwrap();
                        let last_mod_datetime = ZipDateTime::try_from(datetime).unwrap();
                        let crc32 = u32::from_le_bytes(buffer[10..14].try_into().unwrap());
                        let compressed_size =
                            u32::from_le_bytes(buffer[14..18].try_into().unwrap());
                        let uncompressed_size =
                            u32::from_le_bytes(buffer[18..22].try_into().unwrap());
                        let file_name = {
                            let length = u16::from_le_bytes(buffer[22..24].try_into().unwrap());
                            let mut buffer = Vec::with_capacity(length as usize);
                            let mut taker = self.as_mut().take(length as u64);
                            let mut reader = pin::Pin::new(&mut taker);
                            let mut i_buffer = [0; 2048];
                            loop {
                                let slice = &mut i_buffer[..];
                                match ready!(reader.as_mut().poll_read(cx, slice)) {
                                    Ok(read) => {
                                        if read != 0 {
                                            buffer.extend_from_slice(slice)
                                        } else {
                                            break;
                                        }
                                    }
                                    Err(err) => return task::Poll::Ready(Err(ZipError::from(err))),
                                }
                            }
                            let string = String::from_utf8(buffer);
                            match string {
                                Ok(string) => ZipPath::from(string),
                                Err(err) => {
                                    return task::Poll::Ready(Err(ZipError::from(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        err,
                                    ))))
                                }
                            }
                        };
                        let extra_field: Option<Vec<ExtraField>> = {
                            let length = u16::from_le_bytes(buffer[24..26].try_into().unwrap());
                            match ready!(self
                                .as_mut()
                                .poll_seek(cx, SeekFrom::Current(length as i64)))
                            {
                                Ok(_) => None,
                                Err(err) => return task::Poll::Ready(Err(ZipError::from(err))),
                            }
                        };

                        let mut data = Vec::with_capacity(compressed_size as usize);

                        let mut taker = self.take(compressed_size as u64);
                        let mut reader = pin::Pin::new(&mut taker);
                        let mut i_buffer = [0; 2048];
                        loop {
                            let slice = &mut i_buffer[..];
                            match ready!(reader.as_mut().poll_read(cx, slice)) {
                                Ok(read) => {
                                    if read != 0 {
                                        data.extend_from_slice(slice)
                                    } else {
                                        break;
                                    }
                                }
                                Err(err) => return task::Poll::Ready(Err(ZipError::from(err))),
                            }
                        }
                        task::Poll::Ready(Ok(ZipFile {
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
                        }))
                    }
                    Err(err) => task::Poll::Ready(Err(ZipError::from(err))),
                }
            }
            Err(err) => task::Poll::Ready(Err(ZipError::from(err))),
        }
    }
}

impl<T> ZipPollReadExt for T
where
    T: AsyncRead,
    for<'a> pin::Pin<&'a mut T>: AsyncRead,
{
}
