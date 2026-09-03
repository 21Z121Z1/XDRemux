use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use xdremux_codec::{EncodedHevcResource, HeifRasterDecodeRequest, Raster8, RasterPixelFormat};
use xdremux_engine::{
    apple_style_identity_data, apple_style_identity_global_tone_curve, apple_style_light_map,
    apple_style_property_list, resolve_apple_style_scene_type, AppleL8Mask, AppleStyleDistribution,
    AppleStyleLightMapRequest, AppleStylePropertyListRequest, AppleStyleSceneScores,
    AppleStyleStatistics, APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES,
};

use crate::apple_adapter::{AppleAdapterClient, AppleVideoToolboxMain10Encode};
use crate::{
    publication_parent, AtomicFilePublisher, PhotographicStylesFileReceipt, PortableRuntime,
    Result, RuntimeError,
};
use xdremux_engine::RasterDecoder;
use xdremux_format::{parse_hvcc_profile, ChromaSampling};

const STYLE_DELTA_TILE_SIZE: u32 = 512;
const LINEAR_THUMBNAIL_MAX_SIZE: u32 = 1024;
const LINEAR_THUMBNAIL_QUALITY: f64 = 0.85;
const SEMANTIC_MATTE_MAX_SIZE: u32 = 2016;
const STYLE_DELTA_ROWS_LANDSCAPE: u32 = 5;
const STYLE_DELTA_COLUMNS_LANDSCAPE: u32 = 6;
const STYLE_DELTA_ROWS_PORTRAIT: u32 = 6;
const STYLE_DELTA_COLUMNS_PORTRAIT: u32 = 5;

// Apple ImageIO accepts the Styles auxiliary graph only when these resources
// use the Main10 4:2:0 contract. These bytes are the checked-in neutral Style
// Delta protocol resource already validated by the Swift migration oracle
// (VideoToolbox Main10, 0.5 RGB, keyframe-only). They are a protocol fixture,
// not a photo-specific payload or a product fallback.
const NEUTRAL_STYLE_DELTA_ITEM_PAYLOAD: &[u8] = &[
    0x00, 0x00, 0x00, 0x97, 0x28, 0x01, 0xaf, 0x84, 0x09, 0x95, 0x53, 0x30, 0xee, 0xee, 0xcc, 0xcc,
    0xd0, 0xf5, 0x98, 0x1e, 0x2f, 0x16, 0x75, 0x47, 0x56, 0x83, 0x56, 0x00, 0x01, 0xd3, 0x9b, 0x87,
    0xaf, 0xde, 0x6b, 0x13, 0x30, 0x00, 0x07, 0x84, 0xb8, 0x90, 0x38, 0x5b, 0x19, 0xc0, 0x00, 0x00,
    0x03, 0x01, 0x4b, 0x6a, 0x82, 0x7b, 0xab, 0xcf, 0x50, 0x00, 0x00, 0x24, 0x23, 0xc0, 0x07, 0x98,
    0x1d, 0xa4, 0x00, 0x00, 0x03, 0x01, 0xa8, 0x34, 0x03, 0xc5, 0x70, 0x18, 0x00, 0x00, 0x2a, 0x5e,
    0x80, 0x05, 0x5c, 0xd3, 0xf0, 0x00, 0x00, 0xe4, 0x1e, 0x08, 0x21, 0xf3, 0xe0, 0x00, 0x04, 0x3b,
    0x50, 0x0a, 0x1f, 0x5e, 0x00, 0x00, 0x19, 0xc2, 0x80, 0x0a, 0xa2, 0x00, 0x00, 0x03, 0x00, 0x3a,
    0x57, 0x0d, 0xfe, 0xaf, 0x00, 0x00, 0xad, 0x52, 0x0f, 0xcb, 0x33, 0x00, 0x00, 0xd8, 0xc4, 0x13,
    0x1b, 0x86, 0x00, 0x03, 0xb0, 0xb0, 0x15, 0xfa, 0x86, 0x00, 0x04, 0x61, 0xd0, 0x16, 0x1f, 0xd8,
    0x00, 0x0b, 0x85, 0x40, 0x00, 0x00, 0x03, 0x00, 0x00, 0x04, 0xbc,
];

