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
#[cfg(target_os = "macos")]
use xdremux_engine::{
    build_apple_portrait_disparity, derive_apple_portrait_camera_calibration,
    fuse_apple_portrait_hair_mask, fuse_apple_portrait_person_mask,
    resolve_apple_portrait_base_orientation, ApplePortraitCaptureFacts, ApplePortraitImageGeometry,
    APPLE_PORTRAIT_SEMANTIC_ROLES,
};

#[cfg(target_os = "macos")]
use crate::apple_adapter::{
    AppleAdapterClient, AppleImageProperties, APPLE_COMPUTE_TIMEOUT, PYTORCH_DEPTHPRO2_TIMEOUT,
};

#[cfg(any(target_os = "macos", test))]
const MAX_DECODED_PORTRAIT_DEPTH_BYTES: u64 = 256 * 1024 * 1024;
const PRIVATE_GAIN_MAP_INFO_BYTES: usize = 20 * std::mem::size_of::<f32>();
const PRIVATE_GAIN_MAP_ALTERNATE_HEADROOM_INDEX: usize = 17;
#[cfg(target_os = "macos")]
const OPPO_PORTRAIT_PRIOR_SPATIAL_SIGMA: f32 = 3.0;
#[cfg(target_os = "macos")]
const OPPO_PORTRAIT_PRIOR_LUMA_SIGMA: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApplePortraitSemanticProfile {
    /// Preserve the established full Vision-backed resource contract.
    #[default]
    Complete,
    /// Use only source-provided semantic planes; never invoke Vision.
    OppoNative,
}

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
    pub portrait_effects_matte: Option<AppleL8Mask>,
    pub subject_prior_used: bool,
    pub semantic_profile: ApplePortraitSemanticProfile,
    pub skin_matte: Option<AppleL8Mask>,
    pub hair_matte: Option<AppleL8Mask>,
    pub hair_prior_added_high_confidence: bool,
    pub teeth_matte: Option<AppleL8Mask>,
    pub glasses_matte: Option<AppleL8Mask>,
    pub simulated_aperture: f64,
}

impl ApplePortraitSourcePreflight {
    /// Consume a completed Rust-owned Portrait preflight into the resource set
    /// selected by `semantic_profile`.
    pub fn into_auxiliary_payloads(self) -> Result<Vec<AppleAuxiliaryPayload>> {
        if self.semantic_profile == ApplePortraitSemanticProfile::Complete
            && (self.portrait_effects_matte.is_none() || self.hair_matte.is_none())
        {
            return Err(RuntimeError::new(
                "Apple Portrait semantic matte",
                "complete profile requires portrait and hair mattes",
            ));
        }
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

        let mut payloads = Vec::with_capacity(match self.semantic_profile {
            ApplePortraitSemanticProfile::Complete => 6,
            ApplePortraitSemanticProfile::OppoNative => 3,
        });
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
        if let Some(matte) = self.portrait_effects_matte {
            payloads.push(
                build_apple_portrait_effects_matte_payload(matte.width, matte.height, matte.pixels)
                    .map_err(|error| {
                        RuntimeError::external("Apple Portrait effects matte payload", error)
                    })?,
            );
        }

        let mut push_semantic = |role: AppleSemanticRole, matte: AppleL8Mask| -> Result<()> {
            payloads.push(
                build_apple_semantic_matte_payload(role, matte.width, matte.height, matte.pixels)
                    .map_err(|error| {
                    RuntimeError::external("Apple Portrait semantic matte payload", error)
                })?,
            );
            Ok(())
        };

        match self.semantic_profile {
            ApplePortraitSemanticProfile::Complete => {
                let skin = self.skin_matte.ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait semantic matte",
                        "complete profile is missing skin matte",
                    )
                })?;
                let teeth = self.teeth_matte.ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait semantic matte",
                        "complete profile is missing teeth matte",
                    )
                })?;
                let glasses = self.glasses_matte.ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait semantic matte",
                        "complete profile is missing glasses matte",
                    )
                })?;
                push_semantic(AppleSemanticRole::Skin, skin)?;
                if let Some(hair) = self.hair_matte {
                    push_semantic(AppleSemanticRole::Hair, hair)?;
                }
                push_semantic(AppleSemanticRole::Teeth, teeth)?;
                push_semantic(AppleSemanticRole::Glasses, glasses)?;
            }
            ApplePortraitSemanticProfile::OppoNative => {
                if self.skin_matte.is_some()
                    || self.teeth_matte.is_some()
                    || self.glasses_matte.is_some()
                {
                    return Err(RuntimeError::new(
                        "OPPO-native Portrait",
                        "profile must not synthesize skin, teeth or glasses mattes",
                    ));
                }
                if let Some(hair) = self.hair_matte {
                    push_semantic(AppleSemanticRole::Hair, hair)?;
                }
            }
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
    adapter: &AppleAdapterClient,
    input: &[u8],
) -> Result<ApplePortraitSourcePreflight> {
    prepare_apple_portrait_source_with_profile(
        adapter,
        input,
        ApplePortraitSemanticProfile::Complete,
    )
}

