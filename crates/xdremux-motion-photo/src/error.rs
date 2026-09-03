use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionPhotoError {
    FileTooSmall,
    XmpTooLarge,
    MalformedXmp,
    UnsupportedVersion(Option<i64>),
    InvalidByteRange,
    InvalidDirectory,
    InvalidItemLength,
    InvalidPrimaryItem,
    InvalidMotionPhotoItem,
    InvalidVideoPayload,
    PayloadTooLarge,
    ArithmeticOverflow,
    MalformedLpex,
}

impl MotionPhotoError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::FileTooSmall => "fileTooSmall",
            Self::XmpTooLarge => "xmpTooLarge",
            Self::MalformedXmp => "malformedXMP",
            Self::UnsupportedVersion(_) => "unsupportedVersion",
            Self::InvalidByteRange => "invalidByteRange",
            Self::InvalidDirectory => "invalidDirectory",
            Self::InvalidItemLength => "invalidItemLength",
            Self::InvalidPrimaryItem => "invalidPrimaryItem",
            Self::InvalidMotionPhotoItem => "invalidMotionPhotoItem",
            Self::InvalidVideoPayload => "invalidVideoPayload",
            Self::PayloadTooLarge => "payloadTooLarge",
            Self::ArithmeticOverflow => "arithmeticOverflow",
            Self::MalformedLpex => "malformedLpex",
        }
    }
}

impl fmt::Display for MotionPhotoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileTooSmall => f.write_str("Motion Photo input is too small"),
            Self::XmpTooLarge => f.write_str("Motion Photo XMP exceeds the safety scan limit"),
            Self::MalformedXmp => f.write_str("Motion Photo XMP is malformed"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported Motion Photo version: {version:?}")
            }
            Self::InvalidByteRange => f.write_str("Motion Photo contains an invalid byte range"),
            Self::InvalidDirectory => f.write_str("Motion Photo container directory is invalid"),
            Self::InvalidItemLength => {
                f.write_str("Motion Photo item length or padding is invalid")
            }
            Self::InvalidPrimaryItem => f.write_str("Motion Photo Primary item is invalid"),
            Self::InvalidMotionPhotoItem => f.write_str("MotionPhoto video item is invalid"),
            Self::InvalidVideoPayload => {
                f.write_str("Motion Photo video payload is not a valid ISO BMFF stream")
            }
            Self::PayloadTooLarge => {
                f.write_str("Motion Photo payload exceeds the configured extraction limit")
            }
            Self::ArithmeticOverflow => {
                f.write_str("Motion Photo byte-range arithmetic overflowed")
            }
            Self::MalformedLpex => f.write_str("OPPO LPEX metadata is malformed"),
        }
    }
}

impl std::error::Error for MotionPhotoError {}

pub type Result<T> = std::result::Result<T, MotionPhotoError>;