const NEUTRAL_STYLE_DELTA_HVCC: &[u8] = &[
    0x01, 0x02, 0x20, 0x00, 0x00, 0x00, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5a, 0xf0, 0x00, 0xfc,
    0xfd, 0xfa, 0xfa, 0x00, 0x00, 0x0b, 0x03, 0xa0, 0x00, 0x01, 0x00, 0x18, 0x40, 0x01, 0x0c, 0x01,
    0xff, 0xff, 0x02, 0x20, 0x00, 0x00, 0x03, 0x00, 0xb0, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x00,
    0x5a, 0x17, 0x02, 0x40, 0xa1, 0x00, 0x01, 0x00, 0x23, 0x42, 0x01, 0x01, 0x02, 0x20, 0x00, 0x00,
    0x03, 0x00, 0xb0, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x00, 0x5a, 0xa0, 0x04, 0x02, 0x00, 0x80,
    0x4d, 0x88, 0x17, 0xb9, 0x16, 0x55, 0x35, 0x01, 0x01, 0x06, 0x00, 0x80, 0xa2, 0x00, 0x01, 0x00,
    0x08, 0x44, 0x01, 0xc0, 0x61, 0x61, 0x82, 0x99, 0x20,
];

fn verified_neutral_style_delta() -> Result<(&'static [u8], &'static [u8])> {
    let item_digest = Sha256::digest(NEUTRAL_STYLE_DELTA_ITEM_PAYLOAD);
    let hvcc_digest = Sha256::digest(NEUTRAL_STYLE_DELTA_HVCC);
    let expected_item = [
        0x14, 0xb0, 0x4f, 0xcd, 0xe0, 0x24, 0x76, 0xf2, 0x4f, 0x83, 0xa8, 0x93, 0xd2, 0x45, 0xb4,
        0xd0, 0x67, 0x28, 0x95, 0x4e, 0x8a, 0xd0, 0x04, 0xf4, 0x16, 0xb6, 0xe3, 0xa9, 0x56, 0xeb,
        0xa2, 0x16,
    ];
    let expected_hvcc = [
        0x35, 0xec, 0xc0, 0x04, 0xd0, 0x71, 0x92, 0xf4, 0xe9, 0xc8, 0xa4, 0x4c, 0x0a, 0x9e, 0xdb,
        0x59, 0x85, 0x99, 0xb7, 0xa6, 0xd0, 0xc5, 0x9b, 0x81, 0x65, 0xa5, 0xfb, 0x43, 0x3f, 0x57,
        0x46, 0xa5,
    ];
    if item_digest.as_slice() != expected_item || hvcc_digest.as_slice() != expected_hvcc {
        return Err(RuntimeError::new(
            "Photographic Styles Style Delta",
            "bundled neutral Main10 protocol resource failed integrity validation",
        ));
    }
    Ok((NEUTRAL_STYLE_DELTA_ITEM_PAYLOAD, NEUTRAL_STYLE_DELTA_HVCC))
}

