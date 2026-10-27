use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use serde_json::json;
use sha2::{Digest, Sha256};
use xdremux_codec::EncodedHevcResource;
#[cfg(test)]
use xdremux_codec::{HeifRasterDecodeRequest, Raster8, RasterPixelFormat};
use xdremux_engine::{
    apple_style_apply_global_tone_curve, apple_style_distribution, apple_style_face_exposure_boost,
    apple_style_light_map, apple_style_linear_metadata, apple_style_linear_thumbnail_codes,
    apple_style_maker_apple_dictionary, apple_style_monotonic_global_tone_curve,
    apple_style_property_list, apple_style_revert_local_data, resolve_apple_style_scene_type,
    AppleL8Mask, AppleStyleGlobalToneCurve, AppleStyleLightMapRequest, AppleStyleLinearMetadata,
    AppleStylePropertyListRequest, AppleStyleRevertRequest, AppleStyleStatistics,
    APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES,
};
use xdremux_hdr::{area_resize_linear_rgb, SceneLinearRgb};
#[cfg(test)]
use xdremux_hdr::{iso_gain_application_for_alternate, reconstruct_scene_linear_hdr, GainMapCodes};

use crate::apple_adapter::{AppleAdapterClient, AppleVideoToolboxMain10Encode};
use crate::{
    publication_parent, AtomicFilePublisher, PhotographicStylesFileReceipt, PortableRuntime,
    Result, RuntimeError,
};
#[cfg(test)]
use xdremux_engine::RasterDecoder;
use xdremux_format::{parse_hvcc_profile, ChromaSampling};
#[cfg(test)]
use xdremux_hdr::ResolvedScale;

const STYLE_DELTA_TILE_SIZE: u32 = 512;
const LINEAR_THUMBNAIL_MAX_SIZE: u32 = 1024;
const LINEAR_THUMBNAIL_QUALITY: f64 = 0.85;
const SEMANTIC_MATTE_MAX_SIZE: u32 = 2016;
const STYLE_DELTA_ROWS_LANDSCAPE: u32 = 5;
const STYLE_DELTA_COLUMNS_LANDSCAPE: u32 = 6;
const STYLE_DELTA_ROWS_PORTRAIT: u32 = 6;
const STYLE_DELTA_COLUMNS_PORTRAIT: u32 = 5;

fn profile_start() -> Option<Instant> {
    std::env::var("XDREMUX_STYLES_PROFILE")
        .is_ok_and(|value| value == "1")
        .then(Instant::now)
}

fn profile_stage(profile: &mut Option<Instant>, stage: &'static str) {
    if let Some(started) = profile {
        eprintln!(
            "xdremux_styles_profile stage={stage} seconds={:.9}",
            started.elapsed().as_secs_f64()
        );
        *started = Instant::now();
    }
}

// Apple ImageIO accepts the Styles auxiliary graph only when these resources
// use the Main10 4:2:0 contract. These bytes are the checked-in neutral Style
// Delta protocol resource already validated by the native consumer probe
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

