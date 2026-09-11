use std::fmt;

pub type Result<T> = std::result::Result<T, FormatError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    UnexpectedEof {
        context: &'static str,
        offset: usize,
        needed: usize,
        end: usize,
    },
    InvalidData {
        context: &'static str,
        message: String,
    },
    Overflow {
        context: &'static str,
    },
    Unsupported {
        context: &'static str,
        value: u64,
    },
}

impl FormatError {
    pub(crate) fn invalid(context: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidData {
            context,
            message: message.into(),
        }
    }

    pub(crate) const fn overflow(context: &'static str) -> Self {
        Self::Overflow { context }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof {
                context,
                offset,
                needed,
                end,
            } => write!(
                f,
                "truncated {context}: need {needed} bytes at offset {offset}, bounded region ends at {end}"
            ),
            Self::InvalidData { context, message } => write!(f, "invalid {context}: {message}"),
            Self::Overflow { context } => write!(f, "integer overflow while parsing {context}"),
            Self::Unsupported { context, value } => {
                write!(f, "unsupported {context} value {value}")
            }
        }
    }
}

impl std::error::Error for FormatError {}
