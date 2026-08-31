use std::ops::Range;

use crate::cursor::{Cursor, Endian};
use crate::error::{FormatError, Result};
use crate::fourcc::FourCC;

use super::model::{BoxHeader, TopLevelScan, IROT, ISPE, MDAT};

pub(crate) fn read_full_box_header(cursor: &mut Cursor<'_>) -> Result<(u8, u32)> {
    let version = cursor.read_u8()?;
    let flags = cursor.read_u24()?;
    Ok((version, flags))
}

pub(crate) fn validate_field_width(width: u8, context: &'static str) -> Result<()> {
    if width <= 8 {
        Ok(())
    } else {
        Err(FormatError::Unsupported {
            context,
            value: u64::from(width),
        })
    }
}

fn parse_box_at(data: &[u8], position: usize, end: usize, context: &'static str) -> Result<BoxHeader> {
    let mut cursor = Cursor::bounded(data, position..end, Endian::Big, context)?;
    let size32 = cursor.read_u32()?;
    let kind = FourCC::from_slice(cursor.take(4)?)?;

    let (size, header_size) = match size32 {
        0 => (end - position, 8usize),
        1 => {
            let large = cursor.read_u64()?;
            let size = usize::try_from(large)
                .map_err(|_| FormatError::overflow("ISOBMFF largesize"))?;
            if size == 0 {
                return Err(FormatError::invalid(context, "largesize cannot be zero"));
            }
            (size, 16usize)
        }
        value => (
            usize::try_from(value).map_err(|_| FormatError::overflow("ISOBMFF box size"))?,
            8usize,
        ),
    };

    if size < header_size {
        return Err(FormatError::invalid(
            context,
            format!("box {kind} declares size {size}, smaller than header {header_size}"),
        ));
    }
    let data_end = position
        .checked_add(size)
        .ok_or_else(|| FormatError::overflow("ISOBMFF box size"))?;
    if data_end > end {
        return Err(FormatError::invalid(
            context,
            format!("box {kind} ends at {data_end}, bounded region ends at {end}"),
        ));
    }
    let data_start = position
        .checked_add(header_size)
        .ok_or_else(|| FormatError::overflow("ISOBMFF box header"))?;
    if data_end <= position {
        return Err(FormatError::invalid(context, "box does not advance the cursor"));
    }

    Ok(BoxHeader {
        kind,
        box_start: position,
        data_start,
        data_end,
        size,
    })
}

pub fn parse_boxes(data: &[u8], range: Range<usize>) -> Result<Vec<BoxHeader>> {
    if range.start > range.end || range.end > data.len() {
        return Err(FormatError::invalid(
            "ISOBMFF box list",
            format!("range {}..{} exceeds input length {}", range.start, range.end, data.len()),
        ));
    }
    let mut boxes = Vec::new();
    let mut position = range.start;
    while position < range.end {
        let header = parse_box_at(data, position, range.end, "ISOBMFF box")?;
        position = header.data_end;
        boxes.push(header);
    }
    Ok(boxes)
}

/// Top-level HEIF permits XDRemux to retain an opaque vendor trailer after a
/// complete `mdat`. Nested ISO BMFF boxes never use this tolerant mode.
pub fn scan_top_level_boxes(data: &[u8]) -> Result<TopLevelScan> {
    let mut boxes = Vec::new();
    let mut position = 0usize;
    let mut saw_mdat = false;
    while position < data.len() {
        match parse_box_at(data, position, data.len(), "top-level ISOBMFF box") {
            Ok(header) => {
                if header.kind == MDAT {
                    saw_mdat = true;
                }
                position = header.data_end;
                boxes.push(header);
            }
            Err(_) if saw_mdat => {
                return Ok(TopLevelScan {
                    boxes,
                    trailing_range: position..data.len(),
                });
            }
            Err(error) => return Err(error),
        }
    }
    Ok(TopLevelScan {
        boxes,
        trailing_range: data.len()..data.len(),
    })
}

pub fn parse_ispe_dimensions(data: &[u8], header: &BoxHeader) -> Result<(u32, u32)> {
    if header.kind != ISPE {
        return Err(FormatError::invalid(
            "ispe",
            format!("expected ispe, got {}", header.kind),
        ));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "ispe")?;
    let (version, flags) = read_full_box_header(&mut cursor)?;
    if version != 0 || flags != 0 {
        return Err(FormatError::invalid(
            "ispe",
            format!("expected version 0 flags 0, got version {version} flags 0x{flags:06x}"),
        ));
    }
    let width = cursor.read_u32()?;
    let height = cursor.read_u32()?;
    if !cursor.is_empty() {
        return Err(FormatError::invalid("ispe", "unexpected trailing bytes"));
    }
    Ok((width, height))
}

pub fn parse_irot_quarter_turns(data: &[u8], header: &BoxHeader) -> Result<u8> {
    if header.kind != IROT {
        return Err(FormatError::invalid(
            "irot",
            format!("expected irot, got {}", header.kind),
        ));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "irot")?;
    let value = cursor.read_u8()? & 0x03;
    if !cursor.is_empty() {
        return Err(FormatError::invalid("irot", "unexpected trailing bytes"));
    }
    Ok(value)
}
