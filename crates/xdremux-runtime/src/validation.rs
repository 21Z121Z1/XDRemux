use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use xdremux_format::ChromaSampling;
use xdremux_heif::validate_gain_map_structure;
use xdremux_motion_photo::{
    companion_video_path, read_apple_content_identifier, read_live_photo_content_identifier,
    read_live_photo_still_time, validate_live_photo_movie,
};

use crate::{Result, RuntimeError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

fn validate_iso_hdr(input: &Path, bytes: &[u8]) -> Result<ValidationReport> {
    let structure = validate_gain_map_structure(bytes)
        .map_err(|error| RuntimeError::external("ISO HDR HEIF validation", error))?;
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

    let image_identifier = read_apple_content_identifier(&image_bytes)
        .map_err(|error| RuntimeError::external("Live Photo still validation", error))?
        .ok_or_else(|| {
            RuntimeError::new(
                "Live Photo still validation",
                "HEIC/HEIF is missing the Apple ContentIdentifier MakerNote",
            )
        })?;
    let video_identifier = read_live_photo_content_identifier(&video_bytes)
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

    let still_time_seconds = read_live_photo_still_time(&video_bytes)
        .map_err(|error| RuntimeError::external("Live Photo still-time validation", error))?
        .ok_or_else(|| {
            RuntimeError::new(
                "Live Photo still-time validation",
                "MOV is missing the still-image-time metadata sample",
            )
        })?;
    validate_live_photo_movie(&video_bytes, &image_identifier, still_time_seconds)
        .map_err(|error| RuntimeError::external("Live Photo movie validation", error))?;

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
    let apple_identifier = read_apple_content_identifier(&bytes)
        .map_err(|error| RuntimeError::external("Live Photo still probe", error))?;
    if apple_identifier.is_some() {
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

    validate_iso_hdr(input, &bytes)
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
