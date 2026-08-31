use crate::cursor::{Cursor, Endian};
use crate::error::{FormatError, Result};
use crate::fourcc::FourCC;

use super::boxes::{parse_boxes, read_full_box_header, validate_field_width};
use super::model::*;

pub fn parse_iloc(data: &[u8], header: &BoxHeader) -> Result<IlocBox> {
    if header.kind != ILOC {
        return Err(FormatError::invalid("iloc", format!("expected iloc, got {}", header.kind)));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "iloc")?;
    let (version, _) = read_full_box_header(&mut cursor)?;
    if version > 2 {
        return Err(FormatError::Unsupported {
            context: "iloc version",
            value: u64::from(version),
        });
    }
    let sizes0 = cursor.read_u8()?;
    let sizes1 = cursor.read_u8()?;
    let offset_size = (sizes0 >> 4) & 0x0f;
    let length_size = sizes0 & 0x0f;
    let base_offset_size = (sizes1 >> 4) & 0x0f;
    let index_size = if matches!(version, 1 | 2) {
        sizes1 & 0x0f
    } else {
        0
    };
    for (width, context) in [
        (offset_size, "iloc offset_size"),
        (length_size, "iloc length_size"),
        (base_offset_size, "iloc base_offset_size"),
        (index_size, "iloc index_size"),
    ] {
        validate_field_width(width, context)?;
    }

    let item_count = if version >= 2 {
        usize::try_from(cursor.read_u32()?).map_err(|_| FormatError::overflow("iloc item count"))?
    } else {
        usize::from(cursor.read_u16()?)
    };
    let minimum_item_bytes = if version >= 2 {
        10usize
    } else if version == 1 {
        8usize
    } else {
        6usize
    };
    if item_count > cursor.remaining() / minimum_item_bytes {
        return Err(FormatError::invalid(
            "iloc",
            format!("declares {item_count} entries but only {} bytes remain", cursor.remaining()),
        ));
    }

    let mut entries = Vec::new();
    for _ in 0..item_count {
        let item_id = if version >= 2 {
            cursor.read_u32()?
        } else {
            u32::from(cursor.read_u16()?)
        };
        let construction_method = if matches!(version, 1 | 2) {
            cursor.read_u16()? & 0x000f
        } else {
            0
        };
        let data_reference_index = cursor.read_u16()?;
        let base_offset = cursor.read_uint(usize::from(base_offset_size))?;
        let extent_count = usize::from(cursor.read_u16()?);
        let extent_width = usize::from(index_size)
            .checked_add(usize::from(offset_size))
            .and_then(|value| value.checked_add(usize::from(length_size)))
            .ok_or_else(|| FormatError::overflow("iloc extent width"))?;
        if extent_width > 0 && extent_count > cursor.remaining() / extent_width {
            return Err(FormatError::invalid(
                "iloc",
                format!("item {item_id} declares {extent_count} extents beyond the box boundary"),
            ));
        }

        let mut extents = Vec::new();
        for _ in 0..extent_count {
            let index = if index_size > 0 {
                Some(cursor.read_uint(usize::from(index_size))?)
            } else {
                None
            };
            let offset = cursor.read_uint(usize::from(offset_size))?;
            let length = cursor.read_uint(usize::from(length_size))?;
            base_offset
                .checked_add(offset)
                .ok_or_else(|| FormatError::overflow("iloc resolved extent offset"))?;
            extents.push(IlocExtent {
                index,
                offset,
                length,
            });
        }
        entries.push(IlocEntry {
            item_id,
            construction_method,
            data_reference_index,
            base_offset,
            extents,
        });
    }
    if !cursor.is_empty() {
        return Err(FormatError::invalid(
            "iloc",
            format!("{} unconsumed bytes remain", cursor.remaining()),
        ));
    }

    Ok(IlocBox {
        version,
        offset_size,
        length_size,
        base_offset_size,
        index_size,
        entries,
    })
}

fn parse_infe(data: &[u8], header: &BoxHeader) -> Result<ItemInfo> {
    if header.kind != INFE {
        return Err(FormatError::invalid("infe", format!("expected infe, got {}", header.kind)));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "infe")?;
    let (version, flags) = read_full_box_header(&mut cursor)?;
    let (item_id, item_type) = if version >= 2 {
        let item_id = if version >= 3 {
            cursor.read_u32()?
        } else {
            u32::from(cursor.read_u16()?)
        };
        let _protection_index = cursor.read_u16()?;
        let item_type = FourCC::from_slice(cursor.take(4)?)?;
        let _item_name = cursor.read_c_string()?;
        (item_id, Some(item_type))
    } else {
        let item_id = u32::from(cursor.read_u16()?);
        let _protection_index = cursor.read_u16()?;
        let _item_name = cursor.read_c_string()?;
        (item_id, None)
    };
    Ok(ItemInfo {
        item_id,
        item_type,
        flags,
        box_range: header.box_range(),
    })
}

