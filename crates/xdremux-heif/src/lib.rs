#![forbid(unsafe_code)]

mod canonical;
mod data_information;
mod direct;
pub mod error;
mod native;
mod orientation;
mod semantic;
mod style_reconstruction;
mod styles;
mod texture;
mod validation;

pub use direct::{
    replace_private_jpeg_gain_map_with_hevc_tiles, DirectHevcGainMap, GainMapChannels,
    GainMapEncodeProfile, GainMapTile,
};
pub use error::{HeifError, Result};
// Canonical portable final-file assembly. It consumes an ordinary source HEIF
// directly, replaces an existing canonical gain-map graph when present, and
// does not depend on an externally generated intermediate graph.
pub use native::IsoGainMapAssembly;
pub use orientation::{primary_coded_geometry, PrimaryCodedGeometry};
pub use semantic::{merge_apple_semantic_auxiliary_heif, transplant_apple_semantic_auxiliary_heif};
pub use style_reconstruction::{
    extract_style_display_mattes, extract_style_reconstruction_resources, primary_image_signature,
    StyleDisplayMatte, StyleReconstructionResources,
};
pub use styles::{assemble_photographic_styles_heif, PhotographicStylesAssembly};
pub use texture::{
    assemble_texture_styles_heif, has_texture_style_metadata, TextureStyleMask,
    TEXTURE_METADATA_URI,
};
pub use validation::{validate_gain_map_structure, GainMapStructure};

pub fn assemble_iso_gain_map_heif(
    source: &[u8],
    assembly: &IsoGainMapAssembly<'_>,
) -> Result<Vec<u8>> {
    let output = canonical::assemble_iso_gain_map_heif(source, assembly)?;
    data_information::ensure_canonical_data_information_box(&output)
}