/// Execute the Rust-owned Styles baseline transaction.
///
/// This first product slice is deliberately explicit about its evidence:
/// the key-1 style resource is the verified complete identity payload, while
/// all container, metadata, semantic-sky, and publication policy is already
/// owned by Rust. The Apple adapter supplies only Vision/ImageIO observations
/// and the framework write primitive. A later style-solver milestone can
/// replace `apple_style_identity_data()` with Rust-selected coefficients
/// without changing this transaction or its public CLI intent.
pub(crate) fn convert_file(
    runtime: &PortableRuntime,
    executable: impl AsRef<Path>,
    source: &[u8],
    output: impl AsRef<Path>,
) -> Result<PhotographicStylesFileReceipt> {
    let output = output.as_ref();
    let parent = publication_parent(output);
    if !parent.is_dir() {
        return Err(RuntimeError::new(
            "Photographic Styles publication",
            format!("output parent is not a directory: {}", parent.display()),
        ));
    }

    let staging = tempfile::Builder::new()
        .prefix(".xdremux-styles-")
        .tempdir_in(parent)
        .map_err(|error| RuntimeError::external("Photographic Styles staging", error))?;
    let base_path = staging.path().join("base.heic");
    let semantic_scaffold_path = staging.path().join("semantic-scaffold.heic");

    // Build the standard HDR base in memory so an in-place Styles conversion
    // cannot publish an intermediate file. The request is intentionally
    // feature-neutral: this operation owns the Styles graph separately.
    let base = runtime
        .convert_proxdr_bytes(source, xdremux_engine::ConversionRequest::default(), |_| {})?
        .bytes;
    fs::write(&base_path, &base)
        .map_err(|error| RuntimeError::external("Photographic Styles base staging", error))?;

    let adapter = AppleAdapterClient::new(executable.as_ref().to_path_buf());
    if !adapter.supports_photographic_styles()? {
        return Err(RuntimeError::new(
            "Photographic Styles adapter",
            "the Apple adapter does not advertise Photographic Styles capability",
        ));
    }
    let image_properties = adapter.imageio_image_properties(&base_path)?;
    let storage_orientation = image_properties.orientation.unwrap_or(1);
    let (semantic_width, semantic_height) = semantic_matte_dimensions(
        image_properties.width,
        image_properties.height,
        storage_orientation,
        SEMANTIC_MATTE_MAX_SIZE,
    )?;
    let semantic_masks = adapter.vision_semantic_mattes(
        &base_path,
        &APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES,
        None,
    )?;
    let mut semantic_payloads = Vec::with_capacity(APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES.len());
    for role in APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES {
        let matte = semantic_masks.get(&role).ok_or_else(|| {
            RuntimeError::new(
                "Photographic Styles semantic analysis",
                format!("Apple Vision omitted the requested {role:?} observation"),
            )
        })?;
        let matte = resize_semantic_matte(matte, semantic_width, semantic_height)?;
        let payload = match role {
            xdremux_engine::AppleSemanticRole::Person => {
                xdremux_engine::build_apple_portrait_effects_matte_payload(
                    matte.width,
                    matte.height,
                    matte.pixels.clone(),
                )
                .map_err(|error| {
                    RuntimeError::external("Photographic Styles person matte", error)
                })?
            }
            _ => xdremux_engine::build_apple_semantic_matte_payload(
                role,
                matte.width,
                matte.height,
                matte.pixels.clone(),
            )
            .map_err(|error| RuntimeError::external("Photographic Styles semantic matte", error))?,
        };
        semantic_payloads.push(payload);
    }

    adapter.imageio_write_auxiliary(&base_path, &semantic_scaffold_path, &semantic_payloads)?;
    if let Ok(debug_path) = std::env::var("XDREMUX_DEBUG_SEMANTIC_SCAFFOLD") {
        fs::copy(&semantic_scaffold_path, &debug_path).map_err(|error| {
            RuntimeError::external("Photographic Styles semantic scaffold debug", error)
        })?;
    }
    let semantic_scaffold = fs::read(&semantic_scaffold_path).map_err(|error| {
        RuntimeError::external("Photographic Styles semantic scaffold read", error)
    })?;
    let semantic_base = xdremux_heif::transplant_apple_semantic_auxiliary_heif(
        &base,
        &semantic_scaffold,
        APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES.len(),
    )
    .map_err(|error| RuntimeError::external("Photographic Styles semantic graph", error))?;
    if let Ok(debug_path) = std::env::var("XDREMUX_DEBUG_SEMANTIC_BASE") {
        fs::write(&debug_path, &semantic_base).map_err(|error| {
            RuntimeError::external("Photographic Styles semantic graph debug", error)
        })?;
    }
    let primary = runtime
        .heif
        .extract_primary_hevc_resource(&semantic_base)
        .map_err(|error| RuntimeError::external("Photographic Styles primary resource", error))?;
    let raster = runtime
        .heif
        .decode_raster(&HeifRasterDecodeRequest {
            data: semantic_base.clone(),
            format: RasterPixelFormat::Rgb8,
        })
        .map_err(|error| RuntimeError::external("Photographic Styles source raster", error))?;
    let luma = normalized_luma(&raster)?;
    let linear_thumbnail_raster = downsample_raster(&raster, LINEAR_THUMBNAIL_MAX_SIZE)?;
    let linear_thumbnail = encode_linear_thumbnail(
        &adapter,
        staging.path(),
        &linear_thumbnail_raster,
        LINEAR_THUMBNAIL_QUALITY,
    )?;

    let style_data = apple_style_identity_data();
    let global_tone_curve = apple_style_identity_global_tone_curve();
    let statistics = style_statistics(&luma)?;
    let tone_light_map = apple_style_light_map(&AppleStyleLightMapRequest {
        luma: &luma,
        width: usize::try_from(raster.width).map_err(|_| {
            RuntimeError::new("Photographic Styles light map", "width exceeds usize")
        })?,
        height: usize::try_from(raster.height).map_err(|_| {
            RuntimeError::new("Photographic Styles light map", "height exceeds usize")
        })?,
        value_scale: 1.0,
        value_offset: 0.0,
        output_minimum: 0.0,
        output_maximum: 1.0,
        storage_orientation: u8::try_from(storage_orientation).map_err(|_| {
            RuntimeError::new(
                "Photographic Styles light map",
                "ImageIO orientation exceeds the portable wire range",
            )
        })?,
    })
    .map_err(|error| RuntimeError::external("Photographic Styles tone light map", error))?;
    let linear_light_map = tone_light_map.clone();
    let scene = resolve_apple_style_scene_type(AppleStyleSceneScores {
        food: 0.0,
        sunset: 0.0,
        indoor: 0.0,
        outdoor: 0.0,
    })
    .map_err(|error| RuntimeError::external("Photographic Styles scene policy", error))?;
    let style_properties = apple_style_property_list(&AppleStylePropertyListRequest {
        style_data: &style_data,
        global_tone_curve: &global_tone_curve,
        baseline_exposure: 1.0,
        scene_type: scene.scene_type,
        statistics: &statistics,
        people_ratio: 0.0,
        person_masks_valid_hint: -1.0,
        skin_ratio: 0.0,
        tone_light_map: &tone_light_map,
        linear_light_map: &linear_light_map,
        base_gain: 1.0,
        linear_gain: 4.0,
        original_range_min: 0.0,
        original_range_max: 1.0,
        face_exposure_boost: 1.0,
    })
    .map_err(|error| RuntimeError::external("Photographic Styles metadata", error))?;

    // Validate the exact Rust-owned key-1 resource at the private consumer
    // boundary before assembling the final file. The adapter cannot choose
    // style data or alter the property-list policy.
    adapter.semantic_style_properties_facts(&style_properties, &style_data)?;

    let (style_delta_payload, style_delta_hvcc) = verified_neutral_style_delta()?;

    let (style_delta_grid_width, style_delta_grid_height, style_delta_rows, style_delta_columns) =
        style_grid_size(primary.width, primary.height);
    let assembly = crate::PhotographicStylesAssembly {
        style_property_list: &style_properties,
        style_delta_hvcc,
        style_delta_tile_payload: style_delta_payload,
        style_delta_tile_width: STYLE_DELTA_TILE_SIZE,
        style_delta_tile_height: STYLE_DELTA_TILE_SIZE,
        style_delta_grid_width,
        style_delta_grid_height,
        style_delta_rows,
        style_delta_columns,
        linear_thumbnail_hvcc: &linear_thumbnail.hvcc,
        linear_thumbnail_payload: &linear_thumbnail.payload,
        linear_thumbnail_width: linear_thumbnail.width,
        linear_thumbnail_height: linear_thumbnail.height,
    };
    let assembled = xdremux_heif::assemble_photographic_styles_heif(&semantic_base, &assembly)
        .map_err(|error| RuntimeError::external("Rust Photographic Styles graph", error))?;
    xdremux_heif::validate_gain_map_structure(&assembled).map_err(|error| {
        RuntimeError::external("Photographic Styles HDR structural validation", error)
    })?;

    let mut publisher = AtomicFilePublisher::new(output.to_path_buf());
    let published = publisher.publish_bytes(assembled)?;
    Ok(PhotographicStylesFileReceipt { output: published })
}

