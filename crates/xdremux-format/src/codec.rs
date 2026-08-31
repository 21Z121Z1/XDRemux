use crate::error::{FormatError, Result};

/// Storage chroma sampling for a coded image representation.
///
/// This is deliberately independent from semantic image channels. A three-channel
/// Gain Map may be coded as 4:2:0, 4:2:2, or 4:4:4; `Mono400` is the coded form
/// used for a single-channel image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChromaSampling {
    Mono400,
    Yuv420,
    Yuv422,
    Yuv444,
}

impl ChromaSampling {
    pub const fn hevc_chroma_format_idc(self) -> u8 {
        match self {
            Self::Mono400 => 0,
            Self::Yuv420 => 1,
            Self::Yuv422 => 2,
            Self::Yuv444 => 3,
        }
    }

    pub const fn semantic_channel_count(self) -> u8 {
        match self {
            Self::Mono400 => 1,
            Self::Yuv420 | Self::Yuv422 | Self::Yuv444 => 3,
        }
    }

    pub(crate) fn from_hevc_chroma_format_idc(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Mono400),
            1 => Ok(Self::Yuv420),
            2 => Ok(Self::Yuv422),
            3 => Ok(Self::Yuv444),
            value => Err(FormatError::Unsupported {
                context: "hvcC chroma_format_idc",
                value: u64::from(value),
            }),
        }
    }
}
