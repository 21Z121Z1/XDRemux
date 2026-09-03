use std::fmt;

use xdremux_format::FormatError;

pub type Result<T> = std::result::Result<T, MetadataError>;

#[derive(Debug)]
pub enum MetadataError {
    Format(FormatError),
    Invalid {
        context: &'static str,
        message: String,
    },
    Overflow {
        context: &'static str,
    },
}

impl MetadataError {
    pub(crate) fn invalid(context: &'static str, message: impl Into<String>) -> Self {
        Self::Invalid {
            context,
            message: message.into(),
        }
    }

    pub(crate) const fn overflow(context: &'static str) -> Self {
        Self::Overflow { context }
    }
}

impl From<FormatError> for MetadataError {
    fn from(value: FormatError) -> Self {
        Self::Format(value)
    }
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(error) => error.fmt(f),
            Self::Invalid { context, message } => write!(f, "invalid {context}: {message}"),
            Self::Overflow { context } => write!(f, "integer overflow while processing {context}"),
        }
    }
}

impl std::error::Error for MetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Format(error) => Some(error),
            Self::Invalid { .. } | Self::Overflow { .. } => None,
        }
    }
}