fn encode_linear_thumbnail(
    adapter: &AppleAdapterClient,
    staging: &Path,
    raster: &Raster8,
    quality: f64,
) -> Result<EncodedHevcResource> {
    raster
        .validate()
        .map_err(|error| RuntimeError::external("Photographic Styles Linear Thumbnail", error))?;
    if raster.format != RasterPixelFormat::Rgb8 {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "VideoToolbox input raster is not RGB8",
        ));
    }
    let raw_path = staging.join("linear-thumbnail-rgb8.bin");
    let annex_b_path = staging.join("linear-thumbnail.hevc");
    let hvcc_path = staging.join("linear-thumbnail.hvcc");
    fs::write(&raw_path, &raster.data).map_err(|error| {
        RuntimeError::external("Photographic Styles Linear Thumbnail input", error)
    })?;
    let facts = adapter.videotoolbox_encode_main10(&AppleVideoToolboxMain10Encode {
        input: &raw_path,
        output_annex_b: &annex_b_path,
        output_hvcc: &hvcc_path,
        width: raster.width,
        height: raster.height,
        bytes_per_row: u32::try_from(raster.bytes_per_row).map_err(|_| {
            RuntimeError::new(
                "Photographic Styles Linear Thumbnail",
                "RGB8 row stride exceeds the Apple adapter wire range",
            )
        })?,
        quality,
    })?;
    let annex_b = fs::read(&annex_b_path).map_err(|error| {
        RuntimeError::external("Photographic Styles Linear Thumbnail HEVC", error)
    })?;
    let hvcc = fs::read(&hvcc_path).map_err(|error| {
        RuntimeError::external("Photographic Styles Linear Thumbnail hvcC", error)
    })?;
    if facts.width != raster.width
        || facts.height != raster.height
        || facts.annex_b_length != annex_b.len()
        || facts.hvcc_length != hvcc.len()
    {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            format!("VideoToolbox output lengths do not match adapter facts: {facts:?}"),
        ));
    }
    let profile = parse_hvcc_profile(&hvcc).map_err(|error| {
        RuntimeError::external("Photographic Styles Linear Thumbnail hvcC", error)
    })?;
    if profile.chroma_sampling != ChromaSampling::Yuv420
        || profile.luma_bit_depth != 10
        || profile.chroma_bit_depth != 10
    {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            format!(
                "VideoToolbox output is not Main10 4:2:0: chroma={:?}, lumaDepth={}, chromaDepth={}",
                profile.chroma_sampling, profile.luma_bit_depth, profile.chroma_bit_depth
            ),
        ));
    }
    Ok(EncodedHevcResource {
        payload: single_idr_payload(&annex_b)?,
        hvcc,
        width: facts.width,
        height: facts.height,
    })
}

