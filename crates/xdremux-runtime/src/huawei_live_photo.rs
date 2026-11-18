//! Huawei gallery 11.0.19.478 locates the video from a 40-byte tail.
//! `LiveUtils.getVideoOffset` reads the last 20 bytes. After trimming, they must
//! start with `LIVE_`, and the second `_` field is the video length L.
//! The video then starts at `file_size - L - 40`. The preceding 20 bytes are a
//! play-info field. `readPlayInfo` trims them and splits on `:` into two
//! integers. `VideoTextureView.preview` stores them in `mPreviewTime`.
//! `play` seeks with `MediaPlayer.seekTo(mPreviewTime[0])`. The first surface
//! update schedules `MSG_AUTO_STOP` after `mPreviewTime[1]` milliseconds and
//! then pauses. The pair is therefore milliseconds of start and duration, not
//! an end timestamp and not a file offset. A blank field returns null. `time`
//! must be greater than zero.

use xdremux_motion_photo::MediaTime;

use crate::{Result, RuntimeError};

const TAIL_LEN: usize = 40;
const FIELD_LEN: usize = 20;
const LIVE_TAG: &str = "LIVE_";

pub(crate) struct HuaweiPlayInfo {
    pub start_ms: i32,
    pub duration_ms: i32,
    pub notes: Vec<String>,
}

pub(crate) fn assemble(
    prefix: &[u8],
    video: &[u8],
    start_ms: i32,
    duration_ms: i32,
) -> Result<Vec<u8>> {
    if prefix.is_empty() || video.is_empty() {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "the still and the preserved video must both be non-empty",
        ));
    }
    if start_ms < 0 || duration_ms <= 0 {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "startPreview does not call preview when start is negative or time is not positive",
        ));
    }
    let play = format!("{start_ms}:{duration_ms}");
    if play.len() > FIELD_LEN {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "start:time does not fit in the 20-byte play-info field",
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
    tail[..play.len()].copy_from_slice(play.as_bytes());
    tail[FIELD_LEN..FIELD_LEN + tag.len()].copy_from_slice(tag.as_bytes());
    let mut output = Vec::with_capacity(prefix.len() + video.len() + TAIL_LEN);
    output.extend_from_slice(prefix);
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
    let written = parsed_play_info(&output[output.len() - TAIL_LEN..output.len() - FIELD_LEN]);
    if written != Some((start_ms, duration_ms))
        || !play_info_starts_preview(&output[output.len() - TAIL_LEN..output.len() - FIELD_LEN])
    {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "the written play-info field did not round-trip through readPlayInfo and the preview gate",
        ));
    }
    Ok(output)
}

/// `start` is the source presentation instant in milliseconds for `seekTo`.
/// Absent presentation stays 0 ms, which is the start of the preserved video,
/// not a fabricated cover frame. `time` is the `mvhd` duration in milliseconds
/// for the auto-stop delay. Both use integer division. A remainder is reported
/// and is not rounded into another millisecond.
pub(crate) fn play_info_for_preserved_video(
    video: &[u8],
    presentation: Option<MediaTime>,
) -> Result<HuaweiPlayInfo> {
    let (start_ms, start_note) = match presentation {
        None => (
            0,
            "source has no presentation timestamp; seekTo uses 0 ms and no cover frame was invented".to_owned(),
        ),
        Some(time) => {
            let (ms, remainder) = ticks_to_ms(time.value, time.timescale)?;
            let note = if remainder == 0 {
                format!(
                    "cover seek is the source presentation time {}/{} s, exactly {ms} ms",
                    time.value, time.timescale
                )
            } else {
                format!(
                    "cover seek truncates the source presentation time {}/{} s to {ms} ms; {remainder}/{} ms is not an integer millisecond",
                    time.value, time.timescale, time.timescale
                )
            };
            (ms, note)
        }
    };
    let (ticks, timescale) = movie_duration_ticks(video)?;
    let (duration_ms, remainder) = ticks_to_ms(
        i64::try_from(ticks).map_err(|_| {
            RuntimeError::new(
                "Huawei live photo",
                "mvhd duration does not fit a signed length",
            )
        })?,
        timescale,
    )?;
    if duration_ms <= 0 {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "the preserved video duration is not a positive number of milliseconds, so preview would not start",
        ));
    }
    let duration_note = if remainder == 0 {
        format!("preview duration is the preserved mvhd duration {ticks}/{timescale} s, exactly {duration_ms} ms")
    } else {
        format!(
            "preview duration truncates the preserved mvhd duration {ticks}/{timescale} s to {duration_ms} ms; {remainder}/{timescale} ms is not an integer millisecond"
        )
    };
    Ok(HuaweiPlayInfo {
        start_ms,
        duration_ms,
        notes: vec![start_note, duration_note],
    })
}

