use xdremux_engine::OppoCompatibility as EngineCompatibility;
use xdremux_metadata::{
    patch_oppo_user_comment_in_heif, OppoCompatibility as MetadataCompatibility,
};

use crate::{Result, RuntimeError};

pub(crate) const fn metadata_compatibility(
    compatibility: EngineCompatibility,
) -> MetadataCompatibility {
    match compatibility {
        EngineCompatibility::Off => MetadataCompatibility::Off,
        EngineCompatibility::Auto => MetadataCompatibility::Auto,
        EngineCompatibility::On => MetadataCompatibility::On,
        EngineCompatibility::Tail => MetadataCompatibility::Tail,
        EngineCompatibility::Iso => MetadataCompatibility::Iso,
        EngineCompatibility::IsoNoLocal => MetadataCompatibility::IsoNoLocal,
        EngineCompatibility::IsoGraph => MetadataCompatibility::IsoGraph,
    }
}

pub(crate) fn patch_source_metadata(
    source: &[u8],
    compatibility: EngineCompatibility,
) -> Result<Vec<u8>> {
    patch_oppo_user_comment_in_heif(source, metadata_compatibility(compatibility))
        .map_err(|error| RuntimeError::external("OPPO compatibility metadata", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_and_metadata_modes_are_exhaustively_aligned() {
        let cases = [
            (EngineCompatibility::Off, MetadataCompatibility::Off),
            (EngineCompatibility::Auto, MetadataCompatibility::Auto),
            (EngineCompatibility::On, MetadataCompatibility::On),
            (EngineCompatibility::Tail, MetadataCompatibility::Tail),
            (EngineCompatibility::Iso, MetadataCompatibility::Iso),
            (
                EngineCompatibility::IsoNoLocal,
                MetadataCompatibility::IsoNoLocal,
            ),
            (
                EngineCompatibility::IsoGraph,
                MetadataCompatibility::IsoGraph,
            ),
        ];
        for (engine, metadata) in cases {
            assert_eq!(metadata_compatibility(engine), metadata);
        }
    }
}
