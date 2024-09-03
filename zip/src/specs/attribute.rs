use crate::error::ZipError;

macro_rules! mask {
    ($expression:expr, $pattern:expr) => {
        matches!($expression & $pattern, $pattern)
    };
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum AttributeCompatibility {
    MsDos,
    NTFS,
    Unix,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Attributes {
    pub directory: bool,
    pub file: bool,
    pub symbolic: bool,
    pub owner: Permissions,
    pub group: Permissions,
    pub other: Permissions,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl TryFrom<u8> for AttributeCompatibility {
    type Error = ZipError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MsDos),
            3 => Ok(Self::Unix),
            10 => Ok(Self::NTFS),
            _ => Err(ZipError::AttributeCompatibilityNotSupported),
        }
    }
}

impl<'a> From<(u32, &'a AttributeCompatibility)> for Attributes {
    fn from(value: (u32, &'a AttributeCompatibility)) -> Self {
        let (value, attribute) = value;

        match attribute {
            AttributeCompatibility::Unix => {
                let value = value >> 16;
                let directory = mask!(value, 0o040000);
                let file = mask!(value, 0o100000);
                let symbolic = mask!(value, 0o120000);

                let owner = {
                    let read = mask!(value, 0o400);
                    let write = mask!(value, 0o200);
                    let execute = mask!(value, 0o100);

                    Permissions {
                        read,
                        write,
                        execute,
                    }
                };

                let group = {
                    let read = matches!(value & 0o040, 0o040);
                    let write = matches!(value & 0o020, 0o020);
                    let execute = matches!(value & 0o010, 0o010);

                    Permissions {
                        read,
                        write,
                        execute,
                    }
                };

                let other = {
                    let other_permissions = value & 0x007;
                    let execute = matches!(other_permissions & 0x001, 1);
                    let read = matches!(other_permissions & 0x004, 1);
                    let write = matches!(other_permissions & 0x002, 1);

                    Permissions {
                        execute,
                        read,
                        write,
                    }
                };
                Self {
                    directory,
                    symbolic,
                    file,
                    owner,
                    group,
                    other,
                }
            }
            _ => Self::default(),
        }
    }
}

impl From<AttributeCompatibility> for u16 {
    fn from(value: AttributeCompatibility) -> Self {
        match value {
            AttributeCompatibility::MsDos => 0,
            AttributeCompatibility::Unix => 3,
            AttributeCompatibility::NTFS => 10,
        }
    }
}
