//! Combined vivo JPEG+MP4 for gallery 10.0.10.42 on the Y04s/W20 package.
//! Playback seeks with `setDataSource(fd, fileSize - Item:Length, Item:Length)`
//! when Padding is 0. ExtInfo is read only from the bytes before that offset.
//! `motionphoto00010000000000000` is a format type. Cover milliseconds come
//! from `imageTime` at 30 fps, not from a unique hash or the watermark stream.

use crate::{Result, RuntimeError};

pub(crate) const MOTION_PHOTO_TYPE: &str = "motionphoto00010000000000000";

pub(crate) struct VivoCover {
    pub image_time: i32,
    pub cover_ms: i32,
    pub note: String,
}

/// Nearest 30 fps frame to the source presentation time, then the gallery
/// conversion `((frame * 1000) + 15) / 30`.
pub(crate) fn cover_from_presentation_us(presentation_us: i64) -> Result<VivoCover> {
    if presentation_us < 0 {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "the source presentation time is negative, and the muxer rejects that",
        ));
    }
    let frame = presentation_us
        .checked_mul(30)
        .and_then(|value| value.checked_add(500_000))
        .map(|value| value / 1_000_000)
        .ok_or_else(|| {
            RuntimeError::new(
                "vivo Motion Photo",
                "the presentation time does not convert to a 30 fps frame",
            )
        })?;
    let image_time = i32::try_from(frame).map_err(|_| {
        RuntimeError::new(
            "vivo Motion Photo",
            "the 30 fps cover frame does not fit the ExtInfo integer",
        )
    })?;
    let cover_ms = i32::try_from((i64::from(image_time) * 1000 + 15) / 30).map_err(|_| {
        RuntimeError::new(
            "vivo Motion Photo",
            "the cover millisecond value does not fit a signed integer",
        )
    })?;
    Ok(VivoCover {
        image_time,
        cover_ms,
        note: format!(
            "imageTime {image_time} is the 30 fps frame nearest to {presentation_us} us; cover seek is {cover_ms} ms"
        ),
    })
}

pub(crate) fn presentation_us(value: i64, timescale: u32) -> Result<(i64, i128)> {
    if timescale == 0 {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "a zero presentation timescale cannot be converted",
        ));
    }
    if value < 0 {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "the source presentation time is negative, and the muxer rejects that",
        ));
    }
    let scaled = i128::from(value).checked_mul(1_000_000).ok_or_else(|| {
        RuntimeError::new(
            "vivo Motion Photo",
            "the presentation time overflowed the microsecond conversion",
        )
    })?;
    let scale = i128::from(timescale);
    let micros = i64::try_from(scaled / scale).map_err(|_| {
        RuntimeError::new(
            "vivo Motion Photo",
            "the presentation time does not fit a microsecond timestamp",
        )
    })?;
    Ok((micros, scaled % scale))
}

/// JPEG-prefix trailer read backward by the 3.2 extend-info parser: `vivo` +
/// JSON + big-endian length + `cameralbum!` + the 47-byte type record.
pub(crate) fn extinfo_before_video(image_time: i32) -> Result<Vec<u8>> {
    if MOTION_PHOTO_TYPE.len() != 28 {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "the motionphoto type marker is not the 28-byte record payload",
        ));
    }
    let json = format!(
        "{{\"com.android.camera.imageTime\":{image_time},\"com.android.camera.livephoto\":\"{MOTION_PHOTO_TYPE}\"}}"
    );
    let json_bytes = json.as_bytes();
    let length = u32::try_from(json_bytes.len()).map_err(|_| {
        RuntimeError::new(
            "vivo Motion Photo",
            "ExtInfo JSON does not fit its length field",
        )
    })?;
    let mut trailer = Vec::with_capacity(4 + json_bytes.len() + 4 + 11 + 47);
    trailer.extend_from_slice(b"vivo");
    trailer.extend_from_slice(json_bytes);
    trailer.extend_from_slice(&length.to_be_bytes());
    trailer.extend_from_slice(b"cameralbum!");
    let mut record = [0u8; 47];
    record[3] = 47;
    record[4..32].copy_from_slice(MOTION_PHOTO_TYPE.as_bytes());
    record[32..36].fill(0xff);
    record[36..46].copy_from_slice(&[0x1b, 0x2a, 0x39, 0x48, 0x57, 0x66, 0x75, 0x84, 0x93, 0xa2]);
    record[46] = 0xb3;
    trailer.extend_from_slice(&record);
    let parsed = parse_extinfo_tail(&trailer).ok_or_else(|| {
        RuntimeError::new(
            "vivo Motion Photo",
            "the ExtInfo trailer does not round-trip through the backward parser",
        )
    })?;
    if parsed.0 != MOTION_PHOTO_TYPE || parsed.1 != image_time {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "the ExtInfo trailer changed the type marker or imageTime",
        ));
    }
    Ok(trailer)
}

