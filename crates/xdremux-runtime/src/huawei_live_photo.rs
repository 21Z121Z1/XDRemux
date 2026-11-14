//! Huawei gallery 11.0.19.478 locates the video from a 40-byte tail.
//! `LiveUtils.getVideoOffset` reads the last 20 bytes. After trimming, they must
//! start with `LIVE_`, and the second `_` field is the video length L.
//! The video then starts at `file_size - L - 40`. The preceding 20 bytes are a
//! play-info field. `readPlayInfo` trims them and splits on `:` into two
//! integers. A blank field returns null. Those integers are not named here and
//! no fixture supplies them, so this module does not publish a file.

#[cfg(test)]
use crate::{Result, RuntimeError};

#[cfg(test)]
const TAIL_LEN: usize = 40;
#[cfg(test)]
const FIELD_LEN: usize = 20;
#[cfg(test)]
const LIVE_TAG: &str = "LIVE_";

#[cfg(test)]
pub(crate) fn assemble(still: &[u8], video: &[u8]) -> Result<Vec<u8>> {
    if still.is_empty() || video.is_empty() {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "the still and the preserved video must both be non-empty",
        ));
    }
    let video_len = u64::try_from(video.len()).map_err(|_| {
        RuntimeError::new(
            "Huawei live photo",
            "video length does not fit the tail field",
        )
    })?;
    let tag = format!("{LIVE_TAG}{video_len}");
    if tag.len() > FIELD_LEN {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "video length does not fit in the 20-byte LIVE_ field",
        ));
    }
    let mut tail = [b' '; TAIL_LEN];
    tail[FIELD_LEN..FIELD_LEN + tag.len()].copy_from_slice(tag.as_bytes());
    let mut output = Vec::with_capacity(still.len() + video.len() + TAIL_LEN);
    output.extend_from_slice(still);
    output.extend_from_slice(video);
    output.extend_from_slice(&tail);
    let recovered = recover_video(&output).ok_or_else(|| {
        RuntimeError::new(
            "Huawei live photo",
            "the 40-byte tail did not locate the preserved video",
        )
    })?;
    if recovered != video {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "the tail formula did not return the original video bytes",
        ));
    }
    Ok(output)
}

/// Same rule as `LiveUtils.readPlayInfo`: 20 bytes, trim, split on `:`, then
/// two decimal integers. A blank field trims to empty and does not parse.
pub(crate) fn play_info_parses(field: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(field) else {
        return false;
    };
    let parts: Vec<&str> = text.trim().split(':').collect();
    parts.len() >= 2 && parts[0].parse::<i32>().is_ok() && parts[1].parse::<i32>().is_ok()
}

/// Mirror `LiveUtils.getVideoOffset`: last 20 bytes, trim, `LIVE_`, then L,
/// then `file_size - L - 40`.
#[cfg(test)]
pub(crate) fn recover_video(file: &[u8]) -> Option<&[u8]> {
    if file.len() < TAIL_LEN {
        return None;
    }
    let tag = std::str::from_utf8(&file[file.len() - FIELD_LEN..]).ok()?;
    let trimmed = tag.trim();
    if !trimmed.starts_with(LIVE_TAG) {
        return None;
    }
    let length: usize = trimmed.split('_').nth(1)?.parse().ok()?;
    let offset = file.len().checked_sub(length)?.checked_sub(TAIL_LEN)?;
    let video = file.get(offset..offset + length)?;
    Some(video)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_locates_one_copy_of_the_video_and_leaves_play_info_blank() {
        let still = b"\xff\xd8still-exif\xff\xd9";
        let video = b"mp4-video-bytes-0123456789";
        let output = assemble(still, video).unwrap();
        let tag = std::str::from_utf8(&output[output.len() - 20..])
            .unwrap()
            .trim();
        assert_eq!(tag, "LIVE_26");
        assert_eq!(recover_video(&output), Some(video.as_slice()));
        assert_eq!(
            output
                .windows(video.len())
                .filter(|window| *window == video)
                .count(),
            1
        );
        let play = &output[output.len() - 40..output.len() - 20];
        assert!(play.iter().all(|byte| *byte == b' '));
        assert!(!output
            .windows(b"MicroVideoVersion".len())
            .any(|w| w == b"MicroVideoVersion"));
    }

    #[test]
    fn synthetic_tails_match_the_verified_offset_formula() {
        let v1 = [
            0xff, 0xd8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56,
            0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x31, 0x39, 0x3a, 0x33,
            0x30, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x4c, 0x49, 0x56, 0x45, 0x5f, 0x31, 0x36, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        ];
        let video = recover_video(&v1).unwrap();
        assert_eq!(video, &v1[8..24]);
        let v2 = [
            0xff, 0xd8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56,
            0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x76, 0x32, 0x5f, 0x66,
            0x31, 0x39, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x31, 0x39, 0x3a, 0x33, 0x30, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x4c, 0x49, 0x56, 0x45, 0x5f, 0x33,
            0x36, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        ];
        let span = recover_video(&v2).unwrap();
        assert_eq!(span.len(), 36);
        assert_eq!(&span[..16], &v2[8..24]);
        assert!(span.windows(6).any(|window| window == b"v2_f19"));
    }

    #[test]
    fn blank_play_info_does_not_parse_and_the_synthetic_pair_does() {
        assert!(!play_info_parses(&[b' '; 20]));
        let mut field = [b' '; 20];
        field[..5].copy_from_slice(b"19:30");
        assert!(play_info_parses(&field));
    }
}
