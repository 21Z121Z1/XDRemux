use crate::cursor::{Cursor, Endian};
use crate::error::{FormatError, Result};
use crate::isobmff::{BoxHeader, IlocEntry, ItemInfo, EXIF};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Orientation {
    Normal = 1,
    FlipHorizontal = 2,
    Rotate180 = 3,
    FlipVertical = 4,
    Transpose = 5,
    Rotate90Clockwise = 6,
    Transverse = 7,
    Rotate90CounterClockwise = 8,
}

impl Orientation {
    pub fn from_exif(value: u16) -> Result<Self> {
        match value {
            1 => Ok(Self::Normal),
            2 => Ok(Self::FlipHorizontal),
            3 => Ok(Self::Rotate180),
            4 => Ok(Self::FlipVertical),
            5 => Ok(Self::Transpose),
            6 => Ok(Self::Rotate90Clockwise),
            7 => Ok(Self::Transverse),
            8 => Ok(Self::Rotate90CounterClockwise),
            _ => Err(FormatError::Unsupported {
                context: "EXIF orientation",
                value: u64::from(value),
            }),
        }
    }

    pub const fn exif_value(self) -> u16 {
        self as u16
    }

    pub const fn swaps_axes(self) -> bool {
        matches!(
            self,
            Self::Transpose
                | Self::Rotate90Clockwise
                | Self::Transverse
                | Self::Rotate90CounterClockwise
        )
    }

    pub const fn output_dimensions(self, width: u32, height: u32) -> (u32, u32) {
        if self.swaps_axes() {
            (height, width)
        } else {
            (width, height)
        }
    }

    pub const fn requires_mirror(self) -> bool {
        matches!(
            self,
            Self::FlipHorizontal | Self::FlipVertical | Self::Transpose | Self::Transverse
        )
    }

    /// HEIF `irot` stores counter-clockwise quarter turns. Mirror components
    /// are intentionally not represented here and must be handled separately.
    pub const fn irot_quarter_turns_ccw(self) -> u8 {
        match self {
            Self::Rotate180 => 2,
            Self::Rotate90Clockwise => 3,
            Self::Rotate90CounterClockwise => 1,
            _ => 0,
        }
    }
}

pub fn read_item_payload(
    data: &[u8],
    entry: &IlocEntry,
    idat: Option<&BoxHeader>,
) -> Result<Vec<u8>> {
    if entry.data_reference_index != 0 {
        return Err(FormatError::Unsupported {
            context: "iloc data_reference_index",
            value: u64::from(entry.data_reference_index),
        });
    }
    if entry.extents.is_empty() {
        return Err(FormatError::invalid(
            "HEIF item",
            format!("item {} has no extents", entry.item_id),
        ));
    }
    let construction_base = match entry.construction_method {
        0 => 0usize,
        1 => idat
            .ok_or_else(|| {
                FormatError::invalid(
                    "HEIF item",
                    format!("item {} references idat, but meta has no idat box", entry.item_id),
                )
            })?
            .data_start,
        other => {
            return Err(FormatError::Unsupported {
                context: "iloc construction_method",
                value: u64::from(other),
            })
        }
    };
    let mut output = Vec::new();
    for extent in &entry.extents {
        let relative = entry
            .base_offset
            .checked_add(extent.offset)
            .ok_or_else(|| FormatError::overflow("HEIF item extent offset"))?;
        let relative = usize::try_from(relative)
            .map_err(|_| FormatError::overflow("HEIF item extent offset"))?;
        let length = usize::try_from(extent.length)
            .map_err(|_| FormatError::overflow("HEIF item extent length"))?;
        let start = construction_base
            .checked_add(relative)
            .ok_or_else(|| FormatError::overflow("HEIF item absolute offset"))?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| FormatError::overflow("HEIF item extent end"))?;
        let bytes = data.get(start..end).ok_or_else(|| {
            FormatError::invalid(
                "HEIF item",
                format!("item {} extent {start}..{end} is outside the file", entry.item_id),
            )
        })?;
        output.extend_from_slice(bytes);
    }
    Ok(output)
}

pub fn read_heif_exif_orientation(
    data: &[u8],
    items: &[ItemInfo],
    iloc_entries: &[IlocEntry],
    idat: Option<&BoxHeader>,
) -> Result<Orientation> {
    let Some(exif_item) = items.iter().find(|item| item.item_type == Some(EXIF)) else {
        return Ok(Orientation::Normal);
    };
    let entry = iloc_entries
        .iter()
        .find(|entry| entry.item_id == exif_item.item_id)
        .ok_or_else(|| {
            FormatError::invalid(
                "Exif item",
                format!("item {} has no iloc entry", exif_item.item_id),
            )
        })?;
    let payload = read_item_payload(data, entry, idat)?;
    parse_heif_exif_orientation(&payload)
}

