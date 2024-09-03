use crate::error::{ZipError, ZipResult};
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExtraField {
    Zip64ExtendedInfo(Zip64ExtendedInfoExtraField),
    ZipUnicodeCommentInfo(ZipUnicodeCommentInfoExtraField),
    ZipUnicodePathInfo(ZipUnicodePathInfoExtraField),
    Unknown(UnknownExtraField),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ZipUnicodeCommentInfoExtraField {
    V1 { crc32: u32, unicode: Box<[u8]> },
    Unknown { version: u8, data: Box<[u8]> },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ZipUnicodePathInfoExtraField {
    V1 { crc32: u32, unicode: Box<[u8]> },
    Unknown { version: u8, data: Box<[u8]> },
}

pub trait ExtraFieldAsBytes {
    fn as_bytes(&self) -> Vec<u8>;

    fn count_bytes(&self) -> u64;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct HeaderId(pub u16);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownExtraField {
    pub header_id: HeaderId,
    pub data_size: u16,
    pub content: Box<[u8]>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Zip64ExtendedInfoExtraField {
    pub header_id: HeaderId,
    pub uncompressed_size: Option<u64>,
    pub compressed_size: Option<u64>,
    pub relative_header_offset: Option<u64>,
    pub disk_start_number: Option<u32>,
}

impl ExtraFieldAsBytes for &[ExtraField] {
    fn as_bytes(&self) -> Vec<u8> {
        self.iter()
            .flat_map(|field| field.as_bytes().into_iter())
            .collect()
    }

    fn count_bytes(&self) -> u64 {
        self.iter().map(|field| field.count_bytes()).sum()
    }
}

impl ExtraFieldAsBytes for ExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Unknown(value) => value.as_bytes(),
            Self::ZipUnicodeCommentInfo(value) => value.as_bytes(),
            Self::ZipUnicodePathInfo(value) => value.as_bytes(),
            Self::Zip64ExtendedInfo(value) => value.as_bytes(),
        }
    }

    fn count_bytes(&self) -> u64 {
        match self {
            Self::Unknown(value) => value.count_bytes(),
            Self::ZipUnicodeCommentInfo(value) => value.count_bytes(),
            Self::ZipUnicodePathInfo(value) => value.count_bytes(),
            Self::Zip64ExtendedInfo(value) => value.count_bytes(),
        }
    }
}

impl ExtraFieldAsBytes for UnknownExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        let header_id: &[u8] = &self.header_id.0.to_le_bytes();
        let data_size: &[u8] = &self.data_size.to_le_bytes();
        let content = &*self.content;
        [header_id, data_size, content]
            .iter()
            .flat_map(|section| section.into_iter())
            .map(|b| *b)
            .collect()
    }

    fn count_bytes(&self) -> u64 {
        4 + self.content.len() as u64
    }
}

impl ExtraFieldAsBytes for ZipUnicodeCommentInfoExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        let header_id: &[u8] = &HeaderId::ZIP_UNICODE_COMMENT_INFO_EXTRA_FIELD
            .0
            .to_le_bytes();
        match self {
            Self::V1 { crc32, unicode } => {
                let data_size: &[u8] = &(5 + unicode.len() as u16).to_le_bytes();
                let crc32: &[u8] = &crc32.to_le_bytes();
                let unicode = unicode.deref();
                [header_id, data_size, &[1], crc32, unicode]
                    .iter()
                    .flat_map(|f| f.iter())
                    .map(|b| b.to_owned())
                    .collect()
            }
            Self::Unknown { version, data } => {
                let data_size: &[u8] = &(1 + data.len() as u16).to_le_bytes();
                let data = data.deref();
                [data_size, &[*version], data]
                    .iter()
                    .flat_map(|f| f.iter())
                    .map(|b| *b)
                    .collect()
            }
        }
    }

    fn count_bytes(&self) -> u64 {
        match self {
            Self::V1 { unicode, .. } => 9 + unicode.len() as u64,
            Self::Unknown { data, .. } => 5 + data.len() as u64,
        }
    }
}

