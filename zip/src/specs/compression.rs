use {
    crate::error::{ZipError, ZipResult},
    async_compression::futures::bufread::*,
    smol::io::{AsyncBufRead, AsyncReadExt},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Compression {
    Stored,
    Deflate,
    Deflate64,
    Bzip2,
    Lzma,
    Zstd,
    Xz,
}

pub(crate) trait Decode<R> {
    async fn decode(data: R, size: usize) -> ZipResult<Vec<u8>>;
}

impl<R> Decode<R> for ZstdDecoder<R>
where
    R: AsyncBufRead + Unpin,
{
    async fn decode(data: R, size: usize) -> ZipResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(size);
        let mut decoder = Self::new(data);
        decoder.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }
}

impl<R> Decode<R> for XzDecoder<R>
where
    R: AsyncBufRead + Unpin,
{
    async fn decode(data: R, size: usize) -> ZipResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(size);
        let mut decoder = Self::new(data);
        decoder.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }
}

impl<R> Decode<R> for LzmaDecoder<R>
where
    R: AsyncBufRead + Unpin,
{
    async fn decode(data: R, size: usize) -> ZipResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(size);
        let mut decoder = Self::new(data);
        decoder.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }
}

impl<R> Decode<R> for BzDecoder<R>
where
    R: AsyncBufRead + Unpin,
{
    async fn decode(data: R, size: usize) -> ZipResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(size);
        let mut decoder = Self::new(data);
        decoder.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }
}

impl<R> Decode<R> for Deflate64Decoder<R>
where
    R: AsyncBufRead + Unpin,
{
    async fn decode(data: R, size: usize) -> ZipResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(size);
        let mut decoder = Self::new(data);
        decoder.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }
}

impl<R> Decode<R> for DeflateDecoder<R>
where
    R: AsyncBufRead + Unpin,
{
    async fn decode(data: R, size: usize) -> ZipResult<Vec<u8>> {
        let mut buffer = Vec::with_capacity(size);
        let mut decoder = Self::new(data);
        decoder.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }
}

impl TryFrom<u16> for Compression {
    type Error = ZipError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Stored),
            8 => Ok(Self::Deflate),
            9 => Ok(Self::Deflate64),
            12 => Ok(Self::Bzip2),
            14 => Ok(Self::Lzma),
            93 => Ok(Self::Zstd),
            95 => Ok(Self::Xz),
            _ => Err(ZipError::CompressionNotSupported),
        }
    }
}

impl From<Compression> for u16 {
    fn from(value: Compression) -> Self {
        match value {
            Compression::Stored => 0,
            Compression::Deflate => 8,
            Compression::Deflate64 => 9,
            Compression::Bzip2 => 12,
            Compression::Lzma => 14,
            Compression::Zstd => 93,
            Compression::Xz => 95,
        }
    }
}
