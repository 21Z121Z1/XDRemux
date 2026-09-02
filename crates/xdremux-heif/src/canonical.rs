use std::collections::{BTreeMap, BTreeSet};

use xdremux_format::isobmff::{
    make_box, make_iloc_box, make_iref_box, parse_boxes, parse_iref, parse_meta_box,
    scan_top_level_boxes, BoxHeader, IlocEntry, IrefBox, IrefEntry, ParsedMeta, ILOC, IPMA, IPRP,
    IREF, IROT, ISPE, MDAT, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};
use crate::native::{self, IsoGainMapAssembly};
use crate::validation::GainMapStructure;

const PIXI: FourCC = FourCC::new(*b"pixi");
const AUXC: FourCC = FourCC::new(*b"auxC");
const COLR: FourCC = FourCC::new(*b"colr");
const AUXL: FourCC = FourCC::new(*b"auxl");
const GRPL: FourCC = FourCC::new(*b"grpl");
const ALTR: FourCC = FourCC::new(*b"altr");

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn raw_box<'a>(source: &'a [u8], header: &BoxHeader, context: &str) -> Result<&'a [u8]> {
    source
        .get(header.box_range())
        .ok_or_else(|| invalid(format!("{context} box is outside source")))
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

fn read_u32_at(data: &[u8], offset: usize, end: usize, context: &str) -> Result<u32> {
    let mut cursor = offset;
    read_u32(data, &mut cursor, end, context)
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
            if !entry.associations.iter().any(|association| {
                association.property_index == property_index && association.essential
            }) {
                return Err(invalid(format!(
                    "item {item_id} property {property_index} is not essential after normalization"
                )));
            }
        }
    }
    Ok(())
}

fn build_consumer_iref(
    source: &IrefBox,
    primary_item_id: u32,
    gain_map_item_id: u32,
    tmap_item_id: u32,
) -> Result<Vec<u8>> {
    let mut entries = source
        .entries
        .iter()
        .filter(|reference| !(reference.kind == AUXL && reference.from_item_id == gain_map_item_id))
        .cloned()
        .collect::<Vec<_>>();
    entries.push(IrefEntry {
        kind: AUXL,
        from_item_id: gain_map_item_id,
        to_item_ids: vec![primary_item_id, tmap_item_id],
    });

    let maximum_item_id = entries
        .iter()
        .flat_map(|entry| {
            std::iter::once(entry.from_item_id).chain(entry.to_item_ids.iter().copied())
        })
        .max()
        .unwrap_or(0);
    let version = if maximum_item_id > u32::from(u16::MAX) {
        1
    } else {
        source.version
    };
    make_iref_box(version, &entries)
        .map_err(|error| invalid(format!("canonical ImageIO iref: {error}")))
}

fn make_altr_entity_group_box(
    group_id: u32,
    tmap_item_id: u32,
    primary_item_id: u32,
) -> Result<Vec<u8>> {
    let mut payload = vec![0, 0, 0, 0];
    payload.extend_from_slice(&group_id.to_be_bytes());
    payload.extend_from_slice(&2_u32.to_be_bytes());
    payload.extend_from_slice(&tmap_item_id.to_be_bytes());
    payload.extend_from_slice(&primary_item_id.to_be_bytes());
    make_box(ALTR, &payload).map_err(|error| invalid(format!("canonical altr group: {error}")))
}

fn preserved_entity_group_payload(
    source: &[u8],
    grpl: &BoxHeader,
    valid_item_ids: &BTreeSet<u32>,
) -> Result<(Vec<u8>, Vec<u32>)> {
    let mut payload = Vec::new();
    let mut group_ids = Vec::new();
    for child in parse_boxes(source, grpl.payload_range())? {
        let raw = raw_box(source, &child, "entity group")?;
        if child.data_end.saturating_sub(child.data_start) < 12 {
            payload.extend_from_slice(raw);
            continue;
        }

        let group_id = read_u32_at(
            source,
            child.data_start + 4,
            child.data_end,
            "entity group ID",
        )?;
        let entity_count = usize::try_from(read_u32_at(
            source,
            child.data_start + 8,
            child.data_end,
            "entity group count",
        )?)
        .map_err(|_| invalid("entity group count exceeds usize"))?;
        group_ids.push(group_id);

        if child.kind != ALTR {
            payload.extend_from_slice(raw);
            continue;
        }

        let entities_start = child
            .data_start
            .checked_add(12)
            .ok_or_else(|| invalid("altr entity offset overflows"))?;
        let entities_bytes = entity_count
            .checked_mul(4)
            .ok_or_else(|| invalid("altr entity count overflows"))?;
        let entities_end = entities_start
            .checked_add(entities_bytes)
            .ok_or_else(|| invalid("altr entity range overflows"))?;
        if entities_end > child.data_end {
            return Err(invalid("altr entity group is truncated"));
        }
        let mut entities = Vec::with_capacity(entity_count);
        let mut offset = entities_start;
        for _ in 0..entity_count {
            entities.push(read_u32(
                source,
                &mut offset,
                entities_end,
                "altr entity ID",
            )?);
        }

        // The native assembler has already removed the previous gain-map graph.
        // An old altr whose entities no longer exist is therefore stale; keep
        // every unrelated valid group byte-for-byte and replace only that stale
        // alternate-rendering relationship.
        if entities
            .iter()
            .all(|item_id| valid_item_ids.contains(item_id))
        {
            payload.extend_from_slice(raw);
        }
    }
    Ok((payload, group_ids))
}

