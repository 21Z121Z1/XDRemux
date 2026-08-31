use crate::error::{MotionPhotoError, Result};
use crate::model::{ByteRange, MotionPhotoItem};
use crate::scanner::is_ftyp_box_start;

const MAX_TOP_LEVEL_BOXES: usize = 4096;

#[derive(Debug, Clone, Copy)]
struct TopLevelBox {
    offset: u64,
    end_offset: u64,
    header_size: u64,
    kind: [u8; 4],
}

impl TopLevelBox {
    fn payload_offset(self) -> Result<u64> {
        self.offset
            .checked_add(self.header_size)
            .ok_or(MotionPhotoError::ArithmeticOverflow)
    }
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or(MotionPhotoError::ArithmeticOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(MotionPhotoError::InvalidVideoPayload)?;
    Ok(u32::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| MotionPhotoError::InvalidVideoPayload)?,
    ))
}

fn read_u64_be(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset
        .checked_add(8)
        .ok_or(MotionPhotoError::ArithmeticOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(MotionPhotoError::InvalidVideoPayload)?;
    Ok(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| MotionPhotoError::InvalidVideoPayload)?,
    ))
}

fn top_level_boxes(data: &[u8]) -> Result<Vec<TopLevelBox>> {
    let file_size = u64::try_from(data.len()).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
    let mut boxes = Vec::new();
    let mut offset = 0u64;
    while offset < file_size {
        if boxes.len() >= MAX_TOP_LEVEL_BOXES {
            return Err(MotionPhotoError::InvalidVideoPayload);
        }
        let remaining = file_size - offset;
        if remaining < 8 {
            return Err(MotionPhotoError::InvalidVideoPayload);
        }
        let pos = usize::try_from(offset).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
        let size32 = read_u32_be(data, pos)?;
        let kind_slice = data
            .get(pos + 4..pos + 8)
            .ok_or(MotionPhotoError::InvalidVideoPayload)?;
        let kind: [u8; 4] = kind_slice
            .try_into()
            .map_err(|_| MotionPhotoError::InvalidVideoPayload)?;

        let (header_size, box_size) = if size32 == 1 {
            if remaining < 16 {
                return Err(MotionPhotoError::InvalidVideoPayload);
            }
            (16u64, read_u64_be(data, pos + 8)?)
        } else if size32 == 0 {
            (8u64, remaining)
        } else {
            (8u64, u64::from(size32))
        };
        if box_size < header_size {
            return Err(MotionPhotoError::InvalidVideoPayload);
        }
        let end_offset = offset
            .checked_add(box_size)
            .ok_or(MotionPhotoError::ArithmeticOverflow)?;
        if end_offset <= offset || end_offset > file_size {
            return Err(MotionPhotoError::ArithmeticOverflow);
        }
        boxes.push(TopLevelBox {
            offset,
            end_offset,
            header_size,
            kind,
        });
        offset = end_offset;
    }
    if offset != file_size {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    Ok(boxes)
}

pub fn is_heif_mime(mime: &str) -> bool {
    mime.eq_ignore_ascii_case("image/heic") || mime.eq_ignore_ascii_case("image/heif")
}

pub fn resolve_heif_motion_photo_ranges(
    data: &[u8],
    items: &[MotionPhotoItem],
) -> Result<(ByteRange, ByteRange)> {
    let primary = items.first().ok_or(MotionPhotoError::InvalidDirectory)?;
    let motion = items.last().ok_or(MotionPhotoError::InvalidDirectory)?;
    if !is_heif_mime(&primary.mime)
        || !motion.semantic.eq_ignore_ascii_case("MotionPhoto")
    {
        return Err(MotionPhotoError::InvalidDirectory);
    }
    if primary.padding != 8 {
        return Err(MotionPhotoError::InvalidItemLength);
    }

    let boxes = top_level_boxes(data)?;
    if boxes.first().map(|value| value.kind) != Some(*b"ftyp") {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    let mpvd: Vec<_> = boxes.iter().filter(|value| value.kind == *b"mpvd").collect();
    if mpvd.len() != 1 {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    let mpvd = *mpvd[0];
    let payload_start = mpvd.payload_offset()?;
    if payload_start >= mpvd.end_offset {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    let file_size = u64::try_from(data.len()).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
    let declared_start = file_size
        .checked_sub(motion.length)
        .ok_or(MotionPhotoError::InvalidByteRange)?;
    if declared_start != payload_start {
        return Err(MotionPhotoError::InvalidByteRange);
    }
    if !is_ftyp_box_start(data, payload_start, mpvd.end_offset)? {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    Ok((
        ByteRange::new(0, mpvd.offset)?,
        ByteRange::new(payload_start, mpvd.end_offset)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(payload.len() + 8).unwrap();
        let mut output = size.to_be_bytes().to_vec();
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        output
    }

    #[test]
    fn extracts_mpvd_payload_without_trailing_vendor_box() {
        let video = make_box(b"ftyp", b"isom\0\0\0\0");
        let ftyp = make_box(b"ftyp", b"heic\0\0\0\0");
        let mpvd = make_box(b"mpvd", &video);
        let sefd = make_box(b"sefd", &[1, 2, 3, 4]);
        let mut data = ftyp.clone();
        let mpvd_offset = data.len() as u64;
        data.extend_from_slice(&mpvd);
        data.extend_from_slice(&sefd);
        let payload_start = mpvd_offset + 8;
        let motion_length = data.len() as u64 - payload_start;
        let items = vec![
            MotionPhotoItem {
                mime: "image/heic".into(),
                semantic: "Primary".into(),
                length: 0,
                padding: 8,
            },
            MotionPhotoItem {
                mime: "video/mp4".into(),
                semantic: "MotionPhoto".into(),
                length: motion_length,
                padding: 0,
            },
        ];
        let (still, extracted) = resolve_heif_motion_photo_ranges(&data, &items).unwrap();
        assert_eq!(still, ByteRange::new(0, mpvd_offset).unwrap());
        assert_eq!(extracted, ByteRange::new(payload_start, payload_start + video.len() as u64).unwrap());
    }
}
