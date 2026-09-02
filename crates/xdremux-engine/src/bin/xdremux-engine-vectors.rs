use xdremux_engine::{
    plan_conversion, AppleFeatureRequest, CapabilityInventory, ConversionAnalysis,
    ConversionRequest, GainMapChannels, GainMapCodec, GainMapCodecLayout, GainMapSourceProfile,
    GainMapStorageProfile, InputProcessingBranch, OperationCapability, OppoCameraTail, SourceHdrMode,
    TmapFormat,
};
use xdremux_format::ChromaSampling;

fn layout(chroma: ChromaSampling, bit_depth: u8) -> GainMapCodecLayout {
    GainMapCodecLayout {
        chroma,
        luma_bit_depth: bit_depth,
        chroma_bit_depth: bit_depth,
    }
}

fn analysis(chroma: Option<ChromaSampling>, bit_depth: u8) -> ConversionAnalysis {
    ConversionAnalysis {
        hdr_mode: SourceHdrMode::Uhdr,
        gain_map: GainMapSourceProfile {
            width: 1024,
            height: 768,
            channels: GainMapChannels::Rgb,
            storage: GainMapStorageProfile {
                codec: GainMapCodec::Jpeg,
                chroma,
                luma_bit_depth: bit_depth,
                chroma_bit_depth: bit_depth,
            },
        },
    }
}

fn capabilities(layouts: impl IntoIterator<Item = GainMapCodecLayout>) -> CapabilityInventory {
    let mut operations = vec![OperationCapability::RasterDecoder(GainMapCodec::Jpeg)];
    operations.extend(
        layouts
            .into_iter()
            .map(OperationCapability::GainMapTileEncoder),
    );
    CapabilityInventory::new(operations)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let direct = capabilities([
        layout(ChromaSampling::Yuv420, 8),
        layout(ChromaSampling::Yuv444, 8),
    ]);
    let plan = plan_conversion(
        &analysis(Some(ChromaSampling::Yuv420), 8),
        ConversionRequest::default(),
        &direct,
    )?;
    println!(
        "preserve-420|requested={:?}|effective={:?}|chroma={:?}|depth={}",
        plan.requested_input_processing_branch,
        plan.effective_input_processing_branch,
        plan.gain_map_target.layout.chroma,
        plan.gain_map_target.layout.luma_bit_depth
    );

    let only_444 = capabilities([layout(ChromaSampling::Yuv444, 8)]);
    let plan = plan_conversion(
        &analysis(Some(ChromaSampling::Yuv422), 8),
        ConversionRequest::default(),
        &only_444,
    )?;
    println!(
        "promote-422|chroma={:?}|depth={}",
        plan.gain_map_target.layout.chroma, plan.gain_map_target.layout.luma_bit_depth
    );

    let request = ConversionRequest {
        input_processing_branch: InputProcessingBranch::Passthrough,
        oppo_camera_tail: OppoCameraTail::Off,
        tmap_format: TmapFormat::Strict,
        ..ConversionRequest::default()
    };
    let plan = plan_conversion(
        &analysis(Some(ChromaSampling::Yuv444), 8),
        request,
        &only_444,
    )?;
    println!(
        "strict-tmap|requested={:?}|effective={:?}",
        plan.requested_input_processing_branch, plan.effective_input_processing_branch
    );

    let only_420 = capabilities([layout(ChromaSampling::Yuv420, 8)]);
    let error = plan_conversion(
        &analysis(Some(ChromaSampling::Yuv444), 8),
        ConversionRequest::default(),
        &only_420,
    )
    .expect_err("444 must never silently downconvert to 420");
    println!("reject-444-to-420|{error}");

    let error = plan_conversion(
        &analysis(Some(ChromaSampling::Yuv444), 10),
        ConversionRequest::default(),
        &only_444,
    )
    .expect_err("10-bit input must never silently downconvert to 8-bit");
    println!("reject-10-to-8|{error}");

    let encoder_only = CapabilityInventory::new([OperationCapability::GainMapTileEncoder(layout(
        ChromaSampling::Yuv444,
        8,
    ))]);
    let error = plan_conversion(
        &analysis(Some(ChromaSampling::Yuv420), 8),
        ConversionRequest::default(),
        &encoder_only,
    )
    .expect_err("source JPEG must require a JPEG raster decoder capability");
    println!("missing-decoder|{error}");

    let styles_request = ConversionRequest {
        apple_features: AppleFeatureRequest {
            photographic_styles: true,
            portrait: false,
        },
        ..ConversionRequest::default()
    };
    let error = plan_conversion(
        &analysis(Some(ChromaSampling::Yuv420), 8),
        styles_request,
        &only_444,
    )
    .expect_err("Photographic Styles request must require its operation adapter");
    println!("missing-styles|{error}");

    Ok(())
}