fn build_consumer_grpl(
    source: &[u8],
    meta: &ParsedMeta,
    meta_children: &[BoxHeader],
    structure: &GainMapStructure,
) -> Result<Vec<u8>> {
    let mut groups = meta_children.iter().filter(|header| header.kind == GRPL);
    let existing = groups.next();
    if groups.next().is_some() {
        return Err(invalid("source meta contains more than one grpl"));
    }

    let valid_item_ids = meta
        .iinf
        .entries
        .iter()
        .map(|item| item.item_id)
        .collect::<BTreeSet<_>>();
    let (mut payload, group_ids) = match existing {
        Some(grpl) => preserved_entity_group_payload(source, grpl, &valid_item_ids)?,
        None => (Vec::new(), Vec::new()),
    };
    let maximum_item_id = valid_item_ids.iter().copied().max().unwrap_or(0);
    let maximum_group_id = group_ids.into_iter().max().unwrap_or(0);
    let group_id = maximum_item_id
        .max(maximum_group_id)
        .checked_add(1)
        .ok_or_else(|| invalid("canonical altr group ID overflows"))?;
    payload.extend_from_slice(&make_altr_entity_group_box(
        group_id,
        structure.tmap_item_id,
        structure.primary_item_id,
    )?);
    make_box(GRPL, &payload).map_err(|error| invalid(format!("canonical grpl: {error}")))
}

