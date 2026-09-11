use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    InvalidInput(String),
    Unsupported(String),
    LibHeif(String),
    Format(String),
    InconsistentEncoderConfiguration(String),
}

impl CodecError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(message.into())
    }

    pub(crate) fn libheif(error: impl fmt::Display) -> Self {
        Self::LibHeif(error.to_string())
    }

    pub(crate) fn format(error: impl fmt::Display) -> Self {
        Self::Format(error.to_string())
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "invalid codec input: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported codec operation: {message}"),
            Self::LibHeif(message) => write!(f, "libheif operation failed: {message}"),
            Self::Format(message) => write!(f, "encoded HEIF is malformed: {message}"),
            Self::InconsistentEncoderConfiguration(message) => {
                write!(f, "inconsistent HEVC encoder configuration: {message}")
            }
        }
    }
}

impl std::error::Error for CodecError {}

pub type Result<T> = std::result::Result<T, CodecError>;