fn single_idr_payload(annex_b: &[u8]) -> Result<Vec<u8>> {
    let mut starts = Vec::new();
    let mut index = 0;
    while index + 3 < annex_b.len() {
        if annex_b[index..].starts_with(&[0, 0, 0, 1]) {
            starts.push((index, 4));
            index += 4;
        } else if annex_b[index..].starts_with(&[0, 0, 1]) {
            starts.push((index, 3));
            index += 3;
        } else {
            index += 1;
        }
    }
    for (position, (offset, prefix_length)) in starts.iter().copied().enumerate() {
        let start = offset + prefix_length;
        let end = starts
            .get(position + 1)
            .map_or(annex_b.len(), |(next_offset, _)| *next_offset);
        if start >= end {
            continue;
        }
        let nal_type = (annex_b[start] >> 1) & 0x3f;
        if nal_type != 19 && nal_type != 20 {
            continue;
        }
        let length = u32::try_from(end - start).map_err(|_| {
            RuntimeError::new(
                "Photographic Styles Linear Thumbnail",
                "VideoToolbox IDR NAL exceeds the ISO item length range",
            )
        })?;
        let mut payload = Vec::with_capacity(4 + end - start);
        payload.extend_from_slice(&length.to_be_bytes());
        payload.extend_from_slice(&annex_b[start..end]);
        return Ok(payload);
    }
    Err(RuntimeError::new(
        "Photographic Styles Linear Thumbnail",
        "VideoToolbox emitted no HEVC IDR NAL",
    ))
}

fn normalized_luma(raster: &Raster8) -> Result<Vec<f32>> {
    raster
        .validate()
        .map_err(|error| RuntimeError::external("Photographic Styles source raster", error))?;
    if raster.format != RasterPixelFormat::Rgb8 {
        return Err(RuntimeError::new(
            "Photographic Styles source raster",
            "source raster is not RGB8",
        ));
    }
    let width = usize::try_from(raster.width).map_err(|_| {
        RuntimeError::new("Photographic Styles source raster", "width exceeds usize")
    })?;
    let height = usize::try_from(raster.height).map_err(|_| {
        RuntimeError::new("Photographic Styles source raster", "height exceeds usize")
    })?;
    let mut output = Vec::with_capacity(width * height);
    for y in 0..height {
        let row = y.checked_mul(raster.bytes_per_row).ok_or_else(|| {
            RuntimeError::new("Photographic Styles source raster", "row overflows")
        })?;
        for x in 0..width {
            let offset = row
                .checked_add(x.checked_mul(3).ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles source raster",
                        "pixel offset overflows",
                    )
                })?)
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles source raster",
                        "pixel offset overflows",
                    )
                })?;
            let red = f32::from(raster.data[offset]) / 255.0;
            let green = f32::from(raster.data[offset + 1]) / 255.0;
            let blue = f32::from(raster.data[offset + 2]) / 255.0;
            output.push(0.228_974_56 * red + 0.691_738_52 * green + 0.079_286_91 * blue);
        }
    }
    if output.is_empty() || output.iter().any(|value| !value.is_finite()) {
        return Err(RuntimeError::new(
            "Photographic Styles source raster",
            "source luma is empty or non-finite",
        ));
    }
    Ok(output)
}

