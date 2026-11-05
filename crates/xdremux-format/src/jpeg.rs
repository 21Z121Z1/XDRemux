use crate::codec::ChromaSampling;
use crate::error::{FormatError, Result};
use crate::{jpeg_exif_tiff, Cursor, Endian};

/// Read Exif ColorSpace through the active IFD0 -> ExifIFD pointer. This is a
/// source fact; the caller decides the applicable ICC/Exif precedence.
pub fn exif_color_space(jpeg: &[u8]) -> Result<Option<u16>> {
    let Some(tiff) = jpeg_exif_tiff(jpeg)? else {
        return Ok(None);
    };
    let context = "JPEG EXIF ColorSpace";
    let endian = match tiff.get(..2) {
        Some(b"II") => Endian::Little,
        Some(b"MM") => Endian::Big,
        _ => return Err(FormatError::invalid(context, "invalid TIFF byte order")),
    };
    let mut header = Cursor::new(&tiff, endian, context);
    header.skip(2)?;
    if header.read_u16()? != 42 {
        return Err(FormatError::invalid(context, "invalid TIFF magic"));
    }
    let mut value = header.read_u32()?;
    for (wanted_tag, wanted_type) in [(0x8769, 4), (0xa001, 3)] {
        if value == 0 {
            return Ok(None);
        }
        let offset = usize::try_from(value).map_err(|_| FormatError::overflow(context))?;
        let mut ifd = Cursor::bounded(&tiff, offset..tiff.len(), endian, context)?;
        let count = usize::from(ifd.read_u16()?);
        let entries = ifd.take(count * 12)?;
        ifd.read_u32()?; // Required next-IFD field; no other IFD is relevant.
        let mut found = None;
        for bytes in entries.as_chunks::<12>().0 {
            let mut entry = Cursor::new(bytes, endian, context);
            if entry.read_u16()? != wanted_tag {
                continue;
            }
            if found.is_some() || entry.read_u16()? != wanted_type || entry.read_u32()? != 1 {
                return Err(FormatError::invalid(
                    context,
                    "duplicate or invalid TIFF entry",
                ));
            }
            found = Some(if wanted_type == 3 {
                u32::from(entry.read_u16()?)
            } else {
                entry.read_u32()?
            });
        }
        let Some(found) = found else {
            return Ok(None);
        };
        value = found;
    }
    Ok(Some(value as u16)) // The final field was read as SHORT.
}

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

fn parse_sof(
    data: &[u8],
    marker: u8,
    payload_start: usize,
    payload_end: usize,
) -> Result<JpegFrameProfile> {
    let payload = data.get(payload_start..payload_end).ok_or_else(|| {
        truncated(
            payload_start,
            payload_end.saturating_sub(payload_start),
            data.len(),
        )
    })?;
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
    for chunk in payload[6..].as_chunks::<3>().0 {
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

/// One JPEG segment in file order. Entropy-coded scan data is not a segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JpegSegment {
    pub marker: u8,
    /// Range of the marker prefix and, when the marker carries one, its length
    /// field and payload.
    pub range: std::ops::Range<usize>,
    /// Payload range. Standalone markers have an empty payload at the end of
    /// `range`.
    pub payload: std::ops::Range<usize>,
}

/// One entropy-coded scan between its Start Of Scan header and the next marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JpegScan {
    /// Range from the Start Of Scan marker prefix to the end of the entropy
    /// data. The terminating marker is not included.
    pub range: std::ops::Range<usize>,
    /// Restart markers observed inside the entropy-coded data.
    pub restart_marker_count: usize,
}

/// One complete JPEG image inside a byte buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JpegImage {
    /// Range from the Start Of Image marker through the End Of Image marker.
    pub range: std::ops::Range<usize>,
    pub frame: JpegFrameProfile,
    pub segments: Vec<JpegSegment>,
    pub scans: Vec<JpegScan>,
}

const ICC_APP2_IDENTIFIER: &[u8] = b"ICC_PROFILE\0";

