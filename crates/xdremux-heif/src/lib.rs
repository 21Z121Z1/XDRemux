#![forbid(unsafe_code)]

mod direct;
pub mod error;
mod native;
mod validation;

pub use direct::{
    replace_private_jpeg_gain_map_with_hevc_tiles, DirectHevcGainMap, GainMapChannels,
    GainMapEncodeProfile, GainMapTile,
};
pub use error::{HeifError, Result};
pub use native::{assemble_iso_gain_map_heif, IsoGainMapAssembly};
pub use validation::{validate_gain_map_structure, GainMapStructure};