fn rebuild_meta(
    source: &[u8],
    meta_header: &BoxHeader,
    meta_children: &[BoxHeader],
    iloc: &[u8],
    iref: &[u8],
    grpl: &[u8],
) -> Result<Vec<u8>> {
    let full_header_end = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta full-box header offset overflows"))?;
    let full_header = source
        .get(meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    let mut saw_iloc = false;
    let mut saw_iref = false;
    let mut saw_grpl = false;

    for header in meta_children {
        if header.kind == ILOC {
            if saw_iloc {
                return Err(invalid("source meta contains more than one iloc"));
            }
            saw_iloc = true;
            payload.extend_from_slice(iloc);
        } else if header.kind == IREF {
            if saw_iref {
                return Err(invalid("source meta contains more than one iref"));
            }
            saw_iref = true;
            payload.extend_from_slice(iref);
        } else if header.kind == GRPL {
            if saw_grpl {
                return Err(invalid("source meta contains more than one grpl"));
            }
            saw_grpl = true;
            payload.extend_from_slice(grpl);
        } else {
            payload.extend_from_slice(raw_box(source, header, "meta child")?);
        }
    }

    if !saw_iloc {
        return Err(invalid("canonical HEIF output has no iloc"));
    }
    if !saw_iref {
        payload.extend_from_slice(iref);
    }
    if !saw_grpl {
        payload.extend_from_slice(grpl);
    }
    make_box(META, &payload).map_err(|error| invalid(format!("canonical meta: {error}")))
}

fn shifted(value: u64, delta: i128, context: &str) -> Result<u64> {
    let shifted = i128::from(value)
        .checked_add(delta)
        .ok_or_else(|| invalid(format!("{context} overflows")))?;
    u64::try_from(shifted).map_err(|_| invalid(format!("{context} becomes negative or too large")))
}

fn relocated_iloc_entries(
    meta: &ParsedMeta,
    mdat: &BoxHeader,
    delta: i128,
) -> Result<Vec<IlocEntry>> {
    let mdat_start =
        u64::try_from(mdat.data_start).map_err(|_| invalid("mdat data offset exceeds u64"))?;
    let mdat_end =
        u64::try_from(mdat.data_end).map_err(|_| invalid("mdat end offset exceeds u64"))?;
    let mut entries = meta.iloc.entries.clone();

    for entry in &mut entries {
        if entry.construction_method != 0 {
            continue;
        }
        for extent in &mut entry.extents {
            let absolute = entry
                .base_offset
                .checked_add(extent.offset)
                .ok_or_else(|| {
                    invalid(format!("item {} extent offset overflows", entry.item_id))
                })?;
            let end = absolute
                .checked_add(extent.length)
                .ok_or_else(|| invalid(format!("item {} extent end overflows", entry.item_id)))?;
            if absolute < mdat_start || end > mdat_end {
                return Err(invalid(format!(
                    "item {} has file-backed data outside the primary mdat; consumer canonicalization refuses to guess",
                    entry.item_id
                )));
            }
            extent.offset = shifted(extent.offset, delta, "iloc file-backed extent offset")?;
        }
    }
    Ok(entries)
}

fn normalize_consumer_graph(source: &[u8]) -> Result<Vec<u8>> {
    let structure = crate::validation::validate_gain_map_structure(source)?;
    let top = scan_top_level_boxes(source)?;
    let meta_header = top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| invalid("canonical HEIF output has no meta box"))?;
    let mdat = top
        .boxes
        .iter()
        .find(|header| header.kind == MDAT)
        .ok_or_else(|| invalid("canonical HEIF output has no mdat box"))?;
    let meta = parse_meta_box(source, meta_header)?;
    let source_iref = meta
        .iref
        .as_ref()
        .ok_or_else(|| invalid("canonical HEIF output has no iref"))?;
    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta child offset overflows"))?;
    let meta_children = parse_boxes(source, meta_children_start..meta_header.data_end)?;
    let iloc_header = meta_children
        .iter()
        .find(|header| header.kind == ILOC)
        .ok_or_else(|| invalid("canonical HEIF output has no iloc box"))?;

    let iref = build_consumer_iref(
        source_iref,
        structure.primary_item_id,
        structure.gain_map_item_id,
        structure.tmap_item_id,
    )?;
    let grpl = build_consumer_grpl(source, &meta, &meta_children, &structure)?;
    let original_iloc = raw_box(source, iloc_header, "iloc")?;
    let preliminary_meta = rebuild_meta(
        source,
        meta_header,
        &meta_children,
        original_iloc,
        &iref,
        &grpl,
    )?;

    let new_mdat_box_start = top
        .boxes
        .iter()
        .take_while(|header| header.box_start != mdat.box_start)
        .try_fold(0_usize, |offset, header| {
            let replacement_len = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(replacement_len)
                .ok_or_else(|| invalid("canonical mdat file offset overflows"))
        })?;
    let mdat_header_size = mdat
        .data_start
        .checked_sub(mdat.box_start)
        .ok_or_else(|| invalid("mdat header geometry is invalid"))?;
    let new_mdat_data_start = new_mdat_box_start
        .checked_add(mdat_header_size)
        .ok_or_else(|| invalid("canonical mdat data offset overflows"))?;
    let delta = i128::try_from(new_mdat_data_start)
        .map_err(|_| invalid("new mdat data offset exceeds i128"))?
        - i128::try_from(mdat.data_start)
            .map_err(|_| invalid("old mdat data offset exceeds i128"))?;

    let relocated = relocated_iloc_entries(&meta, mdat, delta)?;
    let final_iloc = make_iloc_box(
        meta.iloc.version,
        meta.iloc.offset_size,
        meta.iloc.length_size,
        meta.iloc.base_offset_size,
        meta.iloc.index_size,
        &relocated,
    )
    .map_err(|error| invalid(format!("canonical relocated iloc: {error}")))?;
    let final_meta = rebuild_meta(
        source,
        meta_header,
        &meta_children,
        &final_iloc,
        &iref,
        &grpl,
    )?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid(
            "canonical iloc rewrite changed meta size unexpectedly",
        ));
    }

    let trailing = source
        .get(top.trailing_range.clone())
        .ok_or_else(|| invalid("top-level trailing bytes are outside source"))?;
    let output_capacity = top.boxes.iter().try_fold(trailing.len(), |total, header| {
        total
            .checked_add(if header.kind == META {
                final_meta.len()
            } else {
                header.size
            })
            .ok_or_else(|| invalid("canonical HEIF output size overflows"))
    })?;
    let mut output = Vec::with_capacity(output_capacity);
    for header in &top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else {
            output.extend_from_slice(raw_box(source, header, "top-level")?);
        }
    }
    output.extend_from_slice(trailing);

    validate_consumer_graph(&output, &structure)?;
    crate::validation::validate_gain_map_structure(&output)?;
    Ok(output)
}