impl JpegImage {
    /// Reassemble the ICC profile carried in APP2 chunks.
    ///
    /// Chunk order, chunk count, duplicate sequence numbers and truncation are
    /// checked. A caller that needs the profile must not guess a missing chunk.
    pub fn icc_profile(&self, data: &[u8]) -> Result<Option<Vec<u8>>> {
        let mut chunks: Vec<(u8, u8, &[u8])> = Vec::new();
        for segment in &self.segments {
            if segment.marker != 0xe2 {
                continue;
            }
            let payload = data
                .get(segment.payload.clone())
                .ok_or_else(|| invalid("ICC APP2 payload is outside the image"))?;
            let Some(rest) = payload.strip_prefix(ICC_APP2_IDENTIFIER) else {
                continue;
            };
            if rest.len() < 2 {
                return Err(invalid("ICC APP2 chunk is shorter than its header"));
            }
            chunks.push((rest[0], rest[1], &rest[2..]));
        }
        if chunks.is_empty() {
            return Ok(None);
        }
        let declared = chunks[0].1;
        if declared == 0 {
            return Err(invalid("ICC APP2 declares zero chunks"));
        }
        if usize::from(declared) != chunks.len() {
            return Err(invalid(format!(
                "ICC APP2 declares {declared} chunks but the image carries {}",
                chunks.len()
            )));
        }
        let mut ordered = vec![None; usize::from(declared)];
        for (sequence, count, payload) in chunks {
            if count != declared {
                return Err(invalid("ICC APP2 chunks disagree on their count"));
            }
            if sequence == 0 || sequence > declared {
                return Err(invalid("ICC APP2 sequence number is out of range"));
            }
            let slot = &mut ordered[usize::from(sequence) - 1];
            if slot.replace(payload).is_some() {
                return Err(invalid("ICC APP2 repeats a sequence number"));
            }
        }
        let mut profile = Vec::new();
        for chunk in ordered {
            let chunk = chunk.ok_or_else(|| invalid("ICC APP2 is missing a chunk"))?;
            profile.extend_from_slice(chunk);
        }
        Ok(Some(profile))
    }

    pub fn end_of_image(&self) -> usize {
        self.range.end
    }
}

fn read_marker(data: &[u8], offset: usize) -> Result<(u8, usize)> {
    if data.get(offset) != Some(&0xff) {
        return Err(invalid(format!(
            "expected marker prefix at offset {offset}"
        )));
    }
    let mut cursor = offset
        .checked_add(1)
        .ok_or_else(|| FormatError::overflow("JPEG marker offset"))?;
    while data.get(cursor) == Some(&0xff) {
        cursor = cursor
            .checked_add(1)
            .ok_or_else(|| FormatError::overflow("JPEG fill bytes"))?;
    }
    let marker = *data
        .get(cursor)
        .ok_or_else(|| truncated(cursor, 1, data.len()))?;
    Ok((marker, cursor + 1))
}

/// Advance past entropy-coded data to the next real marker.
///
/// Stuffed `FF 00` bytes and restart markers belong to the scan. A literal
/// `FF D9` byte pair can only appear inside entropy data as `FF 00 D9`, so a
/// raw search for the end marker would truncate the scan.
fn scan_entropy(data: &[u8], offset: usize) -> Result<(usize, usize)> {
    let mut cursor = offset;
    let mut restarts = 0;
    while cursor < data.len() {
        if data[cursor] != 0xff {
            cursor += 1;
            continue;
        }
        let mut next = cursor + 1;
        while data.get(next) == Some(&0xff) {
            next += 1;
        }
        let marker = *data
            .get(next)
            .ok_or_else(|| truncated(next, 1, data.len()))?;
        match marker {
            0x00 => cursor = next + 1,
            0xd0..=0xd7 => {
                restarts += 1;
                cursor = next + 1;
            }
            _ => return Ok((cursor, restarts)),
        }
    }
    Err(invalid(
        "entropy-coded data ends without a marker or End Of Image",
    ))
}