/// Execute the Rust-owned Styles transaction.
///
/// The adapter supplies only Apple framework observations and codec/write
/// primitives. Rust derives the scene raster, tone curve, metadata, style
/// coefficients, HEIF graph, validation, and atomic publication as one
/// source-bound operation.
pub(crate) fn convert_file(
    runtime: &PortableRuntime,
    executable: impl AsRef<Path>,
    source: &[u8],
    output: impl AsRef<Path>,
) -> Result<PhotographicStylesFileReceipt> {
    let mut profile = profile_start();
    let output = output.as_ref();
    let parent = publication_parent(output);
    if !parent.is_dir() {
        return Err(RuntimeError::new(
            "Photographic Styles publication",
            format!("output parent is not a directory: {}", parent.display()),
        ));
    }
    let prepared = runtime.analyze_proxdr(source)?;
    profile_stage(&mut profile, "source_analysis");

    let staging = tempfile::Builder::new()
        .prefix(".xdremux-styles-")
        .tempdir_in(parent)
        .map_err(|error| RuntimeError::external("Photographic Styles staging", error))?;
    let base_path = staging.path().join("base.heic");
    let semantic_scaffold_path = staging.path().join("semantic-scaffold.heic");

    // Build the standard HDR base in memory so an in-place Styles conversion
    // cannot publish an intermediate file. The request is intentionally
    // feature-neutral: this operation owns the Styles graph separately.
    let product_base = runtime
        .convert_proxdr_bytes(source, xdremux_engine::ConversionRequest::default(), |_| {})?
        .bytes;
    let mut system_encoded = if crate::system_heif_hdr::requested()? {
        Some(crate::system_heif_hdr::encode_system_base(
            executable.as_ref(),
            &product_base,
            staging.path(),
            output,
        )?)
    } else {
        None
    };
    let carrier_report = system_encoded
        .as_ref()
        .map(|encoded| encoded.report_path.clone());
    let base = if let Some(encoded) = system_encoded.as_mut() {
        std::mem::take(&mut encoded.bytes)
    } else {
        product_base
    };
    fs::write(&base_path, &base)
        .map_err(|error| RuntimeError::external("Photographic Styles base staging", error))?;
    profile_stage(&mut profile, "base_conversion");

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
    let semantic_masks = adapter
        .vision_semantic_mattes(&base_path, &APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES, None)
        .map_err(|error| note_system_carrier(carrier_report.as_deref(), error))?;
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

    let maker_apple = apple_style_maker_apple_dictionary();
    adapter.imageio_write_auxiliary_with_properties(
        &base_path,
        &semantic_scaffold_path,
        &semantic_payloads,
        maker_apple.clone(),
    )?;
    let semantic_scaffold = fs::read(&semantic_scaffold_path).map_err(|error| {
        RuntimeError::external("Photographic Styles semantic scaffold read", error)
    })?;
    let semantic_base = xdremux_heif::transplant_apple_semantic_auxiliary_heif(
        &base,
        &semantic_scaffold,
        APPLE_PHOTOGRAPHIC_STYLES_SEMANTIC_ROLES.len(),
    )
    .map_err(|error| {
        note_system_carrier(
            carrier_report.as_deref(),
            RuntimeError::external("Photographic Styles semantic graph", error),
        )
    })?;
    profile_stage(&mut profile, "semantic_production");
    runtime
        .heif
        .extract_primary_hevc_resource(&semantic_base)
        .map_err(|error| RuntimeError::external("Photographic Styles primary resource", error))?;
    let geometry = xdremux_heif::primary_coded_geometry(&semantic_base).map_err(|error| {
        RuntimeError::external("Photographic Styles primary coded geometry", error)
    })?;
    xdremux_heif::validate_gain_map_structure(&semantic_base).map_err(|error| {
        note_system_carrier(
            carrier_report.as_deref(),
            RuntimeError::external("Photographic Styles source Gain Map structure", error),
        )
    })?;
    profile_stage(&mut profile, "resource_prepare");
    let scene_rasters = if let Some(encoded) = system_encoded.as_mut() {
        let pair = encoded.submitted_scene.take().ok_or_else(|| {
            RuntimeError::new(
                "Photographic Styles system HDR",
                "submitted scene was already consumed",
            )
        })?;
        derive_style_scene_from_pair(
            pair,
            encoded.content_headroom.log2(),
            LINEAR_THUMBNAIL_MAX_SIZE,
        )?
    } else {
        let pair = scene_linear_pair(
            &adapter,
            &semantic_base,
            staging.path(),
            LINEAR_THUMBNAIL_MAX_SIZE,
        )?;
        let maximum_stops = prepared
            .scale
            .per_channel_gain_map_max
            .iter()
            .copied()
            .chain(std::iter::once(prepared.scale.gain_map_max))
            .fold(f64::NEG_INFINITY, f64::max);
        derive_style_scene_from_pair(pair, maximum_stops as f32, LINEAR_THUMBNAIL_MAX_SIZE)?
    };
    profile_stage(&mut profile, "scene_derivation");
    let display_thumbnail = linear_thumbnail_raster(&scene_rasters)?;
    let (width, height, data) = xdremux_codec::undo_heif_rgb16(
        &display_thumbnail.data,
        display_thumbnail.width,
        display_thumbnail.height,
        geometry.quarter_turns_ccw,
        geometry.mirror_axis,
        geometry.mirror_before_rotation,
    )
    .map_err(|error| {
        RuntimeError::external(
            "Photographic Styles Linear Thumbnail coded orientation",
            error,
        )
    })?;
    let linear_thumbnail_raster = LinearThumbnailCodes {
        width,
        height,
        bytes_per_row: width.checked_mul(6).ok_or_else(|| {
            RuntimeError::new("Photographic Styles Linear Thumbnail", "row size overflows")
        })?,
        data,
    };
    let linear_thumbnail = encode_linear_thumbnail(
        &adapter,
        staging.path(),
        &linear_thumbnail_raster,
        LINEAR_THUMBNAIL_QUALITY,
    )?;
    profile_stage(&mut profile, "thumbnail_encode");

    let style_data = source_derived_style_data(&scene_rasters)?;
    profile_stage(&mut profile, "key1_fit");
    let global_tone_curve = scene_rasters.global_tone_curve.data.clone();
    let statistics = style_scene_statistics(&scene_rasters, &semantic_masks)?;
    let storage_orientation = u8::try_from(storage_orientation).map_err(|_| {
        RuntimeError::new(
            "Photographic Styles light map",
            "ImageIO orientation exceeds the portable wire range",
        )
    })?;
    // Key c is the primary-image guide mean. Key d is the linear-thumbnail
    // guide mean. The consumer upsamples each with its guided filter and mixes
    // them; these maps are not multiplied onto luminance.
    let tone_light_map = apple_style_light_map(&AppleStyleLightMapRequest {
        luma: &scene_rasters.base_luminance,
        width: scene_rasters.width,
        height: scene_rasters.height,
        storage_orientation,
    })
    .map_err(|error| RuntimeError::external("Photographic Styles tone light map", error))?;
    let linear_light_map = apple_style_light_map(&AppleStyleLightMapRequest {
        luma: &scene_rasters.thumbnail_luminance,
        width: scene_rasters.width,
        height: scene_rasters.height,
        storage_orientation,
    })
    .map_err(|error| RuntimeError::external("Photographic Styles linear light map", error))?;
    let scene_scores = adapter.vision_scene_scores(&base_path)?;
    let scene_decision = resolve_apple_style_scene_type(scene_scores)
        .map_err(|error| RuntimeError::external("Photographic Styles scene policy", error))?;
    let person = semantic_masks.get(&xdremux_engine::AppleSemanticRole::Person);
    let skin = semantic_masks.get(&xdremux_engine::AppleSemanticRole::Skin);
    let people_ratio = mask_mean(person);
    let skin_ratio = mask_mean(skin);
    let face_exposure_boost = apple_style_face_exposure_boost(
        statistics.tone_mapped_image.p50,
        statistics.tone_mapped_image_person_segment_based.p50,
        person.is_some_and(AppleL8Mask::has_credible_foreground),
    )
    .map_err(|error| RuntimeError::external("Photographic Styles face exposure policy", error))?;
    let style_properties = apple_style_property_list(&AppleStylePropertyListRequest {
        style_data: &style_data,
        global_tone_curve: &global_tone_curve,
        baseline_exposure: f64::from(scene_rasters.linear_metadata.baseline_exposure),
        scene_type: scene_decision.scene_type,
        statistics: &statistics,
        people_ratio: f64::from(people_ratio),
        person_masks_valid_hint: if person.is_some_and(AppleL8Mask::has_credible_foreground) {
            1.0
        } else {
            -1.0
        },
        skin_ratio: f64::from(skin_ratio),
        tone_light_map: &tone_light_map,
        linear_light_map: &linear_light_map,
        base_gain: f64::from(scene_rasters.linear_metadata.base_gain),
        linear_gain: f64::from(scene_rasters.linear_metadata.encoding_gain),
        original_range_min: f64::from(scene_rasters.renderer_linear_minimum),
        original_range_max: f64::from(scene_rasters.renderer_linear_maximum),
        face_exposure_boost,
    })
    .map_err(|error| RuntimeError::external("Photographic Styles metadata", error))?;
    profile_stage(&mut profile, "style_metadata");

    // Validate the exact Rust-owned key-1 resource at the private consumer
    // boundary before assembling the final file. The adapter cannot choose
    // style data or alter the property-list policy.
    adapter.semantic_style_properties_facts(&style_properties, &style_data)?;
    profile_stage(&mut profile, "native_style_properties");

    let (style_delta_payload, style_delta_hvcc) = verified_neutral_style_delta()?;

    let (style_delta_grid_width, style_delta_grid_height, style_delta_rows, style_delta_columns) =
        style_grid_size(geometry.width, geometry.height);
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
        .map_err(|error| {
            note_system_carrier(
                carrier_report.as_deref(),
                RuntimeError::external("Rust Photographic Styles graph", error),
            )
        })?;
    let styles_path = staging.path().join("styles-for-texture.heic");
    fs::write(&styles_path, &assembled)
        .map_err(|error| RuntimeError::external("Texture source staging", error))?;
    profile_stage(&mut profile, "style_assembly");
    let assembled =
        crate::apple_texture::generate(&adapter, &styles_path, &assembled, staging.path())?;
    profile_stage(&mut profile, "texture_generation");
    xdremux_heif::validate_gain_map_structure(&assembled).map_err(|error| {
        note_system_carrier(
            carrier_report.as_deref(),
            RuntimeError::external("Photographic Styles HDR structural validation", error),
        )
    })?;

    // Inspect the exact final bytes, not the intermediate scaffold or the
    // coefficient plist. An older adapter that ignores the additive metadata
    // request must fail here before it can replace an existing output.
    let final_path = staging.path().join("final-styles.heic");
    fs::write(&final_path, &assembled).map_err(|error| {
        RuntimeError::external("Photographic Styles final metadata staging", error)
    })?;
    let final_properties = adapter.imageio_image_properties(&final_path)?;
    validate_maker_apple_properties(
        &maker_apple,
        &final_properties.maker_apple_integer_dictionaries,
    )?;

    if let Some(encoded) = &system_encoded {
        if let Err(error) = crate::system_heif_hdr::record_after_styles(
            &adapter,
            &final_path,
            &encoded.report_path,
            &encoded.reference_f32_path,
            &encoded.reference_f16_path,
        ) {
            return Err(note_system_carrier(carrier_report.as_deref(), error));
        }
    }

    profile_stage(&mut profile, "final_validation");
    let mut publisher = AtomicFilePublisher::new(output.to_path_buf());
    let published = publisher.publish_bytes(assembled)?;
    profile_stage(&mut profile, "publication");
    Ok(PhotographicStylesFileReceipt { output: published })
}

