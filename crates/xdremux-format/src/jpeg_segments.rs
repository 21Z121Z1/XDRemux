use crate::error::{FormatError, Result};

const JPEG_CONTEXT: &str = "JPEG marker stream";
const ICC_PROFILE_SIGNATURE: &[u8] = b"ICC_PROFILE\0";

fn invalid(message: impl Into<String>) -> FormatError {
    FormatError::invalid(JPEG_CONTEXT, message)
}

fn checked_add(value: usize, amount: usize, context: &'static str) -> Result<usize> {
    value
        .checked_add(amount)
        .ok_or_else(|| FormatError::overflow(context))
}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16> {
    let end = checked_add(offset, 2, "JPEG segment length offset")?;
    let bytes = data.get(offset..end).ok_or(FormatError::UnexpectedEof {
        context: JPEG_CONTEXT,
        offset,
        needed: 2,
        end: data.len(),
    })?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn is_standalone_marker(marker: u8) -> bool {
    marker == 0x01 || marker == 0xd8 || marker == 0xd9 || (0xd0..=0xd7).contains(&marker)
}

/// Return the exclusive end offset of one JPEG image beginning at `start`.
///
/// The parser follows marker lengths and entropy-coded scan escaping instead of
/// searching for the first `FFD9` byte sequence. Progressive/multi-scan JPEGs,
/// byte stuffing, and restart markers are therefore handled without accepting a
/// false EOI from compressed scan data.
pub fn jpeg_image_end(data: &[u8], start: usize) -> Result<usize> {
    let soi_end = checked_add(start, 2, "JPEG SOI offset")?;
    if data.get(start..soi_end) != Some(&[0xff, 0xd8]) {
        return Err(invalid(format!("missing SOI marker at offset {start}")));
    }

    let mut cursor = soi_end;
    let mut entropy_coded = false;
    while cursor < data.len() {
        if entropy_coded {
            let relative = data[cursor..]
                .iter()
                .position(|byte| *byte == 0xff)
                .ok_or_else(|| invalid("entropy-coded scan has no terminating marker"))?;
            let marker_start = checked_add(cursor, relative, "JPEG entropy marker offset")?;
            let mut marker_cursor = checked_add(marker_start, 1, "JPEG entropy marker offset")?;
            while data.get(marker_cursor) == Some(&0xff) {
                marker_cursor = checked_add(marker_cursor, 1, "JPEG entropy fill-byte offset")?;
            }
            let marker = *data.get(marker_cursor).ok_or(FormatError::UnexpectedEof {
                context: JPEG_CONTEXT,
                offset: marker_cursor,
                needed: 1,
                end: data.len(),
            })?;
            if marker == 0x00 || (0xd0..=0xd7).contains(&marker) {
                cursor = checked_add(marker_cursor, 1, "JPEG entropy marker end")?;
                continue;
            }
            cursor = marker_start;
            entropy_coded = false;
            continue;
        }

        if data.get(cursor) != Some(&0xff) {
            return Err(invalid(format!("expected marker prefix at offset {cursor}")));
        }
        while data.get(cursor) == Some(&0xff) {
            cursor = checked_add(cursor, 1, "JPEG marker offset")?;
        }
        let marker = *data.get(cursor).ok_or(FormatError::UnexpectedEof {
            context: JPEG_CONTEXT,
            offset: cursor,
            needed: 1,
            end: data.len(),
        })?;
        cursor = checked_add(cursor, 1, "JPEG marker offset")?;

        if marker == 0x00 {
            return Err(invalid("stuffed marker byte appears outside scan data"));
        }
        if marker == 0xd9 {
            return Ok(cursor);
        }
        if marker == 0xd8 {
            return Err(invalid("unexpected nested SOI marker"));
        }
        if is_standalone_marker(marker) {
            continue;
        }

        let segment_length = usize::from(read_u16_be(data, cursor)?);
        if segment_length < 2 {
            return Err(invalid(format!(
                "marker 0x{marker:02x} declares invalid segment length {segment_length}"
            )));
        }
        let segment_end = checked_add(cursor, segment_length, "JPEG segment end")?;
        if segment_end > data.len() {
            return Err(FormatError::UnexpectedEof {
                context: JPEG_CONTEXT,
                offset: cursor,
                needed: segment_length,
                end: data.len(),
            });
        }
        entropy_coded = marker == 0xda;
        cursor = segment_end;
    }

    Err(invalid("JPEG image has no EOI marker"))
}

/// Reassemble an ICC profile carried by JPEG APP2 `ICC_PROFILE` chunks.
///
/// Missing ICC metadata is represented as `Ok(None)`. Once an ICC sequence is
/// present, malformed sequence numbers, inconsistent chunk counts, duplicates,
/// or missing chunks fail closed instead of silently producing a partial profile.
pub fn jpeg_icc_profile(jpeg: &[u8]) -> Result<Option<Vec<u8>>> {
    let end = jpeg_image_end(jpeg, 0)?;
    let mut cursor = 2usize;
    let mut expected_chunks: Option<u8> = None;
    let mut chunks: Vec<Option<Vec<u8>>> = Vec::new();

    while cursor < end {
        if jpeg.get(cursor) != Some(&0xff) {
            return Err(invalid(format!("expected marker prefix at offset {cursor}")));
        }
        while jpeg.get(cursor) == Some(&0xff) {
            cursor = checked_add(cursor, 1, "JPEG marker offset")?;
        }
        let marker = *jpeg.get(cursor).ok_or(FormatError::UnexpectedEof {
            context: JPEG_CONTEXT,
            offset: cursor,
            needed: 1,
            end,
        })?;
        cursor = checked_add(cursor, 1, "JPEG marker offset")?;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if is_standalone_marker(marker) {
            continue;
        }

        let segment_length = usize::from(read_u16_be(jpeg, cursor)?);
        if segment_length < 2 {
            return Err(invalid("invalid JPEG segment length while reading ICC"));
        }
        let payload_start = checked_add(cursor, 2, "JPEG ICC payload offset")?;
        let segment_end = checked_add(cursor, segment_length, "JPEG ICC segment end")?;
        if segment_end > end {
            return Err(invalid("JPEG ICC segment exceeds image boundary"));
        }
        let payload = &jpeg[payload_start..segment_end];
        if marker == 0xe2 && payload.starts_with(ICC_PROFILE_SIGNATURE) {
            let header = ICC_PROFILE_SIGNATURE.len();
            let sequence = *payload
                .get(header)
                .ok_or_else(|| invalid("truncated ICC APP2 sequence number"))?;
            let count = *payload
                .get(header + 1)
                .ok_or_else(|| invalid("truncated ICC APP2 chunk count"))?;
            if sequence == 0 || count == 0 || sequence > count {
                return Err(invalid("invalid ICC APP2 sequence numbering"));
            }
            if let Some(expected) = expected_chunks {
                if expected != count {
                    return Err(invalid("inconsistent ICC APP2 chunk counts"));
                }
            } else {
                expected_chunks = Some(count);
                chunks.resize_with(usize::from(count), || None);
            }
            let index = usize::from(sequence - 1);
            if chunks[index].is_some() {
                return Err(invalid("duplicate ICC APP2 sequence number"));
            }
            chunks[index] = Some(payload[header + 2..].to_vec());
        }
        cursor = segment_end;
    }

    let Some(_) = expected_chunks else {
        return Ok(None);
    };
    if chunks.iter().any(Option::is_none) {
        return Err(invalid("JPEG ICC profile is missing one or more APP2 chunks"));
    }
    let total = chunks.iter().try_fold(0usize, |sum, chunk| {
        sum.checked_add(chunk.as_ref().map_or(0, Vec::len))
            .ok_or_else(|| FormatError::overflow("JPEG ICC profile length"))
    })?;
    let mut profile = Vec::with_capacity(total);
    for chunk in chunks {
        profile.extend_from_slice(chunk.as_deref().expect("validated ICC chunk presence"));
    }
    Ok(Some(profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let length = u16::try_from(payload.len() + 2).unwrap();
        let mut output = vec![0xff, marker];
        output.extend_from_slice(&length.to_be_bytes());
        output.extend_from_slice(payload);
        output
    }

    #[test]
    fn finds_eoi_after_stuffed_scan_bytes_and_restart_markers() {
        let mut jpeg = vec![0xff, 0xd8];
        jpeg.extend_from_slice(&segment(0xda, b"scan"));
        jpeg.extend_from_slice(&[0x11, 0xff, 0x00, 0xd9, 0xff, 0xd2, 0x22, 0xff, 0xd9]);
        jpeg.extend_from_slice(b"tail");
        assert_eq!(jpeg_image_end(&jpeg, 0).unwrap(), jpeg.len() - 4);
    }

    #[test]
    fn reassembles_icc_chunks_in_sequence_order() {
        let mut jpeg = vec![0xff, 0xd8];
        let mut second = ICC_PROFILE_SIGNATURE.to_vec();
        second.extend_from_slice(&[2, 2]);
        second.extend_from_slice(b"world");
        let mut first = ICC_PROFILE_SIGNATURE.to_vec();
        first.extend_from_slice(&[1, 2]);
        first.extend_from_slice(b"hello ");
        jpeg.extend_from_slice(&segment(0xe2, &second));
        jpeg.extend_from_slice(&segment(0xe2, &first));
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        assert_eq!(jpeg_icc_profile(&jpeg).unwrap().as_deref(), Some(b"hello world".as_slice()));
    }

    #[test]
    fn missing_icc_chunk_fails_closed() {
        let mut jpeg = vec![0xff, 0xd8];
        let mut first = ICC_PROFILE_SIGNATURE.to_vec();
        first.extend_from_slice(&[1, 2]);
        first.extend_from_slice(b"partial");
        jpeg.extend_from_slice(&segment(0xe2, &first));
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        assert!(jpeg_icc_profile(&jpeg).is_err());
    }
}
