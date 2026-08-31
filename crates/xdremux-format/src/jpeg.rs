use crate::codec::ChromaSampling;
use crate::error::{FormatError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegComponent {
    pub id: u8,
    pub horizontal_sampling: u8,
    pub vertical_sampling: u8,
    pub quantization_table: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JpegFrameProfile {
    pub sof_marker: u8,
    pub precision: u8,
    pub width: u16,
    pub height: u16,
    pub components: Vec<JpegComponent>,
    /// Known JPEG sampling class for one- or three-component frames.
    /// Unusual component layouts remain inspectable but report `None` rather
    /// than being silently forced into a 4:2:x class.
    pub chroma_sampling: Option<ChromaSampling>,
}

impl JpegFrameProfile {
    pub fn component_count(&self) -> u8 {
        u8::try_from(self.components.len()).unwrap_or(u8::MAX)
    }
}

fn invalid(message: impl Into<String>) -> FormatError {
    FormatError::invalid("JPEG frame", message)
}

fn truncated(offset: usize, needed: usize, end: usize) -> FormatError {
    FormatError::UnexpectedEof {
        context: "JPEG frame",
        offset,
        needed,
        end,
    }
}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| FormatError::overflow("JPEG frame offset"))?;
    let bytes = data
        .get(offset..end)
        .ok_or_else(|| truncated(offset, 2, data.len()))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn is_standalone_marker(marker: u8) -> bool {
    marker == 0x01 || marker == 0xd8 || marker == 0xd9 || (0xd0..=0xd7).contains(&marker)
}

fn is_sof_marker(marker: u8) -> bool {
    matches!(
        marker,
        0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf
    )
}

fn classify_sampling(components: &[JpegComponent]) -> Option<ChromaSampling> {
    match components {
        [_] => Some(ChromaSampling::Mono400),
        [luma, chroma_a, chroma_b]
            if chroma_a.horizontal_sampling == chroma_b.horizontal_sampling
                && chroma_a.vertical_sampling == chroma_b.vertical_sampling =>
        {
            let chroma_h = chroma_a.horizontal_sampling;
            let chroma_v = chroma_a.vertical_sampling;
            if luma.horizontal_sampling == chroma_h && luma.vertical_sampling == chroma_v {
                Some(ChromaSampling::Yuv444)
            } else if luma.horizontal_sampling == chroma_h.saturating_mul(2)
                && luma.vertical_sampling == chroma_v
            {
                Some(ChromaSampling::Yuv422)
            } else if luma.horizontal_sampling == chroma_h.saturating_mul(2)
                && luma.vertical_sampling == chroma_v.saturating_mul(2)
            {
                Some(ChromaSampling::Yuv420)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_sof(data: &[u8], marker: u8, payload_start: usize, payload_end: usize) -> Result<JpegFrameProfile> {
    let payload = data
        .get(payload_start..payload_end)
        .ok_or_else(|| truncated(payload_start, payload_end.saturating_sub(payload_start), data.len()))?;
    if payload.len() < 6 {
        return Err(invalid("SOF payload is shorter than its fixed header"));
    }

    let precision = payload[0];
    let height = u16::from_be_bytes([payload[1], payload[2]]);
    let width = u16::from_be_bytes([payload[3], payload[4]]);
    let component_count = usize::from(payload[5]);
    if width == 0 || height == 0 {
        return Err(invalid("SOF dimensions must be non-zero"));
    }
    if component_count == 0 {
        return Err(invalid("SOF declares zero components"));
    }
    let component_bytes = component_count
        .checked_mul(3)
        .ok_or_else(|| FormatError::overflow("JPEG SOF component bytes"))?;
    let expected = 6usize
        .checked_add(component_bytes)
        .ok_or_else(|| FormatError::overflow("JPEG SOF payload length"))?;
    if payload.len() != expected {
        return Err(invalid(format!(
            "SOF declares {component_count} components but has {} payload bytes",
            payload.len()
        )));
    }

    let mut components = Vec::with_capacity(component_count);
    for chunk in payload[6..].chunks_exact(3) {
        let sampling = chunk[1];
        let horizontal_sampling = sampling >> 4;
        let vertical_sampling = sampling & 0x0f;
        if horizontal_sampling == 0 || vertical_sampling == 0 {
            return Err(invalid("SOF contains a zero sampling factor"));
        }
        components.push(JpegComponent {
            id: chunk[0],
            horizontal_sampling,
            vertical_sampling,
            quantization_table: chunk[2],
        });
    }
    let chroma_sampling = classify_sampling(&components);
    Ok(JpegFrameProfile {
        sof_marker: marker,
        precision,
        width,
        height,
        components,
        chroma_sampling,
    })
}

/// Probes the first JPEG Start Of Frame segment without decoding image pixels.
///
/// This is intended for source-profile planning: width, height, precision,
/// component count, and JPEG sampling are available before choosing a decoder or
/// encoder backend. The parser is marker-length bounded and fails closed on
/// truncated or malformed input.
pub fn probe_frame_profile(data: &[u8]) -> Result<JpegFrameProfile> {
    if data.get(..2) != Some(&[0xff, 0xd8]) {
        return Err(invalid("missing SOI marker"));
    }

    let mut offset = 2usize;
    while offset < data.len() {
        if data[offset] != 0xff {
            return Err(invalid(format!(
                "expected marker prefix at offset {offset}"
            )));
        }
        while data.get(offset) == Some(&0xff) {
            offset = offset
                .checked_add(1)
                .ok_or_else(|| FormatError::overflow("JPEG marker offset"))?;
        }
        let marker = *data
            .get(offset)
            .ok_or_else(|| truncated(offset, 1, data.len()))?;
        offset = offset
            .checked_add(1)
            .ok_or_else(|| FormatError::overflow("JPEG marker offset"))?;

        if marker == 0x00 {
            return Err(invalid("encountered stuffed marker byte before SOS"));
        }
        if marker == 0xda {
            return Err(invalid("reached SOS before any SOF segment"));
        }
        if marker == 0xd9 {
            return Err(invalid("reached EOI before any SOF segment"));
        }
        if is_standalone_marker(marker) {
            continue;
        }

        let segment_length = usize::from(read_u16_be(data, offset)?);
        if segment_length < 2 {
            return Err(invalid(format!(
                "marker 0x{marker:02x} declares invalid segment length {segment_length}"
            )));
        }
        let payload_start = offset
            .checked_add(2)
            .ok_or_else(|| FormatError::overflow("JPEG segment payload offset"))?;
        let payload_length = segment_length - 2;
        let payload_end = payload_start
            .checked_add(payload_length)
            .ok_or_else(|| FormatError::overflow("JPEG segment end"))?;
        if payload_end > data.len() {
            return Err(truncated(payload_start, payload_length, data.len()));
        }
        if is_sof_marker(marker) {
            return parse_sof(data, marker, payload_start, payload_end);
        }
        offset = payload_end;
    }

    Err(invalid("no SOF segment found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_with_sof(components: &[(u8, u8, u8, u8)], precision: u8) -> Vec<u8> {
        let mut output = vec![0xff, 0xd8];
        output.extend_from_slice(&[
            0xff, 0xe0, 0x00, 0x04, 0x12, 0x34,
            0xff, 0xc0,
        ]);
        let segment_length = 8 + components.len() * 3;
        output.extend_from_slice(&(segment_length as u16).to_be_bytes());
        output.push(precision);
        output.extend_from_slice(&16u16.to_be_bytes());
        output.extend_from_slice(&24u16.to_be_bytes());
        output.push(components.len() as u8);
        for (id, horizontal, vertical, table) in components {
            output.push(*id);
            output.push((horizontal << 4) | vertical);
            output.push(*table);
        }
        output.extend_from_slice(&[0xff, 0xda]);
        output
    }

    #[test]
    fn probes_monochrome_sampling() {
        let profile = probe_frame_profile(&jpeg_with_sof(&[(1, 1, 1, 0)], 8)).unwrap();
        assert_eq!(profile.width, 24);
        assert_eq!(profile.height, 16);
        assert_eq!(profile.component_count(), 1);
        assert_eq!(profile.chroma_sampling, Some(ChromaSampling::Mono400));
    }

    #[test]
    fn distinguishes_three_channel_sampling_classes() {
        let cases = [
            (
                [(1, 2, 2, 0), (2, 1, 1, 1), (3, 1, 1, 1)],
                ChromaSampling::Yuv420,
            ),
            (
                [(1, 2, 1, 0), (2, 1, 1, 1), (3, 1, 1, 1)],
                ChromaSampling::Yuv422,
            ),
            (
                [(1, 1, 1, 0), (2, 1, 1, 1), (3, 1, 1, 1)],
                ChromaSampling::Yuv444,
            ),
        ];
        for (components, expected) in cases {
            let profile = probe_frame_profile(&jpeg_with_sof(&components, 8)).unwrap();
            assert_eq!(profile.component_count(), 3);
            assert_eq!(profile.chroma_sampling, Some(expected));
        }
    }

    #[test]
    fn preserves_unusual_component_layout_without_guessing_sampling() {
        let profile = probe_frame_profile(&jpeg_with_sof(
            &[(1, 1, 1, 0), (2, 1, 1, 0), (3, 1, 1, 0), (4, 1, 1, 0)],
            8,
        ))
        .unwrap();
        assert_eq!(profile.component_count(), 4);
        assert_eq!(profile.chroma_sampling, None);
    }

    #[test]
    fn rejects_truncated_segment() {
        let mut data = vec![0xff, 0xd8, 0xff, 0xe1, 0x00, 0x08, 0x01];
        assert!(probe_frame_profile(&data).is_err());
        data.extend_from_slice(&[0x02, 0x03, 0x04, 0x05]);
        assert!(probe_frame_profile(&data).is_err());
    }

    #[test]
    fn rejects_scan_before_frame_header() {
        assert!(probe_frame_profile(&[0xff, 0xd8, 0xff, 0xda]).is_err());
    }
}
