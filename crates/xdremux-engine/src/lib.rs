#![forbid(unsafe_code)]

mod apple;
mod apple_auxiliary;
mod apple_photographic_styles;
mod apple_portrait;
mod apple_portrait_geometry;
mod capabilities;
mod execution;
mod product_policy;
mod source_profile;

use std::collections::BTreeSet;

use xdremux_format::ChromaSampling;

pub use apple::{
    fuse_apple_portrait_hair_mask, fuse_apple_portrait_person_mask, AppleGainMapFacts,
    AppleImageAuxiliaryFacts, AppleL8Mask, AppleL8MaskError, ApplePortraitHairFusion,
    ApplePortraitPersonFusion, AppleSemanticRole, APPLE_PORTRAIT_SEMANTIC_ROLES,
};
pub use apple_auxiliary::{
    build_apple_portrait_disparity_payload, build_apple_portrait_effects_matte_payload,
    build_apple_semantic_matte_payload, AppleAuxiliaryDescription, AppleAuxiliaryError,
    AppleAuxiliaryKind, AppleAuxiliaryPayload, AppleMetadataNamespace, AppleMetadataTag,
    AppleMetadataValue, APPLE_DEPTH_BLUR_EFFECT_NAMESPACE, APPLE_DEPTH_DATA_NAMESPACE,
    APPLE_PORTRAIT_EFFECTS_MATTE_NAMESPACE, APPLE_PORTRAIT_LIGHTING_EFFECT_NAMESPACE,
    APPLE_SEMANTIC_SEGMENTATION_MATTE_NAMESPACE,
};
pub use apple_photographic_styles::{
    apple_style_data_apply_coefficient_deltas, apple_style_data_from_coefficient_deltas,
    apple_style_data_from_parameters, apple_style_data_sha256, apple_style_face_exposure_boost,
    apple_style_fit_global_polynomial, apple_style_identity_data, apple_style_light_map,
    apple_style_polynomial_basis, apple_style_solve_refinement_update,
    resolve_apple_style_scene_type, validate_apple_style_data, AppleStyleDataError,
    AppleStyleDataFacts, AppleStyleLightMapRequest, AppleStyleSampledJacobian, AppleStyleScalarRow,
    AppleStyleSceneClass, AppleStyleSceneDecision, AppleStyleSceneScores,
    APPLE_STYLE_BLOCK_VALUE_COUNT, APPLE_STYLE_BYTE_COUNT, APPLE_STYLE_CHANNEL_COUNT,
    APPLE_STYLE_GRID_HEIGHT, APPLE_STYLE_GRID_WIDTH, APPLE_STYLE_IDENTITY_SHA256,
    APPLE_STYLE_LIGHT_MAP_BYTE_COUNT, APPLE_STYLE_LIGHT_MAP_SIDE, APPLE_STYLE_PLANE_COUNT,
    APPLE_STYLE_POLYNOMIAL_COUNT, APPLE_STYLE_REFINEMENT_EPSILON,
    APPLE_STYLE_REFINEMENT_MAX_PIXELS, APPLE_STYLE_REFINEMENT_PARAMETER_COUNT,
    APPLE_STYLE_TILE_COUNT,
};
pub use apple_portrait::{
    build_apple_portrait_rendering_parameters, AppleRendDocument, AppleRendError, AppleRendRecord,
    AppleXhlrbControlOutput, APPLE_XHLRB_DYNAMIC_RECORD_IDS,
};
pub use apple_portrait_geometry::{
    apple_portrait_lens_profile, build_apple_portrait_disparity,
    derive_apple_portrait_camera_calibration, focus_disparity,
    optical_equivalent_focal_length_from_lens_model, resolve_apple_portrait_base_orientation,
    transform_apple_portrait_focus_region, ApplePortraitCameraCalibration,
    ApplePortraitCaptureFacts, ApplePortraitDisparity, ApplePortraitGeometryError,
    ApplePortraitImageGeometry, ApplePortraitLensProfile, ApplePortraitLensProfileId,
    ApplePortraitNormalizedFocusRegion, APPLE_PORTRAIT_FUSION_2X_PROFILE,
    APPLE_PORTRAIT_MAIN_1X_PROFILE, APPLE_PORTRAIT_TELE_3X_PROFILE,
    APPLE_PORTRAIT_TETRAPRISM_5X_PROFILE,
};
pub use capabilities::{
    CapabilityInventory, GainMapTileEncoder, OperationCapability, RasterDecoder,
    RasterDecoderCapabilities,
};
pub use execution::{
    execute_conversion, ArtifactBuilder, ArtifactPublisher, ArtifactValidator, ExecutionError,
    ExecutionReceipt, ExecutionResult, ExecutionStage,
};
pub use product_policy::resolve_product_gain_map_encode_profile;
pub use source_profile::{
    gain_map_channels_from_count, gain_map_source_profile_from_hevc,
    gain_map_source_profile_from_jpeg,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputIntent {
    #[default]
    Standard,
    OppoGallery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppleFeatureRequest {
    pub photographic_styles: bool,
    pub portrait: bool,
}

impl AppleFeatureRequest {
    pub const fn any(self) -> bool {
        self.photographic_styles || self.portrait
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConversionRequest {
    pub output: OutputIntent,
    pub apple_features: AppleFeatureRequest,
}

impl ConversionRequest {
    /// Product intent for output that remains recognizable by OPPO Gallery.
    ///
    /// Source layout, routing bits, camera-tail handling and codec policy are
    /// implementation details resolved by the engine/runtime from this intent.
    pub const fn oppo_gallery_compatible() -> Self {
        Self {
            output: OutputIntent::OppoGallery,
            apple_features: AppleFeatureRequest {
                photographic_styles: false,
                portrait: false,
            },
        }
    }

    pub const fn requests_oppo_gallery_compatibility(self) -> bool {
        matches!(self.output, OutputIntent::OppoGallery)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GainMapChannels {
    Mono,
    Rgb,
}

impl GainMapChannels {
    pub const fn semantic_channel_count(self) -> u8 {
        match self {
            Self::Mono => 1,
            Self::Rgb => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GainMapCodec {
    Jpeg,
    Hevc,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainMapStorageProfile {
    pub codec: GainMapCodec,
    pub chroma: Option<ChromaSampling>,
    pub luma_bit_depth: u8,
    pub chroma_bit_depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainMapSourceProfile {
    pub width: u32,
    pub height: u32,
    pub channels: GainMapChannels,
    pub storage: GainMapStorageProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GainMapCodecLayout {
    pub chroma: ChromaSampling,
    pub luma_bit_depth: u8,
    pub chroma_bit_depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainMapEncodeProfile {
    pub width: u32,
    pub height: u32,
    pub channels: GainMapChannels,
    pub layout: GainMapCodecLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GainMapEncoderCapabilities {
    layouts: BTreeSet<GainMapCodecLayout>,
}

impl GainMapEncoderCapabilities {
    pub fn new(layouts: impl IntoIterator<Item = GainMapCodecLayout>) -> Self {
        Self {
            layouts: layouts.into_iter().collect(),
        }
    }

    pub fn supports(&self, layout: GainMapCodecLayout) -> bool {
        self.layouts.contains(&layout)
    }

    pub fn iter(&self) -> impl Iterator<Item = GainMapCodecLayout> + '_ {
        self.layouts.iter().copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionAnalysis {
    pub gain_map: GainMapSourceProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionPlan {
    pub output: OutputIntent,
    pub gain_map_target: GainMapEncodeProfile,
    pub required_capabilities: Vec<OperationCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannerError {
    InvalidGainMapProfile(&'static str),
    UnsupportedGainMapLayout(GainMapCodecLayout),
    IncompatibleProductIntents(&'static str),
    MissingOperationCapabilities(Vec<OperationCapability>),
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGainMapProfile(message) => {
                write!(f, "invalid Gain Map profile: {message}")
            }
            Self::UnsupportedGainMapLayout(layout) => {
                write!(
                    f,
                    "no encoder capability preserves Gain Map layout {layout:?}"
                )
            }
            Self::IncompatibleProductIntents(message) => {
                write!(f, "incompatible product intents: {message}")
            }
            Self::MissingOperationCapabilities(capabilities) => {
                f.write_str("missing required operation capabilities: ")?;
                for (index, capability) in capabilities.iter().enumerate() {
                    if index != 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{capability:?}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for PlannerError {}

pub type Result<T> = std::result::Result<T, PlannerError>;

fn validate_source_profile(source: GainMapSourceProfile) -> Result<()> {
    if source.width == 0 || source.height == 0 {
        return Err(PlannerError::InvalidGainMapProfile(
            "dimensions must be non-zero",
        ));
    }
    if source.storage.luma_bit_depth == 0 || source.storage.chroma_bit_depth == 0 {
        return Err(PlannerError::InvalidGainMapProfile(
            "bit depth must be non-zero",
        ));
    }
    match (source.channels, source.storage.chroma) {
        (GainMapChannels::Mono, Some(ChromaSampling::Mono400)) => {}
        (GainMapChannels::Mono, Some(_)) => {
            return Err(PlannerError::InvalidGainMapProfile(
                "mono semantics cannot use a color chroma layout",
            ));
        }
        (GainMapChannels::Rgb, Some(ChromaSampling::Mono400)) => {
            return Err(PlannerError::InvalidGainMapProfile(
                "RGB semantics cannot use monochrome storage",
            ));
        }
        _ => {}
    }
    Ok(())
}

fn requested_layout(source: GainMapSourceProfile) -> GainMapCodecLayout {
    let chroma = match (source.channels, source.storage.chroma) {
        (GainMapChannels::Mono, _) => ChromaSampling::Mono400,
        (GainMapChannels::Rgb, Some(chroma)) => chroma,
        (GainMapChannels::Rgb, None) => ChromaSampling::Yuv444,
    };
    GainMapCodecLayout {
        chroma,
        luma_bit_depth: source.storage.luma_bit_depth,
        chroma_bit_depth: source.storage.chroma_bit_depth,
    }
}

pub fn resolve_gain_map_encode_profile(
    source: GainMapSourceProfile,
    encoder: &GainMapEncoderCapabilities,
) -> Result<GainMapEncodeProfile> {
    validate_source_profile(source)?;
    let requested = requested_layout(source);
    let selected = if encoder.supports(requested) {
        requested
    } else if requested.chroma == ChromaSampling::Yuv422 {
        let promoted = GainMapCodecLayout {
            chroma: ChromaSampling::Yuv444,
            ..requested
        };
        if encoder.supports(promoted) {
            promoted
        } else {
            return Err(PlannerError::UnsupportedGainMapLayout(requested));
        }
    } else {
        return Err(PlannerError::UnsupportedGainMapLayout(requested));
    };

    Ok(GainMapEncodeProfile {
        width: source.width,
        height: source.height,
        channels: source.channels,
        layout: selected,
    })
}

pub fn plan_conversion(
    analysis: &ConversionAnalysis,
    request: ConversionRequest,
    capabilities: &CapabilityInventory,
) -> Result<ConversionPlan> {
    if request.requests_oppo_gallery_compatibility() && request.apple_features.any() {
        return Err(PlannerError::IncompatibleProductIntents(
            "Apple features cannot be combined with OPPO Gallery compatibility",
        ));
    }

    let gain_map_encoder = capabilities.gain_map_encoder_capabilities();
    let gain_map_target = resolve_product_gain_map_encode_profile(
        analysis.gain_map,
        request.output,
        &gain_map_encoder,
    )?;

    let mut required_capabilities = vec![
        OperationCapability::RasterDecoder(analysis.gain_map.storage.codec),
        OperationCapability::GainMapTileEncoder(gain_map_target.layout),
    ];
    if request.apple_features.photographic_styles {
        required_capabilities.push(OperationCapability::PhotographicStylesAdapter);
    }
    if request.apple_features.portrait {
        required_capabilities.push(OperationCapability::PortraitAdapter);
    }

    let missing = capabilities.missing(required_capabilities.iter().copied());
    if !missing.is_empty() {
        return Err(PlannerError::MissingOperationCapabilities(missing));
    }

    Ok(ConversionPlan {
        output: request.output,
        gain_map_target,
        required_capabilities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(chroma: ChromaSampling, bit_depth: u8) -> GainMapCodecLayout {
        GainMapCodecLayout {
            chroma,
            luma_bit_depth: bit_depth,
            chroma_bit_depth: bit_depth,
        }
    }

    fn source(
        channels: GainMapChannels,
        chroma: Option<ChromaSampling>,
        bit_depth: u8,
    ) -> GainMapSourceProfile {
        GainMapSourceProfile {
            width: 1024,
            height: 768,
            channels,
            storage: GainMapStorageProfile {
                codec: GainMapCodec::Jpeg,
                chroma,
                luma_bit_depth: bit_depth,
                chroma_bit_depth: bit_depth,
            },
        }
    }

    fn capability_inventory(
        layouts: impl IntoIterator<Item = GainMapCodecLayout>,
    ) -> CapabilityInventory {
        let mut operations = vec![OperationCapability::RasterDecoder(GainMapCodec::Jpeg)];
        operations.extend(
            layouts
                .into_iter()
                .map(OperationCapability::GainMapTileEncoder),
        );
        CapabilityInventory::new(operations)
    }

    #[test]
    fn request_exposes_product_intent_only() {
        assert_eq!(ConversionRequest::default().output, OutputIntent::Standard);
        assert!(!ConversionRequest::default().requests_oppo_gallery_compatibility());

        let request = ConversionRequest::oppo_gallery_compatible();
        assert_eq!(request.output, OutputIntent::OppoGallery);
        assert!(request.requests_oppo_gallery_compatibility());
        assert_eq!(request.apple_features, AppleFeatureRequest::default());
        assert!(!request.apple_features.any());
    }

    #[test]
    fn adaptive_policy_preserves_known_mono_420_and_444_layouts() {
        let capabilities = GainMapEncoderCapabilities::new([
            layout(ChromaSampling::Mono400, 8),
            layout(ChromaSampling::Yuv420, 8),
            layout(ChromaSampling::Yuv444, 8),
        ]);
        for (channels, chroma) in [
            (GainMapChannels::Mono, ChromaSampling::Mono400),
            (GainMapChannels::Rgb, ChromaSampling::Yuv420),
            (GainMapChannels::Rgb, ChromaSampling::Yuv444),
        ] {
            let profile =
                resolve_gain_map_encode_profile(source(channels, Some(chroma), 8), &capabilities)
                    .unwrap();
            assert_eq!(profile.layout.chroma, chroma);
            assert_eq!(profile.channels, channels);
        }
    }

    #[test]
    fn rgb_422_may_promote_losslessly_to_444_when_backend_lacks_422() {
        let capabilities = GainMapEncoderCapabilities::new([layout(ChromaSampling::Yuv444, 8)]);
        let profile = resolve_gain_map_encode_profile(
            source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv422), 8),
            &capabilities,
        )
        .unwrap();
        assert_eq!(profile.layout, layout(ChromaSampling::Yuv444, 8));
    }

    #[test]
    fn unknown_rgb_sampling_uses_444_as_information_safe_target() {
        let capabilities = GainMapEncoderCapabilities::new([layout(ChromaSampling::Yuv444, 8)]);
        let profile =
            resolve_gain_map_encode_profile(source(GainMapChannels::Rgb, None, 8), &capabilities)
                .unwrap();
        assert_eq!(profile.layout.chroma, ChromaSampling::Yuv444);
    }

    #[test]
    fn planner_never_silently_downconverts_bit_depth_or_444_chroma() {
        let eight_bit_444 = GainMapEncoderCapabilities::new([layout(ChromaSampling::Yuv444, 8)]);
        assert_eq!(
            resolve_gain_map_encode_profile(
                source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv444), 10),
                &eight_bit_444,
            ),
            Err(PlannerError::UnsupportedGainMapLayout(layout(
                ChromaSampling::Yuv444,
                10,
            )))
        );

        let only_420 = GainMapEncoderCapabilities::new([layout(ChromaSampling::Yuv420, 8)]);
        assert!(resolve_gain_map_encode_profile(
            source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv444), 8),
            &only_420,
        )
        .is_err());
    }

    #[test]
    fn planner_rejects_apple_features_with_oppo_gallery_compatibility() {
        let target = layout(ChromaSampling::Yuv420, 8);
        let capabilities = CapabilityInventory::new([
            OperationCapability::RasterDecoder(GainMapCodec::Jpeg),
            OperationCapability::GainMapTileEncoder(target),
            OperationCapability::PhotographicStylesAdapter,
            OperationCapability::PortraitAdapter,
        ]);
        let analysis = ConversionAnalysis {
            gain_map: source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv420), 8),
        };

        for apple_features in [
            AppleFeatureRequest {
                photographic_styles: true,
                portrait: false,
            },
            AppleFeatureRequest {
                photographic_styles: false,
                portrait: true,
            },
            AppleFeatureRequest {
                photographic_styles: true,
                portrait: true,
            },
        ] {
            let request = ConversionRequest {
                output: OutputIntent::OppoGallery,
                apple_features,
            };
            assert_eq!(
                plan_conversion(&analysis, request, &capabilities),
                Err(PlannerError::IncompatibleProductIntents(
                    "Apple features cannot be combined with OPPO Gallery compatibility",
                ))
            );
        }
    }

    #[test]
    fn planner_requires_decoder_for_the_probed_source_codec() {
        let target = layout(ChromaSampling::Yuv444, 8);
        let capabilities =
            CapabilityInventory::new([OperationCapability::GainMapTileEncoder(target)]);
        let analysis = ConversionAnalysis {
            gain_map: source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv420), 8),
        };

        assert_eq!(
            plan_conversion(&analysis, ConversionRequest::default(), &capabilities),
            Err(PlannerError::MissingOperationCapabilities(vec![
                OperationCapability::RasterDecoder(GainMapCodec::Jpeg),
            ]))
        );
    }

    #[test]
    fn planner_rejects_requested_apple_feature_without_its_adapter() {
        let target = layout(ChromaSampling::Yuv444, 8);
        let capabilities = capability_inventory([target]);
        let analysis = ConversionAnalysis {
            gain_map: source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv420), 8),
        };
        let request = ConversionRequest {
            apple_features: AppleFeatureRequest {
                photographic_styles: true,
                portrait: false,
            },
            ..ConversionRequest::default()
        };

        assert_eq!(
            plan_conversion(&analysis, request, &capabilities),
            Err(PlannerError::MissingOperationCapabilities(vec![
                OperationCapability::PhotographicStylesAdapter,
            ]))
        );
    }

    #[test]
    fn plan_contains_only_execution_relevant_decisions() {
        let analysis = ConversionAnalysis {
            gain_map: source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv420), 8),
        };
        let request = ConversionRequest {
            apple_features: AppleFeatureRequest {
                photographic_styles: true,
                portrait: false,
            },
            ..ConversionRequest::default()
        };
        let target = layout(ChromaSampling::Yuv444, 8);
        let capabilities = CapabilityInventory::new([
            OperationCapability::RasterDecoder(GainMapCodec::Jpeg),
            OperationCapability::GainMapTileEncoder(target),
            OperationCapability::PhotographicStylesAdapter,
        ]);
        let plan = plan_conversion(&analysis, request, &capabilities).unwrap();

        assert_eq!(plan.output, OutputIntent::Standard);
        assert_eq!(plan.gain_map_target.layout, target);
        assert_eq!(
            plan.required_capabilities,
            vec![
                OperationCapability::RasterDecoder(GainMapCodec::Jpeg),
                OperationCapability::GainMapTileEncoder(target),
                OperationCapability::PhotographicStylesAdapter,
            ]
        );
    }
}
