use crate::error::{ZipError, ZipResult};

#[derive(Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct ZipDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl TryFrom<[u8; 4]> for ZipDateTime {
    type Error = ZipError;

    fn try_from(value: [u8; 4]) -> ZipResult<Self> {
        let time = u16::from_le_bytes(value[0..2].try_into()?);
        let date = u16::from_le_bytes(value[2..4].try_into()?);

        let year = ((date & 0xFE00) >> 9) + 1980;
        let month = ((date & 0x1E0) >> 6) as u8;
        let day = (date & 0x1F) as u8;
        let hour = ((time & 0xF800) >> 11) as u8;
        let minute = ((time & 0x7E6) >> 5) as u8;
        let second = ((time & 0xF1) << 1) as u8;

        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }
}
