use std::array::TryFromSliceError;
use std::convert::Infallible;
use std::io;

pub type ZipResult<T> = Result<T, ZipError>;

#[derive(Debug)]
pub enum ZipError {
    AttributeCompatibilityNotSupported,
    CompressionNotSupported,
    FeatureNotSupported(Box<str>),
    InvalidArchive(Box<str>),
    IO(io::Error),
    MissingAttribute,
    SignatureNotFound(Box<str>),
    SliceArray(TryFromSliceError),
    Infallible(Infallible),
    ZipUnicodeCommentExtraFieldInfoIncomplete,
    ZipUnicodePathInfoExtraFieldIncomplete,
}

impl From<io::Error> for ZipError {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
}
impl From<TryFromSliceError> for ZipError {
    fn from(value: TryFromSliceError) -> Self {
        Self::SliceArray(value)
    }
}

impl From<Infallible> for ZipError {
    fn from(value: Infallible) -> Self {
        Self::Infallible(value)
    }
}
