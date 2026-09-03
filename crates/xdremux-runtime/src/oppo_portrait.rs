use crate::{Result, RuntimeError};
#[cfg(target_os = "macos")]
use xdremux_container::select_oppo_portrait_focus;
use xdremux_container::{
    OppoPortraitConfig, OppoPortraitDepth, OppoPortraitFocusRegion, OppoPortraitFocusSelection,
};
use xdremux_engine::{
    build_apple_portrait_disparity_payload, build_apple_portrait_effects_matte_payload,
    build_apple_portrait_rendering_parameters, build_apple_semantic_matte_payload,
    AppleAuxiliaryPayload, AppleGainMapFacts, AppleL8Mask, ApplePortraitCameraCalibration,
    ApplePortraitDisparity, AppleSemanticRole,
};

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
    APPLE_PORTRAIT_SEMANTIC_ROLES,
};

#[cfg(target_os = "macos")]
use crate::apple_adapter::{AppleAdapterClient, AppleImageProperties};

#[cfg(any(target_os = "macos", test))]
const MAX_DECODED_PORTRAIT_DEPTH_BYTES: u64 = 256 * 1024 * 1024;
const PRIVATE_GAIN_MAP_INFO_BYTES: usize = 20 * std::mem::size_of::<f32>();
const PRIVATE_GAIN_MAP_ALTERNATE_HEADROOM_INDEX: usize = 17;
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
    pub focus_region: OppoPortraitFocusRegion,
    pub focus: OppoPortraitFocusSelection,
    pub private_gain_map_info: Option<Vec<u8>>,
    pub base_width: u32,
    pub base_height: u32,
    pub gain_map: AppleGainMapFacts,
    pub base_orientation: u8,
    pub camera_calibration: ApplePortraitCameraCalibration,
    pub disparity: ApplePortraitDisparity,
    pub portrait_effects_matte: AppleL8Mask,
    pub subject_prior_used: bool,
    pub skin_matte: AppleL8Mask,
    pub hair_matte: AppleL8Mask,
    pub hair_prior_added_high_confidence: bool,
    pub teeth_matte: AppleL8Mask,
    pub glasses_matte: AppleL8Mask,
    pub simulated_aperture: f64,
}