pub fn parse_iinf(data: &[u8], header: &BoxHeader) -> Result<IinfBox> {
    if header.kind != IINF {
        return Err(FormatError::invalid("iinf", format!("expected iinf, got {}", header.kind)));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "iinf")?;
    let (version, _) = read_full_box_header(&mut cursor)?;
    let declared_count = if version >= 1 {
        usize::try_from(cursor.read_u32()?).map_err(|_| FormatError::overflow("iinf entry count"))?
    } else {
        usize::from(cursor.read_u16()?)
    };
    if declared_count > cursor.remaining() / 8 {
        return Err(FormatError::invalid(
            "iinf",
            format!("declares {declared_count} entries but only {} bytes remain", cursor.remaining()),
        ));
    }
    let children = parse_boxes(data, cursor.position()..cursor.end())?;
    let mut entries = Vec::new();
    for child in &children {
        if child.kind != INFE {
            return Err(FormatError::invalid(
                "iinf",
                format!("unexpected child box {}", child.kind),
            ));
        }
        entries.push(parse_infe(data, child)?);
    }
    if entries.len() != declared_count {
        return Err(FormatError::invalid(
            "iinf",
            format!("declared {declared_count} entries, parsed {}", entries.len()),
        ));
    }
    Ok(IinfBox { version, entries })
}

pub fn parse_pitm(data: &[u8], header: &BoxHeader) -> Result<u32> {
    if header.kind != PITM {
        return Err(FormatError::invalid("pitm", format!("expected pitm, got {}", header.kind)));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "pitm")?;
    let (version, _) = read_full_box_header(&mut cursor)?;
    let item_id = if version == 0 {
        u32::from(cursor.read_u16()?)
    } else {
        cursor.read_u32()?
    };
    if !cursor.is_empty() {
        return Err(FormatError::invalid("pitm", "unexpected trailing bytes"));
    }
    Ok(item_id)
}

pub fn parse_ipma(data: &[u8], header: &BoxHeader) -> Result<IpmaBox> {
    if header.kind != IPMA {
        return Err(FormatError::invalid("ipma", format!("expected ipma, got {}", header.kind)));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "ipma")?;
    let (version, flags) = read_full_box_header(&mut cursor)?;
    let entry_count = usize::try_from(cursor.read_u32()?)
        .map_err(|_| FormatError::overflow("ipma entry count"))?;
    let item_id_width = if version >= 1 { 4usize } else { 2usize };
    let minimum_entry_bytes = item_id_width + 1;
    if entry_count > cursor.remaining() / minimum_entry_bytes {
        return Err(FormatError::invalid(
            "ipma",
            format!("declares {entry_count} entries beyond the box boundary"),
        ));
    }
    let wide_associations = flags & 1 != 0;
    let association_width = if wide_associations { 2usize } else { 1usize };
    let mut entries = Vec::new();
    for _ in 0..entry_count {
        let item_id = if version >= 1 {
            cursor.read_u32()?
        } else {
            u32::from(cursor.read_u16()?)
        };
        let association_count = usize::from(cursor.read_u8()?);
        if association_count > cursor.remaining() / association_width {
            return Err(FormatError::invalid(
                "ipma",
                format!("item {item_id} declares {association_count} associations beyond the box boundary"),
            ));
        }
        let mut associations = Vec::new();
        for _ in 0..association_count {
            let (property_index, essential) = if wide_associations {
                let raw = cursor.read_u16()?;
                (raw & 0x7fff, raw & 0x8000 != 0)
            } else {
                let raw = cursor.read_u8()?;
                (u16::from(raw & 0x7f), raw & 0x80 != 0)
            };
            associations.push(IpmaAssociation {
                property_index,
                essential,
            });
        }
        entries.push(IpmaEntry {
            item_id,
            associations,
        });
    }
    if !cursor.is_empty() {
        return Err(FormatError::invalid("ipma", "unexpected trailing bytes"));
    }
    Ok(IpmaBox {
        version,
        flags,
        entries,
    })
}

