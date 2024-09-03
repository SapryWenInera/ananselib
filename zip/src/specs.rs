pub(crate) mod attribute;
pub mod compression;
pub mod extra_field;

use {
    crate::{datetime::ZipDateTime, ZipError, ZipPath, ZipResult},
    attribute::{AttributeCompatibility, Attributes},
    compression::Compression,
    extra_field::ExtraField,
};

pub(crate) const DATA_DESCRIPTOR_SIGNATURE: u32 = 0x8074b50;
pub(crate) const DATA_DESCRIPTOR_LENGTH: u8 = 12;
pub(crate) const SIGNATURE_LENGTH: u8 = 4;

pub(crate) trait ZipSpecs {
    const SIZE: usize;
    const SIGNATURE: u32;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DataDescriptor {
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct GeneralPurposeFlag {
    pub(crate) encrypted: bool,
    pub(crate) data_drescriptor: bool,
    pub(crate) utf8_required: bool,
    pub(crate) central_directory_encrypted: bool,
}

#[derive(Debug)]
pub(crate) struct Zip64CentralDirectoryEndRecord {
    pub(crate) version_made_by: u16,
    pub(crate) version_needed: u16,
    pub(crate) disk_number: u32,
    pub(crate) central_directory_start_disk: u32,
    pub(crate) number_of_entries_in_disk: u64,
    pub(crate) number_of_entries: u64,
    pub(crate) central_directory_size: u64,
    pub(crate) central_directory_offset: u64,
}

#[derive(Debug)]
pub(crate) struct Zip64CentralDirectoryEndLocator {
    pub(crate) number_of_disk_with_zip64_central_directory_end: u32,
    pub(crate) relative_offset: u64,
    pub(crate) number_of_disks: u32,
}

#[derive(Debug)]
pub(crate) struct Zip32CentralDirectoryEndRecord {
    pub(crate) disk_number: u16,
    pub(crate) central_directory_start_disk: u16,
    pub(crate) number_of_entries_in_disk: u16,
    pub(crate) number_of_entries: u16,
    pub(crate) central_directory_size: u32,
    pub(crate) central_directory_offset: u32,
    pub(crate) file_comment_length: u16,
}

#[derive(Debug)]
pub(crate) struct ZipCentralDirectoryEndRecord {
    pub(crate) _version_made_by: Option<u16>,
    pub(crate) version_needed: Option<u16>,
    pub(crate) disk_number: u32,
    pub(crate) central_directory_start_disk: u32,
    pub(crate) number_of_entries_in_disk: u64,
    pub(crate) number_of_entries: u64,
    pub(crate) central_directory_size: u64,
    pub(crate) central_directory_offset: u64,
    pub(crate) comment: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ZipEntry {
    pub(crate) version_made_by: AttributeCompatibility,
    pub(crate) version_needed: u16,
    pub(crate) flags: GeneralPurposeFlag,
    pub(crate) compression: Compression,
    pub(crate) last_mod_datetime: ZipDateTime,
    pub(crate) crc32: u32,
    pub(crate) compressed_size: u32,
    pub(crate) uncompressed_size: u32,
    pub(crate) disk_start: u16,
    pub(crate) internal_attribute: u16,
    pub(crate) external_attribute: Attributes,
    pub(crate) file_header_offset: u32,
    pub comment: Option<String>,
    pub extra_field: Option<Vec<ExtraField>>,
    pub file_name: ZipPath,
}

impl From<u16> for GeneralPurposeFlag {
    fn from(value: u16) -> Self {
        let encrypted = matches!(value & 0x1, 1);
        let data_drescriptor = matches!((value & 0x8) >> 3, 1);
        let utf8_required = matches!((value & 0x800) >> 11, 1);
        let central_directory_encrypted = matches!((value & 0x2000) >> 13, 1);

        Self {
            encrypted,
            data_drescriptor,
            utf8_required,
            central_directory_encrypted,
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for ZipEntry {
    type Error = ZipError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let version_made_by = AttributeCompatibility::try_from(value[5])?;
        let version_needed = u16::from_le_bytes(value[6..8].try_into()?);
        let flags = GeneralPurposeFlag::from(u16::from_le_bytes(value[8..10].try_into()?));
        let compression = Compression::try_from(u16::from_le_bytes(value[10..12].try_into()?))?;
        let last_mod_datetime =
            ZipDateTime::try_from(TryInto::<[u8; 4]>::try_into(&value[12..16])?)?;
        let crc32 = u32::from_le_bytes(value[16..20].try_into()?);
        let compressed_size = u32::from_le_bytes(value[20..24].try_into()?);
        let uncompressed_size = u32::from_le_bytes(value[24..28].try_into()?);
        let filename_length = u16::from_le_bytes(value[28..30].try_into()?) as usize;
        let extra_field_length = u16::from_le_bytes(value[30..32].try_into()?) as usize;
        let comment_length = u16::from_le_bytes(value[32..34].try_into()?) as usize;
        let file_name = {
            let end_idx = 46 + filename_length;
            let buffer = Vec::from(&value[46..end_idx]);
            let string = String::from_utf8(buffer)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
            ZipPath::from(string)
        };
        let extra_field: Option<Vec<ExtraField>> = None;
        let comment = {
            let start_idx = 46 + filename_length + extra_field_length;
            let end_idx = start_idx + comment_length;
            if comment_length > 0 {
                let buffer = Vec::from(&value[start_idx..end_idx]);
                Some(
                    String::from_utf8(buffer)
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?,
                )
            } else {
                None
            }
        };
        let disk_start = u16::from_le_bytes(value[34..36].try_into()?);
        let internal_attribute = u16::from_le_bytes(value[36..38].try_into()?);
        let external_attribute = Attributes::try_from((
            u32::from_le_bytes(value[38..42].try_into()?),
            &version_made_by,
        ))?;
        let file_header_offset = u32::from_le_bytes(value[42..46].try_into()?);

        Ok(ZipEntry {
            version_made_by,
            version_needed,
            flags,
            compression,
            last_mod_datetime,
            crc32,
            compressed_size,
            uncompressed_size,
            file_name,
            extra_field,
            comment,
            disk_start,
            internal_attribute,
            external_attribute,
            file_header_offset,
        })
    }
}

impl TryFrom<[u8; 16]> for Zip64CentralDirectoryEndLocator {
    type Error = ZipError;

    fn try_from(value: [u8; 16]) -> Result<Self, Self::Error> {
        let number_of_disk_with_zip64_central_directory_end =
            u32::from_le_bytes(value[0..4].try_into()?);
        let relative_offset = u64::from_le_bytes(value[4..12].try_into()?);
        let number_of_disks = u32::from_le_bytes(value[12..16].try_into()?);
        Ok(Self {
            number_of_disk_with_zip64_central_directory_end,
            relative_offset,
            number_of_disks,
        })
    }
}

impl TryFrom<[u8; 18]> for Zip32CentralDirectoryEndRecord {
    type Error = ZipError;

    fn try_from(value: [u8; 18]) -> Result<Self, Self::Error> {
        let central_directory_size = u32::from_le_bytes(value[8..12].try_into()?);
        let disk_number = u16::from_le_bytes(value[0..2].try_into()?);
        let central_directory_start_disk = u16::from_le_bytes(value[2..4].try_into()?);
        let number_of_entries_in_disk = u16::from_le_bytes(value[4..6].try_into()?);
        let number_of_entries = u16::from_le_bytes(value[6..8].try_into()?);
        let central_directory_offset = u32::from_le_bytes(value[12..16].try_into()?);
        let file_comment_length = u16::from_le_bytes(value[16..18].try_into()?);
        Ok(Self {
            disk_number,
            central_directory_start_disk,
            number_of_entries_in_disk,
            number_of_entries,
            central_directory_size,
            central_directory_offset,
            file_comment_length,
        })
    }
}

impl TryFrom<[u8; 52]> for Zip64CentralDirectoryEndRecord {
    type Error = ZipError;

    fn try_from(value: [u8; 52]) -> Result<Self, Self::Error> {
        let version_made_by = u16::from_le_bytes(value[8..10].try_into()?);
        let version_needed = u16::from_le_bytes(value[10..12].try_into()?);
        let disk_number = u32::from_le_bytes(value[12..16].try_into()?);
        let central_directory_start_disk = u32::from_le_bytes(value[16..20].try_into()?);
        let number_of_entries_in_disk = u64::from_le_bytes(value[20..28].try_into()?);
        let number_of_entries = u64::from_le_bytes(value[28..36].try_into()?);
        let central_directory_size = u64::from_le_bytes(value[36..44].try_into()?);
        let central_directory_offset = u64::from_le_bytes(value[44..52].try_into()?);
        Ok(Self {
            version_made_by,
            version_needed,
            disk_number,
            central_directory_start_disk,
            number_of_entries_in_disk,
            number_of_entries,
            central_directory_size,
            central_directory_offset,
        })
    }
}
