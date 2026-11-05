//! Two-image JPEG assembly with a Multi-Picture Format index.
//!
//! The layout is computed before any offset is written: the second image starts
//! at the end of the first, and the MP entry offsets are relative to the TIFF
//! header inside the first image's APP2 `MPF` segment.
use crate::error::{FormatError, Result};
use crate::jpeg::{scan_jpeg_images, JpegImage};
use crate::mpf::{
    make_mp_index_payload, mp_index_payload_len, parse_mp_index, MpEntry, MpIndex,
    MPF_APP2_IDENTIFIER, MP_ATTR_GAIN_MAP_IMAGE, MP_ATTR_PRIMARY_IMAGE,
};

/// Namespace that identifies an ISO 21496-1 APP2 segment, including its
/// terminating zero byte.
pub const ISO_GAIN_MAP_APP2_IDENTIFIER: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";
const ICC_APP2_IDENTIFIER: &[u8] = b"ICC_PROFILE\0";
/// Largest payload that a segment with a two-byte length field can carry.
pub const MAX_APP_SEGMENT_PAYLOAD: usize = 65_533;

fn invalid(message: impl Into<String>) -> FormatError {
    FormatError::invalid("dual-layer JPEG", message)
}

/// One APP segment to inject after the leading JFIF/Adobe segments.
#[derive(Debug, Clone, Copy)]
pub struct AppSegment<'a> {
    pub marker: u8,
    pub payload: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct DualLayerJpegAssembly<'a> {
    /// Complete primary image, including its End Of Image marker.
    pub base: &'a [u8],
    /// Complete gain-map image, including its End Of Image marker.
    pub gain_map: &'a [u8],
    /// Segments for the primary image. The assembler adds the MPF index.
    pub base_segments: &'a [AppSegment<'a>],
    /// Segments for the gain-map image.
    pub gain_segments: &'a [AppSegment<'a>],
}

/// Parsed layout of an assembled two-image JPEG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualLayerJpegIndex {
    pub base: JpegImage,
    pub gain_map: JpegImage,
    pub mp: MpIndex,
    /// Offset of the TIFF header that MP entry data offsets refer to.
    pub mpf_tiff_start: usize,
    pub base_has_iso_declaration: bool,
    pub gain_map_has_iso_parameters: bool,
    pub base_has_icc: bool,
    pub gain_map_has_icc: bool,
    /// Registered base declaration offsets, when the file carries one.
    pub registered_offsets: Option<RegisteredApp8Offsets>,
    /// Offset of the registered declaration's APP8 marker.
    pub registered_app8_start: Option<usize>,
    pub gain_map_has_registered_payload: bool,
}

