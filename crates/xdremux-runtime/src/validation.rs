use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use xdremux_format::ChromaSampling;
use xdremux_heif::{validate_gain_map_semantics, validate_gain_map_structure, HdrOutputColor};
use xdremux_motion_photo::{apple, companion_video_path, Input, MotionPhoto, ParseOptions};

use crate::{Result, RuntimeError};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IsoHdrValidationReport {
    pub input: PathBuf,
    pub primary_item_id: u32,
    pub tmap_item_id: u32,
    pub gain_map_item_id: u32,
    pub tile_item_ids: Vec<u32>,
    pub width: u32,
    pub height: u32,
    pub rows: u32,
    pub columns: u32,
    pub channel_count: u8,
    pub chroma_format_idc: u8,
    pub chroma_sampling: &'static str,
    pub luma_bit_depth: u8,
    pub chroma_bit_depth: u8,
    pub general_profile_idc: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<IsoHdrSemanticsReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GainMapValidationProfile {
    Structure,
    Semantics,
    Bt2020 {
        transfer: xdremux_metadata::HdrTransfer,
        decode_pixels: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IsoHdrSemanticsReport {
    pub metadata_channel_count: u8,
    pub use_base_color_space: bool,
    pub base_headroom_stops: f64,
    pub alternate_headroom_stops: f64,
    pub output_color: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_nclx: Option<[u16; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hdr_profile: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_color_space: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_metadata: Option<HdrStaticMetadataReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_hdr: Option<DecodedHdrReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HdrStaticMetadataReport {
    pub mastering_display_present: bool,
    pub content_light_level_present: bool,
    pub dynamic_metadata: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DecodedHdrReport {
    pub width: usize,
    pub height: usize,
    pub base_decode_bit_depth: u8,
    pub gain_map_decode_bit_depth: u8,
    pub color_space: &'static str,
    pub sample_units: &'static str,
    pub minimum_rgb: [f32; 3],
    pub maximum_rgb: [f32; 3],
}

fn decode_hdr(
    source: &[u8],
    profile: &xdremux_heif::HdrProfileSemantics,
) -> Result<DecodedHdrReport> {
    let (image, base_bits, gain_bits) = reconstruct_hdr_pixels(source, profile)?;
    let mut minimum_rgb = [f32::INFINITY; 3];
    let mut maximum_rgb = [f32::NEG_INFINITY; 3];
    for pixel in image.samples.as_chunks::<3>().0 {
        for c in 0..3 {
            minimum_rgb[c] = minimum_rgb[c].min(pixel[c]);
            maximum_rgb[c] = maximum_rgb[c].max(pixel[c]);
        }
    }
    Ok(DecodedHdrReport {
        width: image.width,
        height: image.height,
        base_decode_bit_depth: base_bits,
        gain_map_decode_bit_depth: gain_bits,
        color_space: "linear-bt2020",
        sample_units: "relative-to-sdr-white",
        minimum_rgb,
        maximum_rgb,
    })
}

fn reconstruct_hdr_pixels(
    source: &[u8],
    profile: &xdremux_heif::HdrProfileSemantics,
) -> Result<(xdremux_hdr::SceneLinearRgb, u8, u8)> {
    let decoder = xdremux_codec::LibHeifProvider::new();
    let structure = &profile.semantics.structure;
    let mut base = decoder
        .decode_coded_item_normalized(source, structure.primary_item_id, 3)
        .map_err(|e| RuntimeError::external("HDR base decode", e))?;
    let gain = decoder
        .decode_coded_item_normalized(source, structure.gain_map_item_id, structure.channel_count)
        .map_err(|e| RuntimeError::external("HDR Gain Map decode", e))?;
    for value in &mut base.samples {
        *value = xdremux_hdr::decode_srgb(f64::from(*value)) as f32;
    }
    let image = xdremux_hdr::reconstruct_bt2020_hdr(
        &xdremux_hdr::SceneLinearRgb {
            width: base.width as usize,
            height: base.height as usize,
            samples: base.samples,
        },
        xdremux_hdr::GainMapSamples {
            width: gain.width as usize,
            height: gain.height as usize,
            channels: usize::from(gain.channels),
            data: &gain.samples,
        },
        &profile.semantics.metadata,
        profile.base_color_space,
    )
    .map_err(|e| RuntimeError::external("HDR pixel reconstruction", e))?;
    Ok((image, base.bit_depth, gain.bit_depth))
}

/// Decode the final container and reconstruct its alternate as relative linear
/// BT.2020 RGB. This does not apply a display OOTF or dynamic tone mapping.
pub fn reconstruct_hdr_gain_map(
    source: &[u8],
    transfer: xdremux_metadata::HdrTransfer,
) -> Result<xdremux_hdr::SceneLinearRgb> {
    let profile = xdremux_heif::validate_hdr_gain_map_profile(source, transfer)
        .map_err(|e| RuntimeError::external("HDR profile validation", e))?;
    reconstruct_hdr_pixels(source, &profile).map(|(image, _, _)| image)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LivePhotoValidationReport {
    pub input: PathBuf,
    pub image: PathBuf,
    pub video: PathBuf,
    pub content_identifier: String,
    pub still_time_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum ValidationReport {
    IsoHdrHeif(IsoHdrValidationReport),
    LivePhoto(LivePhotoValidationReport),
}

impl ValidationReport {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::IsoHdrHeif(_) => "iso-hdr-heif",
            Self::LivePhoto(_) => "live-photo",
        }
    }
}

fn chroma_sampling_name(value: ChromaSampling) -> &'static str {
    match value {
        ChromaSampling::Mono400 => "4:0:0",
        ChromaSampling::Yuv420 => "4:2:0",
        ChromaSampling::Yuv422 => "4:2:2",
        ChromaSampling::Yuv444 => "4:4:4",
    }
}

fn read_file(path: &Path, context: &'static str) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| RuntimeError::external(context, error))
}

fn validate_iso_hdr(
    input: &Path,
    bytes: &[u8],
    profile: GainMapValidationProfile,
) -> Result<ValidationReport> {
    let structure = validate_gain_map_structure(bytes)
        .map_err(|error| RuntimeError::external("ISO HDR HEIF validation", error))?;
    let semantics = if profile != GainMapValidationProfile::Structure {
        let observed = validate_gain_map_semantics(bytes)
            .map_err(|error| RuntimeError::external("ISO HDR HEIF semantics", error))?;
        let (output_color, output_nclx) = match observed.output_color {
            HdrOutputColor::Nclx {
                primaries,
                transfer,
                matrix,
                ..
            } => ("nclx", Some([primaries, transfer, matrix])),
            HdrOutputColor::Icc => ("icc", None),
        };
        let strict = if let GainMapValidationProfile::Bt2020 { transfer, .. } = profile {
            Some(
                xdremux_heif::validate_hdr_gain_map_profile(bytes, transfer)
                    .map_err(|e| RuntimeError::external("HDR profile validation", e))?,
            )
        } else {
            None
        };
        let decoded_hdr = if matches!(
            profile,
            GainMapValidationProfile::Bt2020 {
                decode_pixels: true,
                ..
            }
        ) {
            Some(decode_hdr(bytes, strict.as_ref().unwrap())?)
        } else {
            None
        };
        Some(IsoHdrSemanticsReport {
            metadata_channel_count: observed.metadata.channel_count,
            use_base_color_space: observed.metadata.use_base_color_space,
            base_headroom_stops: observed.metadata.hdr_capacity_min,
            alternate_headroom_stops: observed.metadata.hdr_capacity_max,
            output_color,
            output_nclx,
            hdr_profile: match profile {
                GainMapValidationProfile::Bt2020 {
                    transfer: xdremux_metadata::HdrTransfer::Pq,
                    ..
                } => Some("bt2020-pq"),
                GainMapValidationProfile::Bt2020 {
                    transfer: xdremux_metadata::HdrTransfer::Hlg,
                    ..
                } => Some("bt2020-hlg"),
                _ => None,
            },
            base_color_space: strict.as_ref().map(|p| match p.base_color_space {
                xdremux_metadata::SdrColorSpace::Srgb => "srgb",
                xdremux_metadata::SdrColorSpace::DisplayP3 => "display-p3",
            }),
            static_metadata: strict.as_ref().map(|p| HdrStaticMetadataReport {
                mastering_display_present: p.static_metadata.mastering_display.is_some(),
                content_light_level_present: p.static_metadata.content_light_level.is_some(),
                dynamic_metadata: if p
                    .registered_metadata
                    .as_ref()
                    .is_some_and(|m| m.dynamic_metadata.is_some())
                {
                    "syntax-checked"
                } else {
                    "not-present"
                },
            }),
            decoded_hdr,
        })
    } else {
        None
    };
    Ok(ValidationReport::IsoHdrHeif(IsoHdrValidationReport {
        input: input.to_path_buf(),
        primary_item_id: structure.primary_item_id,
        tmap_item_id: structure.tmap_item_id,
        gain_map_item_id: structure.gain_map_item_id,
        tile_item_ids: structure.tile_item_ids,
        width: structure.width,
        height: structure.height,
        rows: structure.rows,
        columns: structure.columns,
        channel_count: structure.channel_count,
        chroma_format_idc: structure.chroma_format_idc,
        chroma_sampling: chroma_sampling_name(structure.chroma_sampling),
        luma_bit_depth: structure.luma_bit_depth,
        chroma_bit_depth: structure.chroma_bit_depth,
        general_profile_idc: structure.general_profile_idc,
        semantics,
    }))
}

fn validate_live_photo_pair(
    input: &Path,
    image: &Path,
    video: &Path,
    image_bytes: Option<Vec<u8>>,
) -> Result<ValidationReport> {
    let image_bytes = match image_bytes {
        Some(bytes) => bytes,
        None => read_file(image, "Live Photo still read")?,
    };
    let video_bytes = read_file(video, "Live Photo movie read")?;

    let image_identifier = apple::read_still_identifier(&image_bytes)
        .map_err(|error| RuntimeError::external("Live Photo still validation", error))?
        .ok_or_else(|| {
            RuntimeError::new(
                "Live Photo still validation",
                "HEIC/HEIF is missing the Apple ContentIdentifier MakerNote",
            )
        })?;
    let video_identifier = apple::read_movie_identifier(&video_bytes)
        .map_err(|error| RuntimeError::external("Live Photo movie validation", error))?
        .ok_or_else(|| {
            RuntimeError::new(
                "Live Photo movie validation",
                "MOV is missing the QuickTime ContentIdentifier metadata",
            )
        })?;
    if image_identifier != video_identifier {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            format!(
                "ContentIdentifier mismatch: still {image_identifier}, movie {video_identifier}"
            ),
        ));
    }

    let presentation = apple::read_presentation(&video_bytes)
        .map_err(|error| RuntimeError::external("Live Photo still-time validation", error))?
        .ok_or_else(|| {
            RuntimeError::new(
                "Live Photo still-time validation",
                "MOV is missing the still-image-time metadata sample",
            )
        })?;
    MotionPhoto::parse(
        Input::ApplePair {
            still: &image_bytes,
            movie: &video_bytes,
        },
        ParseOptions::strict(),
    )
    .map_err(|error| RuntimeError::external("Live Photo pair validation", error))?;
    let still_time_seconds = presentation.seconds();

    Ok(ValidationReport::LivePhoto(LivePhotoValidationReport {
        input: input.to_path_buf(),
        image: image.to_path_buf(),
        video: video.to_path_buf(),
        content_identifier: image_identifier,
        still_time_seconds,
    }))
}

fn companion_still_path(video: &Path) -> Result<PathBuf> {
    let heic = video.with_extension("heic");
    if heic.is_file() {
        return Ok(heic);
    }
    let heif = video.with_extension("heif");
    if heif.is_file() {
        return Ok(heif);
    }
    Err(RuntimeError::new(
        "Live Photo pair validation",
        format!("no sibling HEIC/HEIF still exists for {}", video.display()),
    ))
}

pub fn validate_media_file(input: &Path) -> Result<ValidationReport> {
    validate_media_file_with_profile(input, GainMapValidationProfile::Structure)
}

pub fn validate_media_file_with_profile(
    input: &Path,
    profile: GainMapValidationProfile,
) -> Result<ValidationReport> {
    if profile != GainMapValidationProfile::Structure
        && !input.extension().is_some_and(|value| {
            value.eq_ignore_ascii_case("heic") || value.eq_ignore_ascii_case("heif")
        })
    {
        return Err(RuntimeError::new(
            "ISO HDR HEIF semantics",
            "semantic validation requires a standalone HEIC/HEIF input",
        ));
    }
    if input
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pvt"))
    {
        let members = crate::live_photo_package::read_members(input)?;
        return validate_live_photo_pair(input, &members.image, &members.video, None);
    }
    if !input.is_file() {
        return Err(RuntimeError::new(
            "media validation",
            format!("input file does not exist: {}", input.display()),
        ));
    }
    let extension = input
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    if extension.eq_ignore_ascii_case("mov") {
        let image = companion_still_path(input)?;
        return validate_live_photo_pair(input, &image, input, None);
    }
    if !extension.eq_ignore_ascii_case("heic") && !extension.eq_ignore_ascii_case("heif") {
        return Err(RuntimeError::new(
            "media validation",
            "validate currently accepts ISO HDR HEIC/HEIF or a Live Photo HEIC/HEIF/MOV pair",
        ));
    }

    let bytes = read_file(input, "media validation read")?;
    let apple_identifier = apple::read_still_identifier(&bytes)
        .map_err(|error| RuntimeError::external("Live Photo still probe", error))?;
    if apple_identifier.is_some() {
        if profile != GainMapValidationProfile::Structure {
            return Err(RuntimeError::new(
                "ISO HDR HEIF semantics",
                "semantic validation expects a standalone HDR image, not a Live Photo pair",
            ));
        }
        let video = companion_video_path(input);
        if !video.is_file() {
            return Err(RuntimeError::new(
                "Live Photo pair validation",
                format!(
                    "still declares an Apple ContentIdentifier but sibling movie is missing: {}",
                    video.display()
                ),
            ));
        }
        return validate_live_photo_pair(input, input, &video, Some(bytes));
    }

    validate_iso_hdr(input, &bytes, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chroma_names_are_stable_machine_values() {
        assert_eq!(chroma_sampling_name(ChromaSampling::Mono400), "4:0:0");
        assert_eq!(chroma_sampling_name(ChromaSampling::Yuv420), "4:2:0");
        assert_eq!(chroma_sampling_name(ChromaSampling::Yuv422), "4:2:2");
        assert_eq!(chroma_sampling_name(ChromaSampling::Yuv444), "4:4:4");
    }
}
