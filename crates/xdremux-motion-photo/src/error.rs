use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionPhotoError {
    InvalidByteRange,
    InvalidDirectory,
    InvalidItemLength,
    InvalidVideoPayload,
    ArithmeticOverflow,
    MalformedLpex,
}

impl fmt::Display for MotionPhotoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidByteRange => "Motion Photo contains an invalid byte range",
            Self::InvalidDirectory => "Motion Photo container directory is invalid",
            Self::InvalidItemLength => "Motion Photo item length or padding is invalid",
            Self::InvalidVideoPayload => "Motion Photo video payload is not a valid ISO BMFF stream",
            Self::ArithmeticOverflow => "Motion Photo byte-range arithmetic overflowed",
            Self::MalformedLpex => "OPPO LPEX metadata is malformed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for MotionPhotoError {}

pub type Result<T> = std::result::Result<T, MotionPhotoError>;