impl ExtraFieldAsBytes for ZipUnicodePathInfoExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        let header_id = &HeaderId::ZIP_UNICODE_PATH_INFO_EXTRA_FIELD.0.to_le_bytes();
        match self {
            Self::V1 { crc32, unicode } => {
                let data_size: &[u8] = &(5 + unicode.len() as u16).to_le_bytes();
                let crc32: &[u8] = &crc32.to_le_bytes();
                let unicode = unicode.deref();
                [header_id, data_size, &[1], crc32, unicode]
                    .iter()
                    .flat_map(|f| f.iter())
                    .map(|b| b.to_owned())
                    .collect()
            }
            Self::Unknown { version, data } => {
                let data_size: &[u8] = &(1 + data.len() as u16).to_le_bytes();
                let data = data.deref();
                [data_size, &[*version], data]
                    .iter()
                    .flat_map(|f| f.iter())
                    .map(|b| *b)
                    .collect()
            }
        }
    }

    fn count_bytes(&self) -> u64 {
        match self {
            Self::V1 { unicode, .. } => 9 + unicode.len() as u64,
            Self::Unknown { data, .. } => 5 + data.len() as u64,
        }
    }
}

impl ExtraFieldAsBytes for Zip64ExtendedInfoExtraField {
    fn as_bytes(&self) -> Vec<u8> {
        let header_id: &[u8] = &self.header_id.0.to_le_bytes();
        let content: &[u8] = &(self.content_size() as u16).to_le_bytes();
        let uncompressed_size: &[u8] = match self.uncompressed_size {
            Some(value) => &value.to_le_bytes(),
            None => &[0u8],
        };
        let compressed_size: &[u8] = match self.compressed_size {
            Some(value) => &value.to_le_bytes(),
            None => &[0u8],
        };
        let relative_header_offset: &[u8] = match self.relative_header_offset {
            Some(value) => &value.to_le_bytes(),
            None => &[0u8],
        };
        let disk_start_number: &[u8] = match self.disk_start_number {
            Some(value) => &value.to_le_bytes(),
            None => &[0u8],
        };
        [
            header_id,
            content,
            uncompressed_size,
            compressed_size,
            relative_header_offset,
            disk_start_number,
        ]
        .iter()
        .flat_map(|f| f.iter())
        .map(|b| *b)
        .collect()
    }

    fn count_bytes(&self) -> u64 {
        4 + self.content_size()
    }
}

impl HeaderId {
    pub const ZIP64_EXTENDED_INFO_EXTRA_FIELD: Self = Self(0x0001);
    pub const ZIP_UNICODE_COMMENT_INFO_EXTRA_FIELD: Self = Self(0x6375);
    pub const ZIP_UNICODE_PATH_INFO_EXTRA_FIELD: Self = Self(0x7075);
}

impl Zip64ExtendedInfoExtraField {
    pub fn new() -> Self {
        Self {
            header_id: HeaderId::ZIP64_EXTENDED_INFO_EXTRA_FIELD,
            uncompressed_size: None,
            compressed_size: None,
            relative_header_offset: None,
            disk_start_number: None,
        }
    }

    pub fn sizes(&mut self, compressed_size: u64, uncompressed_size: u64) {
        self.compressed_size = Some(compressed_size);
        self.uncompressed_size = Some(uncompressed_size);
    }

    pub fn content_size(&self) -> u64 {
        self.uncompressed_size.map(|_| 8).unwrap_or_default()
            + self.compressed_size.map(|_| 8).unwrap_or_default()
            + self.relative_header_offset.map(|_| 8).unwrap_or_default()
            + self.disk_start_number.map(|_| 8).unwrap_or_default()
    }