fn ticks_to_ms(value: i64, timescale: u32) -> Result<(i32, i128)> {
    if timescale == 0 {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "a zero timescale cannot convert to milliseconds",
        ));
    }
    if value < 0 {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "a negative time is not used as a preview start or duration",
        ));
    }
    let scaled = i128::from(value).checked_mul(1000).ok_or_else(|| {
        RuntimeError::new("Huawei live photo", "millisecond conversion overflowed")
    })?;
    let scale = i128::from(timescale);
    let ms = i32::try_from(scaled / scale).map_err(|_| {
        RuntimeError::new(
            "Huawei live photo",
            "millisecond value does not fit the signed play-info integer",
        )
    })?;
    Ok((ms, scaled % scale))
}

fn movie_duration_ticks(video: &[u8]) -> Result<(u64, u32)> {
    let moov = only_child(video, 0, video.len(), b"moov")?;
    let mvhd = only_child(video, moov.payload, moov.end, b"mvhd")?;
    let version = *video.get(mvhd.start + 8).ok_or_else(|| {
        RuntimeError::new("Huawei live photo", "mvhd is shorter than its version byte")
    })?;
    match version {
        0 => {
            let timescale = read_u32(video, mvhd.start + 20)?;
            let duration = u64::from(read_u32(video, mvhd.start + 24)?);
            Ok((duration, timescale))
        }
        1 => {
            let timescale = read_u32(video, mvhd.start + 28)?;
            let duration = read_u64(video, mvhd.start + 32)?;
            Ok((duration, timescale))
        }
        _ => Err(RuntimeError::new(
            "Huawei live photo",
            "mvhd version is neither 0 nor 1",
        )),
    }
}

struct BoxSpan {
    start: usize,
    payload: usize,
    end: usize,
}

fn only_child(data: &[u8], start: usize, end: usize, kind: &[u8; 4]) -> Result<BoxSpan> {
    let mut found = None;
    let mut cursor = start;
    while cursor + 8 <= end {
        let mut size = u32::from_be_bytes(data[cursor..cursor + 4].try_into().unwrap()) as u64;
        let mut header = 8usize;
        if size == 1 {
            if cursor + 16 > end {
                break;
            }
            size = u64::from_be_bytes(data[cursor + 8..cursor + 16].try_into().unwrap());
            header = 16;
        } else if size == 0 {
            size = (end - cursor) as u64;
        }
        if size < header as u64 || cursor as u64 + size > end as u64 {
            return Err(RuntimeError::new(
                "Huawei live photo",
                "a box in the preserved video is truncated",
            ));
        }
        let box_end = cursor + size as usize;
        if &data[cursor + 4..cursor + 8] == kind {
            if found.is_some() {
                return Err(RuntimeError::new(
                    "Huawei live photo",
                    "the preserved video has more than one moov or mvhd",
                ));
            }
            found = Some(BoxSpan {
                start: cursor,
                payload: cursor + header,
                end: box_end,
            });
        }
        cursor = box_end;
    }
    found.ok_or_else(|| {
        RuntimeError::new(
            "Huawei live photo",
            "the preserved video has no mvhd duration to convert",
        )
    })
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data.get(offset..offset + 4).ok_or_else(|| {
        RuntimeError::new(
            "Huawei live photo",
            "mvhd timescale or duration is truncated",
        )
    })?;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| RuntimeError::new("Huawei live photo", "mvhd duration is truncated"))?;
    Ok(u64::from_be_bytes(bytes.try_into().unwrap()))
}