#[cfg(target_os = "macos")]
pub(crate) fn prepare_apple_portrait_source_with_profile(
    adapter: &AppleAdapterClient,
    input: &[u8],
    semantic_profile: ApplePortraitSemanticProfile,
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
    let subject_prior = if let Some(portrait_plane) = depth.portrait.as_ref().filter(|plane| {
        semantic_profile == ApplePortraitSemanticProfile::Complete || plane.iter().any(|&v| v != 0)
    }) {
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

    let (
        portrait_effects_matte,
        subject_prior_used,
        skin_matte,
        hair_matte,
        hair_prior_added_high_confidence,
        teeth_matte,
        glasses_matte,
    ) = match semantic_profile {
        ApplePortraitSemanticProfile::Complete => {
            // Vision reports native semantic observations only for the complete
            // profile. Rust retains all product policy and OPPO-prior fusion.
            let mut vision_masks = adapter.vision_semantic_mattes(
                source_image_file.path(),
                &APPLE_PORTRAIT_SEMANTIC_ROLES,
                Some(u32::from(base_orientation)),
            )?;
            let native_person =
                vision_masks
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
                fuse_apple_portrait_person_mask(&rendered_person, subject_prior.as_ref()).map_err(
                    |error| RuntimeError::external("Apple Portrait person fusion", error),
                )?;
            let hair_fusion = fuse_apple_portrait_hair_mask(
                &rendered_hair,
                hair_prior.as_ref(),
                &person_fusion.mask,
            )
            .map_err(|error| RuntimeError::external("Apple Portrait hair fusion", error))?;
            (
                Some(person_fusion.mask),
                person_fusion.used_prior,
                Some(rendered_skin),
                Some(hair_fusion.mask),
                hair_fusion.prior_added_high_confidence,
                Some(rendered_teeth),
                Some(rendered_glasses),
            )
        }
        ApplePortraitSemanticProfile::OppoNative => {
            // Empty producer planes are absent resources, not segmentation masks.
            // No Vision request is made on this path, including public person segmentation.
            let subject = subject_prior.filter(AppleL8Mask::has_credible_foreground);
            let hair = hair_prior.filter(AppleL8Mask::has_credible_foreground);
            let subject_used = subject.is_some();
            let hair_used = hair.is_some();
            (subject, subject_used, None, hair, hair_used, None, None)
        }
    };

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
        portrait_effects_matte,
        subject_prior_used,
        semantic_profile,
        skin_matte,
        hair_matte,
        hair_prior_added_high_confidence,
        teeth_matte,
        glasses_matte,
        simulated_aperture,
    })
}

#[cfg(target_os = "macos")]
fn decode_f16_le(bytes: [u8; 2]) -> f32 {
    let bits = u16::from_le_bytes(bytes);
    let sign = u32::from((bits >> 15) & 1);
    let exp = u32::from((bits >> 10) & 0x1f);
    let mant = u32::from(bits & 0x03ff);
    let f32_bits = if exp == 0 {
        if mant == 0 {
            sign << 31
        } else {
            let mut m = mant;
            let mut e = 127_u32 - 14;
            while (m & 0x0400) == 0 {
                m <<= 1;
                e -= 1;
            }
            (sign << 31) | (e << 23) | ((m & 0x03ff) << 13)
        }
    } else if exp == 0x1f {
        (sign << 31) | 0x7f80_0000 | (mant << 13)
    } else {
        (sign << 31) | ((exp + 112) << 23) | (mant << 13)
    };
    f32::from_bits(f32_bits)
}

