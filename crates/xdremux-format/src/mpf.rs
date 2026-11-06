//! Multi-Picture Format index (APP2 `MPF`) for a dual-layer JPEG.
//!
//! The index is a TIFF structure whose data offsets are relative to the TIFF
//! header, not to the JPEG file or the APP2 marker. Readers and writers here
//! keep that origin explicit.
use std::ops::Range;

use crate::error::{FormatError, Result};

pub const MPF_APP2_IDENTIFIER: &[u8] = b"MPF\0";
pub const MP_ENTRY_BYTES: usize = 16;
pub const MPF_VERSION: [u8; 4] = *b"0100";
/// Attribute word of the primary image in real dual-layer JPEG producers.
pub const MP_ATTR_PRIMARY_IMAGE: u32 = 0x0003_0000;
/// Attribute word of the dependent gain-map image in real dual-layer JPEG
/// producers. A reader must confirm the ISO metadata and the image order
/// instead of trusting this word alone.
pub const MP_ATTR_GAIN_MAP_IMAGE: u32 = 0;

const TAG_VERSION: u16 = 0xb000;
const TAG_IMAGE_COUNT: u16 = 0xb001;
const TAG_ENTRIES: u16 = 0xb002;
const TIFF_TYPE_LONG: u16 = 4;
const TIFF_TYPE_UNDEFINED: u16 = 7;

fn invalid(message: impl Into<String>) -> FormatError {
    FormatError::invalid("MPF", message)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpEntry {
    pub attributes: u32,
    pub image_size: u32,
    pub data_offset: u32,
    pub dependent_image_1: u16,
    pub dependent_image_2: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpIndex {
    pub version: [u8; 4],
    pub image_count: u32,
    pub entries: Vec<MpEntry>,
}

#[derive(Debug, Clone, Copy)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn u16(self, bytes: &[u8]) -> u16 {
        match self {
            Self::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
            Self::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
        }
    }

    fn u32(self, bytes: &[u8]) -> u32 {
        match self {
            Self::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Self::Big => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        }
    }
}

fn bounded<'a>(data: &'a [u8], range: Range<usize>, context: &str) -> Result<&'a [u8]> {
    data.get(range.clone())
        .ok_or_else(|| FormatError::UnexpectedEof {
            context: "MPF",
            offset: range.start,
            needed: range.end.saturating_sub(range.start),
            end: data.len(),
        })
        .map_err(|error| match error {
            FormatError::UnexpectedEof { .. } => {
                invalid(format!("{context} is outside the MPF index"))
            }
            other => other,
        })
}

