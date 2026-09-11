use crate::error::{MotionPhotoError, Result};
use crate::model::{ByteRange, VideoStream, VideoStreamLayout, VideoStreamRole};
use crate::scanner::ftyp_box_offsets;

pub fn enrich_oppo_video_range(
    data: &[u8],
    declared_still: ByteRange,
    declared_video: ByteRange,
    lpex_version: i64,
) -> Result<(ByteRange, ByteRange, usize)> {
    let file_size = u64::try_from(data.len()).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
    if declared_still.upper_bound > file_size || declared_video.upper_bound > file_size {
        return Err(MotionPhotoError::InvalidByteRange);
    }
    let tail_start = file_size.saturating_sub(512 * 1024 * 1024);
    let offsets = ftyp_box_offsets(data, ByteRange::new(tail_start, file_size)?, 1 << 20)?;
    if lpex_version >= 1 && offsets.len() >= 2 {
        let stream1_start = offsets[offsets.len() - 2];
        return Ok((
            ByteRange::new(0, stream1_start)?,
            ByteRange::new(stream1_start, file_size)?,
            2,
        ));
    }
    let count = offsets
        .iter()
        .filter(|offset| {
            **offset >= declared_video.lower_bound && **offset < declared_video.upper_bound
        })
        .count()
        .max(1);
    Ok((declared_still, declared_video, count))
}

pub fn resolve_video_stream_layout(
    data: &[u8],
    video_range: ByteRange,
    is_oppo_live_photo: bool,
    stream_count: usize,
) -> Result<VideoStreamLayout> {
    if !is_oppo_live_photo || stream_count < 2 {
        return Ok(VideoStreamLayout {
            primary: VideoStream {
                index: 0,
                role: VideoStreamRole::Primary,
                range: video_range,
            },
            auxiliary_geometry: Vec::new(),
        });
    }
    let offsets = ftyp_box_offsets(data, video_range, 1 << 20)?;
    if offsets.len() < 2 {
        return Ok(VideoStreamLayout {
            primary: VideoStream {
                index: 0,
                role: VideoStreamRole::Primary,
                range: video_range,
            },
            auxiliary_geometry: Vec::new(),
        });
    }
    let stream1_start = offsets[offsets.len() - 2];
    let stream2_start = offsets[offsets.len() - 1];
    if stream1_start < video_range.lower_bound
        || stream2_start <= stream1_start
        || stream2_start >= video_range.upper_bound
    {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    Ok(VideoStreamLayout {
        primary: VideoStream {
            index: 0,
            role: VideoStreamRole::Primary,
            range: ByteRange::new(stream1_start, stream2_start)?,
        },
        auxiliary_geometry: vec![VideoStream {
            index: 1,
            role: VideoStreamRole::AuxiliaryGeometry,
            range: ByteRange::new(stream2_start, video_range.upper_bound)?,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_mp4(brand: &[u8; 4], payload: u8) -> Vec<u8> {
        let mut data = vec![0, 0, 0, 16];
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(brand);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0, 12]);
        data.extend_from_slice(b"mdat");
        data.extend_from_slice(&[payload; 4]);
        data
    }

    #[test]
    fn coloros16_uses_penultimate_ftyp_as_primary_stream_start() {
        let still = vec![0xff, 0xd8, 0xff, 0xd9];
        let stream1 = fake_mp4(b"isom", 0x11);
        let stream2 = fake_mp4(b"mp42", 0x22);
        let stream1_start = still.len() as u64;
        let stream2_start = stream1_start + stream1.len() as u64;
        let mut data = still;
        data.extend_from_slice(&stream1);
        data.extend_from_slice(&stream2);
        let file_size = data.len() as u64;

        let (_, video, count) = enrich_oppo_video_range(
            &data,
            ByteRange::new(0, stream2_start).unwrap(),
            ByteRange::new(stream2_start, file_size).unwrap(),
            1,
        )
        .unwrap();
        assert_eq!(video, ByteRange::new(stream1_start, file_size).unwrap());
        assert_eq!(count, 2);

        let layout = resolve_video_stream_layout(&data, video, true, count).unwrap();
        assert_eq!(
            layout.primary.range,
            ByteRange::new(stream1_start, stream2_start).unwrap()
        );
        assert_eq!(
            layout.auxiliary_geometry[0].range,
            ByteRange::new(stream2_start, file_size).unwrap()
        );
    }

    #[test]
    fn generic_android_never_invents_auxiliary_stream() {
        let data = fake_mp4(b"isom", 0x11);
        let range = ByteRange::new(0, data.len() as u64).unwrap();
        let layout = resolve_video_stream_layout(&data, range, false, 2).unwrap();
        assert_eq!(layout.primary.range, range);
        assert!(layout.auxiliary_geometry.is_empty());
    }
}