fn note_system_carrier(report: Option<&Path>, error: RuntimeError) -> RuntimeError {
    if let Some(report) = report {
        crate::system_heif_hdr::record_carrier_error(report, &error.to_string());
    }
    error
}

pub(super) fn validate_maker_apple_properties(
    expected: &BTreeMap<String, BTreeMap<String, i64>>,
    actual: &BTreeMap<String, BTreeMap<String, i64>>,
) -> Result<()> {
    for (tag, fields) in expected {
        for (key, value) in fields {
            if actual.get(tag).and_then(|fields| fields.get(key)) != Some(value) {
                return Err(RuntimeError::new(
                    "Photographic Styles final metadata",
                    format!("ImageIO did not retain MakerApple {tag} field {key}={value}"),
                ));
            }
        }
    }
    Ok(())
}

fn encode_linear_thumbnail(
    adapter: &AppleAdapterClient,
    staging: &Path,
    raster: &LinearThumbnailCodes,
    quality: f64,
) -> Result<EncodedHevcResource> {
    if raster.width == 0
        || raster.height == 0
        || !raster.width.is_multiple_of(2)
        || !raster.height.is_multiple_of(2)
    {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "VideoToolbox input geometry must be non-zero and even",
        ));
    }
    let minimum_row = raster.width.checked_mul(6).ok_or_else(|| {
        RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "RGB16 row stride overflows",
        )
    })?;
    if raster.bytes_per_row < minimum_row {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "VideoToolbox input raster is shorter than 16-bit RGB",
        ));
    }
    let raw_path = staging.join("linear-thumbnail-rgb16.bin");
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
        bytes_per_row: raster.bytes_per_row,
        component_bits: 16,
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

#[derive(Debug, Clone, PartialEq)]
struct StyleSceneRasters {
    width: usize,
    height: usize,
    base_rgb: Vec<f32>,
    hdr_rgb: Vec<f32>,
    base_luminance: Vec<f32>,
    hdr_luminance: Vec<f32>,
    coded_linear_luminance: Vec<f32>,
    renderer_linear_luminance: Vec<f32>,
    thumbnail_codes: Vec<f32>,
    thumbnail_luminance: Vec<f32>,
    gtc_mapped_luminance: Vec<f32>,
    global_tone_curve: AppleStyleGlobalToneCurve,
    linear_metadata: AppleStyleLinearMetadata,
    renderer_linear_minimum: f32,
    renderer_linear_maximum: f32,
}

#[cfg(test)]
fn derive_style_scene(
    raster: &Raster8,
    gain_map: &Raster8,
    scale: &ResolvedScale,
    maximum_edge: u32,
) -> Result<StyleSceneRasters> {
    raster
        .validate()
        .map_err(|error| RuntimeError::external("Photographic Styles source raster", error))?;
    gain_map
        .validate()
        .map_err(|error| RuntimeError::external("Photographic Styles source Gain Map", error))?;
    if raster.format != RasterPixelFormat::Rgb8 {
        return Err(RuntimeError::new(
            "Photographic Styles source raster",
            "source raster is not RGB8",
        ));
    }
    if !matches!(
        gain_map.format,
        RasterPixelFormat::Mono8 | RasterPixelFormat::Rgb8
    ) {
        return Err(RuntimeError::new(
            "Photographic Styles source Gain Map",
            "source Gain Map is not an 8-bit monochrome or RGB raster",
        ));
    }
    if scale.channel_count == 0
        || scale
            .per_channel_gamma
            .iter()
            .chain(scale.per_channel_gain_map_min.iter())
            .chain(scale.per_channel_gain_map_max.iter())
            .chain(scale.per_channel_base_offset.iter())
            .chain(scale.per_channel_alternate_offset.iter())
            .any(|value| !value.is_finite())
    {
        return Err(RuntimeError::new(
            "Photographic Styles source Gain Map",
            "resolved Gain Map parameters contain a non-finite value",
        ));
    }

    let width = usize::try_from(raster.width).map_err(|_| {
        RuntimeError::new("Photographic Styles source raster", "width exceeds usize")
    })?;
    let height = usize::try_from(raster.height).map_err(|_| {
        RuntimeError::new("Photographic Styles source raster", "height exceeds usize")
    })?;
    let pixel_count = width.checked_mul(height).ok_or_else(|| {
        RuntimeError::new("Photographic Styles source raster", "pixel count overflows")
    })?;
    let srgb8_to_linear: [f32; 256] =
        std::array::from_fn(|code| srgb_decode(f32::from(code as u8) / 255.0));
    let mut full_sdr = Vec::with_capacity(pixel_count.saturating_mul(3));
    for y in 0..height {
        let source_row = y.checked_mul(raster.bytes_per_row).ok_or_else(|| {
            RuntimeError::new("Photographic Styles source raster", "row offset overflows")
        })?;
        for x in 0..width {
            let source_offset = source_row
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
            for component in &raster.data[source_offset..source_offset + 3] {
                full_sdr.push(srgb8_to_linear[usize::from(*component)]);
            }
        }
    }
    let full_sdr = SceneLinearRgb {
        width,
        height,
        samples: full_sdr,
    };
    let application = iso_gain_application_for_alternate(scale)
        .map_err(|error| RuntimeError::external("Photographic Styles source Gain Map", error))?;
    let gain_channels = match gain_map.format {
        RasterPixelFormat::Mono8 => 1,
        RasterPixelFormat::Rgb8 => 3,
    };
    let mut reconstruction_profile = profile_start();
    let full_hdr = reconstruct_scene_linear_hdr(
        &full_sdr,
        GainMapCodes {
            width: usize::try_from(gain_map.width).map_err(|_| {
                RuntimeError::new("Photographic Styles source Gain Map", "width exceeds usize")
            })?,
            height: usize::try_from(gain_map.height).map_err(|_| {
                RuntimeError::new(
                    "Photographic Styles source Gain Map",
                    "height exceeds usize",
                )
            })?,
            bytes_per_row: gain_map.bytes_per_row,
            channels: gain_channels,
            data: &gain_map.data,
        },
        &application,
    )
    .map_err(|error| RuntimeError::external("Photographic Styles source Gain Map", error))?;
    profile_stage(&mut reconstruction_profile, "scene_hdr_reconstruction");
    let gain_map_maximum_stops = scale
        .per_channel_gain_map_max
        .iter()
        .copied()
        .chain(std::iter::once(scale.gain_map_max))
        .filter(|value| value.is_finite())
        .fold(f64::NEG_INFINITY, f64::max);
    if !gain_map_maximum_stops.is_finite() {
        return Err(RuntimeError::new(
            "Photographic Styles source Gain Map",
            "source Gain Map has no finite maximum headroom",
        ));
    }
    derive_style_scene_from_linear_images(
        &full_sdr,
        &full_hdr,
        gain_map_maximum_stops as f32,
        maximum_edge,
    )
}

fn derive_style_scene_from_pair(
    pair: SceneLinearPair,
    maximum_stops: f32,
    maximum_edge: u32,
) -> Result<StyleSceneRasters> {
    let sdr = SceneLinearRgb {
        width: pair.width,
        height: pair.height,
        samples: pair.sdr,
    };
    let hdr = SceneLinearRgb {
        width: pair.width,
        height: pair.height,
        samples: pair.hdr,
    };
    derive_style_scene_from_linear_images(&sdr, &hdr, maximum_stops, maximum_edge)
}

