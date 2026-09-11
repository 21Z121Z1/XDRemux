use xdremux_format::ChromaSampling;

use crate::{
    validate_source_profile, GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile,
    GainMapEncoderCapabilities, GainMapSourceProfile, OutputIntent, PlannerError, Result,
};

/// Resolve the product output profile rather than preserving private source JPEG
/// sampling mechanically.
///
/// Standard output keeps monochrome Gain Maps as 4:0:0 and promotes RGB to
/// 4:4:4. OPPO Gallery output is RGB 4:2:0, including LHDR masks whose luma is
/// replicated by the runtime before encoding.
pub fn resolve_product_gain_map_encode_profile(
    source: GainMapSourceProfile,
    output: OutputIntent,
    encoder: &GainMapEncoderCapabilities,
) -> Result<GainMapEncodeProfile> {
    validate_source_profile(source)?;

    let (channels, chroma) = match output {
        OutputIntent::Standard => match source.channels {
            GainMapChannels::Mono => (GainMapChannels::Mono, ChromaSampling::Mono400),
            GainMapChannels::Rgb => (GainMapChannels::Rgb, ChromaSampling::Yuv444),
        },
        OutputIntent::OppoGallery => (GainMapChannels::Rgb, ChromaSampling::Yuv420),
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
    fn standard_rgb_output_is_444_even_when_private_jpeg_is_420() {
        let target = resolve_product_gain_map_encode_profile(
            source(GainMapChannels::Rgb, ChromaSampling::Yuv420),
            OutputIntent::Standard,
            &encoder(),
        )
        .unwrap();
        assert_eq!(target.channels, GainMapChannels::Rgb);
        assert_eq!(target.layout, layout(ChromaSampling::Yuv444));
    }

    #[test]
    fn standard_mono_output_stays_mono400() {
        let target = resolve_product_gain_map_encode_profile(
            source(GainMapChannels::Mono, ChromaSampling::Mono400),
            OutputIntent::Standard,
            &encoder(),
        )
        .unwrap();
        assert_eq!(target.channels, GainMapChannels::Mono);
        assert_eq!(target.layout, layout(ChromaSampling::Mono400));
    }

    #[test]
    fn oppo_gallery_output_requires_rgb420_even_for_lhdr_mono() {
        let target = resolve_product_gain_map_encode_profile(
            source(GainMapChannels::Mono, ChromaSampling::Mono400),
            OutputIntent::OppoGallery,
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
                OutputIntent::Standard,
                &only_420,
            ),
            Err(PlannerError::UnsupportedGainMapLayout(layout(
                ChromaSampling::Yuv444
            )))
        );
    }
}