    pub fn from_bytes<A>(
        header_id: HeaderId,
        data: A,
        uncompressed_size: u32,
        compressed_size: u32,
    ) -> ZipResult<Self>
    where
        A: AsRef<[u8]>,
    {
        let data = data.as_ref();
        let mut current_index = 0;
        let uncompressed_size =
            if uncompressed_size == u32::MAX && data.len() >= current_index + 8 {
                let val = Some(u64::from_le_bytes(
                    data[current_index..current_index + 8].try_into().unwrap(),
                ));
                current_index += 8;
                val
            } else {
                None
            };
        let compressed_size =
            if compressed_size == u32::MAX && data.len() >= current_index + 8 {
                let val = Some(u64::from_le_bytes(
                    data[current_index..current_index + 8].try_into().unwrap(),
                ));
                current_index += 8;
                val
            } else {
                None
            };
        let relative_header_offset = if data.len() >= current_index + 8 {
            let val = Some(u64::from_le_bytes(
                data[current_index..current_index + 8].try_into().unwrap(),
            ));
            current_index += 8;
            val
        } else {
            None
        };
        let disk_start_number = if data.len() >= current_index + 4 {
            Some(u32::from_le_bytes(
                data[current_index..current_index + 4].try_into().unwrap(),
            ))
        } else {
            None
        };

        Ok(Self {
            header_id,
            uncompressed_size,
            compressed_size,
            relative_header_offset,
            disk_start_number,
        })
    }
}

impl ZipUnicodeCommentInfoExtraField {
    pub fn from_bytes<A>(_header_id: HeaderId, data_size: u16, data: A) -> ZipResult<Self>
    where
        A: AsRef<[u8]>,
    {
        let data = data.as_ref();

        if data.is_empty() {
            return Err(ZipError::ZipUnicodeCommentExtraFieldInfoIncomplete);
        }

        let version = data[0];
        match version {
            1 => {
                if data.len() > 5 {
                    return Err(ZipError::ZipUnicodeCommentExtraFieldInfoIncomplete);
                }
                let crc32 = u32::from_le_bytes(data[1..5].try_into()?);
                let unicode = Box::from(&data[5..data_size as usize]);
                Ok(Self::V1 { crc32, unicode })
            }
            _ => Ok(Self::Unknown {
                version,
                data: Box::from(&data[1..data_size as usize]),
            }),
        }
    }
}

impl ZipUnicodePathInfoExtraField {
    pub fn from_bytes<A>(_header_id: HeaderId, data_size: u16, data: A) -> ZipResult<Self>
    where
        A: AsRef<[u8]>,
    {
        let data = data.as_ref();

        if data.is_empty() {
            return Err(ZipError::ZipUnicodePathInfoExtraFieldIncomplete);
        }

        let version = data[0];
        match version {
            1 => {
                if data.len() < 5 {
                    return Err(ZipError::ZipUnicodeCommentExtraFieldInfoIncomplete);
                }

                let crc32 = u32::from_le_bytes(data[1..5].try_into()?);
                let unicode = Box::from(&data[5..data_size as usize]);
                Ok(Self::V1 { crc32, unicode })
            }
            _ => Ok(Self::Unknown {
                version,
                data: Box::from(&data[1..data_size as usize]),
            }),
        }
    }
}

impl ExtraField {
    pub fn from_bytes<A>(
        header_id: HeaderId,
        data_size: u16,
        data: A,
        uncompressed_size: u32,
        compressed_size: u32,
    ) -> ZipResult<Self>
    where
        A: AsRef<[u8]>,
    {
        match header_id {
            HeaderId::ZIP64_EXTENDED_INFO_EXTRA_FIELD => Ok(Self::Zip64ExtendedInfo(
                Zip64ExtendedInfoExtraField::from_bytes(
                    header_id,
                    data,
                    uncompressed_size,
                    compressed_size,
                )?,
            )),
            HeaderId::ZIP_UNICODE_COMMENT_INFO_EXTRA_FIELD => Ok(Self::ZipUnicodeCommentInfo(
                ZipUnicodeCommentInfoExtraField::from_bytes(header_id, data_size, data)?,
            )),
            HeaderId::ZIP_UNICODE_PATH_INFO_EXTRA_FIELD => Ok(Self::ZipUnicodePathInfo(
                ZipUnicodePathInfoExtraField::from_bytes(header_id, data_size, data)?,
            )),
            _ => Ok(ExtraField::Unknown(UnknownExtraField {
                header_id,
                data_size,
                content: Box::from(data.as_ref()),
            })),
        }
    }
}
