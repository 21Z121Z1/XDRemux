use crate::{MotionPhotoError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedVideo<'a> {
    pub data: &'a [u8],
    pub removed_vendor_bytes: usize,
}

fn printable_fourcc(kind: &[u8]) -> bool {
    kind.len() == 4 && kind.iter().all(|byte| (0x20..=0x7e).contains(byte))
}

fn top_level_box_size(data: &[u8], offset: usize) -> Option<(usize, [u8; 4])> {
    let remaining = data.len().checked_sub(offset)?;
    if remaining < 8 {
        return None;
    }
    let size32 = u32::from_be_bytes(data.get(offset..offset + 4)?.try_into().ok()?);
    let kind: [u8; 4] = data.get(offset + 4..offset + 8)?.try_into().ok()?;
    if !printable_fourcc(&kind) {
        return None;
    }
    let (size, header_size) = match size32 {
        0 => (remaining, 8),
        1 => {
            if remaining < 16 {
                return None;
            }
            let value = u64::from_be_bytes(data.get(offset + 8..offset + 16)?.try_into().ok()?);
            (usize::try_from(value).ok()?, 16)
        }
        value => (usize::try_from(value).ok()?, 8),
    };
    if size < header_size || size > remaining {
        return None;
    }
    Some((size, kind))
}

/// Return the complete standalone BMFF prefix of an embedded Motion Photo video.
///
/// ColorOS 16 Stream 1 may append opaque vendor bytes after a complete `ftyp` /
/// `moov` / `mdat` container. Those bytes are outside the media container and
/// must be excluded before strict parsing or Live Photo remuxing. Invalid bytes
/// before the required box set is complete remain a hard failure.
pub fn standalone_bmff_length(data: &[u8]) -> Result<usize> {
    let mut offset = 0_usize;
    let mut saw_ftyp = false;
    let mut saw_moov = false;
    let mut saw_mdat = false;

    while offset < data.len() {
        let Some((size, kind)) = top_level_box_size(data, offset) else {
            if saw_ftyp && saw_moov && saw_mdat {
                break;
            }
            return Err(MotionPhotoError::InvalidVideoPayload);
        };
        if offset == 0 {
            if kind != *b"ftyp" {
                return Err(MotionPhotoError::InvalidVideoPayload);
            }
            saw_ftyp = true;
        } else if kind == *b"ftyp" {
            // A second top-level ftyp belongs to another embedded stream rather
            // than to the first standalone movie.
            if saw_moov && saw_mdat {
                break;
            }
            return Err(MotionPhotoError::InvalidVideoPayload);
        }
        saw_moov |= kind == *b"moov";
        saw_mdat |= kind == *b"mdat";
        offset = offset
            .checked_add(size)
            .ok_or(MotionPhotoError::ArithmeticOverflow)?;
    }

    if !saw_ftyp || !saw_moov || !saw_mdat {
        return Err(MotionPhotoError::InvalidVideoPayload);
    }
    Ok(offset)
}

pub fn normalize_embedded_video(data: &[u8]) -> Result<NormalizedVideo<'_>> {
    let length = standalone_bmff_length(data)?;
    Ok(NormalizedVideo {
        data: &data[..length],
        removed_vendor_bytes: data.len() - length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(payload.len() + 8).unwrap();
        let mut output = size.to_be_bytes().to_vec();
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        output
    }

    fn standalone_video() -> Vec<u8> {
        [
            boxed(b"ftyp", b"isom\0\0\0\0"),
            boxed(b"mdat", b"media"),
            boxed(b"moov", b"index"),
        ]
        .concat()
    }

    #[test]
    fn strips_only_invalid_suffix_after_complete_container() {
        let clean = standalone_video();
        let mut source = clean.clone();
        source.extend_from_slice(b"opaque-vendor-tail");
        let normalized = normalize_embedded_video(&source).unwrap();
        assert_eq!(normalized.data, clean);
        assert_eq!(normalized.removed_vendor_bytes, 18);
    }

    #[test]
    fn malformed_data_before_required_boxes_fails_closed() {
        let mut source = boxed(b"ftyp", b"isom\0\0\0\0");
        source.extend_from_slice(b"opaque-vendor-tail");
        assert_eq!(
            normalize_embedded_video(&source).unwrap_err(),
            MotionPhotoError::InvalidVideoPayload
        );
    }

    #[test]
    fn second_stream_starts_at_second_ftyp_after_complete_first_stream() {
        let first = standalone_video();
        let second = standalone_video();
        let source = [first.clone(), second].concat();
        let normalized = normalize_embedded_video(&source).unwrap();
        assert_eq!(normalized.data, first);
        assert_eq!(normalized.removed_vendor_bytes, source.len() - first.len());
    }
}