impl ApplePortraitSourcePreflight {
    /// Consume a completed Rust-owned Portrait preflight into the exact
    /// auxiliary resource set required by Apple Photos Portrait editing.
    ///
    /// The caller supplies no REND or Apple product policy. Rust derives the
    /// producer focus state, Gain Map headroom and per-image rendering
    /// parameters, while ImageIO remains only the platform writer.
    pub fn into_auxiliary_payloads(self) -> Result<Vec<AppleAuxiliaryPayload>> {
        let disparity_span = f64::from(self.disparity.near - self.disparity.far);
        let focus_disparity = self
            .disparity
            .focus_disparity(
                self.focus.selected_rank,
                self.depth.header.disparity_exponentiation,
            )
            .map_err(|error| RuntimeError::external("Apple Portrait focus disparity", error))?;
        let gain_map_headroom = private_gain_map_headroom(self.private_gain_map_info.as_deref())?;
        let rendering_parameters = build_apple_portrait_rendering_parameters(
            self.camera_calibration.profile,
            focus_disparity,
            disparity_span,
            gain_map_headroom,
            self.config.aec_lux_index.map(f64::from),
            self.depth.header.near_object_detected,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait REND", error))?;

        let mut payloads = Vec::with_capacity(6);
        payloads.push(
            build_apple_portrait_disparity_payload(
                self.disparity,
                self.base_orientation,
                &self.camera_calibration,
                &rendering_parameters,
                self.simulated_aperture,
            )
            .map_err(|error| RuntimeError::external("Apple Portrait disparity payload", error))?,
        );
        payloads.push(
            build_apple_portrait_effects_matte_payload(
                self.portrait_effects_matte.width,
                self.portrait_effects_matte.height,
                self.portrait_effects_matte.pixels,
            )
            .map_err(|error| {
                RuntimeError::external("Apple Portrait effects matte payload", error)
            })?,
        );
        for (role, matte) in [
            (AppleSemanticRole::Skin, self.skin_matte),
            (AppleSemanticRole::Hair, self.hair_matte),
            (AppleSemanticRole::Teeth, self.teeth_matte),
            (AppleSemanticRole::Glasses, self.glasses_matte),
        ] {
            payloads.push(
                build_apple_semantic_matte_payload(role, matte.width, matte.height, matte.pixels)
                    .map_err(|error| {
                    RuntimeError::external("Apple Portrait semantic matte payload", error)
                })?,
            );
        }
        Ok(payloads)
    }
}

#[cfg(any(target_os = "macos", test))]
fn producer_focus_region(
    config: &OppoPortraitConfig,
    base_width: u32,
    base_height: u32,
) -> Result<OppoPortraitFocusRegion> {
    if base_width == 0 || base_height == 0 {
        return Err(RuntimeError::new(
            "Apple Portrait focus",
            "base image geometry is invalid",
        ));
    }
    let x = f64::from(config.focus_x) / f64::from(base_width);
    let y = f64::from(config.focus_y) / f64::from(base_height);
    if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
        return Err(RuntimeError::new(
            "Apple Portrait focus",
            "OPPO producer focus point is outside the portrait base image",
        ));
    }
    let rectangle = config
        .focus_rectangle
        .filter(|_| config.focus_rectangle_is_valid);
    let width = rectangle
        .and_then(|rectangle| {
            let delta = i64::from(rectangle[2]).checked_sub(i64::from(rectangle[0]))?;
            let value = delta.unsigned_abs() as f64 / f64::from(base_width);
            value.is_finite().then_some(value)
        })
        .unwrap_or(0.12)
        .clamp(0.02, 1.0);
    let height = rectangle
        .and_then(|rectangle| {
            let delta = i64::from(rectangle[3]).checked_sub(i64::from(rectangle[1]))?;
            let value = delta.unsigned_abs() as f64 / f64::from(base_height);
            value.is_finite().then_some(value)
        })
        .unwrap_or(0.12)
        .clamp(0.02, 1.0);
    Ok(OppoPortraitFocusRegion {
        x,
        y,
        width,
        height,
    })
}

