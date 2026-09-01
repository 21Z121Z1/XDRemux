use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use xdremux_motion_photo::{
    companion_video_path, media_mdat_payloads, normalize_embedded_video, parse_oppo_motion_photo,
    publish_live_photo_pair, read_apple_content_identifier, read_live_photo_content_identifier,
    reconcile_live_photo_pair, resolve_live_photo_still_time, validate_live_photo_movie,
    write_live_photo_heif_still, write_live_photo_movie, ByteRange, MotionPhotoSourceKind,
};

use crate::{Result, RuntimeError};

#[derive(Debug, Clone, PartialEq)]
pub struct LivePhotoFileReceipt {
    pub image: PathBuf,
    pub video: PathBuf,
    pub content_identifier: String,
    pub still_time_seconds: f64,
    pub source_kind: String,
    pub removed_vendor_bytes: usize,
}

fn range_slice<'a>(source: &'a [u8], range: ByteRange, context: &'static str) -> Result<&'a [u8]> {
    let start = usize::try_from(range.lower_bound)
        .map_err(|_| RuntimeError::new(context, "range start exceeds usize"))?;
    let end = usize::try_from(range.upper_bound)
        .map_err(|_| RuntimeError::new(context, "range end exceeds usize"))?;
    source
        .get(start..end)
        .ok_or_else(|| RuntimeError::new(context, "range is outside source bytes"))
}

fn generate_content_identifier() -> String {
    Uuid::new_v4().hyphenated().to_string().to_ascii_uppercase()
}

fn write_synced_new(path: &Path, data: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| RuntimeError::external("Live Photo temporary create", error))?;
    file.write_all(data)
        .map_err(|error| RuntimeError::external("Live Photo temporary write", error))?;
    file.sync_all()
        .map_err(|error| RuntimeError::external("Live Photo temporary sync", error))
}

fn pair_matches(image: &Path, video: &Path) -> bool {
    let Ok(image_bytes) = fs::read(image) else {
        return false;
    };
    let Ok(video_bytes) = fs::read(video) else {
        return false;
    };
    let Ok(Some(image_identifier)) = read_apple_content_identifier(&image_bytes) else {
        return false;
    };
    let Ok(Some(video_identifier)) = read_live_photo_content_identifier(&video_bytes) else {
        return false;
    };
    if image_identifier != video_identifier {
        return false;
    }
    let Ok(Some(still_time)) = xdremux_motion_photo::read_live_photo_still_time(&video_bytes)
    else {
        return false;
    };
    validate_live_photo_movie(&video_bytes, &video_identifier, still_time).is_ok()
}

fn temporary_pair_paths(output_image: &Path, identifier: &str) -> Result<(PathBuf, PathBuf)> {
    let parent = match output_image.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let stem = output_image
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| RuntimeError::new("Live Photo output", "output has no UTF-8 stem"))?;
    let token = identifier.replace('-', "").to_ascii_lowercase();
    Ok((
        parent.join(format!(".{stem}.{token}.tmp.heic")),
        parent.join(format!(".{stem}.{token}.tmp.mov")),
    ))
}

