use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HdrError {
    context: &'static str,
    detail: String,
}

impl HdrError {
    pub fn invalid(context: &'static str, detail: impl Into<String>) -> Self {
        Self {
            context,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for HdrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.detail)
    }
}

impl std::error::Error for HdrError {}

pub type Result<T> = std::result::Result<T, HdrError>;