fn derive_style_scene_from_linear_images(
    full_sdr: &SceneLinearRgb,
    full_hdr: &SceneLinearRgb,
    gain_map_maximum_stops: f32,
    maximum_edge: u32,
) -> Result<StyleSceneRasters> {
    // Thumbnail reduction happens only after the gain is applied at the base
    // resolution. Both images use the same linear-light area footprint.
    let base_image = area_resize_linear_rgb(full_sdr, maximum_edge)
        .map_err(|error| RuntimeError::external("Photographic Styles Linear Thumbnail", error))?;
    let hdr_image = area_resize_linear_rgb(full_hdr, maximum_edge)
        .map_err(|error| RuntimeError::external("Photographic Styles Linear Thumbnail", error))?;
    if base_image.width != hdr_image.width
        || base_image.height != hdr_image.height
        || base_image.samples.len() != hdr_image.samples.len()
    {
        return Err(RuntimeError::new(
            "Photographic Styles Linear Thumbnail",
            "linear SDR and HDR reductions have different geometry",
        ));
    }
    let width = base_image.width;
    let height = base_image.height;
    let mut base_luminance = Vec::with_capacity(width * height);
    let mut hdr_luminance = Vec::with_capacity(width * height);
    let mut renderer_linear_minimum = f32::INFINITY;
    let mut renderer_linear_maximum = f32::NEG_INFINITY;
    for (base_pixel, hdr_pixel) in base_image
        .samples
        .as_chunks::<3>()
        .0
        .iter()
        .zip(hdr_image.samples.as_chunks::<3>().0)
    {
        let base = [base_pixel[0], base_pixel[1], base_pixel[2]];
        let hdr = [hdr_pixel[0], hdr_pixel[1], hdr_pixel[2]];
        let base_y = luminance(base);
        let hdr_y = luminance(hdr);
        if !base_y.is_finite() || !hdr_y.is_finite() {
            return Err(RuntimeError::new(
                "Photographic Styles source raster",
                "source-derived luminance contains a non-finite sample",
            ));
        }
        base_luminance.push(base_y);
        hdr_luminance.push(hdr_y);
        for value in hdr {
            renderer_linear_minimum = renderer_linear_minimum.min(value);
            renderer_linear_maximum = renderer_linear_maximum.max(value);
        }
    }
    let base_rgb = base_image.samples;
    let hdr_rgb = hdr_image.samples;

    let linear_metadata =
        apple_style_linear_metadata(&base_luminance, &hdr_luminance, gain_map_maximum_stops)
            .map_err(|error| {
                RuntimeError::external("Photographic Styles linear metadata", error)
            })?;
    let coded_linear_luminance = hdr_luminance
        .iter()
        .map(|value| *value / linear_metadata.baseline_exposure)
        .collect::<Vec<_>>();
    // S in the consumer is the decoded thumbnail after one base-gain multiply.
    // The extra exposure E is a render parameter, so it stays out of this code.
    let thumbnail_codes = hdr_rgb
        .iter()
        .map(|value| {
            let code = f64::from(*value) / f64::from(linear_metadata.baseline_exposure)
                * f64::from(linear_metadata.base_gain);
            code.clamp(0.0, 1.0) as f32
        })
        .collect::<Vec<_>>();
    let global_tone_curve = apple_style_monotonic_global_tone_curve(&thumbnail_codes, &base_rgb)
        .map_err(|error| RuntimeError::external("Photographic Styles global tone curve", error))?;
    let gtc_rgb =
        apple_style_apply_global_tone_curve(&thumbnail_codes, &global_tone_curve.linear_samples)
            .map_err(|error| {
                RuntimeError::external("Photographic Styles global tone curve", error)
            })?;
    let mut thumbnail_luminance = Vec::with_capacity(width * height);
    let mut gtc_mapped_luminance = Vec::with_capacity(width * height);
    for (thumbnail, mapped) in thumbnail_codes
        .as_chunks::<3>()
        .0
        .iter()
        .zip(gtc_rgb.as_chunks::<3>().0)
    {
        thumbnail_luminance.push(luminance([thumbnail[0], thumbnail[1], thumbnail[2]]));
        gtc_mapped_luminance.push(luminance([mapped[0], mapped[1], mapped[2]]));
    }
    let renderer_linear_luminance = coded_linear_luminance
        .iter()
        .map(|value| *value / linear_metadata.encoding_gain)
        .collect::<Vec<_>>();
    let renderer_scale = linear_metadata.baseline_exposure * linear_metadata.encoding_gain;
    renderer_linear_minimum /= renderer_scale;
    renderer_linear_maximum /= renderer_scale;
    if !renderer_linear_minimum.is_finite() || !renderer_linear_maximum.is_finite() {
        return Err(RuntimeError::new(
            "Photographic Styles linear scene",
            "renderer-linear range is non-finite",
        ));
    }
    Ok(StyleSceneRasters {
        width,
        height,
        base_rgb,
        hdr_rgb,
        base_luminance,
        hdr_luminance,
        coded_linear_luminance,
        renderer_linear_luminance,
        thumbnail_codes,
        thumbnail_luminance,
        gtc_mapped_luminance,
        global_tone_curve,
        linear_metadata,
        renderer_linear_minimum,
        renderer_linear_maximum,
    })
}

pub(crate) struct SceneLinearPair {
    pub width: usize,
    pub height: usize,
    pub sdr: Vec<f32>,
    pub hdr: Vec<f32>,
}

pub(crate) fn scene_linear_pair(
    adapter: &AppleAdapterClient,
    source: &[u8],
    staging: &Path,
    maximum_edge: u32,
) -> Result<SceneLinearPair> {
    let mut reconstruction_profile = profile_start();
    let geometry = xdremux_heif::primary_coded_geometry(source)
        .map_err(|error| RuntimeError::external("scene color management", error))?;
    let (width, height) = if geometry.quarter_turns_ccw & 1 == 1 {
        (geometry.height, geometry.width)
    } else {
        (geometry.width, geometry.height)
    };
    let work = tempfile::Builder::new()
        .prefix(".xdremux-linear-scene-")
        .tempdir_in(staging)
        .map_err(|error| RuntimeError::external("scene color management staging", error))?;
    let input = work.path().join("source.heic");
    fs::write(&input, source)
        .map_err(|error| RuntimeError::external("scene color management source", error))?;
    // ImageIO/Core Image honors the base ICC/nclx, the tmap alternate colorimetry
    // and use_base_colour_space. It reconstructs before converting both images
    // to extended-linear P3. Keep the full resolution and float precision here.
    let facts = adapter.styles_primitive(&json!({
        "schema_version": 2, "operation": "styles-readback",
        "input_path": input, "output_path": work.path(),
        "width": width, "height": height, "apply_orientation": true,
    }))?;
    for representation in ["sdr", "hdr"] {
        if facts[representation]["width"].as_u64() != Some(u64::from(width))
            || facts[representation]["height"].as_u64() != Some(u64::from(height))
        {
            return Err(RuntimeError::new(
                "scene color management",
                "native scene geometry differs from the source",
            ));
        }
    }
    profile_stage(&mut reconstruction_profile, "scene_hdr_reconstruction");
    let read = |name: &str| -> Result<SceneLinearRgb> {
        let bytes = fs::read(work.path().join(name))
            .map_err(|error| RuntimeError::external("scene color management pixels", error))?;
        let image = scene_rgb_from_native(&bytes, width as usize, height as usize)?;
        area_resize_linear_rgb(&image, maximum_edge)
            .map_err(|error| RuntimeError::external("scene linear reduction", error))
    };
    let sdr = read("published_sdr.f32")?;
    let hdr = read("published_hdr.f32")?;
    profile_stage(&mut reconstruction_profile, "scene_linear_area_reduction");
    Ok(SceneLinearPair {
        width: sdr.width,
        height: sdr.height,
        sdr: sdr.samples,
        hdr: hdr.samples,
    })
}