#[cfg(target_os = "macos")]
fn synthesize_disparity_foreground_mask(
    disparity: &ApplePortraitDisparity,
    target_width: u32,
    target_height: u32,
    threshold: f32,
) -> Result<AppleL8Mask> {
    let disp_w = disparity.width as usize;
    let disp_h = disparity.height as usize;
    let out_w = target_width as usize;
    let out_h = target_height as usize;
    let mut pixels = vec![0_u8; out_w * out_h];
    let span = (disparity.near - disparity.far).max(1e-4);
    let mut any_foreground = false;
    for y in 0..out_h {
        let sy = (y * disp_h / out_h).min(disp_h.saturating_sub(1));
        for x in 0..out_w {
            let sx = (x * disp_w / out_w).min(disp_w.saturating_sub(1));
            let idx = (sy * disp_w + sx) * 2;
            let value = decode_f16_le([
                disparity.pixels_le_f16[idx],
                disparity.pixels_le_f16[idx + 1],
            ]);
            let normalized = ((value - threshold) / span.max(1e-4)).clamp(0.0, 1.0);
            let luma = if normalized > 0.0 {
                ((normalized * 255.0).round() as u8).max(180)
            } else {
                0
            };
            if luma >= 128 {
                any_foreground = true;
            }
            pixels[y * out_w + x] = luma;
        }
    }
    if !any_foreground {
        for y in (out_h / 4)..((out_h * 3) / 4) {
            for x in (out_w / 4)..((out_w * 3) / 4) {
                pixels[y * out_w + x] = 255;
            }
        }
    }
    AppleL8Mask::new(target_width, target_height, pixels)
        .map_err(|error| RuntimeError::external("Apple Portrait disparity foreground mask", error))
}

