use crate::error::{FormatError, Result};
use crate::fourcc::FourCC;

use super::boxes::validate_field_width;
use super::model::*;

fn push_u16_be(value: u16, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u32_be(value: u32, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u64_be(value: u64, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_uint_be(value: u64, width: u8, output: &mut Vec<u8>, context: &'static str) -> Result<()> {
    validate_field_width(width, context)?;
    if width == 0 {
        if value != 0 {
            return Err(FormatError::invalid(context, "non-zero value cannot fit in zero bytes"));
        }
        return Ok(());
    }
    if width < 8 {
        let bits = u32::from(width) * 8;
        let limit = 1u64 << bits;
        if value >= limit {
            return Err(FormatError::invalid(
                context,
                format!("value {value} does not fit in {width} bytes"),
            ));
        }
    }
    for byte_index in (0..usize::from(width)).rev() {
        output.push((value >> (byte_index * 8)) as u8);
    }
    Ok(())
}

pub fn make_box(kind: FourCC, payload: &[u8]) -> Result<Vec<u8>> {
    let size = payload
        .len()
        .checked_add(8)
        .ok_or_else(|| FormatError::overflow("ISOBMFF box construction"))?;
    if size <= u32::MAX as usize {
        let mut output = Vec::with_capacity(size);
        push_u32_be(size as u32, &mut output);
        output.extend_from_slice(kind.as_bytes());
        output.extend_from_slice(payload);
        Ok(output)
    } else {
        let large_size_usize = payload
            .len()
            .checked_add(16)
            .ok_or_else(|| FormatError::overflow("ISOBMFF largesize construction"))?;
        let large_size = u64::try_from(large_size_usize)
            .map_err(|_| FormatError::overflow("ISOBMFF largesize construction"))?;
        let mut output = Vec::with_capacity(large_size_usize);
        push_u32_be(1, &mut output);
        output.extend_from_slice(kind.as_bytes());
        push_u64_be(large_size, &mut output);
        output.extend_from_slice(payload);
        Ok(output)
    }
}

pub fn make_full_box(kind: FourCC, version: u8, flags: u32, payload: &[u8]) -> Result<Vec<u8>> {
    if flags > 0x00ff_ffff {
        return Err(FormatError::invalid("full box flags", format!("0x{flags:x} exceeds 24 bits")));
    }
    let mut body = Vec::with_capacity(payload.len() + 4);
    body.push(version);
    body.push(((flags >> 16) & 0xff) as u8);
    body.push(((flags >> 8) & 0xff) as u8);
    body.push((flags & 0xff) as u8);
    body.extend_from_slice(payload);
    make_box(kind, &body)
}

pub fn make_pitm_box(version: u8, primary_item_id: u32) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    if version == 0 {
        let item_id = u16::try_from(primary_item_id)
            .map_err(|_| FormatError::invalid("pitm", "version 0 item ID exceeds u16"))?;
        push_u16_be(item_id, &mut payload);
    } else {
        push_u32_be(primary_item_id, &mut payload);
    }
    make_full_box(PITM, version, 0, &payload)
}

pub fn make_infe_box(item_id: u32, item_type: FourCC, flags: u32) -> Result<Vec<u8>> {
    let version = if item_id <= u32::from(u16::MAX) { 2 } else { 3 };
    let mut payload = Vec::new();
    if version == 2 {
        push_u16_be(item_id as u16, &mut payload);
    } else {
        push_u32_be(item_id, &mut payload);
    }
    push_u16_be(0, &mut payload);
    payload.extend_from_slice(item_type.as_bytes());
    payload.push(0);
    make_full_box(INFE, version, flags, &payload)
}

pub fn make_iinf_box(version: u8, entries: &[Vec<u8>]) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    if version == 0 {
        let count = u16::try_from(entries.len())
            .map_err(|_| FormatError::invalid("iinf", "version 0 entry count exceeds u16"))?;
        push_u16_be(count, &mut payload);
    } else {
        let count = u32::try_from(entries.len())
            .map_err(|_| FormatError::overflow("iinf entry count"))?;
        push_u32_be(count, &mut payload);
    }
    for entry in entries {
        payload.extend_from_slice(entry);
    }
    make_full_box(IINF, version, 0, &payload)
}

pub fn make_ipma_box(version: u8, flags: u32, entries: &[IpmaEntry]) -> Result<Vec<u8>> {
    let wide = flags & 1 != 0;
    let mut payload = Vec::new();
    let count = u32::try_from(entries.len()).map_err(|_| FormatError::overflow("ipma entry count"))?;
    push_u32_be(count, &mut payload);
    for entry in entries {
        if version >= 1 {
            push_u32_be(entry.item_id, &mut payload);
        } else {
            let item_id = u16::try_from(entry.item_id)
                .map_err(|_| FormatError::invalid("ipma", "version 0 item ID exceeds u16"))?;
            push_u16_be(item_id, &mut payload);
        }
        let association_count = u8::try_from(entry.associations.len())
            .map_err(|_| FormatError::invalid("ipma", "association count exceeds u8"))?;
        payload.push(association_count);
        for association in &entry.associations {
            if wide {
                if association.property_index > 0x7fff {
                    return Err(FormatError::invalid("ipma", "property index exceeds 15 bits"));
                }
                let raw = association.property_index | if association.essential { 0x8000 } else { 0 };
                push_u16_be(raw, &mut payload);
            } else {
                if association.property_index > 0x7f {
                    return Err(FormatError::invalid("ipma", "property index exceeds 7 bits"));
                }
                let raw = association.property_index as u8 | if association.essential { 0x80 } else { 0 };
                payload.push(raw);
            }
        }
    }
    make_full_box(IPMA, version, flags, &payload)
}

pub fn make_iref_box(version: u8, entries: &[IrefEntry]) -> Result<Vec<u8>> {
    if version > 1 {
        return Err(FormatError::Unsupported {
            context: "iref version",
            value: u64::from(version),
        });
    }
    let mut payload = Vec::new();
    for entry in entries {
        let mut reference = Vec::new();
        if version >= 1 {
            push_u32_be(entry.from_item_id, &mut reference);
        } else {
            let from = u16::try_from(entry.from_item_id)
                .map_err(|_| FormatError::invalid("iref", "version 0 from_item_id exceeds u16"))?;
            push_u16_be(from, &mut reference);
        }
        let to_count = u16::try_from(entry.to_item_ids.len())
            .map_err(|_| FormatError::invalid("iref", "to-item count exceeds u16"))?;
        push_u16_be(to_count, &mut reference);
        for item_id in &entry.to_item_ids {
            if version >= 1 {
                push_u32_be(*item_id, &mut reference);
            } else {
                let item_id = u16::try_from(*item_id)
                    .map_err(|_| FormatError::invalid("iref", "version 0 to_item_id exceeds u16"))?;
                push_u16_be(item_id, &mut reference);
            }
        }
        payload.extend_from_slice(&make_box(entry.kind, &reference)?);
    }
    make_full_box(IREF, version, 0, &payload)
}

pub fn make_iloc_box(
    version: u8,
    offset_size: u8,
    length_size: u8,
    base_offset_size: u8,
    index_size: u8,
    entries: &[IlocEntry],
) -> Result<Vec<u8>> {
    if version > 2 {
        return Err(FormatError::Unsupported {
            context: "iloc version",
            value: u64::from(version),
        });
    }
    for (width, context) in [
        (offset_size, "iloc offset_size"),
        (length_size, "iloc length_size"),
        (base_offset_size, "iloc base_offset_size"),
        (index_size, "iloc index_size"),
    ] {
        validate_field_width(width, context)?;
    }
    if version == 0 && index_size != 0 {
        return Err(FormatError::invalid("iloc", "version 0 cannot encode extent_index"));
    }
    let mut payload = Vec::new();
    payload.push((offset_size << 4) | length_size);
    payload.push((base_offset_size << 4) | if version == 0 { 0 } else { index_size });
    if version >= 2 {
        let count = u32::try_from(entries.len()).map_err(|_| FormatError::overflow("iloc entry count"))?;
        push_u32_be(count, &mut payload);
    } else {
        let count = u16::try_from(entries.len())
            .map_err(|_| FormatError::invalid("iloc", "entry count exceeds u16"))?;
        push_u16_be(count, &mut payload);
    }
    for entry in entries {
        if version >= 2 {
            push_u32_be(entry.item_id, &mut payload);
        } else {
            let item_id = u16::try_from(entry.item_id)
                .map_err(|_| FormatError::invalid("iloc", "item ID exceeds u16"))?;
            push_u16_be(item_id, &mut payload);
        }
        if matches!(version, 1 | 2) {
            push_u16_be(entry.construction_method & 0x000f, &mut payload);
        }
        push_u16_be(entry.data_reference_index, &mut payload);
        push_uint_be(entry.base_offset, base_offset_size, &mut payload, "iloc base_offset")?;
        let extent_count = u16::try_from(entry.extents.len())
            .map_err(|_| FormatError::invalid("iloc", "extent count exceeds u16"))?;
        push_u16_be(extent_count, &mut payload);
        for extent in &entry.extents {
            if index_size > 0 {
                push_uint_be(extent.index.unwrap_or(0), index_size, &mut payload, "iloc extent_index")?;
            }
            push_uint_be(extent.offset, offset_size, &mut payload, "iloc extent_offset")?;
            push_uint_be(extent.length, length_size, &mut payload, "iloc extent_length")?;
        }
    }
    make_full_box(ILOC, version, 0, &payload)
}

pub fn make_ispe_box(width: u32, height: u32) -> Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(8);
    push_u32_be(width, &mut payload);
    push_u32_be(height, &mut payload);
    make_full_box(ISPE, 0, 0, &payload)
}

pub fn make_irot_box(quarter_turns_ccw: u8) -> Result<Vec<u8>> {
    make_box(IROT, &[quarter_turns_ccw & 0x03])
}