/// Read the MP index from the TIFF payload that follows `MPF\0`.
///
/// Both byte orders are supported. Every offset is bounded by the TIFF payload,
/// and the entry count must agree with the declared image count.
pub fn parse_mp_index(tiff: &[u8]) -> Result<MpIndex> {
    let endian = match tiff.get(..2) {
        Some(b"II") => Endian::Little,
        Some(b"MM") => Endian::Big,
        _ => return Err(invalid("invalid TIFF byte order")),
    };
    if endian.u16(bounded(tiff, 2..4, "TIFF magic")?) != 42 {
        return Err(invalid("invalid TIFF magic"));
    }
    let ifd_offset = usize::try_from(endian.u32(bounded(tiff, 4..8, "IFD offset")?))
        .map_err(|_| invalid("IFD offset exceeds usize"))?;
    let entry_count = usize::from(endian.u16(bounded(tiff, ifd_offset..ifd_offset + 2, "IFD")?));
    let ifd_end = ifd_offset
        .checked_add(2)
        .and_then(|start| start.checked_add(entry_count.checked_mul(12)?))
        .ok_or_else(|| invalid("IFD length overflows"))?;
    bounded(tiff, ifd_offset..ifd_end, "IFD entries")?;

    let mut version = None;
    let mut image_count = None;
    let mut entries = None;
    for index in 0..entry_count {
        let start = ifd_offset + 2 + index * 12;
        let entry = bounded(tiff, start..start + 12, "IFD entry")?;
        let tag = endian.u16(&entry[0..2]);
        let field_type = endian.u16(&entry[2..4]);
        let count = endian.u32(&entry[4..8]);
        match tag {
            TAG_VERSION => {
                if field_type != TIFF_TYPE_UNDEFINED || count != 4 {
                    return Err(invalid("MPFVersion must be four undefined bytes"));
                }
                version = Some([entry[8], entry[9], entry[10], entry[11]]);
            }
            TAG_IMAGE_COUNT => {
                if field_type != TIFF_TYPE_LONG || count != 1 {
                    return Err(invalid("NumberOfImages must be one LONG"));
                }
                if image_count.replace(endian.u32(&entry[8..12])).is_some() {
                    return Err(invalid("NumberOfImages appears more than once"));
                }
            }
            TAG_ENTRIES => {
                if field_type != TIFF_TYPE_UNDEFINED {
                    return Err(invalid("MPEntry must be undefined bytes"));
                }
                let bytes = usize::try_from(count).map_err(|_| invalid("MPEntry length"))?;
                if bytes == 0 || bytes % MP_ENTRY_BYTES != 0 {
                    return Err(invalid("MPEntry length is not a multiple of 16"));
                }
                let offset = usize::try_from(endian.u32(&entry[8..12]))
                    .map_err(|_| invalid("MPEntry offset exceeds usize"))?;
                let payload = bounded(tiff, offset..offset + bytes, "MPEntry data")?;
                let mut parsed = Vec::with_capacity(bytes / MP_ENTRY_BYTES);
                for chunk in payload.as_chunks::<MP_ENTRY_BYTES>().0 {
                    parsed.push(MpEntry {
                        attributes: endian.u32(&chunk[0..4]),
                        image_size: endian.u32(&chunk[4..8]),
                        data_offset: endian.u32(&chunk[8..12]),
                        dependent_image_1: endian.u16(&chunk[12..14]),
                        dependent_image_2: endian.u16(&chunk[14..16]),
                    });
                }
                if entries.replace(parsed).is_some() {
                    return Err(invalid("MPEntry appears more than once"));
                }
            }
            _ => {}
        }
    }
    let version = version.ok_or_else(|| invalid("MPFVersion is missing"))?;
    let image_count = image_count.ok_or_else(|| invalid("NumberOfImages is missing"))?;
    let entries = entries.ok_or_else(|| invalid("MPEntry is missing"))?;
    if usize::try_from(image_count).ok() != Some(entries.len()) {
        return Err(invalid(format!(
            "NumberOfImages is {image_count} but MPEntry declares {} entries",
            entries.len()
        )));
    }
    Ok(MpIndex {
        version,
        image_count,
        entries,
    })
}

/// Length of the APP2 payload that carries an index for `entry_count` images.
pub fn mp_index_payload_len(entry_count: usize) -> Result<usize> {
    let entry_bytes = entry_count
        .checked_mul(MP_ENTRY_BYTES)
        .ok_or_else(|| invalid("MPEntry length overflows"))?;
    // TIFF header, one IFD with three entries and its next-IFD field, then data.
    MPF_APP2_IDENTIFIER
        .len()
        .checked_add(8 + 2 + 3 * 12 + 4)
        .and_then(|fixed| fixed.checked_add(entry_bytes))
        .ok_or_else(|| invalid("MPF payload length overflows"))
}