fn scene_rgb_from_native(bytes: &[u8], width: usize, height: usize) -> Result<SceneLinearRgb> {
    let row_bytes = width
        .checked_mul(16)
        .ok_or_else(|| RuntimeError::new("scene color management", "native row size overflows"))?;
    if row_bytes == 0 || height == 0 || row_bytes.checked_mul(height) != Some(bytes.len()) {
        return Err(RuntimeError::new(
            "scene color management",
            "native float pixels do not match their geometry",
        ));
    }
    let mut samples = Vec::with_capacity(width * height * 3);
    // CIContext bitmap rows already run top-down, independently of the
    // lower-left coordinate origin used for render bounds.
    for pixel in bytes.as_chunks::<16>().0 {
        for component in pixel[..12].as_chunks::<4>().0 {
            let value = f32::from_le_bytes(*component);
            if !value.is_finite() {
                return Err(RuntimeError::new(
                    "scene color management",
                    "native scene contains non-finite RGB",
                ));
            }
            samples.push(value);
        }
    }
    Ok(SceneLinearRgb {
        width,
        height,
        samples,
    })
}

struct LinearThumbnailCodes {
    width: u32,
    height: u32,
    bytes_per_row: u32,
    data: Vec<u8>,
}

fn linear_thumbnail_raster(scene: &StyleSceneRasters) -> Result<LinearThumbnailCodes> {
    let codes =
        apple_style_linear_thumbnail_codes(&scene.hdr_rgb, scene.linear_metadata.baseline_exposure)
            .map_err(|error| {
                RuntimeError::external("Photographic Styles Linear Thumbnail", error)
            })?;
    // Keep display order here; the caller restores coded order before encode.
    // The
    // samples are Apple Log codes at 16-bit, not RGB8 and not a different
    // transfer function.
    let width = u32::try_from(scene.width).map_err(|_| {
        RuntimeError::new("Photographic Styles Linear Thumbnail", "width exceeds u32")
    })?;
    let height = u32::try_from(scene.height).map_err(|_| {
        RuntimeError::new("Photographic Styles Linear Thumbnail", "height exceeds u32")
    })?;
    let bytes_per_row = width.checked_mul(6).ok_or_else(|| {
        RuntimeError::new("Photographic Styles Linear Thumbnail", "row size overflows")
    })?;
    let mut data = Vec::with_capacity(codes.len().saturating_mul(2));
    for code in codes {
        data.extend_from_slice(&code.to_le_bytes());
    }
    Ok(LinearThumbnailCodes {
        width,
        height,
        bytes_per_row,
        data,
    })
}

fn source_derived_style_data(scene: &StyleSceneRasters) -> Result<Vec<u8>> {
    if scene.base_rgb.len() != scene.thumbnail_codes.len()
        || scene.base_rgb.is_empty()
        || scene.base_rgb.len() != scene.width.saturating_mul(scene.height).saturating_mul(3)
    {
        return Err(RuntimeError::new(
            "Photographic Styles style data",
            "source-derived style raster geometry or values are invalid",
        ));
    }
    // Remove-style reads the displayed primary and applies key1. The recovered
    // target is the same per-channel thumbnail code the semantic-tone stage
    // uses. A forward primary-to-styled residual would point the other way.
    apple_style_revert_local_data(&AppleStyleRevertRequest {
        width: scene.width,
        height: scene.height,
        styled_rgb: &scene.base_rgb,
        recovered_rgb: &scene.thumbnail_codes,
    })
    .map_err(|error| RuntimeError::external("Photographic Styles style data", error))
}