fn validate_consumer_graph(data: &[u8], structure: &GainMapStructure) -> Result<()> {
    let top = scan_top_level_boxes(data)?;
    let meta_header = top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| invalid("canonical HEIF output has no meta box"))?;
    let meta = parse_meta_box(data, meta_header)?;
    let auxl_ok = meta.iref.as_ref().is_some_and(|iref| {
        iref.entries.iter().any(|reference| {
            reference.kind == AUXL
                && reference.from_item_id == structure.gain_map_item_id
                && reference.to_item_ids == vec![structure.primary_item_id, structure.tmap_item_id]
        })
    });
    if !auxl_ok {
        return Err(invalid(
            "canonical Gain Map auxl must target primary and tmap",
        ));
    }

    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta child offset overflows"))?;
    let meta_children = parse_boxes(data, meta_children_start..meta_header.data_end)?;
    let grpl = meta_children
        .iter()
        .find(|header| header.kind == GRPL)
        .ok_or_else(|| invalid("canonical HEIF output has no grpl"))?;
    let mut matched = false;
    for child in parse_boxes(data, grpl.payload_range())? {
        if child.kind != ALTR || child.data_end.saturating_sub(child.data_start) < 20 {
            continue;
        }
        let entity_count = read_u32_at(data, child.data_start + 8, child.data_end, "altr count")?;
        if entity_count != 2 {
            continue;
        }
        let first = read_u32_at(data, child.data_start + 12, child.data_end, "altr tmap")?;
        let second = read_u32_at(data, child.data_start + 16, child.data_end, "altr primary")?;
        if first == structure.tmap_item_id && second == structure.primary_item_id {
            matched = true;
            break;
        }
    }
    if !matched {
        return Err(invalid(
            "canonical HEIF output has no altr(tmap, primary) entity group",
        ));
    }
    Ok(())
}

/// Construct the canonical portable ISO Gain Map HEIF and normalize the
/// consumer-facing graph without moving product policy into Apple frameworks.
/// Rust remains the owner of the final container; ImageIO is only a downstream
/// consumer used by the macOS conformance gate.
pub fn assemble_iso_gain_map_heif(
    source: &[u8],
    assembly: &IsoGainMapAssembly<'_>,
) -> Result<Vec<u8>> {
    let output = native::assemble_iso_gain_map_heif(source, assembly)?;
    let mut output = normalize_consumer_graph(&output)?;
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

    #[test]
    fn consumer_iref_replaces_gain_auxl_with_primary_and_tmap_targets() {
        let source = IrefBox {
            version: 0,
            entries: vec![IrefEntry {
                kind: AUXL,
                from_item_id: 9,
                to_item_ids: vec![1],
            }],
        };
        let encoded = build_consumer_iref(&source, 1, 9, 10).unwrap();
        let header = parse_boxes(&encoded, 0..encoded.len()).unwrap().remove(0);
        let parsed = parse_iref(&encoded, &header).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].kind, AUXL);
        assert_eq!(parsed.entries[0].from_item_id, 9);
        assert_eq!(parsed.entries[0].to_item_ids, vec![1, 10]);
    }

    #[test]
    fn altr_group_encodes_tmap_before_primary() {
        let encoded = make_altr_entity_group_box(12, 10, 1).unwrap();
        let child = parse_boxes(&encoded, 0..encoded.len()).unwrap().remove(0);
        assert_eq!(child.kind, ALTR);
        assert_eq!(
            read_u32_at(&encoded, child.data_start + 4, child.data_end, "group").unwrap(),
            12
        );
        assert_eq!(
            read_u32_at(&encoded, child.data_start + 8, child.data_end, "count").unwrap(),
            2
        );
        assert_eq!(
            read_u32_at(&encoded, child.data_start + 12, child.data_end, "tmap").unwrap(),
            10
        );
        assert_eq!(
            read_u32_at(&encoded, child.data_start + 16, child.data_end, "primary").unwrap(),
            1
        );
    }
}