fn private_gain_map_headroom(private_info: Option<&[u8]>) -> Result<f64> {
    let private_info = private_info.ok_or_else(|| {
        RuntimeError::new(
            "Apple Portrait Gain Map headroom",
            "private local.uhdr.gainmap.info is unavailable",
        )
    })?;
    if private_info.len() != PRIVATE_GAIN_MAP_INFO_BYTES {
        return Err(RuntimeError::new(
            "Apple Portrait Gain Map headroom",
            format!(
                "private gain info has {} bytes; expected {PRIVATE_GAIN_MAP_INFO_BYTES}",
                private_info.len()
            ),
        ));
    }
    let offset = PRIVATE_GAIN_MAP_ALTERNATE_HEADROOM_INDEX * std::mem::size_of::<f32>();
    let raw: [u8; 4] = private_info[offset..offset + 4]
        .try_into()
        .expect("validated private gain-info length");
    let alternate_headroom_ratio = f64::from(f32::from_le_bytes(raw));
    if !alternate_headroom_ratio.is_finite() || alternate_headroom_ratio <= 0.0 {
        return Err(RuntimeError::new(
            "Apple Portrait Gain Map headroom",
            "private alternate headroom ratio must be finite and positive",
        ));
    }
    let headroom = alternate_headroom_ratio.max(1.0).log2().max(0.0);
    if !headroom.is_finite() {
        return Err(RuntimeError::new(
            "Apple Portrait Gain Map headroom",
            "derived Gain Map headroom is not finite",
        ));
    }
    Ok(headroom)
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

    let focus_region = producer_focus_region(&source.config, split.base_width, split.base_height)?;
    let focus = select_oppo_portrait_focus(
        &depth,
        &source.config,
        split.base_width,
        split.base_height,
        focus_region,
    )
    .map_err(|error| RuntimeError::external("Apple Portrait focus selection", error))?;

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

    // Vision reports native semantic observations. Rust chooses the complete
    // Portrait role set, orientation and target geometry. Core Image only
    // reproduces the stored-pixel transform; product validity and fusion remain
    // Rust-owned.
    let mut vision_masks = adapter.vision_semantic_mattes(
        source_image_file.path(),
        &APPLE_PORTRAIT_SEMANTIC_ROLES,
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
    if !native_person.has_credible_foreground() {
        return Err(RuntimeError::new(
            "Apple Portrait unavailable",
            "Vision returned no credible person foreground",
        ));
    }
    let rendered_person = adapter.coreimage_render_l8(
        &native_person,
        target_width,
        target_height,
        base_orientation,
    )?;

    let mut render_role = |role| -> Result<AppleL8Mask> {
        let native = vision_masks.remove(&role).ok_or_else(|| {
            RuntimeError::new(
                "Apple Portrait semantic matte",
                format!("Vision omitted the requested {role:?} matte"),
            )
        })?;
        adapter.coreimage_render_l8(&native, target_width, target_height, base_orientation)
    };
    let rendered_skin = render_role(AppleSemanticRole::Skin)?;
    let rendered_hair = render_role(AppleSemanticRole::Hair)?;
    let rendered_teeth = render_role(AppleSemanticRole::Teeth)?;
    let rendered_glasses = render_role(AppleSemanticRole::Glasses)?;

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
        focus_region,
        focus,
        private_gain_map_info: source.private_gain_map_info,
        base_width: split.base_width,
        base_height: split.base_height,
        gain_map,
        base_orientation,
        camera_calibration,
        disparity,
        portrait_effects_matte: person_fusion.mask,
        subject_prior_used: person_fusion.used_prior,
        skin_matte: rendered_skin,
        hair_matte: hair_fusion.mask,
        hair_prior_added_high_confidence: hair_fusion.prior_added_high_confidence,
        teeth_matte: rendered_teeth,
        glasses_matte: rendered_glasses,
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
    fn private_gain_map_headroom_matches_the_swift_stop_mapping() {
        let expected = 3.466_976_881_027_221_7_f64;
        let ratio = 2.0_f32.powf(expected as f32);
        let mut info = vec![0_u8; PRIVATE_GAIN_MAP_INFO_BYTES];
        let offset = PRIVATE_GAIN_MAP_ALTERNATE_HEADROOM_INDEX * std::mem::size_of::<f32>();
        info[offset..offset + 4].copy_from_slice(&ratio.to_le_bytes());
        let actual = private_gain_map_headroom(Some(&info)).expect("derive Gain Map headroom");
        assert!(
            (actual - expected).abs() < 1e-5,
            "actual={actual} expected={expected}"
        );
        assert!(private_gain_map_headroom(None).is_err());
        assert!(private_gain_map_headroom(Some(&info[..info.len() - 1])).is_err());
    }

    #[test]
    fn producer_focus_region_uses_the_stored_oppo_focus_point() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let split = split_portrait_source_image(&source.source_image)
            .expect("split adjacent base/Gain Map JPEGs");
        let mut config = source.config;
        config.focus_rectangle = None;
        config.focus_rectangle_is_valid = false;
        let focus = producer_focus_region(&config, split.base_width, split.base_height)
            .expect("resolve producer focus region");
        assert_eq!(
            focus.x,
            f64::from(config.focus_x) / f64::from(split.base_width)
        );
        assert_eq!(
            focus.y,
            f64::from(config.focus_y) / f64::from(split.base_height)
        );
        assert_eq!(focus.width, 0.12);
        assert_eq!(focus.height, 0.12);
    }

    #[test]
    fn producer_focus_region_uses_valid_focus_rectangle_dimensions() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let split = split_portrait_source_image(&source.source_image)
            .expect("split adjacent base/Gain Map JPEGs");
        let mut config = source.config;
        config.focus_rectangle = Some([10, 20, 110, 220]);
        config.focus_rectangle_is_valid = true;

        let focus = producer_focus_region(&config, split.base_width, split.base_height)
            .expect("resolve producer focus region with rectangle");
        assert!((focus.width - 100.0 / f64::from(split.base_width)).abs() < 1e-12);
        assert!((focus.height - 200.0 / f64::from(split.base_height)).abs() < 1e-12);
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