pub fn parse_iref(data: &[u8], header: &BoxHeader) -> Result<IrefBox> {
    if header.kind != IREF {
        return Err(FormatError::invalid("iref", format!("expected iref, got {}", header.kind)));
    }
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "iref")?;
    let (version, _) = read_full_box_header(&mut cursor)?;
    if version > 1 {
        return Err(FormatError::Unsupported {
            context: "iref version",
            value: u64::from(version),
        });
    }
    let children = parse_boxes(data, cursor.position()..cursor.end())?;
    let mut entries = Vec::new();
    for child in children {
        let mut reference = Cursor::bounded(data, child.payload_range(), Endian::Big, "iref entry")?;
        let from_item_id = if version >= 1 {
            reference.read_u32()?
        } else {
            u32::from(reference.read_u16()?)
        };
        let to_count = usize::from(reference.read_u16()?);
        let id_width = if version >= 1 { 4usize } else { 2usize };
        if to_count > reference.remaining() / id_width {
            return Err(FormatError::invalid(
                "iref",
                format!("reference {} from item {from_item_id} exceeds child box", child.kind),
            ));
        }
        let mut to_item_ids = Vec::new();
        for _ in 0..to_count {
            to_item_ids.push(if version >= 1 {
                reference.read_u32()?
            } else {
                u32::from(reference.read_u16()?)
            });
        }
        if !reference.is_empty() {
            return Err(FormatError::invalid("iref", "unexpected trailing bytes in reference"));
        }
        entries.push(IrefEntry {
            kind: child.kind,
            from_item_id,
            to_item_ids,
        });
    }
    Ok(IrefBox { version, entries })
}

pub fn parse_ipco_properties(data: &[u8], iprp: &BoxHeader) -> Result<Vec<PropertyInfo>> {
    if iprp.kind != IPRP {
        return Err(FormatError::invalid("iprp", format!("expected iprp, got {}", iprp.kind)));
    }
    let children = parse_boxes(data, iprp.payload_range())?;
    let ipco = children
        .iter()
        .find(|child| child.kind == IPCO)
        .ok_or_else(|| FormatError::invalid("iprp", "ipco child is missing"))?;
    let properties = parse_boxes(data, ipco.payload_range())?;
    let mut result = Vec::new();
    for (position, property) in properties.into_iter().enumerate() {
        let index = u32::try_from(position + 1)
            .map_err(|_| FormatError::overflow("ipco property index"))?;
        result.push(PropertyInfo {
            index,
            kind: property.kind,
            box_range: property.box_range(),
        });
    }
    Ok(result)
}

pub fn parse_meta_box(data: &[u8], meta: &BoxHeader) -> Result<ParsedMeta> {
    if meta.kind != META {
        return Err(FormatError::invalid("meta", format!("expected meta, got {}", meta.kind)));
    }
    let mut cursor = Cursor::bounded(data, meta.payload_range(), Endian::Big, "meta")?;
    let _ = read_full_box_header(&mut cursor)?;
    let children = parse_boxes(data, cursor.position()..cursor.end())?;
    let find_required = |kind: FourCC| -> Result<&BoxHeader> {
        children
            .iter()
            .find(|child| child.kind == kind)
            .ok_or_else(|| FormatError::invalid("meta", format!("required child {kind} is missing")))
    };
    let iloc_header = find_required(ILOC)?;
    let iinf_header = find_required(IINF)?;
    let pitm_header = find_required(PITM)?;
    let iprp_header = find_required(IPRP)?;
    let iprp_children = parse_boxes(data, iprp_header.payload_range())?;
    let ipma_header = iprp_children
        .iter()
        .find(|child| child.kind == IPMA)
        .ok_or_else(|| FormatError::invalid("iprp", "ipma child is missing"))?;
    let iref = children
        .iter()
        .find(|child| child.kind == IREF)
        .map(|header| parse_iref(data, header))
        .transpose()?;
    let idat = children.iter().find(|child| child.kind == IDAT).cloned();

    Ok(ParsedMeta {
        iloc: parse_iloc(data, iloc_header)?,
        iinf: parse_iinf(data, iinf_header)?,
        primary_item_id: parse_pitm(data, pitm_header)?,
        ipma: parse_ipma(data, ipma_header)?,
        properties: parse_ipco_properties(data, iprp_header)?,
        iref,
        idat,
    })
}
