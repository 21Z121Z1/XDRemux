#![forbid(unsafe_code)]

mod canonical;
mod direct;
pub mod error;
mod native;
mod validation;

pub use canonical::assemble_iso_gain_map_heif;
pub use direct::{
    replace_private_jpeg_gain_map_with_hevc_tiles, DirectHevcGainMap, GainMapChannels,
    GainMapEncodeProfile, GainMapTile,
};
pub use error::{HeifError, Result};
// Canonical portable final-file assembly. It consumes an ordinary source HEIF
// directly, replaces an existing canonical gain-map graph when present, and
// does not depend on a Python/Swift-generated intermediate graph.
pub use native::IsoGainMapAssembly;
pub use validation::{validate_gain_map_structure, GainMapStructure};
