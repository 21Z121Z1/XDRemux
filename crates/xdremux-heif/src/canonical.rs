use std::collections::{BTreeMap, BTreeSet};

use xdremux_format::isobmff::{
    parse_boxes, parse_meta_box, scan_top_level_boxes, BoxHeader, ParsedMeta, IPMA, IPRP, IROT,
    ISPE, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};
use crate::native::{self, IsoGainMapAssembly};

const PIXI: FourCC = FourCC::new(*b"pixi");
const AUXC: FourCC = FourCC::new(*b"auxC");
const COLR: FourCC = FourCC::new(*b"colr");

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn read_u16(data: &[u8], offset: &mut usize, end: usize, context: &str) -> Result<u16> {
    let next = offset
        .checked_add(2)
        .ok_or_else(|| invalid(format!("{context} offset overflows")))?;
    let bytes = data
        .get(*offset..next)
        .filter(|_| next <= end)
        .ok_or_else(|| invalid(format!("{context} is truncated")))?;
    *offset = next;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: &mut usize, end: usize, context: &str) -> Result<u32> {
    let next = offset
        .checked_add(4)
        .ok_or_else(|| invalid(format!("{context} offset overflows")))?;
    let bytes = data
        .get(*offset..next)
        .filter(|_| next <= end)
        .ok_or_else(|| invalid(format!("{context} is truncated")))?;
    *offset = next;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn mark_associations_essential(
    data: &mut [u8],
    ipma: &BoxHeader,
    targets: &BTreeMap<u32, BTreeSet<u16>>,
) -> Result<()> {
    let start = ipma.data_start;
    let end = ipma.data_end;
    if end > data.len() || end.saturating_sub(start) < 8 {
        return Err(invalid("ipma payload is truncated"));
    }

    let version = data[start];
    let flags = u32::from_be_bytes([0, data[start + 1], data[start + 2], data[start + 3]]);
    let wide_associations = flags & 1 != 0;
    let mut offset = start + 4;
    let entry_count = usize::try_from(read_u32(data, &mut offset, end, "ipma entry count")?)
        .map_err(|_| invalid("ipma entry count exceeds usize"))?;

    for _ in 0..entry_count {
        let item_id = if version >= 1 {
            read_u32(data, &mut offset, end, "ipma item ID")?
        } else {
            u32::from(read_u16(data, &mut offset, end, "ipma item ID")?)
        };
        let association_count = usize::from(
            *data
                .get(offset)
                .filter(|_| offset < end)
                .ok_or_else(|| invalid("ipma association count is truncated"))?,
        );
        offset += 1;

        for _ in 0..association_count {
            if wide_associations {
                let raw_offset = offset;
                let raw = read_u16(data, &mut offset, end, "ipma association")?;
                let property_index = raw & 0x7fff;
                if targets
                    .get(&item_id)
                    .is_some_and(|indices| indices.contains(&property_index))
                {
                    let updated = raw | 0x8000;
                    data[raw_offset..raw_offset + 2].copy_from_slice(&updated.to_be_bytes());
                }
            } else {
                let raw = *data
                    .get(offset)
                    .filter(|_| offset < end)
                    .ok_or_else(|| invalid("ipma association is truncated"))?;
                let property_index = u16::from(raw & 0x7f);
                if targets
                    .get(&item_id)
                    .is_some_and(|indices| indices.contains(&property_index))
                {
                    data[offset] = raw | 0x80;
                }
                offset += 1;
            }
        }
    }

    if offset != end {
        return Err(invalid("ipma has unexpected trailing bytes"));
    }
    Ok(())
}

fn required_property_indices(
    meta: &ParsedMeta,
    item_id: u32,
    kinds: &[FourCC],
) -> Result<BTreeSet<u16>> {
    let entry = meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .ok_or_else(|| invalid(format!("item {item_id} has no ipma entry")))?;

    let mut indices = BTreeSet::new();
    for kind in kinds {
        let association = entry
            .associations
            .iter()
            .find(|association| {
                meta.properties.iter().any(|property| {
                    property.index == u32::from(association.property_index)
                        && property.kind == *kind
                })
            })
            .ok_or_else(|| {
                invalid(format!(
                    "item {item_id} is missing required {kind} property association"
                ))
            })?;
        indices.insert(association.property_index);
    }
    Ok(indices)
}

fn normalize_consumer_associations(data: &mut [u8]) -> Result<()> {
    let structure = crate::validation::validate_gain_map_structure(data)?;
    let top = scan_top_level_boxes(data)?;
    let meta_header = top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| invalid("canonical HEIF output has no meta box"))?;
    let meta = parse_meta_box(data, meta_header)?;

    // ImageIO's proven ISO Gain Map graph treats these descriptive properties
    // as essential. The native assembler already authors the correct values;
    // this normalization preserves bytes and only restores the association
    // contract so consumer recognition does not depend on permissive parsing.
    let mut targets = BTreeMap::new();
    targets.insert(
        structure.gain_map_item_id,
        required_property_indices(
            &meta,
            structure.gain_map_item_id,
            &[ISPE, PIXI, COLR, IROT, AUXC],
        )?,
    );
    targets.insert(
        structure.tmap_item_id,
        required_property_indices(&meta, structure.tmap_item_id, &[ISPE, PIXI, COLR])?,
    );

    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta child offset overflows"))?;
    let meta_children = parse_boxes(data, meta_children_start..meta_header.data_end)?;
    let iprp = meta_children
        .iter()
        .find(|header| header.kind == IPRP)
        .ok_or_else(|| invalid("canonical HEIF output has no iprp box"))?;
    let iprp_children = parse_boxes(data, iprp.payload_range())?;
    let ipma = iprp_children
        .iter()
        .find(|header| header.kind == IPMA)
        .ok_or_else(|| invalid("canonical HEIF output has no ipma box"))?
        .clone();
    mark_associations_essential(data, &ipma, &targets)?;

    let reparsed = parse_meta_box(data, meta_header)?;
    for (item_id, required) in targets {
        let entry = reparsed
            .ipma
            .entries
            .iter()
            .find(|entry| entry.item_id == item_id)
            .ok_or_else(|| invalid(format!("item {item_id} disappeared after normalization")))?;
        for property_index in required {
            if !entry
                .associations
                .iter()
                .any(|association| association.property_index == property_index && association.essential)
            {
                return Err(invalid(format!(
                    "item {item_id} property {property_index} is not essential after normalization"
                )));
            }
        }
    }
    Ok(())
}