fn semantic_matte_dimensions(
    pixel_width: u32,
    pixel_height: u32,
    orientation: u32,
    maximum_size: u32,
) -> Result<(u32, u32)> {
    if pixel_width == 0 || pixel_height == 0 || maximum_size < 2 {
        return Err(RuntimeError::new(
            "Photographic Styles semantic matte",
            "semantic matte dimensions must be positive and the maximum must be at least two",
        ));
    }
    let (display_width, display_height) = if matches!(orientation, 5..=8) {
        (pixel_height, pixel_width)
    } else {
        (pixel_width, pixel_height)
    };
    let scale = (f64::from(maximum_size) / f64::from(display_width.max(display_height))).min(1.0);
    let fitted_width = (f64::from(display_width) * scale / 2.0).round() * 2.0;
    let fitted_height = (f64::from(display_height) * scale / 2.0).round() * 2.0;
    Ok((
        fitted_width.max(2.0).min(f64::from(maximum_size)) as u32,
        fitted_height.max(2.0).min(f64::from(maximum_size)) as u32,
    ))
}

fn resize_semantic_matte(
    matte: &AppleL8Mask,
    target_width: u32,
    target_height: u32,
) -> Result<AppleL8Mask> {
    if target_width == 0 || target_height == 0 {
        return Err(RuntimeError::new(
            "Photographic Styles semantic matte",
            "target dimensions must be positive",
        ));
    }
    if matte.width == target_width && matte.height == target_height {
        return Ok(matte.clone());
    }
    let source_width = usize::try_from(matte.width).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles semantic matte",
            "source width exceeds usize",
        )
    })?;
    let source_height = usize::try_from(matte.height).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles semantic matte",
            "source height exceeds usize",
        )
    })?;
    let destination_width = usize::try_from(target_width).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles semantic matte",
            "target width exceeds usize",
        )
    })?;
    let destination_height = usize::try_from(target_height).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles semantic matte",
            "target height exceeds usize",
        )
    })?;
    let source_len = source_width.checked_mul(source_height).ok_or_else(|| {
        RuntimeError::new(
            "Photographic Styles semantic matte",
            "source size overflows",
        )
    })?;
    if matte.pixels.len() != source_len {
        return Err(RuntimeError::new(
            "Photographic Styles semantic matte",
            "source mask geometry does not match its pixel payload",
        ));
    }
    let destination_len = destination_width
        .checked_mul(destination_height)
        .ok_or_else(|| {
            RuntimeError::new(
                "Photographic Styles semantic matte",
                "target size overflows",
            )
        })?;
    let mut pixels = vec![0_u8; destination_len];
    for y in 0..destination_height {
        let source_y = y.checked_mul(source_height).ok_or_else(|| {
            RuntimeError::new("Photographic Styles semantic matte", "row index overflows")
        })? / destination_height;
        for x in 0..destination_width {
            let source_x = x.checked_mul(source_width).ok_or_else(|| {
                RuntimeError::new(
                    "Photographic Styles semantic matte",
                    "column index overflows",
                )
            })? / destination_width;
            let source_offset = source_y
                .checked_mul(source_width)
                .and_then(|row| row.checked_add(source_x))
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles semantic matte",
                        "source pixel offset overflows",
                    )
                })?;
            let destination_offset = y
                .checked_mul(destination_width)
                .and_then(|row| row.checked_add(x))
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles semantic matte",
                        "target pixel offset overflows",
                    )
                })?;
            pixels[destination_offset] = matte.pixels[source_offset];
        }
    }
    AppleL8Mask::new(target_width, target_height, pixels)
        .map_err(|error| RuntimeError::external("Photographic Styles semantic matte", error))
}