fn scan_image_at(data: &[u8], start: usize) -> Result<JpegImage> {
    if data.get(start..start + 2) != Some(&[0xff, 0xd8]) {
        return Err(invalid(format!(
            "expected a Start Of Image marker at offset {start}"
        )));
    }
    let mut segments = vec![JpegSegment {
        marker: 0xd8,
        range: start..start + 2,
        payload: start + 2..start + 2,
    }];
    let mut scans = Vec::new();
    let mut frame = None;
    let mut cursor = start + 2;
    loop {
        let marker_start = cursor;
        let (marker, after_marker) = read_marker(data, marker_start)?;
        if marker == 0x00 {
            return Err(invalid("stuffed byte outside entropy-coded data"));
        }
        if marker == 0xd9 {
            segments.push(JpegSegment {
                marker,
                range: marker_start..after_marker,
                payload: after_marker..after_marker,
            });
            let Some(frame) = frame else {
                return Err(invalid("End Of Image precedes any Start Of Frame"));
            };
            return Ok(JpegImage {
                range: start..after_marker,
                frame,
                segments,
                scans,
            });
        }
        if marker == 0x01 || marker == 0xd8 || (0xd0..=0xd7).contains(&marker) {
            segments.push(JpegSegment {
                marker,
                range: marker_start..after_marker,
                payload: after_marker..after_marker,
            });
            cursor = after_marker;
            continue;
        }
        let segment_length = usize::from(read_u16_be(data, after_marker)?);
        if segment_length < 2 {
            return Err(invalid(format!(
                "marker 0x{marker:02x} declares invalid segment length {segment_length}"
            )));
        }
        let payload_start = after_marker + 2;
        let payload_end = after_marker
            .checked_add(segment_length)
            .ok_or_else(|| FormatError::overflow("JPEG segment end"))?;
        if payload_end > data.len() {
            return Err(truncated(
                payload_start,
                segment_length.saturating_sub(2),
                data.len(),
            ));
        }
        if marker == 0xda {
            if frame.is_none() {
                return Err(invalid("Start Of Scan precedes any Start Of Frame"));
            }
            let (entropy_end, restart_marker_count) = scan_entropy(data, payload_end)?;
            scans.push(JpegScan {
                range: marker_start..entropy_end,
                restart_marker_count,
            });
            segments.push(JpegSegment {
                marker,
                range: marker_start..payload_end,
                payload: payload_start..payload_end,
            });
            cursor = entropy_end;
            continue;
        }
        if is_sof_marker(marker) {
            if frame.is_some() {
                return Err(invalid("image declares more than one Start Of Frame"));
            }
            frame = Some(parse_sof(data, marker, payload_start, payload_end)?);
        }
        segments.push(JpegSegment {
            marker,
            range: marker_start..payload_end,
            payload: payload_start..payload_end,
        });
        cursor = payload_end;
    }
}

/// Index the JPEG image that starts at `start`.
///
/// Use this when a container appends non-JPEG data after the images, such as a
/// Motion Photo video. The returned range ends exactly after the End Of Image
/// marker, so a caller can continue at `image.range.end`.
pub fn scan_jpeg_image_at(data: &[u8], start: usize) -> Result<JpegImage> {
    scan_image_at(data, start)
}

