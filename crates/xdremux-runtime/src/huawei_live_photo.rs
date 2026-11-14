//! Huawei gallery 11.0.19.478 locates the video from a 40-byte tail.
//! `LiveUtils.getVideoOffset` reads the last 20 bytes. After trimming, they must
//! start with `LIVE_`, and the second `_` field is the video length L.
//! The video then starts at `file_size - L - 40`. The preceding 20 bytes are a
//! play-info field. This build only shows that `readPlayInfo` splits them on
//! `:` into two integers. Their meaning is not established, so they stay blank.

use crate::{Result, RuntimeError};

const TAIL_LEN: usize = 40;
const FIELD_LEN: usize = 20;
const LIVE_TAG: &str = "LIVE_";

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

/// Mirror `LiveUtils.getVideoOffset`: last 20 bytes, trim, `LIVE_`, then L,
/// then `file_size - L - 40`.
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
        let tag = std::str::from_utf8(&output[output.len() - 20..]).unwrap().trim();
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
}
