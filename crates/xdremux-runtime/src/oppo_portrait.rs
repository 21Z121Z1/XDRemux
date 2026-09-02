use xdremux_container::{OppoPortraitConfig, OppoPortraitDepth};
use xdremux_engine::{
    AppleGainMapFacts, AppleL8Mask, ApplePortraitCameraCalibration, ApplePortraitDisparity,
};

#[cfg(any(target_os = "macos", test))]
use crate::{Result, RuntimeError};
#[cfg(any(target_os = "macos", test))]
use ruzstd::decoding::StreamingDecoder;
#[cfg(any(target_os = "macos", test))]
use std::io::Read;
#[cfg(any(target_os = "macos", test))]
use xdremux_container::{extract_oppo_portrait_source, parse_oppo_portrait_depth};
#[cfg(any(target_os = "macos", test))]
use xdremux_format::{jpeg_image_end, probe_jpeg_frame_profile};

#[cfg(target_os = "macos")]
use std::io::Write;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use xdremux_engine::{
    build_apple_portrait_disparity, derive_apple_portrait_camera_calibration,
    fuse_apple_portrait_hair_mask, fuse_apple_portrait_person_mask,
    resolve_apple_portrait_base_orientation, ApplePortraitCaptureFacts, ApplePortraitImageGeometry,
    AppleSemanticRole,
};

#[cfg(target_os = "macos")]
use crate::apple_adapter::{AppleAdapterClient, AppleImageProperties};

#[cfg(any(target_os = "macos", test))]
const MAX_DECODED_PORTRAIT_DEPTH_BYTES: u64 = 256 * 1024 * 1024;
#[cfg(target_os = "macos")]
const OPPO_PORTRAIT_PRIOR_SPATIAL_SIGMA: f32 = 3.0;
#[cfg(target_os = "macos")]
const OPPO_PORTRAIT_PRIOR_LUMA_SIGMA: f32 = 0.15;