/// Same rule as `LiveUtils.readPlayInfo`: 20 bytes, trim, split on `:`, then
/// two decimal integers. A blank field trims to empty and does not parse.
#[cfg(test)]
pub(crate) fn play_info_parses(field: &[u8]) -> bool {
    parsed_play_info(field).is_some()
}

/// `LivePlayer.startPreview` calls `preview` only when both integers parse,
/// `start >= 0`, and `time > 0`.
pub(crate) fn play_info_starts_preview(field: &[u8]) -> bool {
    parsed_play_info(field).is_some_and(|(start, time)| start >= 0 && time > 0)
}

fn parsed_play_info(field: &[u8]) -> Option<(i32, i32)> {
    let text = std::str::from_utf8(field).ok()?;
    let parts: Vec<&str> = text.trim().split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    Some((parts[0].parse().ok()?, parts[1].parse().ok()?))
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
    fn tail_locates_one_copy_of_the_video_and_writes_the_supplied_times() {
        let still = b"\xff\xd8still-exif\xff\xd9";
        let video = b"mp4-video-bytes-0123456789";
        let output = assemble(still, video, 0, 26).unwrap();
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
        let play = std::str::from_utf8(&output[output.len() - 40..output.len() - 20])
            .unwrap()
            .trim();
        assert_eq!(play, "0:26");
        assert!(play_info_starts_preview(
            &output[output.len() - 40..output.len() - 20]
        ));
        assert!(assemble(still, video, 0, 0).is_err());
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
    fn blank_play_info_does_not_parse_and_zero_time_does_not_preview() {
        assert!(!play_info_parses(&[b' '; 20]));
        assert!(!play_info_starts_preview(&[b' '; 20]));
        let mut zero = [b' '; 20];
        zero[..3].copy_from_slice(b"0:0");
        assert!(play_info_parses(&zero));
        assert!(!play_info_starts_preview(&zero));
        let mut negative = [b' '; 20];
        negative[..5].copy_from_slice(b"-1:30");
        assert!(play_info_parses(&negative));
        assert!(!play_info_starts_preview(&negative));
        // 19:30 only proves the parser. The writer uses the source presentation
        // time and the preserved mvhd duration, not this synthetic pair.
        let mut synthetic = [b' '; 20];
        synthetic[..5].copy_from_slice(b"19:30");
        assert!(play_info_starts_preview(&synthetic));
    }

    #[test]
    fn public_vivo_samples_use_presentation_and_mvhd_milliseconds() {
        use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions};
        for (name, expected) in [
            ("android-v1-sdr-01.jpg", (1333, 2501)),
            ("android-v1-sdr-02.jpg", (838, 1786)),
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/motion-photo/vivo")
                .join(name);
            let bytes = std::fs::read(&path).unwrap();
            let photo =
                MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::compatible()).unwrap();
            let video = photo.motion_video_bytes().unwrap();
            let info = play_info_for_preserved_video(video, photo.presentation_time()).unwrap();
            assert_eq!(
                (info.start_ms, info.duration_ms),
                expected,
                "{name} {:?}",
                info.notes
            );
            assert!(info.notes.iter().any(|note| note.contains("truncat")));
            let at = bytes
                .windows(video.len())
                .position(|window| window == video)
                .unwrap();
            assert_eq!(at + video.len(), bytes.len(), "{name}");
            let output = assemble(&bytes[..at], video, info.start_ms, info.duration_ms).unwrap();
            assert_eq!(&output[..at], &bytes[..at]);
            assert_eq!(recover_video(&output), Some(video));
        }
    }
}