#[cfg(target_os = "macos")]
fn foreground_focus_region(mask: &AppleL8Mask) -> OppoPortraitFocusRegion {
    let width = mask.width as usize;
    let height = mask.height as usize;
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut weight_sum = 0.0_f64;
    let mut min_x = width;
    let mut max_x = 0_usize;
    let mut min_y = height;
    let mut max_y = 0_usize;
    for y in 0..height {
        for x in 0..width {
            let value = mask.pixels[y * width + x];
            if value >= 128 {
                let w = f64::from(value);
                sum_x += (x as f64 + 0.5) * w;
                sum_y += (y as f64 + 0.5) * w;
                weight_sum += w;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if weight_sum > 0.0 && width > 0 && height > 0 {
        let cx = (sum_x / weight_sum / width as f64).clamp(0.05, 0.95);
        let cy = (sum_y / weight_sum / height as f64).clamp(0.05, 0.95);
        let rw = ((max_x.saturating_sub(min_x) + 1) as f64 / width as f64).clamp(0.06, 0.50);
        let rh = ((max_y.saturating_sub(min_y) + 1) as f64 / height as f64).clamp(0.06, 0.50);
        OppoPortraitFocusRegion {
            x: cx,
            y: cy,
            width: rw,
            height: rh,
        }
    } else {
        OppoPortraitFocusRegion {
            x: 0.5,
            y: 0.5,
            width: 0.2,
            height: 0.2,
        }
    }
}

#[cfg(target_os = "macos")]
fn sample_focus_disparity(
    disparity: &ApplePortraitDisparity,
    region: OppoPortraitFocusRegion,
) -> f64 {
    let w = disparity.width as usize;
    let h = disparity.height as usize;
    let cx = region.x.clamp(0.0, 1.0) * w as f64;
    let cy = region.y.clamp(0.0, 1.0) * h as f64;
    let half_w = (region.width * w as f64 * 0.25).max(2.0);
    let half_h = (region.height * h as f64 * 0.25).max(2.0);
    let x0 = ((cx - half_w).floor() as isize).clamp(0, w.saturating_sub(1) as isize) as usize;
    let x1 = ((cx + half_w).ceil() as isize).clamp(0, w.saturating_sub(1) as isize) as usize;
    let y0 = ((cy - half_h).floor() as isize).clamp(0, h.saturating_sub(1) as isize) as usize;
    let y1 = ((cy + half_h).ceil() as isize).clamp(0, h.saturating_sub(1) as isize) as usize;
    let mut samples = Vec::with_capacity((x1 - x0 + 1) * (y1 - y0 + 1));
    for y in y0..=y1 {
        for x in x0..=x1 {
            let idx = (y * w + x) * 2;
            let v = decode_f16_le([
                disparity.pixels_le_f16[idx],
                disparity.pixels_le_f16[idx + 1],
            ]);
            if v.is_finite() && v >= 0.0 {
                samples.push(f64::from(v));
            }
        }
    }
    if samples.is_empty() {
        return f64::from((disparity.near - disparity.far) * 0.5);
    }
    samples.sort_by(f64::total_cmp);
    let median = samples[samples.len() / 2];
    (median - f64::from(disparity.far)).clamp(0.0, f64::from(disparity.near - disparity.far))
}

#[cfg(target_os = "macos")]
pub(crate) fn convert_depthpro2_apple_portrait_file(
    runtime: &crate::PortableRuntime,
    adapter: &AppleAdapterClient,
    source: &[u8],
    input_path: &std::path::Path,
    output: &std::path::Path,
    staging: &std::path::Path,
) -> Result<crate::ApplePortraitFileReceipt> {
    let metadata_carrier_path = staging.join("carrier-metadata.heic");
    if runtime.analyze_proxdr(source).is_ok() {
        runtime.convert_proxdr_file(
            source,
            &metadata_carrier_path,
            xdremux_engine::ConversionRequest::default(),
            |_| {},
        )?;
    } else {
        std::fs::copy(input_path, &metadata_carrier_path)
            .map_err(|error| RuntimeError::external("Apple Portrait carrier copy", error))?;
    }

    let carrier_facts = adapter.imageio_auxiliary_facts(&metadata_carrier_path)?;
    if !carrier_facts.iso_gain_map {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            "input is not an OPPO Portrait source or ISO Gain Map still image",
        ));
    }
    let gain_map = adapter.imageio_gain_map_facts(&metadata_carrier_path)?;
    let supported_gain_map = gain_map.width > 0
        && gain_map.height > 0
        && matches!(
            gain_map.pixel_format.as_bytes(),
            b"420f" | b"444f" | b"L008"
        );
    if !supported_gain_map {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            format!(
                "unsupported ImageIO Gain Map pixel format {}",
                gain_map.pixel_format
            ),
        ));
    }

    let properties = adapter.imageio_image_properties(&metadata_carrier_path)?;
    let max_edge = f64::from(properties.width.max(properties.height));
    let min_edge = f64::from(properties.width.min(properties.height));
    if min_edge <= 0.0 || ((max_edge / min_edge) - (4.0 / 3.0)).abs() > 0.02 {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            format!(
                "Depth Pro 2 Portrait requires a 4:3 or 3:4 still image, got {}x{}",
                properties.width, properties.height
            ),
        ));
    }

    let base_orientation = resolve_apple_portrait_base_orientation(
        None,
        ApplePortraitImageGeometry {
            width: properties.width,
            height: properties.height,
            orientation: orientation(&properties),
        },
    )
    .map_err(|error| RuntimeError::external("Apple Portrait orientation", error))?;

    let recorded_focal = properties
        .focal_length_in_35mm_film
        .filter(|value| value.is_finite() && *value > 0.0);
    let (equivalent_focal_length_mm, torch_disparity) =
        if let Some(helper) = std::env::var_os("XDREMUX_DEPTHPRO2_PYTORCH") {
            let (disparity, facts) = AppleAdapterClient::new(helper).depthpro2_disparity(
                &metadata_carrier_path,
                properties.width.div_ceil(2),
                properties.height.div_ceil(2),
                base_orientation,
                recorded_focal,
                PYTORCH_DEPTHPRO2_TIMEOUT,
            )?;
            let focal = recorded_focal
                .or(facts.focal_length_35mm)
                .filter(|value| value.is_finite() && *value > 0.0)
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Depth Pro 2 PyTorch",
                        "missing or invalid 35mm focal length",
                    )
                })?;
            (focal, Some((disparity, facts)))
        } else {
            let focal = match recorded_focal {
                Some(value) => value,
                None => {
                    adapter
                        .depthpro2_camera_estimate(&metadata_carrier_path, Some(1))?
                        .equivalent_focal_length_35mm
                }
            };
            (focal, None)
        };
    let physical_focal_length_mm = properties
        .focal_length_mm
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or_else(|| (equivalent_focal_length_mm / 3.9).clamp(2.0, 35.0));

    let capture_facts = ApplePortraitCaptureFacts {
        physical_focal_length_mm,
        equivalent_focal_length_mm,
        digital_zoom_ratio: positive(properties.digital_zoom_ratio, None),
        lens_model: nonempty_string(properties.lens_model.as_deref(), None),
        base_width: properties.width,
        base_height: properties.height,
    };
    let camera_calibration = derive_apple_portrait_camera_calibration(&capture_facts)
        .map_err(|error| RuntimeError::external("Apple Portrait calibration", error))?;

    let (disparity, disparity_facts) = match torch_disparity {
        Some(result) => result,
        None => {
            let (width, height) = if properties.width >= properties.height {
                (768, 576)
            } else {
                (576, 768)
            };
            adapter.depthpro2_disparity(
                &metadata_carrier_path,
                width,
                height,
                base_orientation,
                Some(equivalent_focal_length_mm),
                APPLE_COMPUTE_TIMEOUT,
            )?
        }
    };

    let target_width = properties.width / 2;
    let target_height = properties.height / 2;
    if target_width == 0 || target_height == 0 {
        return Err(RuntimeError::new(
            "Apple Portrait semantic matte",
            "half-resolution target geometry is invalid",
        ));
    }

    let mut vision_masks = adapter.vision_semantic_mattes(
        &metadata_carrier_path,
        &APPLE_PORTRAIT_SEMANTIC_ROLES,
        Some(u32::from(base_orientation)),
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
    let rendered_person = render_role(AppleSemanticRole::Person)?;
    let skin_matte = render_role(AppleSemanticRole::Skin)?;
    let hair_matte = render_role(AppleSemanticRole::Hair)?;
    let teeth_matte = render_role(AppleSemanticRole::Teeth)?;
    let glasses_matte = render_role(AppleSemanticRole::Glasses)?;

    let portrait_effects_matte = if rendered_person.has_credible_foreground() {
        rendered_person
    } else {
        let fg = adapter
            .vision_foreground_matte(&metadata_carrier_path, Some(u32::from(base_orientation)))?;
        let rendered_fg =
            adapter.coreimage_render_l8(&fg, target_width, target_height, base_orientation)?;
        if rendered_fg.has_credible_foreground() {
            rendered_fg
        } else {
            synthesize_disparity_foreground_mask(
                &disparity,
                target_width,
                target_height,
                disparity_facts.p50_disparity,
            )?
        }
    };

    let focus_region = foreground_focus_region(&portrait_effects_matte);
    let focus_disparity = sample_focus_disparity(&disparity, focus_region);
    let disparity_span = f64::from(disparity.near - disparity.far);
    let carrier_bytes = std::fs::read(&metadata_carrier_path)
        .map_err(|error| RuntimeError::external("Apple Portrait carrier read", error))?;
    let gain_map_headroom = xdremux_heif::derived_tmap_payload(&carrier_bytes)
        .ok()
        .and_then(|tmap| xdremux_metadata::parse_heif_tmap_metadata(&tmap).ok())
        .map(|meta| meta.hdr_capacity_max)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(2.0);
    let rend = build_apple_portrait_rendering_parameters(
        camera_calibration.profile,
        focus_disparity,
        disparity_span,
        gain_map_headroom,
        None,
        true,
    )
    .map_err(|error| RuntimeError::external("Apple Portrait REND", error))?;

    let simulated_aperture = resolve_simulated_aperture(0.0, None, properties.f_number, None);

    let payloads = vec![
        build_apple_portrait_disparity_payload(
            disparity,
            base_orientation,
            &camera_calibration,
            &rend,
            simulated_aperture,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait disparity payload", error))?,
        build_apple_portrait_effects_matte_payload(
            portrait_effects_matte.width,
            portrait_effects_matte.height,
            portrait_effects_matte.pixels,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait effects matte payload", error))?,
        build_apple_semantic_matte_payload(
            AppleSemanticRole::Skin,
            skin_matte.width,
            skin_matte.height,
            skin_matte.pixels,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait skin matte payload", error))?,
        build_apple_semantic_matte_payload(
            AppleSemanticRole::Hair,
            hair_matte.width,
            hair_matte.height,
            hair_matte.pixels,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait hair matte payload", error))?,
        build_apple_semantic_matte_payload(
            AppleSemanticRole::Teeth,
            teeth_matte.width,
            teeth_matte.height,
            teeth_matte.pixels,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait teeth matte payload", error))?,
        build_apple_semantic_matte_payload(
            AppleSemanticRole::Glasses,
            glasses_matte.width,
            glasses_matte.height,
            glasses_matte.pixels,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait glasses matte payload", error))?,
    ];

    let focus_xmp = xdremux_metadata::make_apple_portrait_focus_xmp(
        properties.width,
        properties.height,
        focus_region.x,
        focus_region.y,
        focus_region.width,
        focus_region.height,
    )
    .map_err(|error| RuntimeError::external("Apple Portrait Focus XMP", error))?;

    let assembled_path = staging.join("assembled.heic");
    adapter.imageio_write_auxiliary(&metadata_carrier_path, &assembled_path, &payloads)?;

    let validated_path = staging.join("validated.heic");
    adapter.imageio_merge_xmp_metadata(&assembled_path, &focus_xmp, &validated_path)?;

    let facts = adapter.imageio_auxiliary_facts(&validated_path)?;
    if !facts.satisfies_portrait_editing() {
        return Err(RuntimeError::new(
            "Apple Portrait consumer validation",
            format!("ImageIO did not expose complete Portrait resource set: {facts:?}"),
        ));
    }
    let bytes = std::fs::read(&validated_path)
        .map_err(|error| RuntimeError::external("Apple Portrait output read", error))?;
    xdremux_heif::validate_gain_map_structure(&bytes)
        .map_err(|error| RuntimeError::external("Apple Portrait structural validation", error))?;

    let mut publisher = crate::AtomicFilePublisher::new(output.to_path_buf());
    let published = publisher.publish_bytes(bytes)?;
    Ok(crate::ApplePortraitFileReceipt {
        output: published,
        auxiliary: facts,
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
    fn committed_portrait_fixture_has_depth_without_subject_matte() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let depth = decode_oppo_portrait_depth(&source.compressed_depth).expect("decode depth");
        assert!(depth.ranks.iter().any(|&v| v != 0));
        assert!(!depth
            .portrait
            .as_ref()
            .is_some_and(|plane| plane.iter().any(|&v| v != 0)));
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

    #[cfg(target_os = "macos")]
    #[test]
    fn depthpro2_f16_decode_focus_region_and_mask_synthesis_stay_bounded() {
        assert!((decode_f16_le([0x00, 0x3c]) - 1.0).abs() < 1e-6);
        assert!((decode_f16_le([0x00, 0x40]) - 2.0).abs() < 1e-6);
        assert_eq!(decode_f16_le([0x00, 0x00]), 0.0);

        let mut pixels = vec![0_u8; 40 * 30];
        for y in 6..18 {
            for x in 20..32 {
                pixels[y * 40 + x] = 255;
            }
        }
        let mask = AppleL8Mask::new(40, 30, pixels).unwrap();
        let region = foreground_focus_region(&mask);
        assert!((region.x - 26.0 / 40.0).abs() < 0.05, "{region:?}");
        assert!((region.y - 12.0 / 30.0).abs() < 0.05, "{region:?}");
        assert!((0.06..=0.50).contains(&region.width));
        assert!((0.06..=0.50).contains(&region.height));

        let mut disp_bytes = Vec::with_capacity(8 * 6 * 2);
        for idx in 0..(8 * 6) {
            let bits: u16 = if idx % 2 == 0 { 0x3c00 } else { 0x3400 };
            disp_bytes.extend_from_slice(&bits.to_le_bytes());
        }
        let disp = ApplePortraitDisparity {
            width: 8,
            height: 6,
            far: 0.25,
            near: 1.0,
            pixels_le_f16: disp_bytes,
        };
        let synth = synthesize_disparity_foreground_mask(&disp, 16, 12, 0.5).unwrap();
        assert!(synth.has_credible_foreground());
        let focused = sample_focus_disparity(&disp, region);
        assert!(focused.is_finite() && focused >= 0.0);
    }
}