/// Index every complete JPEG image in a byte buffer.
///
/// Images must be contiguous and each must end with an End Of Image marker.
/// Trailing bytes that are not another image, a scan that runs past the buffer,
/// and a missing End Of Image are errors: a writer that rewrites offsets must
/// never guess an image boundary.
pub fn scan_jpeg_images(data: &[u8]) -> Result<Vec<JpegImage>> {
    let mut images = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let image = scan_image_at(data, offset)?;
        offset = image.range.end;
        images.push(image);
    }
    if images.is_empty() {
        return Err(invalid("buffer contains no JPEG image"));
    }
    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_with_exif(tiff: &[u8]) -> Vec<u8> {
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
        jpeg.extend(u16::try_from(tiff.len() + 8).unwrap().to_be_bytes());
        jpeg.extend(b"Exif\0\0");
        jpeg.extend(tiff);
        jpeg.extend([0xff, 0xd9]);
        jpeg
    }

    #[test]
    fn reads_exif_color_space_in_both_byte_orders() {
        for tiff in [
            b"MM\0*\0\0\0\x08\0\x01\x87\x69\0\x04\0\0\0\x01\0\0\0\x1a\0\0\0\0\0\x01\xa0\x01\0\x03\0\0\0\x01\0\x01\0\0\0\0\0\0".as_slice(),
            b"II*\0\x08\0\0\0\x01\0\x69\x87\x04\0\x01\0\0\0\x1a\0\0\0\0\0\0\0\x01\0\x01\xa0\x03\0\x01\0\0\0\x01\0\0\0\0\0\0\0".as_slice(),
        ] {
            assert_eq!(exif_color_space(&jpeg_with_exif(tiff)).unwrap(), Some(1));
        }
    }

    #[test]
    fn exif_color_space_requires_an_active_bounded_ifd_pointer() {
        let mut tiff = b"MM\0*\0\0\0\x08\0\x01\x87\x69\0\x04\0\0\0\x01\0\0\0\x1a\0\0\0\0\0\x01\xa0\x01\0\x03\0\0\0\x01\0\x01\0\0\0\0\0\0".to_vec();
        tiff[10..12].copy_from_slice(&[0x87, 0x68]);
        assert_eq!(exif_color_space(&jpeg_with_exif(&tiff)).unwrap(), None);
        tiff[10..12].copy_from_slice(&[0x87, 0x69]);
        tiff[18..22].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(exif_color_space(&jpeg_with_exif(&tiff)).is_err());
        assert_eq!(exif_color_space(&[0xff, 0xd8, 0xff, 0xd9]).unwrap(), None);
    }

    fn jpeg_with_sof(components: &[(u8, u8, u8, u8)], precision: u8) -> Vec<u8> {
        let mut output = vec![0xff, 0xd8];
        output.extend_from_slice(&[0xff, 0xe0, 0x00, 0x04, 0x12, 0x34, 0xff, 0xc0]);
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

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xff, marker];
        bytes.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn frame_header(width: u16, height: u16, components: &[(u8, u8, u8, u8)]) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&height.to_be_bytes());
        payload.extend_from_slice(&width.to_be_bytes());
        payload.push(u8::try_from(components.len()).unwrap());
        for (id, horizontal, vertical, table) in components {
            payload.push(*id);
            payload.push((horizontal << 4) | vertical);
            payload.push(*table);
        }
        segment(0xc0, &payload)
    }

    fn scan_header() -> Vec<u8> {
        segment(0xda, &[1, 1, 0, 0, 63, 0])
    }

    fn gray_jpeg(entropy: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend_from_slice(&segment(0xe0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        bytes.extend_from_slice(&frame_header(8, 4, &[(1, 1, 1, 0)]));
        bytes.extend_from_slice(&scan_header());
        bytes.extend_from_slice(entropy);
        bytes.extend_from_slice(&[0xff, 0xd9]);
        bytes
    }

    #[test]
    fn indexes_one_image_with_its_segments_and_scan() {
        let bytes = gray_jpeg(&[0x12, 0x34, 0x56]);
        let images = scan_jpeg_images(&bytes).unwrap();
        assert_eq!(images.len(), 1);
        let image = &images[0];
        assert_eq!(image.range, 0..bytes.len());
        assert_eq!(image.end_of_image(), bytes.len());
        assert_eq!(
            image.segments.iter().map(|s| s.marker).collect::<Vec<_>>(),
            vec![0xd8, 0xe0, 0xc0, 0xda, 0xd9]
        );
        assert_eq!(image.scans.len(), 1);
        assert_eq!(image.scans[0].restart_marker_count, 0);
        assert_eq!(image.frame.width, 8);
        assert_eq!(image.frame.height, 4);
        assert_eq!(
            &bytes[image.segments[1].payload.clone()],
            b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"
        );
    }

    #[test]
    fn entropy_stuffing_and_restart_markers_do_not_end_the_image() {
        // FF 00 is a stuffed byte, FF D0..D7 are restart markers, and the only
        // real End Of Image marker is the last one.
        let entropy = [
            0x01, 0xff, 0x00, 0x02, 0xff, 0xd0, 0x03, 0xff, 0x00, 0xd9, 0x04,
        ];
        let bytes = gray_jpeg(&entropy);
        let images = scan_jpeg_images(&bytes).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].range.end, bytes.len());
        assert_eq!(images[0].scans[0].restart_marker_count, 1);
    }

    #[test]
    fn progressive_images_keep_every_scan() {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend_from_slice(&frame_header(8, 4, &[(1, 1, 1, 0)]));
        bytes.extend_from_slice(&scan_header());
        bytes.extend_from_slice(&[0x11, 0x22]);
        bytes.extend_from_slice(&scan_header());
        bytes.extend_from_slice(&[0x33, 0x44]);
        bytes.extend_from_slice(&[0xff, 0xd9]);
        let images = scan_jpeg_images(&bytes).unwrap();
        assert_eq!(images[0].scans.len(), 2);
        assert_eq!(images[0].range.end, bytes.len());
    }

    #[test]
    fn concatenated_images_report_their_own_boundaries() {
        let first = gray_jpeg(&[0x01]);
        let second = gray_jpeg(&[0x02, 0x03]);
        let mut bytes = first.clone();
        bytes.extend_from_slice(&second);
        let images = scan_jpeg_images(&bytes).unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].range, 0..first.len());
        assert_eq!(images[1].range, first.len()..bytes.len());
    }

    #[test]
    fn a_missing_end_marker_or_trailing_byte_is_rejected() {
        let bytes = gray_jpeg(&[0x01]);
        assert!(scan_jpeg_images(&bytes[..bytes.len() - 2]).is_err());
        let mut trailing = bytes.clone();
        trailing.push(0x00);
        assert!(scan_jpeg_images(&trailing).is_err());
        assert!(scan_jpeg_images(&[]).is_err());
        assert!(scan_jpeg_images(&[0x00, 0x00]).is_err());
    }

    #[test]
    fn a_scan_that_runs_past_the_buffer_is_rejected() {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend_from_slice(&frame_header(8, 4, &[(1, 1, 1, 0)]));
        bytes.extend_from_slice(&scan_header());
        bytes.extend_from_slice(&[0x01, 0x02]);
        assert!(scan_jpeg_images(&bytes).is_err());
    }

    fn icc_chunk(sequence: u8, count: u8, payload: &[u8]) -> Vec<u8> {
        let mut body = b"ICC_PROFILE\0".to_vec();
        body.push(sequence);
        body.push(count);
        body.extend_from_slice(payload);
        segment(0xe2, &body)
    }

    #[test]
    fn reassembles_icc_chunks_in_sequence_order() {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend_from_slice(&icc_chunk(2, 2, b"second"));
        bytes.extend_from_slice(&icc_chunk(1, 2, b"first-"));
        bytes.extend_from_slice(&frame_header(8, 4, &[(1, 1, 1, 0)]));
        bytes.extend_from_slice(&scan_header());
        bytes.extend_from_slice(&[0x01, 0xff, 0xd9]);
        let images = scan_jpeg_images(&bytes).unwrap();
        assert_eq!(
            images[0].icc_profile(&bytes).unwrap(),
            Some(b"first-second".to_vec())
        );
        let plain = gray_jpeg(&[0x01]);
        assert_eq!(
            scan_jpeg_images(&plain).unwrap()[0]
                .icc_profile(&plain)
                .unwrap(),
            None
        );
    }

    #[test]
    fn rejects_incomplete_or_repeated_icc_chunks() {
        let mut missing = vec![0xff, 0xd8];
        missing.extend_from_slice(&icc_chunk(1, 2, b"only"));
        missing.extend_from_slice(&frame_header(8, 4, &[(1, 1, 1, 0)]));
        missing.extend_from_slice(&scan_header());
        missing.extend_from_slice(&[0x01, 0xff, 0xd9]);
        let images = scan_jpeg_images(&missing).unwrap();
        assert!(images[0].icc_profile(&missing).is_err());

        let mut repeated = vec![0xff, 0xd8];
        repeated.extend_from_slice(&icc_chunk(1, 2, b"a"));
        repeated.extend_from_slice(&icc_chunk(1, 2, b"b"));
        repeated.extend_from_slice(&frame_header(8, 4, &[(1, 1, 1, 0)]));
        repeated.extend_from_slice(&scan_header());
        repeated.extend_from_slice(&[0x01, 0xff, 0xd9]);
        let images = scan_jpeg_images(&repeated).unwrap();
        assert!(images[0].icc_profile(&repeated).is_err());
    }

    #[test]
    fn rejects_scan_before_frame_header() {
        assert!(probe_frame_profile(&[0xff, 0xd8, 0xff, 0xda]).is_err());
    }
}