fn downsample_raster(raster: &Raster8, maximum_size: u32) -> Result<Raster8> {
    raster
        .validate()
        .map_err(|error| RuntimeError::external("Photographic Styles source raster", error))?;
    if raster.format != RasterPixelFormat::Rgb8 {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "source raster is not RGB8",
        ));
    }
    if maximum_size < 2 {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "maximum size must be at least two pixels",
        ));
    }
    let scale = (f64::from(maximum_size) / f64::from(raster.width))
        .min(f64::from(maximum_size) / f64::from(raster.height))
        .min(1.0);
    let target_width = ((f64::from(raster.width) * scale / 2.0).round() * 2.0)
        .clamp(2.0, f64::from(maximum_size)) as u32;
    let target_height = ((f64::from(raster.height) * scale / 2.0).round() * 2.0)
        .clamp(2.0, f64::from(maximum_size)) as u32;
    let target_width_usize = usize::try_from(target_width).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "width exceeds usize",
        )
    })?;
    let target_height_usize = usize::try_from(target_height).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "height exceeds usize",
        )
    })?;
    let row_bytes = target_width_usize.checked_mul(3).ok_or_else(|| {
        RuntimeError::new("Photographic Styles Linear Thumbnail", "row size overflows")
    })?;
    let byte_count = row_bytes.checked_mul(target_height_usize).ok_or_else(|| {
        RuntimeError::new("Photographic Styles Linear Thumbnail", "size overflows")
    })?;
    let mut data = vec![0_u8; byte_count];
    let source_width = usize::try_from(raster.width).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "source width exceeds usize",
        )
    })?;
    let source_height = usize::try_from(raster.height).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "source height exceeds usize",
        )
    })?;
    for y in 0..target_height_usize {
        let source_y = y.checked_mul(source_height).ok_or_else(|| {
            RuntimeError::new("Photographic Styles Linear Thumbnail", "source y overflows")
        })? / target_height_usize;
        let destination_row = y.checked_mul(row_bytes).ok_or_else(|| {
            RuntimeError::new(
                "Photographic Styles Linear Thumbnail",
                "destination row overflows",
            )
        })?;
        let source_row = source_y.checked_mul(raster.bytes_per_row).ok_or_else(|| {
            RuntimeError::new(
                "Photographic Styles Linear Thumbnail",
                "source row overflows",
            )
        })?;
        for x in 0..target_width_usize {
            let source_x = x.checked_mul(source_width).ok_or_else(|| {
                RuntimeError::new("Photographic Styles Linear Thumbnail", "source x overflows")
            })? / target_width_usize;
            let source_offset = source_row
                .checked_add(source_x.checked_mul(3).ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles Linear Thumbnail",
                        "source pixel offset overflows",
                    )
                })?)
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles Linear Thumbnail",
                        "source pixel offset overflows",
                    )
                })?;
            let destination_offset = destination_row
                .checked_add(x.checked_mul(3).ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles Linear Thumbnail",
                        "destination pixel offset overflows",
                    )
                })?)
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Photographic Styles Linear Thumbnail",
                        "destination pixel offset overflows",
                    )
                })?;
            data[destination_offset..destination_offset + 3]
                .copy_from_slice(&raster.data[source_offset..source_offset + 3]);
        }
    }
    Raster8::new(
        target_width,
        target_height,
        row_bytes,
        RasterPixelFormat::Rgb8,
        data,
    )
    .map_err(|error| RuntimeError::external("Photographic Styles Linear Thumbnail", error))
}

fn style_statistics(values: &[f32]) -> Result<AppleStyleStatistics> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(RuntimeError::new(
            "Photographic Styles statistics",
            "statistics input is empty or non-finite",
        ));
    }
    let mut sorted = values
        .iter()
        .map(|value| f64::from(*value))
        .collect::<Vec<_>>();
    sorted.sort_by(f64::total_cmp);
    let distribution = AppleStyleDistribution {
        black_point: percentile(&sorted, 0.5),
        high_key: percentile(&sorted, 95.0),
        p02: percentile(&sorted, 2.0),
        p10: percentile(&sorted, 10.0),
        p25: percentile(&sorted, 25.0),
        p50: percentile(&sorted, 50.0),
        p75: percentile(&sorted, 75.0),
        p98: percentile(&sorted, 98.0),
        white_point: percentile(&sorted, 99.5),
    };
    Ok(AppleStyleStatistics {
        linear_gtc_image: distribution,
        linear_image: distribution,
        linear_image_person_segment_based: distribution,
        linear_image_skin_based: distribution,
        tone_mapped_image: distribution,
        tone_mapped_image_blue_channel_skin_based: distribution,
        tone_mapped_image_green_channel_skin_based: distribution,
        tone_mapped_image_person_segment_based: distribution,
        tone_mapped_image_red_channel_skin_based: distribution,
        tone_mapped_image_skin_based: distribution,
    })
}

