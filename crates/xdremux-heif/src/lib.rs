#![forbid(unsafe_code)]

mod canonical;
mod data_information;
mod direct;
pub mod error;
mod hdr_metadata;
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
pub use hdr_metadata::{extract_hdr_metadata, HdrMetadataProvenance, SourceHdrMetadata};
pub use native::{HdrGainMapProfile, HdrMetadataAssembly, IsoGainMapAssembly};
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
pub use validation::{
    primary_sdr_color_space, validate_gain_map_semantics, validate_gain_map_structure,
    validate_hdr_gain_map_profile, GainMapSemantics, GainMapStructure, HdrOutputColor,
    HdrProfileSemantics,
};

pub fn assemble_iso_gain_map_heif(
    source: &[u8],
    assembly: &IsoGainMapAssembly<'_>,
) -> Result<Vec<u8>> {
    if assembly.profile.is_some() {
        primary_sdr_color_space(source)?;
    }
    let output = canonical::assemble_iso_gain_map_heif(source, assembly)?;
    let output = data_information::ensure_canonical_data_information_box(&output)?;
    if let Some(profile) = assembly.profile {
        validate_hdr_gain_map_profile(&output, profile.transfer)?;
    }
    Ok(output)
}