#[derive(Debug, Clone, PartialEq)]
pub struct ApplePortraitSourcePreflight {
    pub base_jpeg: Vec<u8>,
    pub gain_map_jpeg: Vec<u8>,
    pub depth: OppoPortraitDepth,
    pub config: OppoPortraitConfig,
    pub private_gain_map_info: Option<Vec<u8>>,
    pub base_width: u32,
    pub base_height: u32,
    pub gain_map: AppleGainMapFacts,
    pub base_orientation: u8,
    pub camera_calibration: ApplePortraitCameraCalibration,
    pub disparity: ApplePortraitDisparity,
    pub portrait_effects_matte: AppleL8Mask,
    pub subject_prior_used: bool,
    pub hair_matte: AppleL8Mask,
    pub hair_prior_added_high_confidence: bool,
    pub simulated_aperture: f64,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SplitPortraitSourceImage {
    base_jpeg: Vec<u8>,
    gain_map_jpeg: Vec<u8>,
    base_width: u32,
    base_height: u32,
    gain_map_width: u32,
    gain_map_height: u32,
}

#[cfg(any(target_os = "macos", test))]
fn split_portrait_source_image(source_image: &[u8]) -> Result<SplitPortraitSourceImage> {
    let base_end = jpeg_image_end(source_image, 0)
        .map_err(|error| RuntimeError::external("Portrait src.image base JPEG", error))?;
    let second_marker_end = base_end.checked_add(3).ok_or_else(|| {
        RuntimeError::new("Portrait src.image", "second JPEG marker offset overflows")
    })?;
    if source_image.get(base_end..second_marker_end) != Some(&[0xff, 0xd8, 0xff]) {
        return Err(RuntimeError::new(
            "Portrait src.image",
            "does not contain adjacent base and Gain Map JPEGs",
        ));
    }

    jpeg_image_end(source_image, base_end)
        .map_err(|error| RuntimeError::external("Portrait src.image Gain Map JPEG", error))?;

    let base_jpeg = source_image
        .get(..base_end)
        .ok_or_else(|| RuntimeError::new("Portrait src.image", "base JPEG is out of bounds"))?
        .to_vec();
    let gain_map_jpeg = source_image
        .get(base_end..)
        .ok_or_else(|| RuntimeError::new("Portrait src.image", "Gain Map JPEG is out of bounds"))?
        .to_vec();
    let base_profile = probe_jpeg_frame_profile(&base_jpeg)
        .map_err(|error| RuntimeError::external("Portrait src.image base JPEG profile", error))?;
    let gain_profile = probe_jpeg_frame_profile(&gain_map_jpeg).map_err(|error| {
        RuntimeError::external("Portrait src.image Gain Map JPEG profile", error)
    })?;

    Ok(SplitPortraitSourceImage {
        base_jpeg,
        gain_map_jpeg,
        base_width: u32::from(base_profile.width),
        base_height: u32::from(base_profile.height),
        gain_map_width: u32::from(gain_profile.width),
        gain_map_height: u32::from(gain_profile.height),
    })
}

#[cfg(any(target_os = "macos", test))]
fn decode_oppo_portrait_depth(compressed: &[u8]) -> Result<OppoPortraitDepth> {
    if compressed.is_empty() {
        return Err(RuntimeError::new(
            "OPPO Portrait depth",
            "compressed rear.depth is empty",
        ));
    }

    let decoder = StreamingDecoder::new(compressed)
        .map_err(|error| RuntimeError::external("OPPO Portrait zstd decoder", error))?;
    let mut bounded = decoder.take(MAX_DECODED_PORTRAIT_DEPTH_BYTES + 1);
    let mut decoded = Vec::new();
    bounded
        .read_to_end(&mut decoded)
        .map_err(|error| RuntimeError::external("OPPO Portrait zstd decode", error))?;
    if u64::try_from(decoded.len()).unwrap_or(u64::MAX) > MAX_DECODED_PORTRAIT_DEPTH_BYTES {
        return Err(RuntimeError::new(
            "OPPO Portrait depth",
            format!(
                "decoded rear.depth exceeds {} MiB safety limit",
                MAX_DECODED_PORTRAIT_DEPTH_BYTES / (1024 * 1024)
            ),
        ));
    }

    parse_oppo_portrait_depth(&decoded)
        .map_err(|error| RuntimeError::external("OPPO Portrait depth parse", error))
}

#[cfg(target_os = "macos")]
fn positive(primary: Option<f64>, fallback: Option<f64>) -> Option<f64> {
    primary
        .filter(|value| value.is_finite() && *value > 0.0)
        .or_else(|| fallback.filter(|value| value.is_finite() && *value > 0.0))
}

#[cfg(target_os = "macos")]
fn nonempty_string(primary: Option<&str>, fallback: Option<&str>) -> Option<String> {
    primary
        .filter(|value| !value.is_empty())
        .or_else(|| fallback.filter(|value| !value.is_empty()))
        .map(ToOwned::to_owned)
}

#[cfg(target_os = "macos")]
fn orientation(properties: &AppleImageProperties) -> Option<u8> {
    properties
        .orientation
        .and_then(|value| u8::try_from(value).ok())
}

#[cfg(any(target_os = "macos", test))]
fn resolve_simulated_aperture(
    config_version: f32,
    current_f_number: Option<f32>,
    input_f_number: Option<f64>,
    base_f_number: Option<f64>,
) -> f64 {
    if (config_version - 4.0).abs() < 0.001 {
        if let Some(value) = current_f_number
            .map(f64::from)
            .filter(|value| value.is_finite() && (1.0..=32.0).contains(value))
        {
            return value;
        }
    }

    input_f_number
        .filter(|value| value.is_finite() && (1.0..=32.0).contains(value))
        .or_else(|| base_f_number.filter(|value| value.is_finite() && (1.0..=32.0).contains(value)))
        .unwrap_or(1.4)
}

#[cfg(target_os = "macos")]
pub(crate) fn prepare_apple_portrait_source(
    adapter_executable: &Path,
    input: &[u8],
) -> Result<ApplePortraitSourcePreflight> {
    let source = extract_oppo_portrait_source(input)
        .map_err(|error| RuntimeError::external("OPPO Portrait source extraction", error))?;
    let split = split_portrait_source_image(&source.source_image)?;
    let depth = decode_oppo_portrait_depth(&source.compressed_depth)?;

    let mut source_image_file = tempfile::Builder::new()
        .prefix("xdremux-portrait-src-")
        .suffix(".jpg")
        .tempfile()
        .map_err(|error| RuntimeError::external("Portrait src.image temporary file", error))?;
    source_image_file
        .write_all(&source.source_image)
        .map_err(|error| RuntimeError::external("Portrait src.image temporary write", error))?;
    source_image_file
        .flush()
        .map_err(|error| RuntimeError::external("Portrait src.image temporary flush", error))?;

    let mut input_file = tempfile::Builder::new()
        .prefix("xdremux-portrait-input-")
        .suffix(".heic")
        .tempfile()
        .map_err(|error| RuntimeError::external("Portrait input temporary file", error))?;
    input_file
        .write_all(input)
        .map_err(|error| RuntimeError::external("Portrait input temporary write", error))?;
    input_file
        .flush()
        .map_err(|error| RuntimeError::external("Portrait input temporary flush", error))?;

    let adapter = AppleAdapterClient::new(adapter_executable.to_path_buf());
    let gain_map = adapter.imageio_gain_map_facts(source_image_file.path())?;
    if !gain_map.supports_portrait_source() {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            format!(
                "unsupported ImageIO Gain Map pixel format {}",
                gain_map.pixel_format
            ),
        ));
    }
    if !gain_map.has_geometry(split.gain_map_width, split.gain_map_height) {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            format!(
                "ImageIO Gain Map geometry {}x{} does not match JPEG {}x{}",
                gain_map.width, gain_map.height, split.gain_map_width, split.gain_map_height
            ),
        ));
    }

    let input_properties = adapter.imageio_image_properties(input_file.path())?;
    let base_properties = adapter.imageio_image_properties(source_image_file.path())?;
    if base_properties.width != split.base_width || base_properties.height != split.base_height {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            format!(
                "ImageIO base geometry {}x{} does not match JPEG {}x{}",
                base_properties.width, base_properties.height, split.base_width, split.base_height
            ),
        ));
    }

    let physical_focal_length_mm = positive(
        input_properties.focal_length_mm,
        base_properties.focal_length_mm,
    )
    .ok_or_else(|| {
        RuntimeError::new(
            "Apple Portrait calibration",
            "EXIF FocalLength is required on the input or portrait base image",
        )
    })?;
    let equivalent_focal_length_mm = positive(
        input_properties.focal_length_in_35mm_film,
        base_properties.focal_length_in_35mm_film,
    )
    .ok_or_else(|| {
        RuntimeError::new(
            "Apple Portrait calibration",
            "EXIF FocalLengthIn35mmFilm is required on the input or portrait base image",
        )
    })?;
    let capture_facts = ApplePortraitCaptureFacts {
        physical_focal_length_mm,
        equivalent_focal_length_mm,
        digital_zoom_ratio: positive(
            input_properties.digital_zoom_ratio,
            base_properties.digital_zoom_ratio,
        ),
        lens_model: nonempty_string(
            input_properties.lens_model.as_deref(),
            base_properties.lens_model.as_deref(),
        ),
        base_width: split.base_width,
        base_height: split.base_height,
    };
    let camera_calibration = derive_apple_portrait_camera_calibration(&capture_facts)
        .map_err(|error| RuntimeError::external("Apple Portrait calibration", error))?;

    let base_orientation = resolve_apple_portrait_base_orientation(
        Some(ApplePortraitImageGeometry {
            width: input_properties.width,
            height: input_properties.height,
            orientation: orientation(&input_properties),
        }),
        ApplePortraitImageGeometry {
            width: split.base_width,
            height: split.base_height,
            orientation: orientation(&base_properties),
        },
    )
    .map_err(|error| RuntimeError::external("Apple Portrait orientation", error))?;

    let disparity = build_apple_portrait_disparity(
        &depth.ranks,
        depth.header.width,
        depth.header.height,
        depth.header.rank_disparity_scale,
        depth.header.disparity_exponentiation,
    )
    .map_err(|error| RuntimeError::external("Apple Portrait disparity", error))?;

    let target_width = split.base_width / 2;
    let target_height = split.base_height / 2;
    if target_width == 0 || target_height == 0 {
        return Err(RuntimeError::new(
            "Apple Portrait semantic matte",
            "half-resolution target geometry is invalid",
        ));
    }

    // Producer semantic planes are topology priors only. Rust owns whether they
    // participate in final Apple mattes; Core Image executes only the shared
    // edge-preserving resize primitive.
    let subject_prior = if let Some(portrait_plane) = depth.portrait.as_ref() {
        let small_mask = AppleL8Mask::new(
            depth.header.width,
            depth.header.height,
            portrait_plane.clone(),
        )
        .map_err(|error| RuntimeError::external("OPPO Portrait subject prior", error))?;
        Some(adapter.coreimage_edge_preserve_upsample_l8(
            source_image_file.path(),
            &small_mask,
            target_width,
            target_height,
            OPPO_PORTRAIT_PRIOR_SPATIAL_SIGMA,
            OPPO_PORTRAIT_PRIOR_LUMA_SIGMA,
        )?)
    } else {
        None
    };
    let hair_prior = if let Some(hair_plane) = depth
        .hair
        .as_ref()
        .filter(|plane| plane.iter().any(|&pixel| pixel != 0))
    {
        let small_mask =
            AppleL8Mask::new(depth.header.width, depth.header.height, hair_plane.clone())
                .map_err(|error| RuntimeError::external("OPPO Portrait hair prior", error))?;
        Some(adapter.coreimage_edge_preserve_upsample_l8(
            source_image_file.path(),
            &small_mask,
            target_width,
            target_height,
            OPPO_PORTRAIT_PRIOR_SPATIAL_SIGMA,
            OPPO_PORTRAIT_PRIOR_LUMA_SIGMA,
        )?)
    } else {
        None
    };

    // Vision reports native semantic observations. Rust chooses the requested
    // roles, orientation and target geometry, then Core Image reproduces the
    // legacy stored-pixel transform before Rust-owned fusion policy runs.
    let mut vision_masks = adapter.vision_semantic_mattes(
        source_image_file.path(),
        &[AppleSemanticRole::Person, AppleSemanticRole::Hair],
        Some(u32::from(base_orientation)),
    )?;
    let native_person = vision_masks
        .remove(&AppleSemanticRole::Person)
        .ok_or_else(|| {
            RuntimeError::new(
                "Apple Portrait person matte",
                "Vision omitted the requested person matte",
            )
        })?;
    let native_hair = vision_masks
        .remove(&AppleSemanticRole::Hair)
        .ok_or_else(|| {
            RuntimeError::new(
                "Apple Portrait hair matte",
                "Vision omitted the requested hair matte",
            )
        })?;
    let rendered_person = adapter.coreimage_render_l8(
        &native_person,
        target_width,
        target_height,
        base_orientation,
    )?;
    let rendered_hair =
        adapter.coreimage_render_l8(&native_hair, target_width, target_height, base_orientation)?;
    let person_fusion =
        fuse_apple_portrait_person_mask(&rendered_person, subject_prior.as_ref())
            .map_err(|error| RuntimeError::external("Apple Portrait person fusion", error))?;
    let hair_fusion =
        fuse_apple_portrait_hair_mask(&rendered_hair, hair_prior.as_ref(), &person_fusion.mask)
            .map_err(|error| RuntimeError::external("Apple Portrait hair fusion", error))?;

    let simulated_aperture = resolve_simulated_aperture(
        source.config.version,
        source.config.current_f_number,
        input_properties.f_number,
        base_properties.f_number,
    );

    Ok(ApplePortraitSourcePreflight {
        base_jpeg: split.base_jpeg,
        gain_map_jpeg: split.gain_map_jpeg,
        depth,
        config: source.config,
        private_gain_map_info: source.private_gain_map_info,
        base_width: split.base_width,
        base_height: split.base_height,
        gain_map,
        base_orientation,
        camera_calibration,
        disparity,
        portrait_effects_matte: person_fusion.mask,
        subject_prior_used: person_fusion.used_prior,
        hair_matte: hair_fusion.mask,
        hair_prior_added_high_confidence: hair_fusion.prior_added_high_confidence,
        simulated_aperture,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn portrait_fixture() -> Vec<u8> {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic");
        fs::read(fixture).expect("read committed portrait fixture")
    }

    #[test]
    fn splits_committed_portrait_source_with_hardened_jpeg_boundaries() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let split = split_portrait_source_image(&source.source_image)
            .expect("split adjacent base/Gain Map JPEGs");

        assert!(split.base_width > 0);
        assert!(split.base_height > 0);
        assert!(split.gain_map_width > 0);
        assert!(split.gain_map_height > 0);
        assert_eq!(split.base_jpeg.get(..2), Some(&[0xff, 0xd8][..]));
        assert_eq!(split.gain_map_jpeg.get(..2), Some(&[0xff, 0xd8][..]));
    }

    #[test]
    fn decodes_committed_portrait_depth_without_external_zstd() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let depth = decode_oppo_portrait_depth(&source.compressed_depth)
            .expect("decode and parse OPPO Portrait depth");

        assert!(depth.header.width > 0);
        assert!(depth.header.height > 0);
        assert!(!depth.ranks.is_empty());
        assert_eq!(
            depth.ranks.len(),
            usize::try_from(depth.header.width).unwrap()
                * usize::try_from(depth.header.height).unwrap()
        );
    }

    #[test]
    fn simulated_aperture_matches_the_swift_oracle_precedence() {
        assert_eq!(
            resolve_simulated_aperture(4.0, Some(2.8), Some(1.7), Some(1.9)),
            f64::from(2.8_f32)
        );
        assert_eq!(
            resolve_simulated_aperture(3.0, Some(2.8), Some(1.7), Some(1.9)),
            1.7
        );
        assert_eq!(
            resolve_simulated_aperture(4.0, Some(48.0), Some(1.7), Some(1.9)),
            1.7
        );
        assert_eq!(
            resolve_simulated_aperture(4.0, None, Some(f64::NAN), Some(2.0)),
            2.0
        );
        assert_eq!(resolve_simulated_aperture(4.0, None, None, None), 1.4);
    }

    #[test]
    fn rejects_one_jpeg_from_the_portrait_pair_as_a_complete_source_image() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let split = split_portrait_source_image(&source.source_image)
            .expect("split adjacent base/Gain Map JPEGs");

        assert!(split_portrait_source_image(&split.base_jpeg).is_err());
    }

    #[test]
    fn rejects_empty_compressed_depth() {
        assert!(decode_oppo_portrait_depth(&[]).is_err());
    }
}
