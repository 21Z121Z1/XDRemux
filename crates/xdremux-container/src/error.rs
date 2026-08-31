use std::fmt;

pub type Result<T> = std::result::Result<T, ContainerError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerError {
    ManifestNotFound,
    QtiMarkerNotFound,
    Invalid {
        context: &'static str,
        detail: String,
    },
}

impl ContainerError {
    pub fn invalid(context: &'static str, detail: impl Into<String>) -> Self {
        Self::Invalid {
            context,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ContainerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestNotFound => formatter.write_str("manifest not found"),
            Self::QtiMarkerNotFound => formatter.write_str("QTI/jxrs marker not found"),
            Self::Invalid { context, detail } => write!(formatter, "{context}: {detail}"),
        }
    }
}

impl std::error::Error for ContainerError {}