/// Returns the type string and `imageTime` when the tail matches the 3.2 record.
pub(crate) fn parse_extinfo_tail(bytes: &[u8]) -> Option<(&str, i32)> {
    if bytes.len() < 47 + 15 {
        return None;
    }
    let record = &bytes[bytes.len() - 47..];
    if record[36..46] != [0x1b, 0x2a, 0x39, 0x48, 0x57, 0x66, 0x75, 0x84, 0x93, 0xa2]
        || record[46] != 0xb3
    {
        return None;
    }
    let marker = std::str::from_utf8(&record[4..32]).ok()?;
    let header = &bytes[bytes.len() - 47 - 15..bytes.len() - 47];
    if &header[4..] != b"cameralbum!" {
        return None;
    }
    let json_len = u32::from_be_bytes(header[..4].try_into().ok()?) as usize;
    let json_end = bytes.len() - 47 - 15;
    let json_start = json_end.checked_sub(json_len)?;
    if json_start < 4 || &bytes[json_start - 4..json_start] != b"vivo" {
        return None;
    }
    let json = std::str::from_utf8(&bytes[json_start..json_end]).ok()?;
    let key = "\"com.android.camera.imageTime\":";
    let at = json.find(key)? + key.len();
    let end = json[at..]
        .find(|ch: char| !ch.is_ascii_digit())
        .map(|offset| at + offset)
        .unwrap_or(json.len());
    let image_time = json[at..end].parse().ok()?;
    Some((marker, image_time))
}

pub(crate) fn insert_extinfo_before_video(
    bytes: &mut Vec<u8>,
    video: &[u8],
    image_time: i32,
) -> Result<usize> {
    if video.is_empty() || !bytes.ends_with(video) {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "the preserved video is not a single suffix, so ExtInfo cannot be placed before it",
        ));
    }
    let video_at = bytes.len() - video.len();
    let trailer = extinfo_before_video(image_time)?;
    bytes.splice(video_at..video_at, trailer.iter().copied());
    let start = video_at + trailer.len();
    if &bytes[start..] != video {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "inserting ExtInfo moved the preserved video",
        ));
    }
    if parse_extinfo_tail(&bytes[..start]).is_none() {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "ExtInfo is not readable at the end of the photo region",
        ));
    }
    Ok(start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_frame_numbers_match_the_gallery_millisecond_formula() {
        assert_eq!(cover_from_presentation_us(0).unwrap().cover_ms, 0);
        let thirty_one = (31_i64 * 1000 + 15) / 30;
        let twelve = (12_i64 * 1000 + 15) / 30;
        assert_eq!(thirty_one, 1033);
        assert_eq!(twelve, 400);
    }

    #[test]
    fn presentation_time_selects_a_30fps_frame_without_copying_sample_frames() {
        let cover = cover_from_presentation_us(1_333_944).unwrap();
        assert_ne!(cover.image_time, 31);
        assert_eq!(
            i64::from(cover.cover_ms),
            (i64::from(cover.image_time) * 1000 + 15) / 30
        );
        let tail = extinfo_before_video(cover.image_time).unwrap();
        assert_eq!(
            parse_extinfo_tail(&tail),
            Some((MOTION_PHOTO_TYPE, cover.image_time))
        );
        assert!(!tail
            .windows(b"vivoMediaEStream".len())
            .any(|window| window == b"vivoMediaEStream"));
        assert!(!tail
            .windows(b"firstFrame".len())
            .any(|window| window == b"firstFrame"));
    }
}
