#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use xdremux_format::ChromaSampling;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FamilyPreference {
    #[default]
    Auto,
    X6,
    X7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFamily {
    X6,
    X7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceHdrMode {
    Lhdr,
    Uhdr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputProcessingBranch {
    System,
    SystemDecoded,
    #[default]
    Hybrid,
    Passthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TmapFormat {
    Strict,
    #[default]
    ImageIo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OppoCompatibility {
    Auto,
    Iso,
    IsoNoLocal,
    IsoGraph,
    On,
    Tail,
    #[default]
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OppoCameraTail {
    Off,
    Watermark,
    Compact,
    Preserve,
    PreserveWithoutPortrait,
    PreserveWithoutPortraitOrPrivateHdr,
    PreserveWithoutPrivateUhdr,
    #[default]
    PreserveWithoutPrivateHdr,
    PreserveNoUhdr,
    PreserveNoHdr,
}

impl OppoCameraTail {
    pub const fn forces_hybrid_branch(self) -> bool {
        matches!(
            self,
            Self::Preserve
                | Self::PreserveWithoutPortrait
                | Self::PreserveWithoutPortraitOrPrivateHdr
                | Self::PreserveWithoutPrivateUhdr
                | Self::PreserveWithoutPrivateHdr
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppleFeatureRequest {
    pub photographic_styles: bool,
    pub portrait: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionRequest {
    pub family: FamilyPreference,
    pub oppo_compatibility: OppoCompatibility,
    pub input_processing_branch: InputProcessingBranch,
    pub oppo_camera_tail: OppoCameraTail,
    pub tmap_format: TmapFormat,
    pub apple_features: AppleFeatureRequest,
}

impl Default for ConversionRequest {
    fn default() -> Self {
        Self {
            family: FamilyPreference::Auto,
            oppo_compatibility: OppoCompatibility::Off,
            input_processing_branch: InputProcessingBranch::Hybrid,
            oppo_camera_tail: OppoCameraTail::PreserveWithoutPrivateHdr,
            tmap_format: TmapFormat::ImageIo,
            apple_features: AppleFeatureRequest::default(),
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseStrategy {
    PreserveCompressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerWriter {
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformCapability {
    GainMapEncoder(GainMapCodecLayout),
    ApplePhotographicStyles,
    ApplePortrait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionAnalysis {
    pub source_family: SourceFamily,
    pub hdr_mode: SourceHdrMode,
    pub gain_map: GainMapSourceProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionPlan {
    pub effective_family: SourceFamily,
    pub requested_input_processing_branch: InputProcessingBranch,
    pub effective_input_processing_branch: InputProcessingBranch,
    pub base_strategy: BaseStrategy,
    pub gain_map_target: GainMapEncodeProfile,
    pub container_writer: ContainerWriter,
    pub oppo_compatibility: OppoCompatibility,
    pub oppo_camera_tail: OppoCameraTail,
    pub tmap_format: TmapFormat,
    pub required_capabilities: Vec<PlatformCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannerError {
    InvalidGainMapProfile(&'static str),
    UnsupportedGainMapLayout(GainMapCodecLayout),
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
        }
    }
}

impl std::error::Error for PlannerError {}

pub type Result<T> = std::result::Result<T, PlannerError>;

pub fn resolve_effective_input_processing_branch(
    requested: InputProcessingBranch,
    tail: OppoCameraTail,
    tmap_format: TmapFormat,
) -> InputProcessingBranch {
    if tail.forces_hybrid_branch() || tmap_format == TmapFormat::Strict {
        InputProcessingBranch::Hybrid
    } else {
        requested
    }
}

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
    gain_map_encoder: &GainMapEncoderCapabilities,
) -> Result<ConversionPlan> {
    let effective_family = match request.family {
        FamilyPreference::Auto => analysis.source_family,
        FamilyPreference::X6 => SourceFamily::X6,
        FamilyPreference::X7 => SourceFamily::X7,
    };
    let gain_map_target = resolve_gain_map_encode_profile(analysis.gain_map, gain_map_encoder)?;
    let effective_input_processing_branch = resolve_effective_input_processing_branch(
        request.input_processing_branch,
        request.oppo_camera_tail,
        request.tmap_format,
    );

    let mut required_capabilities =
        vec![PlatformCapability::GainMapEncoder(gain_map_target.layout)];
    if request.apple_features.photographic_styles {
        required_capabilities.push(PlatformCapability::ApplePhotographicStyles);
    }
    if request.apple_features.portrait {
        required_capabilities.push(PlatformCapability::ApplePortrait);
    }

    Ok(ConversionPlan {
        effective_family,
        requested_input_processing_branch: request.input_processing_branch,
        effective_input_processing_branch,
        base_strategy: BaseStrategy::PreserveCompressed,
        gain_map_target,
        container_writer: ContainerWriter::Rust,
        oppo_compatibility: request.oppo_compatibility,
        oppo_camera_tail: request.oppo_camera_tail,
        tmap_format: request.tmap_format,
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

    #[test]
    fn defaults_match_current_swift_product_configuration() {
        let request = ConversionRequest::default();
        assert_eq!(request.family, FamilyPreference::Auto);
        assert_eq!(request.oppo_compatibility, OppoCompatibility::Off);
        assert_eq!(
            request.input_processing_branch,
            InputProcessingBranch::Hybrid
        );
        assert_eq!(
            request.oppo_camera_tail,
            OppoCameraTail::PreserveWithoutPrivateHdr
        );
        assert_eq!(request.tmap_format, TmapFormat::ImageIo);
        assert_eq!(request.apple_features, AppleFeatureRequest::default());
    }

    #[test]
    fn current_swift_tail_and_strict_tmap_rules_force_hybrid() {
        let forcing_tails = [
            OppoCameraTail::Preserve,
            OppoCameraTail::PreserveWithoutPortrait,
            OppoCameraTail::PreserveWithoutPortraitOrPrivateHdr,
            OppoCameraTail::PreserveWithoutPrivateUhdr,
            OppoCameraTail::PreserveWithoutPrivateHdr,
        ];
        for tail in forcing_tails {
            assert_eq!(
                resolve_effective_input_processing_branch(
                    InputProcessingBranch::Passthrough,
                    tail,
                    TmapFormat::ImageIo,
                ),
                InputProcessingBranch::Hybrid
            );
        }
        assert_eq!(
            resolve_effective_input_processing_branch(
                InputProcessingBranch::Passthrough,
                OppoCameraTail::Off,
                TmapFormat::Strict,
            ),
            InputProcessingBranch::Hybrid
        );
        assert_eq!(
            resolve_effective_input_processing_branch(
                InputProcessingBranch::Passthrough,
                OppoCameraTail::Off,
                TmapFormat::ImageIo,
            ),
            InputProcessingBranch::Passthrough
        );
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
    fn full_plan_preserves_base_and_declares_operation_capabilities() {
        let analysis = ConversionAnalysis {
            source_family: SourceFamily::X7,
            hdr_mode: SourceHdrMode::Uhdr,
            gain_map: source(GainMapChannels::Rgb, Some(ChromaSampling::Yuv420), 8),
        };
        let request = ConversionRequest {
            input_processing_branch: InputProcessingBranch::Passthrough,
            oppo_camera_tail: OppoCameraTail::Off,
            apple_features: AppleFeatureRequest {
                photographic_styles: true,
                portrait: false,
            },
            ..ConversionRequest::default()
        };
        let capabilities = GainMapEncoderCapabilities::new([layout(ChromaSampling::Yuv420, 8)]);
        let plan = plan_conversion(&analysis, request, &capabilities).unwrap();

        assert_eq!(plan.effective_family, SourceFamily::X7);
        assert_eq!(plan.base_strategy, BaseStrategy::PreserveCompressed);
        assert_eq!(plan.container_writer, ContainerWriter::Rust);
        assert_eq!(
            plan.effective_input_processing_branch,
            InputProcessingBranch::Passthrough
        );
        assert_eq!(
            plan.gain_map_target.layout,
            layout(ChromaSampling::Yuv420, 8)
        );
        assert_eq!(
            plan.required_capabilities,
            vec![
                PlatformCapability::GainMapEncoder(layout(ChromaSampling::Yuv420, 8)),
                PlatformCapability::ApplePhotographicStyles,
            ]
        );
    }
}