pub fn parse_heif_exif_orientation(exif_item: &[u8]) -> Result<Orientation> {
    if exif_item.starts_with(b"II") || exif_item.starts_with(b"MM") {
        return parse_tiff_orientation(exif_item);
    }
    if exif_item.len() < 4 {
        return Err(FormatError::invalid("Exif item", "missing 4-byte TIFF offset"));
    }
    let mut offset_cursor = Cursor::new(exif_item, Endian::Big, "Exif item");
    let offset = usize::try_from(offset_cursor.read_u32()?)
        .map_err(|_| FormatError::overflow("Exif TIFF offset"))?;
    let after_prefix = offset
        .checked_add(4)
        .ok_or_else(|| FormatError::overflow("Exif TIFF offset"))?;
    for candidate in [offset, after_prefix] {
        if let Some(tiff) = exif_item.get(candidate..) {
            if tiff.starts_with(b"II") || tiff.starts_with(b"MM") {
                return parse_tiff_orientation(tiff);
            }
        }
    }
    Err(FormatError::invalid(
        "Exif item",
        "TIFF offset does not point to an II/MM TIFF header",
    ))
}

pub fn parse_tiff_orientation(tiff: &[u8]) -> Result<Orientation> {
    if tiff.len() < 8 {
        return Err(FormatError::invalid("TIFF", "header is shorter than 8 bytes"));
    }
    let endian = match tiff.get(0..2) {
        Some(bytes) if bytes == b"II" => Endian::Little,
        Some(bytes) if bytes == b"MM" => Endian::Big,
        _ => return Err(FormatError::invalid("TIFF", "invalid byte-order marker")),
    };
    let mut header = Cursor::new(tiff, endian, "TIFF header");
    header.skip(2)?;
    if header.read_u16()? != 42 {
        return Err(FormatError::invalid("TIFF", "magic is not 42"));
    }
    let ifd0_offset = usize::try_from(header.read_u32()?)
        .map_err(|_| FormatError::overflow("TIFF IFD0 offset"))?;
    if ifd0_offset == 0 {
        return Ok(Orientation::Normal);
    }
    let mut ifd = Cursor::bounded(tiff, ifd0_offset..tiff.len(), endian, "TIFF IFD0")?;
    let entry_count = usize::from(ifd.read_u16()?);
    let entries_bytes = entry_count
        .checked_mul(12)
        .ok_or_else(|| FormatError::overflow("TIFF IFD0 entry count"))?;
    let trailer_bytes = entries_bytes
        .checked_add(4)
        .ok_or_else(|| FormatError::overflow("TIFF IFD0 length"))?;
    if trailer_bytes > ifd.remaining() {
        return Err(FormatError::invalid(
            "TIFF IFD0",
            format!("declares {entry_count} entries beyond the TIFF boundary"),
        ));
    }

    for _ in 0..entry_count {
        let entry_start = ifd.position();
        let entry_end = entry_start
            .checked_add(12)
            .ok_or_else(|| FormatError::overflow("TIFF IFD entry"))?;
        let mut entry = Cursor::bounded(
            tiff,
            entry_start..entry_end,
            endian,
            "TIFF IFD entry",
        )?;
        let tag = entry.read_u16()?;
        let field_type = entry.read_u16()?;
        let count = entry.read_u32()?;
        if tag == 0x0112 {
            if count != 1 {
                return Err(FormatError::invalid(
                    "EXIF orientation",
                    format!("expected one value, got {count}"),
                ));
            }
            let value = match field_type {
                3 => entry.read_u16()?,
                4 => {
                    let long = entry.read_u32()?;
                    u16::try_from(long).map_err(|_| {
                        FormatError::invalid(
                            "EXIF orientation",
                            format!("LONG value {long} exceeds u16"),
                        )
                    })?
                }
                other => {
                    return Err(FormatError::Unsupported {
                        context: "EXIF orientation field type",
                        value: u64::from(other),
                    })
                }
            };
            return Orientation::from_exif(value);
        }
        ifd.skip(12)?;
    }
    Ok(Orientation::Normal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fourcc::FourCC;
    use crate::isobmff::IlocExtent;

    fn tiff_with_orientation(endian: Endian, orientation: u16, long_type: bool) -> Vec<u8> {
        let mut tiff = Vec::new();
        match endian {
            Endian::Little => {
                tiff.extend_from_slice(b"II");
                tiff.extend_from_slice(&42u16.to_le_bytes());
                tiff.extend_from_slice(&8u32.to_le_bytes());
                tiff.extend_from_slice(&1u16.to_le_bytes());
                tiff.extend_from_slice(&0x0112u16.to_le_bytes());
                tiff.extend_from_slice(&(if long_type { 4u16 } else { 3u16 }).to_le_bytes());
                tiff.extend_from_slice(&1u32.to_le_bytes());
                if long_type {
                    tiff.extend_from_slice(&u32::from(orientation).to_le_bytes());
                } else {
                    tiff.extend_from_slice(&orientation.to_le_bytes());
                    tiff.extend_from_slice(&[0, 0]);
                }
                tiff.extend_from_slice(&0u32.to_le_bytes());
            }
            Endian::Big => {
                tiff.extend_from_slice(b"MM");
                tiff.extend_from_slice(&42u16.to_be_bytes());
                tiff.extend_from_slice(&8u32.to_be_bytes());
                tiff.extend_from_slice(&1u16.to_be_bytes());
                tiff.extend_from_slice(&0x0112u16.to_be_bytes());
                tiff.extend_from_slice(&(if long_type { 4u16 } else { 3u16 }).to_be_bytes());
                tiff.extend_from_slice(&1u32.to_be_bytes());
                if long_type {
                    tiff.extend_from_slice(&u32::from(orientation).to_be_bytes());
                } else {
                    tiff.extend_from_slice(&orientation.to_be_bytes());
                    tiff.extend_from_slice(&[0, 0]);
                }
                tiff.extend_from_slice(&0u32.to_be_bytes());
            }
        }
        tiff
    }

    #[test]
    fn all_eight_exif_orientations_parse_in_both_byte_orders() {
        for value in 1..=8 {
            let expected = Orientation::from_exif(value).unwrap();
            for endian in [Endian::Little, Endian::Big] {
                assert_eq!(
                    parse_tiff_orientation(&tiff_with_orientation(endian, value, false)).unwrap(),
                    expected
                );
                assert_eq!(
                    parse_tiff_orientation(&tiff_with_orientation(endian, value, true)).unwrap(),
                    expected
                );
            }
        }
    }

    #[test]
    fn heif_exif_offset_accepts_both_observed_bases() {
        let tiff = tiff_with_orientation(Endian::Little, 6, false);

        let mut relative_to_after_prefix = 0u32.to_be_bytes().to_vec();
        relative_to_after_prefix.extend_from_slice(&tiff);
        assert_eq!(
            parse_heif_exif_orientation(&relative_to_after_prefix).unwrap(),
            Orientation::Rotate90Clockwise
        );

        let mut relative_to_item_start = 4u32.to_be_bytes().to_vec();
        relative_to_item_start.extend_from_slice(&tiff);
        assert_eq!(
            parse_heif_exif_orientation(&relative_to_item_start).unwrap(),
            Orientation::Rotate90Clockwise
        );
    }

    #[test]
    fn malformed_tiff_offsets_and_ifd_counts_fail_closed() {
        let mut bad_offset = 100u32.to_be_bytes().to_vec();
        bad_offset.extend_from_slice(b"xxxx");
        assert!(parse_heif_exif_orientation(&bad_offset).is_err());

        let mut truncated = Vec::new();
        truncated.extend_from_slice(b"II");
        truncated.extend_from_slice(&42u16.to_le_bytes());
        truncated.extend_from_slice(&8u32.to_le_bytes());
        truncated.extend_from_slice(&u16::MAX.to_le_bytes());
        assert!(parse_tiff_orientation(&truncated).is_err());
    }

    #[test]
    fn orientation_geometry_matches_heif_rotation_contract() {
        assert_eq!(Orientation::Rotate90Clockwise.irot_quarter_turns_ccw(), 3);
        assert_eq!(
            Orientation::Rotate90CounterClockwise.irot_quarter_turns_ccw(),
            1
        );
        assert_eq!(Orientation::Rotate180.irot_quarter_turns_ccw(), 2);
        assert_eq!(
            Orientation::Rotate90Clockwise.output_dimensions(4080, 3064),
            (3064, 4080)
        );
        assert!(Orientation::Transpose.requires_mirror());
    }

    #[test]
    fn item_payload_supports_mdat_absolute_and_idat_relative_extents() {
        let data = b"0123456789abcdef";
        let absolute = IlocEntry {
            item_id: 1,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 2,
            extents: vec![IlocExtent {
                index: None,
                offset: 2,
                length: 4,
            }],
        };
        assert_eq!(read_item_payload(data, &absolute, None).unwrap(), b"4567");

        let idat = BoxHeader {
            kind: FourCC::new(*b"idat"),
            box_start: 0,
            data_start: 8,
            data_end: data.len(),
            size: data.len(),
        };
        let relative = IlocEntry {
            item_id: 2,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 1,
                length: 3,
            }],
        };
        assert_eq!(
            read_item_payload(data, &relative, Some(&idat)).unwrap(),
            b"9ab"
        );
    }
}