#[cfg(test)]
fn srgb_decode(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(rgb: [f32; 3]) -> f32 {
    0.228_974_56 * rgb[0] + 0.691_738_52 * rgb[1] + 0.079_286_91 * rgb[2]
}

fn semantic_mask_value(
    mask: Option<&AppleL8Mask>,
    x: usize,
    y: usize,
    raster_width: usize,
    raster_height: usize,
) -> f32 {
    let Some(mask) = mask else { return 0.0 };
    let width = usize::try_from(mask.width).unwrap_or(0);
    let height = usize::try_from(mask.height).unwrap_or(0);
    if width == 0 || height == 0 {
        return 0.0;
    }
    let source_x =
        (((x as f64 + 0.5) * width as f64 / raster_width as f64) as usize).min(width - 1);
    let source_y =
        (((y as f64 + 0.5) * height as f64 / raster_height as f64) as usize).min(height - 1);
    f32::from(mask.pixels[source_y * width + source_x]) / 255.0
}

fn mask_mean(mask: Option<&AppleL8Mask>) -> f32 {
    let Some(mask) = mask else { return 0.0 };
    if mask.pixels.is_empty() {
        return 0.0;
    }
    mask.pixels
        .iter()
        .map(|value| f32::from(*value))
        .sum::<f32>()
        / (mask.pixels.len() as f32 * 255.0)
}

fn style_scene_statistics(
    scene: &StyleSceneRasters,
    masks: &BTreeMap<xdremux_engine::AppleSemanticRole, AppleL8Mask>,
) -> Result<AppleStyleStatistics> {
    let pixel_count = scene.base_luminance.len();
    if scene.base_rgb.len() != pixel_count.saturating_mul(3)
        || scene.renderer_linear_luminance.len() != pixel_count
    {
        return Err(RuntimeError::new(
            "Photographic Styles statistics",
            "scene raster channels have inconsistent lengths",
        ));
    }
    let person = masks.get(&xdremux_engine::AppleSemanticRole::Person);
    let skin = masks.get(&xdremux_engine::AppleSemanticRole::Skin);
    let mut person_tone = Vec::new();
    let mut person_hdr = Vec::new();
    let mut skin_tone = Vec::new();
    let mut skin_hdr = Vec::new();
    let mut skin_red = Vec::new();
    let mut skin_green = Vec::new();
    let mut skin_blue = Vec::new();
    for y in 0..scene.height {
        let width = scene.width;
        for x in 0..width {
            let pixel = y * width + x;
            if semantic_mask_value(person, x, y, width, scene.height) >= 0.5 {
                person_tone.push(scene.base_luminance[pixel]);
                person_hdr.push(scene.renderer_linear_luminance[pixel]);
            }
            if semantic_mask_value(skin, x, y, width, scene.height) >= 0.5 {
                skin_tone.push(scene.base_luminance[pixel]);
                skin_hdr.push(scene.renderer_linear_luminance[pixel]);
                skin_red.push(scene.base_rgb[pixel * 3]);
                skin_green.push(scene.base_rgb[pixel * 3 + 1]);
                skin_blue.push(scene.base_rgb[pixel * 3 + 2]);
            }
        }
    }
    Ok(AppleStyleStatistics {
        linear_gtc_image: apple_style_distribution(&scene.gtc_mapped_luminance),
        linear_image: apple_style_distribution(&scene.renderer_linear_luminance),
        linear_image_person_segment_based: apple_style_distribution(&person_hdr),
        linear_image_skin_based: apple_style_distribution(&skin_hdr),
        tone_mapped_image: apple_style_distribution(&scene.base_luminance),
        tone_mapped_image_blue_channel_skin_based: apple_style_distribution(&skin_blue),
        tone_mapped_image_green_channel_skin_based: apple_style_distribution(&skin_green),
        tone_mapped_image_person_segment_based: apple_style_distribution(&person_tone),
        tone_mapped_image_red_channel_skin_based: apple_style_distribution(&skin_red),
        tone_mapped_image_skin_based: apple_style_distribution(&skin_tone),
    })
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

#[cfg(test)]
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
    fn native_float_scene_keeps_precision_hdr_values_and_row_order() {
        let pixels = [
            [-0.125_f32, 4.000_31, 0.000_123, 1.0],
            [0.75, 0.5, 0.25, 1.0],
        ];
        let bytes = pixels
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        let scene = scene_rgb_from_native(&bytes, 1, 2).unwrap();
        assert_eq!(
            scene.samples,
            vec![-0.125, 4.000_31, 0.000_123, 0.75, 0.5, 0.25]
        );
        assert!(scene_rgb_from_native(&bytes[..bytes.len() - 1], 1, 2).is_err());
        let invalid = [f32::NAN, 0.0, 0.0, 1.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        assert!(scene_rgb_from_native(&invalid, 1, 1).is_err());
    }

    #[test]
    fn native_scene_keeps_top_and_bottom_in_display_order() {
        use xdremux_codec::PrimaryHeifEncodeRequest;
        let Some(executable) = std::env::var_os("XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE") else {
            return;
        };
        // Encode a known top-down HEIF raster, independently of CIImage's raw
        // bitmap initializer: red on top, blue on the bottom.
        let data = (0..320)
            .flat_map(|y| {
                if y < 160 {
                    [255, 0, 0].repeat(64)
                } else {
                    [0, 0, 255].repeat(64)
                }
            })
            .collect();
        let source = xdremux_codec::LibHeifProvider::new()
            .encode_primary_heif(&PrimaryHeifEncodeRequest::live_photo(
                Raster8::new(64, 320, 192, RasterPixelFormat::Rgb8, data).unwrap(),
                None,
            ))
            .unwrap();
        let adapter = AppleAdapterClient::new(executable);
        let work = tempfile::tempdir().unwrap();
        let pair = scene_linear_pair(&adapter, &source, work.path(), 320).unwrap();
        assert_eq!((pair.width, pair.height), (64, 320));
        for representation in [&pair.sdr, &pair.hdr] {
            let top = &representation[(32 * 64 + 32) * 3..][..3];
            let bottom = &representation[(288 * 64 + 32) * 3..][..3];
            assert!(top[0] > 0.75 && top[2] < 0.1, "top must stay red: {top:?}");
            assert!(
                bottom[2] > 0.8 && bottom[0] < 0.1,
                "bottom must stay blue: {bottom:?}"
            );
        }
        println!(
            "Native scene display order: encoded top red / bottom blue preserved in SDR and HDR"
        );
    }

    #[test]
    fn native_scene_applies_rgb_gain_before_converting_srgb_to_p3() {
        use xdremux_codec::{GainMapTileEncodeRequest, PrimaryHeifEncodeRequest};
        use xdremux_engine::{
            GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile, GainMapTileEncoder,
        };
        use xdremux_heif::{DirectHevcGainMap, GainMapTile, IsoGainMapAssembly};

        let Some(executable) = std::env::var_os("XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE") else {
            return;
        };
        let provider = xdremux_codec::LibHeifProvider::new();
        let raster = |rgb: [u8; 3]| {
            Raster8::new(64, 64, 64 * 3, RasterPixelFormat::Rgb8, rgb.repeat(64 * 64)).unwrap()
        };
        let profile = GainMapEncodeProfile {
            width: 64,
            height: 64,
            channels: GainMapChannels::Rgb,
            layout: GainMapCodecLayout {
                chroma: ChromaSampling::Yuv444,
                luma_bit_depth: 8,
                chroma_bit_depth: 8,
            },
        };
        let encoded = provider
            .encode_gain_map_tiles(&GainMapTileEncodeRequest::reference_compatible(
                raster([255, 128, 0]),
                profile,
            ))
            .unwrap();
        let tiles = encoded
            .tiles
            .iter()
            .map(|t| GainMapTile {
                payload: &t.payload,
                width: t.width,
                height: t.height,
            })
            .collect::<Vec<_>>();
        let info = [
            1.0, 1.0, 1.0, 1.0, 4.0, 4.0, 4.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            4.0, 4.0, 0.0,
        ];
        let tmap = xdremux_metadata::make_apple_tmap_payload(&info).unwrap();
        let xmp = xdremux_metadata::make_hdrgm_xmp(&info).unwrap();
        let jpeg = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"),
        )
        .unwrap();
        let p3_icc = xdremux_format::jpeg_icc_profile(&jpeg).unwrap().unwrap();
        let adapter = AppleAdapterClient::new(executable);
        // The second case is saturated Display P3 red, outside sRGB.
        for (icc, expected_red) in [
            (None, [0.822_461_97_f32, 0.033_194_2, 0.017_082_63]),
            (Some(p3_icc), [1.0, 0.0, 0.0]),
        ] {
            let base = provider
                .encode_primary_heif(&PrimaryHeifEncodeRequest::live_photo(
                    raster([255, 0, 0]),
                    icc,
                ))
                .unwrap();
            let source = xdremux_heif::assemble_iso_gain_map_heif(
                &base,
                &IsoGainMapAssembly {
                    alternate_icc: None,
                    gain_map: DirectHevcGainMap {
                        gain_map_width: 64,
                        gain_map_height: 64,
                        tile_width: encoded.tile_width,
                        tile_height: encoded.tile_height,
                        tiles: &tiles,
                        hvcc: &encoded.hvcc,
                        profile: xdremux_heif::GainMapEncodeProfile {
                            channels: xdremux_heif::GainMapChannels::Rgb,
                            chroma: ChromaSampling::Yuv444,
                            luma_bit_depth: 8,
                            chroma_bit_depth: 8,
                        },
                    },
                    tmap_payload: &tmap,
                    xmp_payload: &xmp,
                },
            )
            .unwrap();
            let work = tempfile::tempdir().unwrap();
            let pair = scene_linear_pair(&adapter, &source, work.path(), 64).unwrap();
            let i = (32 * 64 + 32) * 3;
            // Independently derived from the sRGB and Display P3 D65 xy primaries:
            // P3 = inverse(P3_to_XYZ) * sRGB_to_XYZ * [1,0,0]. Gain is applied
            // in sRGB first. Applying the unequal gains after conversion gives
            // different green/blue values; treating the source as P3 gives zero.
            for (component, coefficient) in expected_red.iter().enumerate() {
                assert!((pair.sdr[i + component] - coefficient).abs() < 0.015);
                assert!(
                    (pair.hdr[i + component] - 4.0 * coefficient).abs()
                        < 0.005 + 0.08 * coefficient
                );
            }
        }
    }

    #[test]
    fn missing_or_changed_smartstyle_state_is_rejected_before_publication() {
        let expected = apple_style_maker_apple_dictionary();
        assert!(validate_maker_apple_properties(&expected, &BTreeMap::new()).is_err());
        let mut actual = expected.clone();
        actual.get_mut("84").unwrap().remove("5");
        assert!(validate_maker_apple_properties(&expected, &actual).is_err());
        actual.get_mut("84").unwrap().insert("5".to_owned(), 0);
        assert!(validate_maker_apple_properties(&expected, &actual).is_err());
        actual.get_mut("84").unwrap().insert("5".to_owned(), 1);
        actual.get_mut("84").unwrap().insert("99".to_owned(), 7);
        validate_maker_apple_properties(&expected, &actual).unwrap();
        assert_eq!(expected.keys().collect::<Vec<_>>(), vec!["84"]);
    }

    #[test]
    fn real_styles_thumbnail_round_trips_hdr_log_through_videotoolbox() {
        use xdremux_format::isobmff::{parse_meta_box, scan_top_level_boxes, META};
        use xdremux_format::FourCC;

        let Some(adapter) = std::env::var_os("XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE") else {
            return;
        };
        let runtime = PortableRuntime::new();
        let temporary = tempfile::tempdir().unwrap();
        let output_dir = std::env::var_os("XDREMUX_TEST_STYLES_OUTPUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| temporary.path().to_path_buf());
        fs::create_dir_all(&output_dir).unwrap();
        let mut evidence = Vec::new();
        for fixture in [
            "oppo/find-x9-ultra/uhdr-portrait-01.heic",
            "oppo/find-x7-ultra/lhdr-v2-01.heic",
        ] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/proxdr")
                .join(fixture);
            let source = fs::read(&path).unwrap();
            let output_path = output_dir.join(path.file_name().unwrap());
            convert_file(&runtime, &adapter, &source, &output_path).unwrap();
            let output = fs::read(&output_path).unwrap();
            xdremux_heif::validate_gain_map_structure(&output).unwrap();
            let top = scan_top_level_boxes(&output).unwrap();
            let meta_header = top.boxes.iter().find(|header| header.kind == META).unwrap();
            let meta = parse_meta_box(&output, meta_header).unwrap();
            let thumbnail_id = meta
                .ipma
                .entries
                .iter()
                .find(|entry| {
                    entry.associations.iter().any(|association| {
                        meta.properties.iter().any(|property| {
                            property.index == u32::from(association.property_index)
                                && property.kind == FourCC::new(*b"auxC")
                                && output[property.box_range.clone()]
                                    .windows(b"tag:apple.com,2023:photo:aux:linearthumbnail".len())
                                    .any(|bytes| {
                                        bytes == b"tag:apple.com,2023:photo:aux:linearthumbnail"
                                    })
                        })
                    })
                })
                .expect("Styles graph must expose its Linear Thumbnail")
                .item_id;
            let decoded = runtime
                .heif
                .decode_item_raster(&output, thumbnail_id, RasterPixelFormat::Rgb8)
                .unwrap();
            let base = runtime
                .heif
                .decode_raster(&HeifRasterDecodeRequest {
                    data: output.clone(),
                    format: RasterPixelFormat::Rgb8,
                })
                .unwrap();
            let sdr_preview = downsample_raster(&base, LINEAR_THUMBNAIL_MAX_SIZE).unwrap();
            let prepared = runtime.analyze_proxdr(&source).unwrap();
            let pair = scene_linear_pair(
                &AppleAdapterClient::new(adapter.clone()),
                &output,
                temporary.path(),
                LINEAR_THUMBNAIL_MAX_SIZE,
            )
            .unwrap();
            let maximum_stops = prepared
                .scale
                .per_channel_gain_map_max
                .iter()
                .copied()
                .chain(std::iter::once(prepared.scale.gain_map_max))
                .fold(f64::NEG_INFINITY, f64::max);
            let scene =
                derive_style_scene_from_pair(pair, maximum_stops as f32, LINEAR_THUMBNAIL_MAX_SIZE)
                    .unwrap();
            let expected = linear_thumbnail_raster(&scene).unwrap();
            assert_eq!(
                (decoded.width, decoded.height),
                (expected.width, expected.height)
            );
            assert_eq!(
                (sdr_preview.width, sdr_preview.height),
                (decoded.width, decoded.height)
            );
            let mut log_error = 0.0;
            let mut sdr_error = 0.0;
            for y in 0..scene.height {
                for x in 0..scene.width {
                    for component in 0..3 {
                        let actual =
                            f64::from(decoded.data[y * decoded.bytes_per_row + x * 3 + component]);
                        let code_offset = (y * scene.width + x) * 6 + component * 2;
                        let code = u16::from_le_bytes([
                            expected.data[code_offset],
                            expected.data[code_offset + 1],
                        ]);
                        let expected_code = f64::from(code) / 65_535.0 * 255.0;
                        log_error += (actual - expected_code).abs();
                        sdr_error += (actual
                            - f64::from(
                                sdr_preview.data[y * sdr_preview.bytes_per_row + x * 3 + component],
                            ))
                        .abs();
                    }
                }
            }
            let count = (scene.width * scene.height * 3) as f64;
            let log_mae = log_error / count;
            let sdr_mae = sdr_error / count;
            assert!(
                log_mae < 3.0,
                "{fixture}: decoded HDR Log MAE {log_mae}/255"
            );
            assert!(
                sdr_mae > 10.0,
                "{fixture}: thumbnail still resembles copied SDR: {sdr_mae}/255"
            );
            let standard = runtime
                .convert_proxdr_bytes(
                    &source,
                    xdremux_engine::ConversionRequest::default(),
                    |_| {},
                )
                .unwrap()
                .bytes;
            let before = runtime
                .heif
                .extract_primary_hevc_resource(&standard)
                .unwrap();
            let after = runtime.heif.extract_primary_hevc_resource(&output).unwrap();
            assert_eq!(
                before.payload, after.payload,
                "Styles must preserve the primary HEVC"
            );
            assert_eq!(before.hvcc, after.hvcc);
            eprintln!(
                "hdr-log-thumbnail: fixture={fixture} dimensions={}x{} log_mae={log_mae:.4}/255 sdr_mae={sdr_mae:.4}/255 output={}",
                decoded.width,
                decoded.height,
                output_path.display()
            );
            evidence.push(serde_json::json!({
                "fixture": fixture,
                "output": output_path.file_name().unwrap().to_string_lossy(),
                "width": decoded.width,
                "height": decoded.height,
                "log_mae": log_mae,
                "sdr_mae": sdr_mae,
                "hdr_above_sdr_white_pixels": scene.hdr_luminance.iter().filter(|value| **value > 1.0).count(),
                "pixel_count": scene.width * scene.height,
                "source": "linear SDR RGB + ISO Gain Map; no PQ intermediate",
                "verification": "decoded Main10 Linear Thumbnail versus expected Apple Log RGB16 codes"
            }));
        }
        if let Some(report_dir) =
            std::env::var_os("XDREMUX_TEST_STYLES_REPORT").filter(|value| !value.is_empty())
        {
            let report_dir = std::path::PathBuf::from(report_dir);
            fs::create_dir_all(&report_dir).unwrap();
            fs::write(
                report_dir.join("linear-thumbnail-roundtrip.json"),
                serde_json::to_vec_pretty(&evidence).unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn submitted_system_pair_drives_styles_without_original_gain_parameters() {
        let sdr = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.2, 0.4, 0.6];
        let hdr = vec![0.2, 0.8, 0.6, 1.6, 1.0, 2.4, 1.4, 3.2, 1.8, 0.8, 0.8, 2.4];
        let scene = derive_style_scene_from_pair(
            SceneLinearPair {
                width: 2,
                height: 2,
                sdr: sdr.clone(),
                hdr: hdr.clone(),
            },
            2.0,
            2,
        )
        .unwrap();
        assert_eq!(scene.base_rgb, sdr);
        assert_eq!(scene.hdr_rgb, hdr);
        let thumbnail = linear_thumbnail_raster(&scene).unwrap();
        let expected =
            apple_style_linear_thumbnail_codes(&hdr, scene.linear_metadata.baseline_exposure)
                .unwrap();
        let actual = thumbnail
            .data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_le_bytes(*bytes))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn linear_thumbnail_reconstructs_rgb_gain_and_keeps_storage_geometry() {
        let base = Raster8::new(
            2,
            2,
            6,
            RasterPixelFormat::Rgb8,
            vec![64, 96, 128, 128, 160, 192, 192, 224, 255, 32, 48, 64],
        )
        .unwrap();
        let gain = Raster8::new(
            2,
            2,
            6,
            RasterPixelFormat::Rgb8,
            vec![0, 128, 255, 255, 128, 0, 64, 192, 255, 255, 0, 128],
        )
        .unwrap();
        let mut parameters = vec![0.0; 20];
        parameters[0..3].fill(1.0);
        parameters[4..7].copy_from_slice(&[2.0, 4.0, 8.0]);
        parameters[7..10].fill(1.0);
        parameters[16] = 1.0;
        parameters[17] = 8.0;
        parameters[18] = 8.0;
        let scale = xdremux_hdr::resolve(&parameters, xdremux_hdr::ExtractionMode::Uhdr).unwrap();
        let scene = derive_style_scene(&base, &gain, &scale, LINEAR_THUMBNAIL_MAX_SIZE).unwrap();
        let thumbnail = linear_thumbnail_raster(&scene).unwrap();
        assert_eq!(
            (thumbnail.width, thumbnail.height, thumbnail.bytes_per_row),
            (2, 2, 12)
        );
        assert_eq!(thumbnail.data.len(), 24);
        // Independent scalar reconstruction verifies channel order, exposure,
        // and position. There is no display rotation in this storage raster.
        // Gamma is 1, so the ISO reciprocal exponent matches code ^ gamma.
        for index in 0..12 {
            let channel = index % 3;
            let sdr = f64::from(base.data[index]) / 255.0;
            let linear = if sdr <= 0.04045 {
                sdr / 12.92
            } else {
                ((sdr + 0.055) / 1.055).powf(2.4)
            };
            let hdr =
                linear * [2.0_f64, 4.0, 8.0][channel].powf(f64::from(gain.data[index]) / 255.0);
            let normalized = hdr / f64::from(scene.linear_metadata.baseline_exposure);
            let code = xdremux_engine::apple_log_encode(normalized);
            let expected = (code.clamp(0.0, 1.0) * 65_535.0).round() as u16;
            let actual =
                u16::from_le_bytes([thumbnail.data[index * 2], thumbnail.data[index * 2 + 1]]);
            assert_eq!(actual, expected, "RGB index {index}");
        }
    }

    #[test]
    fn style_scene_preserves_every_rgb8_code_and_row_stride() {
        let width = 256;
        let stride = width * 3 + 7;
        let mut pixels = vec![0x7f; stride * 2];
        let mut expected = Vec::with_capacity(width * 2 * 3);
        for row in 0..2 {
            for code in 0_u8..=255 {
                let channels = if row == 0 {
                    [code, 255 - code, code.wrapping_add(31)]
                } else {
                    [code.wrapping_add(73), code, 255 - code]
                };
                let offset = row * stride + usize::from(code) * 3;
                pixels[offset..offset + 3].copy_from_slice(&channels);
                for channel in channels {
                    // Preserve the original scalar operation at runtime, even
                    // in optimized builds; do not constant-fold the reference.
                    let encoded = std::hint::black_box(f32::from(channel) / 255.0);
                    expected.push(if encoded <= 0.040_45 {
                        encoded / 12.92
                    } else {
                        ((encoded + 0.055) / 1.055).powf(2.4)
                    });
                }
            }
        }
        let base = Raster8::new(width as u32, 2, stride, RasterPixelFormat::Rgb8, pixels).unwrap();
        let gain = Raster8::new(1, 1, 1, RasterPixelFormat::Mono8, vec![128]).unwrap();
        let mut parameters = vec![0.0; 20];
        parameters[0..3].fill(1.0);
        parameters[4..7].fill(2.0);
        parameters[7..10].fill(1.0);
        parameters[16] = 1.0;
        parameters[17] = 2.0;
        parameters[18] = 2.0;
        let scale = xdremux_hdr::resolve(&parameters, xdremux_hdr::ExtractionMode::Uhdr).unwrap();
        let scene = derive_style_scene(&base, &gain, &scale, width as u32).unwrap();
        assert_eq!((scene.width, scene.height), (width, 2));
        assert_eq!(scene.base_rgb.len(), expected.len());
        for (index, (actual, expected)) in scene.base_rgb.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "RGB component {index}"
            );
        }
    }

    #[test]
    fn style_scene_applies_gain_at_full_resolution_before_area_resize() {
        let mut pixels = vec![255_u8; 4 * 2 * 3];
        pixels[3] = 0;
        pixels[4] = 0;
        pixels[5] = 0;
        let base = Raster8::new(4, 2, 12, RasterPixelFormat::Rgb8, pixels).unwrap();
        let gain = Raster8::new(2, 2, 2, RasterPixelFormat::Mono8, vec![255; 4]).unwrap();
        let mut parameters = vec![0.0; 20];
        parameters[0..3].fill(1.0);
        parameters[4..7].fill(2.0);
        parameters[7..10].fill(1.0);
        parameters[16] = 1.0;
        parameters[17] = 2.0;
        parameters[18] = 2.0;
        let scale = xdremux_hdr::resolve(&parameters, xdremux_hdr::ExtractionMode::Uhdr).unwrap();
        let scene = derive_style_scene(&base, &gain, &scale, 2).unwrap();
        assert_eq!((scene.width, scene.height), (2, 2));
        // The black source pixel shares a 2x1 footprint with a white pixel.
        // Full-resolution gain application then area averaging yields 1.
        // Downsampling the SDR first with nearest sampling keeps only white
        // and stays at 2.
        assert!((scene.hdr_rgb[0] - 1.0).abs() < 1.0e-5);
        assert!((scene.hdr_rgb[3] - 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn identity_statistics_match_percentile_contract() {
        let values = [0.0, 0.25, 0.5, 0.75, 1.0];
        let distribution = xdremux_engine::apple_style_distribution(&values);
        assert_eq!(distribution.black_point, 0.005);
        assert_eq!(distribution.p50, 0.5);
        assert_eq!(distribution.white_point, 0.995);
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
    fn semantic_matte_dimensions_match_scaffold_fit_for_all_orientation_classes() {
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
