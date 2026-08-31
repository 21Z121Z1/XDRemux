#![forbid(unsafe_code)]

pub mod codec;
pub mod cursor;
pub mod error;
pub mod exif;
pub mod fourcc;
pub mod hevc;
pub mod isobmff;
pub mod jpeg;

pub use codec::ChromaSampling;
pub use cursor::{Cursor, Endian};
pub use error::{FormatError, Result};
pub use exif::Orientation;
pub use fourcc::FourCC;
pub use hevc::{parse_hvcc_profile, HevcDecoderConfigurationProfile};
pub use jpeg::{probe_frame_profile as probe_jpeg_frame_profile, JpegComponent, JpegFrameProfile};