fn percentile(sorted: &[f64], percent: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let position = (percent.clamp(0.0, 100.0) / 100.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

fn style_grid_size(width: u32, height: u32) -> (u32, u32, u32, u32) {
    let landscape = width >= height;
    let (max_width, max_height, rows, columns) = if landscape {
        (
            2_880.0_f64,
            2_560.0_f64,
            STYLE_DELTA_ROWS_LANDSCAPE,
            STYLE_DELTA_COLUMNS_LANDSCAPE,
        )
    } else {
        (
            2_560.0_f64,
            2_880.0_f64,
            STYLE_DELTA_ROWS_PORTRAIT,
            STYLE_DELTA_COLUMNS_PORTRAIT,
        )
    };
    let scale = (max_width / f64::from(width))
        .min(max_height / f64::from(height))
        .min(1.0);
    let scaled_width =
        ((f64::from(width) * scale / 2.0).round() * 2.0).clamp(2.0, max_width) as u32;
    let scaled_height =
        ((f64::from(height) * scale / 2.0).round() * 2.0).clamp(2.0, max_height) as u32;
    (scaled_width, scaled_height, rows, columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_statistics_match_swift_percentile_contract() {
        let values = [0.0, 0.25, 0.5, 0.75, 1.0];
        let statistics = style_statistics(&values).unwrap();
        assert_eq!(statistics.tone_mapped_image.black_point, 0.005);
        assert_eq!(statistics.tone_mapped_image.p50, 0.5);
        assert_eq!(statistics.tone_mapped_image.white_point, 0.995);
        assert_eq!(statistics.linear_image, statistics.tone_mapped_image);
    }

    #[test]
    fn style_grid_size_stays_inside_the_fixed_thirty_tile_layout() {
        assert_eq!(style_grid_size(4032, 3024), (2880, 2160, 5, 6));
        assert_eq!(style_grid_size(3024, 4032), (2160, 2880, 6, 5));
        let (width, height, rows, columns) = style_grid_size(640, 480);
        assert_eq!((width, height), (640, 480));
        assert_eq!(rows * columns, 30);
    }

    #[test]
    fn semantic_matte_dimensions_match_swift_scaffold_fit_for_all_orientation_classes() {
        assert_eq!(
            semantic_matte_dimensions(3064, 4080, 1, 2016).unwrap(),
            (1514, 2016)
        );
        assert_eq!(
            semantic_matte_dimensions(3064, 4080, 3, 2016).unwrap(),
            (1514, 2016)
        );
        assert_eq!(
            semantic_matte_dimensions(3064, 4080, 6, 2016).unwrap(),
            (2016, 1514)
        );
        assert_eq!(
            semantic_matte_dimensions(3064, 4080, 8, 2016).unwrap(),
            (2016, 1514)
        );
    }

    #[test]
    fn semantic_matte_resize_is_deterministic_and_preserves_corners() {
        let source = AppleL8Mask::new(3, 2, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let resized = resize_semantic_matte(&source, 2, 4).unwrap();
        assert_eq!(resized.width, 2);
        assert_eq!(resized.height, 4);
        assert_eq!(resized.pixels, vec![1, 2, 1, 2, 4, 5, 4, 5]);
    }

    #[test]
    fn neutral_style_delta_uses_verified_main10_protocol_resource() {
        let (payload, hvcc) = verified_neutral_style_delta().unwrap();
        assert_eq!(payload.len(), 155);
        assert_eq!(hvcc.len(), 105);
        assert_eq!(hvcc[1] & 0x1f, 2);
        assert_eq!(hvcc[16] & 0x03, 1);
        assert_eq!((hvcc[17] & 0x07) + 8, 10);
        assert_eq!((hvcc[18] & 0x07) + 8, 10);
    }

    #[test]
    fn single_idr_payload_converts_annex_b_to_length_prefixed_item_data() {
        let annex_b = [
            0, 0, 0, 1, 64, 1, 2, 3, // VPS
            0, 0, 1, 66, 4, 5, // SPS
            0, 0, 0, 1, 78, 6, // prefix SEI, not an IDR
            0, 0, 0, 1, 0x26, 7, // IDR type 19
        ];
        assert_eq!(single_idr_payload(&annex_b).unwrap(), [0, 0, 0, 2, 0x26, 7]);
    }
}
