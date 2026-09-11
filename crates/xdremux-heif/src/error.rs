use std::error::Error;
use std::fmt;

use xdremux_format::FormatError;

pub type Result<T> = std::result::Result<T, HeifError>;

#[derive(Debug)]
pub enum HeifError {
    Invalid(String),
    Format(FormatError),
}

impl HeifError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}

impl fmt::Display for HeifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "invalid HEIF: {message}"),
            Self::Format(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl Error for HeifError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Invalid(_) => None,
            Self::Format(error) => Some(error),
        }
    }
}

impl From<FormatError> for HeifError {
    fn from(value: FormatError) -> Self {
        Self::Format(value)
    }
}