/// Build the APP2 payload (`MPF\0` plus TIFF) for the given entries.
pub fn make_mp_index_payload(entries: &[MpEntry]) -> Result<Vec<u8>> {
    if entries.is_empty() {
        return Err(invalid("MPF requires at least one image entry"));
    }
    let count = u32::try_from(entries.len()).map_err(|_| invalid("image count exceeds u32"))?;
    let entry_bytes = entries
        .len()
        .checked_mul(MP_ENTRY_BYTES)
        .ok_or_else(|| invalid("MPEntry length overflows"))?;
    let ifd_offset = 8_u32;
    let entries_offset = ifd_offset + 2 + 3 * 12 + 4;
    let mut tiff = Vec::with_capacity(entries_offset as usize + entry_bytes);
    tiff.extend_from_slice(b"II");
    tiff.extend_from_slice(&42_u16.to_le_bytes());
    tiff.extend_from_slice(&ifd_offset.to_le_bytes());
    tiff.extend_from_slice(&3_u16.to_le_bytes());
    // MPFVersion: four undefined bytes stored inline.
    tiff.extend_from_slice(&TAG_VERSION.to_le_bytes());
    tiff.extend_from_slice(&TIFF_TYPE_UNDEFINED.to_le_bytes());
    tiff.extend_from_slice(&4_u32.to_le_bytes());
    tiff.extend_from_slice(&MPF_VERSION);
    // NumberOfImages: one LONG stored inline.
    tiff.extend_from_slice(&TAG_IMAGE_COUNT.to_le_bytes());
    tiff.extend_from_slice(&TIFF_TYPE_LONG.to_le_bytes());
    tiff.extend_from_slice(&1_u32.to_le_bytes());
    tiff.extend_from_slice(&count.to_le_bytes());
    // MPEntry: undefined bytes at an offset from the TIFF header.
    tiff.extend_from_slice(&TAG_ENTRIES.to_le_bytes());
    tiff.extend_from_slice(&TIFF_TYPE_UNDEFINED.to_le_bytes());
    tiff.extend_from_slice(
        &u32::try_from(entry_bytes)
            .map_err(|_| invalid("MPEntry length exceeds u32"))?
            .to_le_bytes(),
    );
    tiff.extend_from_slice(&entries_offset.to_le_bytes());
    tiff.extend_from_slice(&0_u32.to_le_bytes()); // no next IFD
    for entry in entries {
        tiff.extend_from_slice(&entry.attributes.to_le_bytes());
        tiff.extend_from_slice(&entry.image_size.to_le_bytes());
        tiff.extend_from_slice(&entry.data_offset.to_le_bytes());
        tiff.extend_from_slice(&entry.dependent_image_1.to_le_bytes());
        tiff.extend_from_slice(&entry.dependent_image_2.to_le_bytes());
    }
    let mut payload = MPF_APP2_IDENTIFIER.to_vec();
    payload.extend_from_slice(&tiff);
    if payload.len() != mp_index_payload_len(entries.len())? {
        return Err(invalid("MPF payload length disagrees with its layout"));
    }
    // Round-trip the bytes so a writer defect cannot publish a bad index.
    parse_mp_index(&payload[MPF_APP2_IDENTIFIER.len()..])?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<MpEntry> {
        vec![
            MpEntry {
                attributes: MP_ATTR_PRIMARY_IMAGE,
                image_size: 2105870,
                data_offset: 0,
                dependent_image_1: 0,
                dependent_image_2: 0,
            },
            MpEntry {
                attributes: MP_ATTR_GAIN_MAP_IMAGE,
                image_size: 79632,
                data_offset: 2037043,
                dependent_image_1: 0,
                dependent_image_2: 0,
            },
        ]
    }

    #[test]
    fn payload_roundtrips_with_its_layout_length() {
        let payload = make_mp_index_payload(&entries()).unwrap();
        assert_eq!(payload.len(), mp_index_payload_len(2).unwrap());
        assert_eq!(&payload[..4], MPF_APP2_IDENTIFIER);
        let index = parse_mp_index(&payload[4..]).unwrap();
        assert_eq!(index.version, MPF_VERSION);
        assert_eq!(index.image_count, 2);
        assert_eq!(index.entries, entries());
    }

    #[test]
    fn version_bytes_match_a_real_producer_index() {
        // Little-endian TIFF with the same tags as a real dual-layer JPEG.
        let payload = make_mp_index_payload(&entries()).unwrap();
        assert_eq!(&payload[4..6], b"II");
        assert_eq!(
            &payload[4 + 8 + 2..4 + 8 + 2 + 2],
            &TAG_VERSION.to_le_bytes()
        );
        assert_eq!(&payload[4 + 10 + 8..4 + 10 + 12], b"0100");
    }

    #[test]
    fn rejects_inconsistent_counts_and_truncation() {
        let payload = make_mp_index_payload(&entries()).unwrap();
        let mut bad = payload.clone();
        // NumberOfImages is the inline value of the second IFD entry.
        bad[4 + 10 + 12 + 8..4 + 10 + 12 + 12].copy_from_slice(&3_u32.to_le_bytes());
        assert!(parse_mp_index(&bad[4..]).is_err());
        for end in 4..payload.len() {
            assert!(
                parse_mp_index(&payload[4..end]).is_err(),
                "truncated at {end}"
            );
        }
        assert!(parse_mp_index(b"").is_err());
        assert!(make_mp_index_payload(&[]).is_err());
    }
}
