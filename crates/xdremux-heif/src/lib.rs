#![forbid(unsafe_code)]

mod direct;
pub mod error;

pub use direct::{
    replace_private_jpeg_gain_map_with_hevc_tiles, DirectHevcGainMap, GainMapTile,
};
pub use error::{HeifError, Result};
