use xdremux_format::isobmff::{
    parse_boxes, parse_irot_quarter_turns, parse_ispe_dimensions, parse_meta_box,
    scan_top_level_boxes, ParsedMeta, PropertyInfo, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const ISPE: FourCC = FourCC::new(*b"ispe");
const IROT: FourCC = FourCC::new(*b"irot");
const IMIR: FourCC = FourCC::new(*b"imir");

/// Relative aspect tolerance. A one-pixel rounding error on a half-resolution
/// map stays inside this bound. A display-oriented transpose of a 4:3 primary
/// does not.
const CODED_ASPECT_TOLERANCE_DENOMINATOR: u128 = 64;

/// Coded primary geometry, before `irot` is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryCodedGeometry {
    pub width: u32,
    pub height: u32,
    /// HEIF `irot` quarter turns counter-clockwise. Absent `irot` is 0.
    pub quarter_turns_ccw: u8,
    /// HEIF imir axis: 0 flips rows, 1 flips columns. Absent imir is None.
    pub mirror_axis: Option<u8>,
    /// Transformative properties apply in their ipma association order.
    pub mirror_before_rotation: bool,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

pub fn display_dimensions(width: u32, height: u32, quarter_turns_ccw: u8) -> (u32, u32) {
    if quarter_turns_ccw & 1 == 1 {
        (height, width)
    } else {
        (width, height)
    }
}

/// Reject an auxiliary raster whose aspect follows the oriented display instead
/// of the primary item's coded `ispe`.
pub fn require_coded_aspect(
    primary_width: u32,
    primary_height: u32,
    item_width: u32,
    item_height: u32,
    item_name: &str,
) -> Result<()> {
    if primary_width == 0 || primary_height == 0 || item_width == 0 || item_height == 0 {
        return Err(invalid(format!(
            "{item_name} dimensions must be compared with a non-zero primary coded size"
        )));
    }
    let left = u128::from(primary_width) * u128::from(item_height);
    let right = u128::from(primary_height) * u128::from(item_width);
    let difference = left.abs_diff(right);
    let scale = left.max(right).max(1);
    if difference.saturating_mul(CODED_ASPECT_TOLERANCE_DENOMINATOR) > scale {
        return Err(invalid(format!(
            "{item_name} is {item_width}x{item_height} and does not match primary coded orientation {primary_width}x{primary_height}"
        )));
    }
    Ok(())
}

pub fn primary_coded_geometry(source: &[u8]) -> Result<PrimaryCodedGeometry> {
    let top = scan_top_level_boxes(source)?;
    let meta_header = top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| invalid("HEIF meta box is missing"))?;
    let meta = parse_meta_box(source, meta_header)?;
    coded_geometry(source, &meta)
}

pub(crate) fn coded_geometry(source: &[u8], meta: &ParsedMeta) -> Result<PrimaryCodedGeometry> {
    let ispe = associated_property(meta, meta.primary_item_id, ISPE).ok_or_else(|| {
        invalid(format!(
            "primary item {} has no ispe property association",
            meta.primary_item_id
        ))
    })?;
    let (width, height) = read_ispe(source, ispe)?;
    let quarter_turns_ccw = match associated_property(meta, meta.primary_item_id, IROT) {
        Some(irot) => read_irot(source, irot)?,
        None => 0,
    };
    let mirror_axis = associated_property(meta, meta.primary_item_id, IMIR)
        .map(|property| read_imir(source, property))
        .transpose()?;
    let associations = &meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == meta.primary_item_id)
        .ok_or_else(|| invalid("primary item has no property associations"))?
        .associations;
    let position = |kind| {
        associations.iter().position(|association| {
            meta.properties.iter().any(|property| {
                property.index == u32::from(association.property_index) && property.kind == kind
            })
        })
    };
    let mirror_before_rotation = matches!((position(IMIR), position(IROT)), (Some(mirror), Some(rotation)) if mirror < rotation);
    Ok(PrimaryCodedGeometry {
        width,
        height,
        quarter_turns_ccw,
        mirror_axis,
        mirror_before_rotation,
    })
}

pub(crate) fn coded_geometry_if_present(
    source: &[u8],
    meta: &ParsedMeta,
) -> Result<Option<PrimaryCodedGeometry>> {
    if associated_property(meta, meta.primary_item_id, ISPE).is_none() {
        return Ok(None);
    }
    coded_geometry(source, meta).map(Some)
}

fn associated_property(meta: &ParsedMeta, item_id: u32, kind: FourCC) -> Option<&PropertyInfo> {
    let entry = meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)?;
    entry.associations.iter().find_map(|association| {
        meta.properties.iter().find(|property| {
            property.index == u32::from(association.property_index) && property.kind == kind
        })
    })
}

fn read_ispe(source: &[u8], property: &PropertyInfo) -> Result<(u32, u32)> {
    let raw = source.get(property.box_range.clone()).ok_or_else(|| {
        invalid(format!(
            "ispe property {} is outside the HEIF",
            property.index
        ))
    })?;
    let headers = parse_boxes(raw, 0..raw.len())?;
    if headers.len() != 1 {
        return Err(invalid(format!(
            "ispe property {} does not contain exactly one box",
            property.index
        )));
    }
    let (width, height) = parse_ispe_dimensions(raw, &headers[0])?;
    if width == 0 || height == 0 {
        return Err(invalid("primary ispe dimensions must be non-zero"));
    }
    Ok((width, height))
}

fn read_irot(source: &[u8], property: &PropertyInfo) -> Result<u8> {
    let raw = source.get(property.box_range.clone()).ok_or_else(|| {
        invalid(format!(
            "irot property {} is outside the HEIF",
            property.index
        ))
    })?;
    let headers = parse_boxes(raw, 0..raw.len())?;
    if headers.len() != 1 {
        return Err(invalid(format!(
            "irot property {} does not contain exactly one box",
            property.index
        )));
    }
    Ok(parse_irot_quarter_turns(raw, &headers[0])?)
}

fn read_imir(source: &[u8], property: &PropertyInfo) -> Result<u8> {
    let raw = source
        .get(property.box_range.clone())
        .ok_or_else(|| invalid("imir property is outside the HEIF"))?;
    let headers = parse_boxes(raw, 0..raw.len())?;
    if headers.len() != 1 || headers[0].payload_range().len() != 1 {
        return Err(invalid("imir property must contain exactly one axis byte"));
    }
    Ok(raw[headers[0].data_start] & 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coded_aspect_accepts_an_integer_scale_and_rejects_a_transpose() {
        require_coded_aspect(5712, 4284, 2856, 2142, "Gain Map").unwrap();
        require_coded_aspect(4000, 3000, 16, 12, "Gain Map").unwrap();
        require_coded_aspect(4000, 3000, 644, 483, "Gain Map").unwrap();
        assert!(require_coded_aspect(5712, 4284, 2142, 2856, "Gain Map").is_err());
        assert!(require_coded_aspect(4000, 3000, 8, 8, "Gain Map").is_err());
        assert!(require_coded_aspect(4000, 3000, 513, 641, "Gain Map").is_err());
    }

    #[test]
    fn odd_irot_swaps_display_dimensions() {
        assert_eq!(display_dimensions(5712, 4284, 3), (4284, 5712));
        assert_eq!(display_dimensions(5712, 4284, 1), (4284, 5712));
        assert_eq!(display_dimensions(5712, 4284, 0), (5712, 4284));
        assert_eq!(display_dimensions(5712, 4284, 2), (5712, 4284));
    }
}
