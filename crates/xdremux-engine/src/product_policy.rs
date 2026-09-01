use xdremux_format::ChromaSampling;

use crate::{
    validate_source_profile, GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile,
    GainMapEncoderCapabilities, GainMapSourceProfile, OppoCompatibility, PlannerError, Result,
    SourceFamily, SourceHdrMode,
};

pub const fn wants_oppo_compatibility(compatibility: OppoCompatibility) -> bool {
    !matches!(compatibility, OppoCompatibility::Off)
}

/// Canonical source-family detection shared by every front end and runtime.
///
/// UHDR is an X7-family source. LHDR uses the same metadata-version boundary
/// as the current product: version >= 3 is X7, earlier versions are X6.
pub fn detect_source_family(hdr_mode: SourceHdrMode, meta_floats: &[f64]) -> SourceFamily {
    match hdr_mode {
        SourceHdrMode::Uhdr => SourceFamily::X7,
        SourceHdrMode::Lhdr => {
            if meta_floats.first().copied().unwrap_or_default() >= 3.0 {
                SourceFamily::X7
            } else {
                SourceFamily::X6
            }
        }
    }
}

/// Resolve the product output profile rather than merely preserving source JPEG
/// sampling. This is the canonical policy boundary shared by every front end.
///
/// High-spec output keeps monochrome Gain Maps as 4:0:0 and promotes RGB to
/// 4:4:4. OPPO-compatible output is RGB 4:2:0, including LHDR masks whose luma
/// is replicated by the execution layer before encoding.
pub fn resolve_product_gain_map_encode_profile(
    source: GainMapSourceProfile,
    compatibility: OppoCompatibility,
    encoder: &GainMapEncoderCapabilities,
) -> Result<GainMapEncodeProfile> {
    validate_source_profile(source)?;

    let (channels, chroma) = if wants_oppo_compatibility(compatibility) {
        (GainMapChannels::Rgb, ChromaSampling::Yuv420)
    } else {
        match source.channels {
            GainMapChannels::Mono => (GainMapChannels::Mono, ChromaSampling::Mono400),
            GainMapChannels::Rgb => (GainMapChannels::Rgb, ChromaSampling::Yuv444),
        }
    };
    let layout = GainMapCodecLayout {
        chroma,
        luma_bit_depth: source.storage.luma_bit_depth,
        chroma_bit_depth: source.storage.chroma_bit_depth,
    };
    if !encoder.supports(layout) {
        return Err(PlannerError::UnsupportedGainMapLayout(layout));
    }

    Ok(GainMapEncodeProfile {
        width: source.width,
        height: source.height,
        channels,
        layout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GainMapCodec, GainMapStorageProfile};

    fn layout(chroma: ChromaSampling) -> GainMapCodecLayout {
        GainMapCodecLayout {
            chroma,
            luma_bit_depth: 8,
            chroma_bit_depth: 8,
        }
    }

    fn source(channels: GainMapChannels, chroma: ChromaSampling) -> GainMapSourceProfile {
        GainMapSourceProfile {
            width: 640,
            height: 480,
            channels,
            storage: GainMapStorageProfile {
                codec: GainMapCodec::Jpeg,
                chroma: Some(chroma),
                luma_bit_depth: 8,
                chroma_bit_depth: 8,
            },
        }
    }

    fn encoder() -> GainMapEncoderCapabilities {
        GainMapEncoderCapabilities::new([
            layout(ChromaSampling::Mono400),
            layout(ChromaSampling::Yuv420),
            layout(ChromaSampling::Yuv444),
        ])
    }

    #[test]
    fn source_family_policy_matches_current_product_boundary() {
        assert_eq!(
            detect_source_family(SourceHdrMode::Uhdr, &[]),
            SourceFamily::X7
        );
        assert_eq!(
            detect_source_family(SourceHdrMode::Lhdr, &[2.999]),
            SourceFamily::X6
        );
        assert_eq!(
            detect_source_family(SourceHdrMode::Lhdr, &[3.0]),
            SourceFamily::X7
        );
    }

    #[test]
    fn high_spec_rgb_output_is_444_even_when_private_jpeg_is_420() {
        let target = resolve_product_gain_map_encode_profile(
            source(GainMapChannels::Rgb, ChromaSampling::Yuv420),
            OppoCompatibility::Off,
            &encoder(),
        )
        .unwrap();
        assert_eq!(target.channels, GainMapChannels::Rgb);
        assert_eq!(target.layout, layout(ChromaSampling::Yuv444));
    }

    #[test]
    fn high_spec_mono_output_stays_mono400() {
        let target = resolve_product_gain_map_encode_profile(
            source(GainMapChannels::Mono, ChromaSampling::Mono400),
            OppoCompatibility::Off,
            &encoder(),
        )
        .unwrap();
        assert_eq!(target.channels, GainMapChannels::Mono);
        assert_eq!(target.layout, layout(ChromaSampling::Mono400));
    }

    #[test]
    fn oppo_compatibility_requires_rgb420_even_for_lhdr_mono() {
        let target = resolve_product_gain_map_encode_profile(
            source(GainMapChannels::Mono, ChromaSampling::Mono400),
            OppoCompatibility::On,
            &encoder(),
        )
        .unwrap();
        assert_eq!(target.channels, GainMapChannels::Rgb);
        assert_eq!(target.layout, layout(ChromaSampling::Yuv420));
    }

    #[test]
    fn product_policy_fails_closed_when_required_layout_is_unavailable() {
        let only_420 = GainMapEncoderCapabilities::new([layout(ChromaSampling::Yuv420)]);
        assert_eq!(
            resolve_product_gain_map_encode_profile(
                source(GainMapChannels::Rgb, ChromaSampling::Yuv420),
                OppoCompatibility::Off,
                &only_420,
            ),
            Err(PlannerError::UnsupportedGainMapLayout(layout(
                ChromaSampling::Yuv444
            )))
        );
    }
}