pub(crate) fn convert_heif_motion_photo_file(
    source: &[u8],
    input: &Path,
    output_image: &Path,
) -> Result<LivePhotoFileReceipt> {
    let asset = parse_oppo_motion_photo(source)
        .map_err(|error| RuntimeError::external("Motion Photo analysis", error))?
        .ok_or_else(|| RuntimeError::new("Motion Photo analysis", "input is not a Motion Photo"))?;
    if asset.source_kind != MotionPhotoSourceKind::AndroidHeifMotionPhotoV1 {
        return Err(RuntimeError::new(
            "portable Live Photo runtime",
            "this Rust execution slice currently accepts HEIF Motion Photo inputs only",
        ));
    }

    let extension = output_image
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("heic") && !extension.eq_ignore_ascii_case("heif") {
        return Err(RuntimeError::new(
            "Live Photo output",
            "still output must use .heic or .heif",
        ));
    }
    if output_image == input {
        return Err(RuntimeError::new(
            "Live Photo output",
            "Motion Photo conversion never overwrites the source",
        ));
    }
    let output_video = companion_video_path(output_image);
    if output_image.exists() || output_video.exists() {
        return Err(RuntimeError::new(
            "Live Photo output",
            "output HEIC/MOV pair already exists; refusing to overwrite unknown provenance",
        ));
    }

    reconcile_live_photo_pair(output_image, &output_video, pair_matches)
        .map_err(|error| RuntimeError::external("Live Photo pair reconciliation", error))?;

    let static_heif = range_slice(
        source,
        asset.still_resource_range,
        "Motion Photo still range",
    )?;
    let embedded_video = range_slice(
        source,
        asset.video_resource_range,
        "Motion Photo video range",
    )?;
    let normalized_video = normalize_embedded_video(embedded_video)
        .map_err(|error| RuntimeError::external("Motion Photo video normalization", error))?;
    let still_time_seconds =
        resolve_live_photo_still_time(normalized_video.data, asset.presentation_timestamp_us)
            .map_err(|error| RuntimeError::external("Live Photo still-time resolution", error))?;

    let content_identifier = generate_content_identifier();
    let still = write_live_photo_heif_still(static_heif, &content_identifier)
        .map_err(|error| RuntimeError::external("Live Photo HEIF still", error))?;
    let movie = write_live_photo_movie(
        normalized_video.data,
        &content_identifier,
        still_time_seconds,
        asset.vendor_metadata.as_ref(),
    )
    .map_err(|error| RuntimeError::external("Live Photo MOV", error))?;

    let still_identifier = read_apple_content_identifier(&still)
        .map_err(|error| RuntimeError::external("Live Photo still validation", error))?;
    if still_identifier.as_deref() != Some(content_identifier.as_str()) {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "HEIF Apple MakerNote ContentIdentifier mismatch",
        ));
    }
    if read_live_photo_content_identifier(&movie)
        .map_err(|error| RuntimeError::external("Live Photo movie validation", error))?
        .as_deref()
        != Some(content_identifier.as_str())
    {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "MOV QuickTime ContentIdentifier mismatch",
        ));
    }
    validate_live_photo_movie(&movie, &content_identifier, still_time_seconds)
        .map_err(|error| RuntimeError::external("Live Photo pair validation", error))?;

    let source_media = media_mdat_payloads(normalized_video.data)
        .map_err(|error| RuntimeError::external("source Motion Photo media validation", error))?;
    let output_media = media_mdat_payloads(&movie)
        .map_err(|error| RuntimeError::external("Live Photo media validation", error))?;
    if source_media != output_media {
        return Err(RuntimeError::new(
            "Live Photo media validation",
            "compressed video/audio mdat payload changed during MOV remux",
        ));
    }

    let (temporary_image, temporary_video) =
        temporary_pair_paths(output_image, &content_identifier)?;
    let publish_result: Result<()> = (|| {
        write_synced_new(&temporary_image, &still)?;
        write_synced_new(&temporary_video, &movie)?;
        publish_live_photo_pair(
            &temporary_image,
            &temporary_video,
            output_image,
            &output_video,
        )
        .map_err(|error| RuntimeError::external("Live Photo pair publication", error))?;
        Ok(())
    })();
    if publish_result.is_err() {
        let _ = fs::remove_file(&temporary_image);
        let _ = fs::remove_file(&temporary_video);
    }
    publish_result?;

    Ok(LivePhotoFileReceipt {
        image: output_image.to_path_buf(),
        video: output_video,
        content_identifier,
        still_time_seconds,
        source_kind: asset.source_kind.as_str().to_owned(),
        removed_vendor_bytes: normalized_video.removed_vendor_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_pair_identifier_is_uuid_v4() {
        let value = generate_content_identifier();
        let parsed = Uuid::parse_str(&value).expect("generated identifier must parse as UUID");
        assert_eq!(parsed.get_version_num(), 4);
        assert_eq!(value, value.to_ascii_uppercase());
    }
}