/// Construct the canonical portable ISO Gain Map HEIF and normalize the
/// consumer-facing property-association contract without changing payloads or
/// file offsets. Product policy remains in Rust; no Apple framework is used.
pub fn assemble_iso_gain_map_heif(
    source: &[u8],
    assembly: &IsoGainMapAssembly<'_>,
) -> Result<Vec<u8>> {
    let mut output = native::assemble_iso_gain_map_heif(source, assembly)?;
    normalize_consumer_associations(&mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::{make_ipma_box, parse_ipma, IpmaAssociation, IpmaEntry};

    #[test]
    fn essential_normalization_changes_only_requested_associations() {
        let original = make_ipma_box(
            0,
            0,
            &[
                IpmaEntry {
                    item_id: 7,
                    associations: vec![
                        IpmaAssociation {
                            property_index: 2,
                            essential: false,
                        },
                        IpmaAssociation {
                            property_index: 3,
                            essential: false,
                        },
                    ],
                },
                IpmaEntry {
                    item_id: 8,
                    associations: vec![IpmaAssociation {
                        property_index: 4,
                        essential: false,
                    }],
                },
            ],
        )
        .unwrap();
        let mut data = original.clone();
        let header = parse_boxes(&data, 0..data.len()).unwrap().remove(0);
        let mut targets = BTreeMap::new();
        targets.insert(7, BTreeSet::from([3]));

        mark_associations_essential(&mut data, &header, &targets).unwrap();
        assert_eq!(data.len(), original.len());
        let parsed = parse_ipma(&data, &header).unwrap();
        assert!(!parsed.entries[0].associations[0].essential);
        assert!(parsed.entries[0].associations[1].essential);
        assert!(!parsed.entries[1].associations[0].essential);
    }
}