fn app_segment_bytes(segment: &AppSegment<'_>) -> Result<Vec<u8>> {
    if !(0xe0..=0xef).contains(&segment.marker) {
        return Err(invalid(format!(
            "segment marker 0x{:02x} is not an APP marker",
            segment.marker
        )));
    }
    if segment.payload.is_empty() || segment.payload.len() > MAX_APP_SEGMENT_PAYLOAD {
        return Err(invalid("APP segment payload length is not representable"));
    }
    let mut bytes = vec![0xff, segment.marker];
    bytes.extend_from_slice(
        &u16::try_from(segment.payload.len() + 2)
            .map_err(|_| invalid("APP segment length exceeds u16"))?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(segment.payload);
    Ok(bytes)
}

/// Split an ICC profile into APP2 chunk payloads in carrier order.
///
/// Each chunk reserves the identifier, its sequence number and the chunk count
/// inside the 65,533-byte segment limit.
pub fn split_icc_app2(profile: &[u8]) -> Result<Vec<Vec<u8>>> {
    const MAX_CHUNK: usize = MAX_APP_SEGMENT_PAYLOAD - ICC_APP2_IDENTIFIER.len() - 2;
    if profile.is_empty() {
        return Err(invalid("ICC profile is empty"));
    }
    let count = profile.len().div_ceil(MAX_CHUNK);
    if count > 255 {
        return Err(invalid("ICC profile needs more than 255 APP2 chunks"));
    }
    Ok(profile
        .chunks(MAX_CHUNK)
        .enumerate()
        .map(|(index, chunk)| {
            let mut payload = ICC_APP2_IDENTIFIER.to_vec();
            payload.push(u8::try_from(index + 1).unwrap());
            payload.push(u8::try_from(count).unwrap());
            payload.extend_from_slice(chunk);
            payload
        })
        .collect())
}

/// Offset after the leading JFIF and Adobe APP segments.
fn injection_offset(image: &JpegImage) -> usize {
    let mut offset = image.range.start + 2;
    for segment in image.segments.iter().skip(1) {
        if matches!(segment.marker, 0xe0 | 0xee) {
            offset = segment.range.end;
        } else {
            break;
        }
    }
    offset
}

fn existing_segments(image: &JpegImage, data: &[u8]) -> Result<(bool, bool, bool)> {
    let mut mpf = false;
    let mut iso = false;
    let mut icc = false;
    for segment in &image.segments {
        if segment.marker != 0xe2 {
            continue;
        }
        let payload = data
            .get(segment.payload.clone())
            .ok_or_else(|| invalid("APP2 payload is outside the image"))?;
        if payload.starts_with(MPF_APP2_IDENTIFIER) {
            mpf = true;
        } else if payload.starts_with(ISO_GAIN_MAP_APP2_IDENTIFIER) {
            iso = true;
        } else if payload.starts_with(ICC_APP2_IDENTIFIER) {
            icc = true;
        }
    }
    Ok((mpf, iso, icc))
}

fn single_image(data: &[u8], context: &str) -> Result<JpegImage> {
    let mut images = scan_jpeg_images(data)?;
    if images.len() != 1 {
        return Err(invalid(format!(
            "{context} must contain exactly one JPEG image"
        )));
    }
    Ok(images.remove(0))
}

fn build_image(image: &JpegImage, data: &[u8], injected: &[AppSegment<'_>]) -> Result<Vec<u8>> {
    let inject_at = injection_offset(image);
    let mut output = data
        .get(image.range.start..inject_at)
        .ok_or_else(|| invalid("image prefix is outside the source"))?
        .to_vec();
    for segment in injected {
        output.extend_from_slice(&app_segment_bytes(segment)?);
    }
    output.extend_from_slice(
        data.get(inject_at..image.range.end)
            .ok_or_else(|| invalid("image body is outside the source"))?,
    );
    Ok(output)
}

/// Remove the carriers that the assembler owns from one complete JPEG image.
///
/// The MPF index and ISO 21496-1 APP2 segments are dropped; every other
/// segment, including ICC and Exif, keeps its bytes and order. Use this to
/// repackage a source that already declares these carriers.
pub fn strip_assembly_owned_segments(data: &[u8]) -> Result<Vec<u8>> {
    let image = single_image(data, "image")?;
    let mut output = Vec::with_capacity(image.range.len());
    let mut cursor = image.range.start;
    for segment in &image.segments {
        if segment.range.start > cursor {
            // Entropy-coded data sits between a scan header and the next marker.
            output.extend_from_slice(
                data.get(cursor..segment.range.start)
                    .ok_or_else(|| invalid("scan data is outside the image"))?,
            );
        }
        let mut drop = false;
        if segment.marker == 0xe2 {
            let payload = data
                .get(segment.payload.clone())
                .ok_or_else(|| invalid("APP2 payload is outside the image"))?;
            drop = payload.starts_with(MPF_APP2_IDENTIFIER)
                || payload.starts_with(ISO_GAIN_MAP_APP2_IDENTIFIER);
        }
        if !drop {
            output.extend_from_slice(
                data.get(segment.range.clone())
                    .ok_or_else(|| invalid("segment is outside the image"))?,
            );
        }
        cursor = segment.range.end;
    }
    output.extend_from_slice(
        data.get(cursor..image.range.end)
            .ok_or_else(|| invalid("image tail is outside the source"))?,
    );
    let stripped = single_image(&output, "stripped image")?;
    if stripped.range.len() != output.len() || stripped.scans.len() != image.scans.len() {
        return Err(invalid("stripping changed the image structure"));
    }
    Ok(output)
}

/// Offsets that a registered base declaration must contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisteredApp8Offsets {
    /// Distance from the declaration's APP8 marker to the second image.
    pub raw: u32,
    /// The same distance in a view without APP segments other than JFIF.
    pub reduced: u32,
}

/// Compute the registered base-declaration offsets for a planned assembly.
///
/// The declaration is expected as the last injected primary-image segment and
/// carries `base_app8_payload_len` payload bytes. The reduced offset removes
/// every APP segment other than APP0 that lies between the declaration and the
/// second image, including the declaration itself. This rule is implemented
/// here and is not yet confirmed by an independent producer sample.
pub fn registered_app8_offsets(
    base: &[u8],
    gain_map: &[u8],
    base_segments: &[AppSegment<'_>],
    gain_segments: &[AppSegment<'_>],
    base_app8_payload_len: usize,
) -> Result<RegisteredApp8Offsets> {
    if base_app8_payload_len == 0 || base_app8_payload_len > MAX_APP_SEGMENT_PAYLOAD {
        return Err(invalid(
            "registered APP8 payload length is not representable",
        ));
    }
    let placeholder_payload = vec![0u8; base_app8_payload_len];
    let mpf_payload = vec![0u8; mp_index_payload_len(2)?];
    let mut segments = base_segments.to_vec();
    segments.push(AppSegment {
        marker: 0xe8,
        payload: &placeholder_payload,
    });
    // The assembler adds the MPF index after the injected segments, so the
    // planned layout must include it.
    segments.push(AppSegment {
        marker: 0xe2,
        payload: &mpf_payload,
    });
    let image = single_image(base, "primary image")?;
    let gain_image = single_image(gain_map, "gain-map image")?;
    let image_one = build_image(&image, base, &segments)?;
    let image_two = build_image(&gain_image, gain_map, gain_segments)?;
    let indexed = single_image(&image_one, "planned primary image")?;
    let planned_start = base_segments
        .iter()
        .try_fold(injection_offset(&image), |start, segment| {
            start
                .checked_add(app_segment_bytes(segment)?.len())
                .ok_or_else(|| invalid("registered APP8 planned position overflows"))
        })?;
    let declaration = indexed
        .segments
        .iter()
        .find(|segment| segment.marker == 0xe8 && segment.range.start == planned_start)
        .ok_or_else(|| invalid("planned primary image lost its registered APP8 segment"))?;
    let start = declaration.range.start;
    let raw = image_one
        .len()
        .checked_sub(start)
        .ok_or_else(|| invalid("registered APP8 offset underflows"))?;
    let removed = indexed
        .segments
        .iter()
        .filter(|segment| (0xe1..=0xef).contains(&segment.marker) && segment.range.start >= start)
        .try_fold(0usize, |total, segment| {
            total.checked_add(segment.range.len())
        })
        .ok_or_else(|| invalid("registered APP8 reduced offset overflows"))?;
    let reduced = raw
        .checked_sub(removed)
        .ok_or_else(|| invalid("registered APP8 reduced offset underflows"))?;
    let _ = image_two.len();
    Ok(RegisteredApp8Offsets {
        raw: u32::try_from(raw).map_err(|_| invalid("registered APP8 offset exceeds u32"))?,
        reduced: u32::try_from(reduced)
            .map_err(|_| invalid("registered APP8 reduced offset exceeds u32"))?,
    })
}

/// Assemble a primary image and a gain-map image into one two-image JPEG.
///
/// Neither input may already carry an MPF index or an ISO 21496-1 APP2
/// segment, because the assembler owns those carriers. An existing ICC profile
/// is kept and the caller must not inject a second one.
pub fn assemble_dual_layer_jpeg(assembly: &DualLayerJpegAssembly<'_>) -> Result<Vec<u8>> {
    let base = single_image(assembly.base, "primary image")?;
    let gain_map = single_image(assembly.gain_map, "gain-map image")?;
    for (image, data, injected, context) in [
        (
            &base,
            assembly.base,
            assembly.base_segments,
            "primary image",
        ),
        (
            &gain_map,
            assembly.gain_map,
            assembly.gain_segments,
            "gain-map image",
        ),
    ] {
        let (mpf, iso, icc) = existing_segments(image, data)?;
        if mpf {
            return Err(invalid(format!("{context} already declares an MPF index")));
        }
        if iso {
            return Err(invalid(format!(
                "{context} already declares ISO 21496-1 metadata"
            )));
        }
        let injects_icc = injected
            .iter()
            .any(|segment| segment.payload.starts_with(ICC_APP2_IDENTIFIER));
        if icc && injects_icc {
            return Err(invalid(format!("{context} already carries an ICC profile")));
        }
    }

    let mpf_len = mp_index_payload_len(2)?;
    let placeholder = AppSegment {
        marker: 0xe2,
        payload: &vec![0u8; mpf_len],
    };
    let mut base_segments = assembly.base_segments.to_vec();
    base_segments.push(placeholder);
    let mut image_one = build_image(&base, assembly.base, &base_segments)?;
    let image_two = build_image(&gain_map, assembly.gain_map, assembly.gain_segments)?;

    // The placeholder occupies the same bytes as the final index.
    let placeholder_start = injection_offset(&base)
        + assembly
            .base_segments
            .iter()
            .map(|segment| segment.payload.len() + 4)
            .sum::<usize>();
    let tiff_start = placeholder_start
        .checked_add(4 + MPF_APP2_IDENTIFIER.len())
        .ok_or_else(|| invalid("MPF offset overflows"))?;
    let first_length = image_one.len();
    let second_length = image_two.len();
    let second_offset = first_length
        .checked_sub(tiff_start)
        .ok_or_else(|| invalid("second image starts before the MPF TIFF header"))?;
    let entries = [
        MpEntry {
            attributes: MP_ATTR_PRIMARY_IMAGE,
            image_size: u32::try_from(first_length)
                .map_err(|_| invalid("primary image exceeds u32 bytes"))?,
            data_offset: 0,
            dependent_image_1: 0,
            dependent_image_2: 0,
        },
        MpEntry {
            attributes: MP_ATTR_GAIN_MAP_IMAGE,
            image_size: u32::try_from(second_length)
                .map_err(|_| invalid("gain-map image exceeds u32 bytes"))?,
            data_offset: u32::try_from(second_offset)
                .map_err(|_| invalid("gain-map offset exceeds u32 bytes"))?,
            dependent_image_1: 0,
            dependent_image_2: 0,
        },
    ];
    let payload = make_mp_index_payload(&entries)?;
    if payload.len() != mpf_len {
        return Err(invalid("MPF index length changed after layout"));
    }
    let payload_start = placeholder_start + 4;
    let end = payload_start
        .checked_add(payload.len())
        .ok_or_else(|| invalid("MPF payload end overflows"))?;
    image_one
        .get_mut(payload_start..end)
        .ok_or_else(|| invalid("MPF placeholder is outside the primary image"))?
        .copy_from_slice(&payload);

    let mut output = image_one;
    output.extend_from_slice(&image_two);
    Ok(output)
}

/// Read back the two-image layout and every association a writer must keep.
///
/// Offsets are checked against real image boundaries, not against declared
/// sizes alone.
pub fn index_dual_layer_jpeg(data: &[u8]) -> Result<DualLayerJpegIndex> {
    let mut images = scan_jpeg_images(data)?;
    if images.len() != 2 {
        return Err(invalid(format!(
            "a dual-layer JPEG must contain exactly two images, found {}",
            images.len()
        )));
    }
    let gain_map = images.remove(1);
    let base = images.remove(0);

    let mut mp = None;
    let mut tiff_start = 0usize;
    let mut iso_declaration = false;
    let mut icc = false;
    for segment in &base.segments {
        if segment.marker != 0xe2 {
            continue;
        }
        let payload = data
            .get(segment.payload.clone())
            .ok_or_else(|| invalid("APP2 payload is outside the primary image"))?;
        if let Some(tiff) = payload.strip_prefix(MPF_APP2_IDENTIFIER) {
            if mp.is_some() {
                return Err(invalid("primary image declares more than one MPF index"));
            }
            mp = Some(parse_mp_index(tiff)?);
            tiff_start = segment.payload.start + MPF_APP2_IDENTIFIER.len();
        } else if let Some(payload) = payload.strip_prefix(ISO_GAIN_MAP_APP2_IDENTIFIER) {
            if payload.len() != 4 {
                return Err(invalid(
                    "primary ISO APP2 must declare only the four-byte version",
                ));
            }
            if u16::from_be_bytes([payload[0], payload[1]]) != 0 {
                return Err(invalid("unsupported primary ISO APP2 minimum version"));
            }
            if iso_declaration {
                return Err(invalid("primary image has duplicate ISO APP2 declarations"));
            }
            iso_declaration = true;
        } else if payload.starts_with(ICC_APP2_IDENTIFIER) {
            icc = true;
        }
    }
    let mp = mp.ok_or_else(|| invalid("primary image has no MPF index"))?;
    if mp.entries.len() != 2 {
        return Err(invalid("MPF index must describe exactly two images"));
    }
    if mp.entries[0].data_offset != 0 || mp.entries[0].image_size as usize != base.range.len() {
        return Err(invalid(
            "first MP entry must describe the primary image from the file start",
        ));
    }
    let expected = base
        .range
        .end
        .checked_sub(tiff_start)
        .ok_or_else(|| invalid("second image starts before the MPF TIFF header"))?;
    if u64::from(mp.entries[1].data_offset) != u64::try_from(expected).unwrap()
        || mp.entries[1].image_size as usize != gain_map.range.len()
    {
        return Err(invalid(
            "second MP entry does not describe the gain-map image",
        ));
    }

    let mut gain_iso = false;
    let mut gain_icc = false;
    for segment in &gain_map.segments {
        if segment.marker != 0xe2 {
            continue;
        }
        let payload = data
            .get(segment.payload.clone())
            .ok_or_else(|| invalid("APP2 payload is outside the gain-map image"))?;
        if let Some(payload) = payload.strip_prefix(ISO_GAIN_MAP_APP2_IDENTIFIER) {
            if payload.len() <= 4 {
                return Err(invalid(
                    "gain-map ISO APP2 must carry parameters after its version",
                ));
            }
            if gain_iso {
                return Err(invalid("gain-map image has duplicate ISO APP2 parameters"));
            }
            gain_iso = true;
        } else if payload.starts_with(ICC_APP2_IDENTIFIER) {
            gain_icc = true;
        }
    }
    let gain_registered = registered_app8_start(&gain_map, data).is_some();

    let offsets = registered_offsets(&base, data, tiff_start);
    let app8_start = registered_app8_start(&base, data);
    Ok(DualLayerJpegIndex {
        base,
        gain_map,
        mp,
        mpf_tiff_start: tiff_start,
        base_has_iso_declaration: iso_declaration,
        gain_map_has_iso_parameters: gain_iso,
        base_has_icc: icc,
        gain_map_has_icc: gain_icc,
        registered_offsets: offsets,
        registered_app8_start: app8_start,
        gain_map_has_registered_payload: gain_registered,
    })
}

fn registered_app8_start(image: &JpegImage, data: &[u8]) -> Option<usize> {
    image
        .segments
        .iter()
        .find(|segment| {
            segment.marker == 0xe8
                && data
                    .get(segment.payload.clone())
                    .is_some_and(|payload| payload.starts_with(b"ITUT35"))
        })
        .map(|segment| segment.range.start)
}

fn registered_offsets(
    base: &JpegImage,
    data: &[u8],
    _tiff_start: usize,
) -> Option<RegisteredApp8Offsets> {
    let start = registered_app8_start(base, data)?;
    let removed = base
        .segments
        .iter()
        .filter(|segment| (0xe1..=0xef).contains(&segment.marker) && segment.range.start >= start)
        .map(|segment| segment.range.len())
        .sum::<usize>();
    let raw = base.range.end.checked_sub(start)?;
    let reduced = raw.checked_sub(removed)?;
    Some(RegisteredApp8Offsets {
        raw: u32::try_from(raw).ok()?,
        reduced: u32::try_from(reduced).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xff, marker];
        bytes.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn jpeg(app0: bool, entropy: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        if app0 {
            bytes.extend_from_slice(&segment(0xe0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        }
        let mut sof = vec![8];
        sof.extend_from_slice(&4u16.to_be_bytes());
        sof.extend_from_slice(&8u16.to_be_bytes());
        sof.push(1);
        sof.extend_from_slice(&[1, 0x11, 0]);
        bytes.extend_from_slice(&segment(0xc0, &sof));
        bytes.extend_from_slice(&segment(0xda, &[1, 1, 0, 0, 63, 0]));
        bytes.extend_from_slice(entropy);
        bytes.extend_from_slice(&[0xff, 0xd9]);
        bytes
    }

    fn iso_base() -> Vec<u8> {
        let mut payload = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
        payload.extend_from_slice(&[0, 0, 0, 0]);
        payload
    }

    fn iso_gain() -> Vec<u8> {
        let mut payload = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
        payload.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0]);
        payload
    }

    fn icc(chunk: &[u8]) -> Vec<u8> {
        let mut payload = ICC_APP2_IDENTIFIER.to_vec();
        payload.push(1);
        payload.push(1);
        payload.extend_from_slice(chunk);
        payload
    }

    #[test]
    fn assembles_two_images_with_a_consistent_index() {
        let base = jpeg(true, &[0x11, 0xff, 0x00, 0x22]);
        let gain = jpeg(true, &[0x33, 0x44]);
        let base_iso = iso_base();
        let gain_iso = iso_gain();
        let base_icc = icc(b"base-profile");
        let gain_icc = icc(b"gain-profile");
        let base_segments = [
            AppSegment {
                marker: 0xe2,
                payload: &base_icc,
            },
            AppSegment {
                marker: 0xe2,
                payload: &base_iso,
            },
        ];
        let gain_segments = [
            AppSegment {
                marker: 0xe2,
                payload: &gain_icc,
            },
            AppSegment {
                marker: 0xe2,
                payload: &gain_iso,
            },
        ];
        let bytes = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &base_segments,
            gain_segments: &gain_segments,
        })
        .unwrap();

        let index = index_dual_layer_jpeg(&bytes).unwrap();
        assert_eq!(index.base.range.start, 0);
        assert_eq!(index.base.range.end, index.gain_map.range.start);
        assert_eq!(index.gain_map.range.end, bytes.len());
        assert_eq!(index.mp.image_count, 2);
        assert_eq!(index.mp.entries[0].attributes, MP_ATTR_PRIMARY_IMAGE);
        assert_eq!(index.mp.entries[1].attributes, MP_ATTR_GAIN_MAP_IMAGE);
        assert_eq!(
            index.mpf_tiff_start + index.mp.entries[1].data_offset as usize,
            index.base.range.end
        );
        assert!(index.base_has_iso_declaration);
        assert!(index.gain_map_has_iso_parameters);
        assert!(index.base_has_icc);
        assert!(index.gain_map_has_icc);
        assert_eq!(
            index.base.icc_profile(&bytes).unwrap(),
            Some(b"base-profile".to_vec())
        );
        assert_eq!(
            index.gain_map.icc_profile(&bytes).unwrap(),
            Some(b"gain-profile".to_vec())
        );
    }

    #[test]
    fn injected_segments_follow_the_leading_jfif_segment() {
        let base = jpeg(true, &[0x01]);
        let gain = jpeg(false, &[0x02]);
        let payload = icc(b"x");
        let bytes = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &[AppSegment {
                marker: 0xe2,
                payload: &payload,
            }],
            gain_segments: &[],
        })
        .unwrap();
        let index = index_dual_layer_jpeg(&bytes).unwrap();
        let markers = index
            .base
            .segments
            .iter()
            .map(|segment| segment.marker)
            .collect::<Vec<_>>();
        assert_eq!(markers[0], 0xd8);
        assert_eq!(markers[1], 0xe0, "JFIF stays the first segment after SOI");
        assert_eq!(markers[2], 0xe2);
        // An image without a leading APP0 segment receives injected segments
        // directly after its SOI.
        let gain_markers = index
            .gain_map
            .segments
            .iter()
            .map(|segment| segment.marker)
            .collect::<Vec<_>>();
        assert_eq!(gain_markers[0], 0xd8);
        assert_eq!(gain_markers[1], 0xc0);
    }

    #[test]
    fn refuses_sources_that_already_own_the_carriers() {
        let base = jpeg(true, &[0x01]);
        let gain = jpeg(true, &[0x02]);
        let mpf_payload = make_mp_index_payload(&[MpEntry {
            attributes: MP_ATTR_PRIMARY_IMAGE,
            image_size: 1,
            data_offset: 0,
            dependent_image_1: 0,
            dependent_image_2: 0,
        }])
        .unwrap();
        let mut with_mpf = base.clone();
        let injected = segment(0xe2, &mpf_payload);
        with_mpf.splice(2..2, injected);
        assert!(assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &with_mpf,
            gain_map: &gain,
            base_segments: &[],
            gain_segments: &[],
        })
        .is_err());

        let iso = iso_base();
        let mut with_iso = base.clone();
        with_iso.splice(2..2, segment(0xe2, &iso));
        assert!(assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &with_iso,
            gain_map: &gain,
            base_segments: &[],
            gain_segments: &[],
        })
        .is_err());

        let existing = icc(b"existing");
        let mut with_icc = base.clone();
        with_icc.splice(2..2, segment(0xe2, &existing));
        let profile = icc(b"p");
        assert!(
            assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
                base: &with_icc,
                gain_map: &gain,
                base_segments: &[AppSegment {
                    marker: 0xe2,
                    payload: &profile,
                }],
                gain_segments: &[],
            })
            .is_err(),
            "an injected ICC profile collides with an existing one"
        );
    }

    #[test]
    fn refuses_malformed_injection_and_malformed_inputs() {
        let base = jpeg(true, &[0x01]);
        let gain = jpeg(true, &[0x02]);
        let payload = b"payload";
        for marker in [0xc0, 0x00, 0xf0] {
            assert!(assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
                base: &base,
                gain_map: &gain,
                base_segments: &[AppSegment { marker, payload }],
                gain_segments: &[],
            })
            .is_err());
        }
        assert!(assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &base[..base.len() - 2],
            base_segments: &[],
            gain_segments: &[],
        })
        .is_err());
        let mut two = base.clone();
        two.extend_from_slice(&gain);
        assert!(assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &two,
            gain_map: &gain,
            base_segments: &[],
            gain_segments: &[],
        })
        .is_err());
        assert!(index_dual_layer_jpeg(&base).is_err());
    }

    #[test]
    fn stripping_removes_only_the_assembly_owned_carriers() {
        let base = jpeg(true, &[0x11, 0xff, 0x00, 0x22]);
        assert_eq!(strip_assembly_owned_segments(&base).unwrap(), base);

        let iso = iso_base();
        let mpf = make_mp_index_payload(&[MpEntry {
            attributes: MP_ATTR_PRIMARY_IMAGE,
            image_size: 1,
            data_offset: 0,
            dependent_image_1: 0,
            dependent_image_2: 0,
        }])
        .unwrap();
        let profile = icc(b"keep-me");
        let mut decorated = base.clone();
        // Keep the JFIF APP0 segment first, then the carriers in file order.
        let insert_at = single_image(&base, "base").unwrap().segments[1].range.end;
        decorated.splice(insert_at..insert_at, segment(0xe2, &profile));
        decorated.splice(insert_at..insert_at, segment(0xe2, &mpf));
        decorated.splice(insert_at..insert_at, segment(0xe2, &iso));
        let stripped = strip_assembly_owned_segments(&decorated).unwrap();
        let image = single_image(&stripped, "stripped").unwrap();
        let markers = image
            .segments
            .iter()
            .map(|segment| segment.marker)
            .collect::<Vec<_>>();
        assert_eq!(markers, vec![0xd8, 0xe0, 0xe2, 0xc0, 0xda, 0xd9]);
        assert_eq!(
            image.icc_profile(&stripped).unwrap(),
            Some(b"keep-me".to_vec())
        );
        // Entropy-coded data survives unchanged.
        let scan_end = image.scans[0].range.end;
        assert_eq!(stripped[scan_end - 4..scan_end], [0x11, 0xff, 0x00, 0x22]);
    }

    #[test]
    fn registered_offsets_account_for_the_carriers_between_the_declaration_and_the_second_image() {
        let base = jpeg(true, &[0x11]);
        let gain = jpeg(true, &[0x22]);
        let iso = iso_base();
        let profile = icc(b"profile");
        let base_segments = [
            AppSegment {
                marker: 0xe2,
                payload: &profile,
            },
            AppSegment {
                marker: 0xe2,
                payload: &iso,
            },
        ];
        let gain_iso = iso_gain();
        let gain_segments = [AppSegment {
            marker: 0xe2,
            payload: &gain_iso,
        }];
        let payload_len = 24;
        let offsets =
            registered_app8_offsets(&base, &gain, &base_segments, &gain_segments, payload_len)
                .unwrap();
        assert!(offsets.raw > offsets.reduced);

        // Assemble with the real declaration in the planned position.
        let mut declaration = vec![0xee; payload_len];
        declaration[..6].copy_from_slice(b"ITUT35");
        let base_segments = [
            AppSegment {
                marker: 0xe2,
                payload: &profile,
            },
            AppSegment {
                marker: 0xe2,
                payload: &iso,
            },
            AppSegment {
                marker: 0xe8,
                payload: &declaration,
            },
        ];
        let bytes = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &base_segments,
            gain_segments: &gain_segments,
        })
        .unwrap();
        let index = index_dual_layer_jpeg(&bytes).unwrap();
        assert_eq!(index.registered_offsets, Some(offsets));
        assert_eq!(
            index
                .registered_app8_start
                .map(|start| start + offsets.raw as usize),
            Some(index.base.range.end),
            "the raw offset must reach the second Start Of Image"
        );
        assert!(!index.gain_map_has_registered_payload);
    }

    #[test]
    fn unrelated_app8_is_not_a_registered_carrier_or_a_planned_declaration() {
        let mut base = jpeg(true, &[0x11]);
        let unrelated = segment(0xe8, b"other APP8 data");
        let jfif_end = single_image(&base, "base").unwrap().segments[1].range.end;
        base.splice(jfif_end..jfif_end, unrelated);
        let mut gain = jpeg(true, &[0x22]);
        let unrelated = segment(0xe8, b"other APP8 data");
        let jfif_end = single_image(&gain, "gain").unwrap().segments[1].range.end;
        gain.splice(jfif_end..jfif_end, unrelated);

        let iso = iso_base();
        let gain_iso = iso_gain();
        let base_segments = [
            AppSegment {
                marker: 0xe8,
                payload: b"other injected APP8",
            },
            AppSegment {
                marker: 0xe2,
                payload: &iso,
            },
        ];
        let gain_segments = [AppSegment {
            marker: 0xe2,
            payload: &gain_iso,
        }];
        let bytes = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &base_segments,
            gain_segments: &gain_segments,
        })
        .unwrap();
        let index = index_dual_layer_jpeg(&bytes).unwrap();
        assert_eq!(index.registered_app8_start, None);
        assert_eq!(index.registered_offsets, None);
        assert!(!index.gain_map_has_registered_payload);

        // The offset planner must locate the newly injected declaration,
        // even when an unrelated APP8 already appears in the source.
        let offsets = registered_app8_offsets(
            &base,
            &gain,
            &base_segments,
            &gain_segments,
            24,
        )
        .unwrap();
        let mut registered = vec![0u8; 24];
        registered[..6].copy_from_slice(b"ITUT35");
        let registered_segments = [
            base_segments[0],
            base_segments[1],
            AppSegment {
                marker: 0xe8,
                payload: &registered,
            },
        ];
        let registered_bytes = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &registered_segments,
            gain_segments: &gain_segments,
        })
        .unwrap();
        let index = index_dual_layer_jpeg(&registered_bytes).unwrap();
        assert_eq!(index.registered_offsets, Some(offsets));
        assert_eq!(
            index.registered_app8_start.unwrap() + offsets.raw as usize,
            index.gain_map.range.start
        );
    }

    #[test]
    fn index_rejects_unsupported_or_duplicate_iso_app2_declarations() {
        let base = jpeg(true, &[0x11]);
        let gain = jpeg(true, &[0x22]);
        let base_iso = iso_base();
        let gain_iso = iso_gain();
        let primary = [AppSegment {
            marker: 0xe2,
            payload: &base_iso,
        }];
        let secondary = [AppSegment {
            marker: 0xe2,
            payload: &gain_iso,
        }];
        let bytes = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &primary,
            gain_segments: &secondary,
        })
        .unwrap();
        let index = index_dual_layer_jpeg(&bytes).unwrap();
        let primary_version = index
            .base
            .segments
            .iter()
            .find(|segment| {
                segment.marker == 0xe2
                    && bytes[segment.payload.clone()]
                        .starts_with(ISO_GAIN_MAP_APP2_IDENTIFIER)
            })
            .unwrap()
            .payload
            .start
            + ISO_GAIN_MAP_APP2_IDENTIFIER.len();
        let mut unsupported = bytes.clone();
        unsupported[primary_version + 1] = 1;
        assert!(index_dual_layer_jpeg(&unsupported).is_err());

        for (primary_segments, gain_segments) in [
            (vec![primary[0], primary[0]], vec![secondary[0]]),
            (vec![primary[0]], vec![secondary[0], secondary[0]]),
        ] {
            let duplicate = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
                base: &base,
                gain_map: &gain,
                base_segments: &primary_segments,
                gain_segments: &gain_segments,
            })
            .unwrap();
            assert!(index_dual_layer_jpeg(&duplicate).is_err());
        }
    }

    #[test]
    fn index_rejects_an_offset_that_does_not_reach_the_second_image() {
        let base = jpeg(true, &[0x01]);
        let gain = jpeg(true, &[0x02]);
        let mut bytes = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &[],
            gain_segments: &[],
        })
        .unwrap();
        let index = index_dual_layer_jpeg(&bytes).unwrap();
        // Move the second MP entry offset one byte past the real SOI.
        let entry_start = index.mpf_tiff_start + 8 + 2 + 3 * 12 + 4 + 16;
        bytes[entry_start + 8] = bytes[entry_start + 8].wrapping_add(1);
        assert!(index_dual_layer_jpeg(&bytes).is_err());
    }
}
