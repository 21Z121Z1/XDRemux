//! Append image-derived Texture resources without re-encoding source items.
use std::collections::{BTreeMap, BTreeSet};

use crate::error::{HeifError, Result};
use xdremux_format::exif::read_item_payload;
use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_ipma_box, make_iref_box,
    parse_boxes, parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry, IlocExtent,
    IpmaAssociation, IpmaEntry, IrefEntry, ParsedMeta, FTYP, IDAT, IINF, ILOC, INFE, IPCO, IPRP,
    IREF, MDAT, META,
};
use xdremux_format::{Cursor, Endian, FourCC};

pub const TEXTURE_METADATA_URI: &str = "tag:apple.com,2026:photo:metadata:texture_styles";
const AUXC: FourCC = FourCC::new(*b"auxC");
const AUXL: FourCC = FourCC::new(*b"auxl");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const DIMG: FourCC = FourCC::new(*b"dimg");
const URI: FourCC = FourCC::new(*b"uri ");
const MIME: FourCC = FourCC::new(*b"mime");

/// A separately encoded native mask. The codec's primary image is not imported.
#[derive(Debug, Clone, Copy)]
pub struct TextureStyleMask<'a> {
    pub auxiliary_type: &'a str,
    pub codec: &'a [u8],
    pub instance_reference_key: Option<&'a str>,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn meta(data: &[u8]) -> Result<(Vec<BoxHeader>, BoxHeader, ParsedMeta)> {
    let scan = scan_top_level_boxes(data)?;
    if !scan.trailing_range.is_empty() {
        return Err(invalid("Texture source has trailing bytes"));
    }
    let headers: Vec<_> = scan
        .boxes
        .iter()
        .filter(|b| b.kind == META)
        .cloned()
        .collect();
    if headers.len() != 1 || !scan.boxes.iter().any(|b| b.kind == FTYP) {
        return Err(invalid("Texture source requires one meta and an ftyp"));
    }
    let parsed = parse_meta_box(data, &headers[0])?;
    Ok((scan.boxes, headers[0].clone(), parsed))
}

fn item_uri<'a>(
    data: &'a [u8],
    info: &xdremux_format::isobmff::ItemInfo,
) -> Result<Option<&'a [u8]>> {
    if info.item_type != Some(URI) {
        return Ok(None);
    }
    let header = parse_boxes(data, info.box_range.clone())?.remove(0);
    let mut cursor = Cursor::bounded(data, header.payload_range(), Endian::Big, "Texture infe")?;
    let version = cursor.read_u8()?;
    cursor.skip(3)?;
    cursor.skip(match version {
        2 => 2,
        3 => 4,
        _ => return Err(invalid("unsupported URI infe")),
    })?;
    cursor.skip(6)?;
    let _ = cursor.read_c_string()?;
    Ok(Some(cursor.read_c_string()?))
}

pub fn has_texture_style_metadata(data: &[u8]) -> Result<bool> {
    let (_, _, parsed) = meta(data)?;
    for item in &parsed.iinf.entries {
        if item_uri(data, item)? == Some(TEXTURE_METADATA_URI.as_bytes()) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn metadata_info(id: u32, kind: FourCC, tail: &[u8]) -> Result<Vec<u8>> {
    let mut data = id.to_be_bytes().to_vec();
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(kind.as_bytes());
    data.push(0);
    data.extend_from_slice(tail);
    Ok(make_full_box(INFE, 3, 1, &data)?)
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Copy every source item payload exactly, preserve all original references,
/// and add a distinct FSINC descriptor for each actual native mask.
pub fn assemble_texture_styles_heif(
    source: &[u8],
    plist: &[u8],
    masks: &[TextureStyleMask<'_>],
) -> Result<Vec<u8>> {
    if !plist.starts_with(b"bplist00") || has_texture_style_metadata(source)? {
        return Err(invalid(
            "Texture assembly needs a new binary plist and no existing Texture metadata",
        ));
    }
    let (top, header, parsed) = meta(source)?;
    let mut infos = BTreeMap::new();
    let mut payloads = BTreeMap::new();
    let mut properties: Vec<Vec<u8>> = parsed
        .properties
        .iter()
        .map(|p| source[p.box_range.clone()].to_vec())
        .collect();
    let mut associations: BTreeMap<u32, Vec<IpmaAssociation>> = parsed
        .ipma
        .entries
        .iter()
        .map(|e| (e.item_id, e.associations.clone()))
        .collect();
    let mut refs = parsed
        .iref
        .as_ref()
        .map_or_else(Vec::new, |r| r.entries.clone());
    for info in &parsed.iinf.entries {
        if infos
            .insert(info.item_id, source[info.box_range.clone()].to_vec())
            .is_some()
        {
            return Err(invalid("Texture source has duplicate IDs"));
        }
        let location = parsed
            .iloc
            .entries
            .iter()
            .find(|e| e.item_id == info.item_id)
            .ok_or_else(|| invalid("Texture source item has no location"))?;
        if location.data_reference_index != 0 || location.construction_method > 1 {
            return Err(invalid("Texture source item is not local"));
        }
        payloads.insert(
            info.item_id,
            read_item_payload(source, location, parsed.idat.as_ref())?,
        );
    }
    let mut next = infos
        .keys()
        .next_back()
        .copied()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| invalid("Texture item ID overflow"))?;
    let mut seen = BTreeSet::new();
    for mask in masks {
        let instance =
            mask.auxiliary_type == "tag:apple.com,2026:photo:aux:semanticpersoninstances";
        if !(mask
            .auxiliary_type
            .starts_with("tag:apple.com,2026:photo:aux:")
            || mask.auxiliary_type == "urn:com:apple:photo:2019:aux:semantichairmatte")
            || instance != mask.instance_reference_key.is_some()
            || mask.instance_reference_key == Some("")
            || !seen.insert((mask.auxiliary_type, mask.instance_reference_key))
        {
            return Err(invalid("Texture mask has invalid or duplicate role/key"));
        }
        let (_, _, codec) = meta(mask.codec)?;
        let codec_refs = codec
            .iref
            .as_ref()
            .ok_or_else(|| invalid("mask has no references"))?;
        let roots: Vec<_> = codec_refs
            .entries
            .iter()
            .filter(|r| r.kind == AUXL && r.to_item_ids == [codec.primary_item_id])
            .map(|r| r.from_item_id)
            .collect();
        if roots.len() != 1 {
            return Err(invalid(
                "codec must contain exactly one native auxiliary mask",
            ));
        }
        let root = roots[0];
        let mut selected = BTreeSet::from([root]);
        loop {
            let old = selected.len();
            for r in &codec_refs.entries {
                if r.kind == DIMG && selected.contains(&r.from_item_id) {
                    selected.extend(r.to_item_ids.iter().copied());
                }
            }
            if selected.len() == old {
                break;
            }
        }
        let mut mapping = BTreeMap::new();
        for old in &selected {
            mapping.insert(*old, next);
            next = next
                .checked_add(1)
                .ok_or_else(|| invalid("Texture ID overflow"))?;
        }
        for old in &selected {
            let new = mapping[old];
            let info = codec
                .iinf
                .entries
                .iter()
                .find(|i| i.item_id == *old)
                .ok_or_else(|| invalid("mask dependency missing"))?;
            let raw_header = parse_boxes(mask.codec, info.box_range.clone())?.remove(0);
            let mut raw = mask.codec[raw_header.payload_range()].to_vec();
            let version = raw[0];
            let width = match version {
                2 => 2,
                3 => 4,
                _ => return Err(invalid("unsupported mask infe")),
            };
            // Rebuild the ID width while retaining item name and type-specific fields.
            raw.drain(0..4 + width);
            let mut body = new.to_be_bytes().to_vec();
            body.extend(raw);
            infos.insert(new, make_full_box(INFE, 3, 1, &body)?);
            let loc = codec
                .iloc
                .entries
                .iter()
                .find(|e| e.item_id == *old)
                .ok_or_else(|| invalid("mask location missing"))?;
            if loc.data_reference_index != 0 || loc.construction_method > 1 {
                return Err(invalid("mask is not local"));
            }
            payloads.insert(
                new,
                read_item_payload(mask.codec, loc, codec.idat.as_ref())?,
            );
            let owner = codec
                .ipma
                .entries
                .iter()
                .find(|e| e.item_id == *old)
                .ok_or_else(|| invalid("mask properties missing"))?;
            let mut values = Vec::new();
            for a in &owner.associations {
                if a.property_index == 0 {
                    continue;
                }
                let p = codec
                    .properties
                    .iter()
                    .find(|p| p.index == u32::from(a.property_index))
                    .ok_or_else(|| invalid("mask property missing"))?;
                let bytes = if p.kind == AUXC {
                    let mut body = vec![0; 4];
                    body.extend_from_slice(mask.auxiliary_type.as_bytes());
                    body.push(0);
                    make_box(AUXC, &body)?
                } else {
                    mask.codec[p.box_range.clone()].to_vec()
                };
                properties.push(bytes);
                values.push(IpmaAssociation {
                    property_index: u16::try_from(properties.len())
                        .map_err(|_| invalid("too many Texture properties"))?,
                    essential: a.essential,
                });
            }
            associations.insert(new, values);
        }
        for r in &codec_refs.entries {
            if r.kind == DIMG && selected.contains(&r.from_item_id) {
                let targets = r
                    .to_item_ids
                    .iter()
                    .map(|id| {
                        mapping
                            .get(id)
                            .copied()
                            .ok_or_else(|| invalid("dangling mask dependency"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                refs.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: mapping[&r.from_item_id],
                    to_item_ids: targets,
                });
            }
        }
        let aux = mapping[&root];
        refs.push(IrefEntry {
            kind: AUXL,
            from_item_id: aux,
            to_item_ids: vec![parsed.primary_item_id],
        });
        let key = mask.instance_reference_key.map(|k| format!("<fsincMattes:InstanceMaskReferenceKey>{}</fsincMattes:InstanceMaskReferenceKey>", xml(k))).unwrap_or_default();
        let xmp = if mask.auxiliary_type.starts_with("tag:apple.com,2026:") {
            format!("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:fsincMattes=\"http://ns.apple.com/fsinc/1.0/\"><fsincMattes:FSINCMatteVersion>0</fsincMattes:FSINCMatteVersion>{key}</rdf:Description></rdf:RDF></x:xmpmeta>").into_bytes()
        } else {
            // Hair keeps the native 2019 semantic metadata and its actual
            // version. It is shared by the two editing schemas.
            let ids: Vec<_> = codec_refs
                .entries
                .iter()
                .filter(|r| r.kind == CDSC && r.to_item_ids.contains(&root))
                .map(|r| r.from_item_id)
                .collect();
            if ids.len() != 1 {
                return Err(invalid("native hair mask needs one metadata descriptor"));
            }
            let location = codec
                .iloc
                .entries
                .iter()
                .find(|e| e.item_id == ids[0])
                .ok_or_else(|| invalid("native hair metadata location missing"))?;
            read_item_payload(mask.codec, location, codec.idat.as_ref())?
        };
        infos.insert(next, metadata_info(next, MIME, b"application/rdf+xml\0\0")?);
        payloads.insert(next, xmp);
        refs.push(IrefEntry {
            kind: CDSC,
            from_item_id: next,
            to_item_ids: vec![aux],
        });
        next = next
            .checked_add(1)
            .ok_or_else(|| invalid("Texture ID overflow"))?;
    }
    let mut uri = TEXTURE_METADATA_URI.as_bytes().to_vec();
    uri.push(0);
    infos.insert(next, metadata_info(next, URI, &uri)?);
    payloads.insert(next, plist.to_vec());
    refs.push(IrefEntry {
        kind: CDSC,
        from_item_id: next,
        to_item_ids: vec![parsed.primary_item_id],
    });
    // Drop unreferenced descriptors and deduplicate exact property bytes. Native
    // HEIF decoders have a practical property-count limit well below 15 bits.
    let used: BTreeSet<_> = associations
        .values()
        .flatten()
        .map(|a| usize::from(a.property_index))
        .filter(|i| *i != 0)
        .collect();
    let mut compact = Vec::new();
    let mut index = BTreeMap::from([(0, 0)]);
    for old in used {
        let p = properties
            .get(old - 1)
            .ok_or_else(|| invalid("invalid source property index"))?;
        let pos = compact.iter().position(|v| v == p).unwrap_or_else(|| {
            compact.push(p.clone());
            compact.len() - 1
        });
        index.insert(
            old,
            u16::try_from(pos + 1).map_err(|_| invalid("property index overflow"))?,
        );
    }
    if compact.len() > 100 {
        return Err(invalid(
            "Texture output exceeds supported native property count",
        ));
    }
    let owners: Vec<_> = associations
        .into_iter()
        .map(|(id, a)| IpmaEntry {
            item_id: id,
            associations: a
                .into_iter()
                .map(|a| IpmaAssociation {
                    property_index: index[&usize::from(a.property_index)],
                    essential: a.essential,
                })
                .collect(),
        })
        .collect();
    let iprp = make_box(
        IPRP,
        &[
            make_box(IPCO, &compact.concat())?,
            make_ipma_box(1, 1, &owners)?,
        ]
        .concat(),
    )?;
    let iinf = make_iinf_box(1, &infos.into_values().collect::<Vec<_>>())?;
    let iref = make_iref_box(1, &refs)?;
    let mut locations = Vec::new();
    let mut media = Vec::new();
    for (id, payload) in payloads {
        locations.push(IlocEntry {
            item_id: id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: media.len() as u64,
                length: payload.len() as u64,
            }],
        });
        media.extend(payload);
    }
    let children = parse_boxes(source, header.data_start + 4..header.data_end)?;
    let build_meta = |locations: &[IlocEntry]| -> Result<Vec<u8>> {
        let iloc = make_iloc_box(2, 8, 8, 0, 0, locations)?;
        let mut body = source[header.data_start..header.data_start + 4].to_vec();
        for child in &children {
            let replacement = match child.kind {
                IINF => &iinf,
                ILOC => &iloc,
                IPRP => &iprp,
                IREF => &iref,
                IDAT => continue,
                _ => {
                    body.extend_from_slice(&source[child.box_range()]);
                    continue;
                }
            };
            body.extend_from_slice(replacement);
        }
        if !children.iter().any(|c| c.kind == IREF) {
            body.extend_from_slice(&iref);
        }
        Ok(make_box(META, &body)?)
    };
    let provisional = build_meta(&locations)?;
    let prefix_size: usize = top
        .iter()
        .filter(|b| b.kind != MDAT)
        .map(|b| {
            if b.kind == META {
                provisional.len()
            } else {
                b.size
            }
        })
        .sum();
    let mdat_header = if media.len() as u64 + 8 > u64::from(u32::MAX) {
        16
    } else {
        8
    };
    for location in &mut locations {
        location.extents[0].offset = location.extents[0]
            .offset
            .checked_add((prefix_size + mdat_header) as u64)
            .ok_or_else(|| invalid("Texture offset overflow"))?;
    }
    let final_meta = build_meta(&locations)?;
    if final_meta.len() != provisional.len() {
        return Err(invalid("unstable Texture meta size"));
    }
    let mut output = Vec::new();
    for b in top {
        if b.kind == META {
            output.extend_from_slice(&final_meta);
        } else if b.kind != MDAT {
            output.extend_from_slice(&source[b.box_range()]);
        }
    }
    output.extend(make_box(MDAT, &media)?);
    // Independently prove that every original payload survived the relocation.
    let (_, _, result) = meta(&output)?;
    for original in &parsed.iloc.entries {
        let location = result
            .iloc
            .entries
            .iter()
            .find(|e| e.item_id == original.item_id)
            .ok_or_else(|| invalid("Texture output lost source item"))?;
        if read_item_payload(source, original, parsed.idat.as_ref())?
            != read_item_payload(&output, location, result.idat.as_ref())?
        {
            return Err(invalid("Texture assembly changed an original payload"));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::{make_infe_box, make_pitm_box};

    fn fixture(aux: bool) -> Vec<u8> {
        let mut infos = vec![make_infe_box(1, FourCC::new(*b"hvc1"), 0).unwrap()];
        let mut locations = vec![IlocEntry {
            item_id: 1,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: 4,
            }],
        }];
        let mut data = b"base".to_vec();
        let mut properties = Vec::new();
        let mut owners = Vec::new();
        let mut refs = Vec::new();
        if aux {
            infos.push(make_infe_box(2, FourCC::new(*b"hvc1"), 1).unwrap());
            locations.push(IlocEntry {
                item_id: 2,
                construction_method: 1,
                data_reference_index: 0,
                base_offset: 0,
                extents: vec![IlocExtent {
                    index: None,
                    offset: 4,
                    length: 4,
                }],
            });
            data.extend_from_slice(b"mask");
            properties = make_full_box(
                AUXC,
                0,
                0,
                b"urn:com:apple:photo:2019:aux:semanticskinmatte\0",
            )
            .unwrap();
            owners.push(IpmaEntry {
                item_id: 2,
                associations: vec![IpmaAssociation {
                    property_index: 1,
                    essential: true,
                }],
            });
            refs.push(IrefEntry {
                kind: AUXL,
                from_item_id: 2,
                to_item_ids: vec![1],
            });
        }
        let iprp = make_box(
            IPRP,
            &[
                make_box(IPCO, &properties).unwrap(),
                make_ipma_box(0, 0, &owners).unwrap(),
            ]
            .concat(),
        )
        .unwrap();
        let children = [
            make_pitm_box(0, 1).unwrap(),
            make_iinf_box(0, &infos).unwrap(),
            make_iloc_box(1, 4, 4, 0, 0, &locations).unwrap(),
            iprp,
            make_iref_box(0, &refs).unwrap(),
            make_box(IDAT, &data).unwrap(),
        ]
        .concat();
        [
            make_box(FTYP, b"mif1\0\0\0\0mif1heic").unwrap(),
            make_full_box(META, 0, 0, &children).unwrap(),
        ]
        .concat()
    }

    #[test]
    fn appends_measured_masks_without_changing_original_payloads() {
        let source = fixture(false);
        let codec = fixture(true);
        assert!(!has_texture_style_metadata(&source).unwrap());
        let masks = [
            TextureStyleMask {
                auxiliary_type: "tag:apple.com,2026:photo:aux:semanticpersoninstances",
                codec: &codec,
                instance_reference_key: Some("face&1"),
            },
            TextureStyleMask {
                auxiliary_type: "tag:apple.com,2026:photo:aux:semanticpersoninstances",
                codec: &codec,
                instance_reference_key: Some("face10"),
            },
        ];
        let output = assemble_texture_styles_heif(&source, b"bplist00measured", &masks).unwrap();
        assert!(has_texture_style_metadata(&output).unwrap());
        let (_, _, parsed) = meta(&output).unwrap();
        let base = &parsed.iloc.entries.iter().find(|e| e.item_id == 1).unwrap();
        assert_eq!(
            read_item_payload(&output, base, parsed.idat.as_ref()).unwrap(),
            b"base"
        );
        assert!(output.windows(10).any(|b| b == b"face&amp;1"));
        assert!(output.windows(6).any(|b| b == b"face10"));
        assert_eq!(
            parsed
                .iref
                .unwrap()
                .entries
                .iter()
                .filter(|r| r.kind == AUXL)
                .count(),
            2
        );
        assert!(assemble_texture_styles_heif(&output, b"bplist00new", &masks).is_err());
    }

    #[test]
    fn a_role_label_cannot_replace_a_missing_native_codec_or_distinct_instance() {
        let source = fixture(false);
        let codec = fixture(true);
        let missing = TextureStyleMask {
            auxiliary_type: "tag:apple.com,2026:photo:aux:semanticpersonmatte",
            codec: &source,
            instance_reference_key: None,
        };
        assert!(assemble_texture_styles_heif(&source, b"bplist00new", &[missing]).is_err());
        let instance = TextureStyleMask {
            auxiliary_type: "tag:apple.com,2026:photo:aux:semanticpersoninstances",
            codec: &codec,
            instance_reference_key: Some("one"),
        };
        assert!(
            assemble_texture_styles_heif(&source, b"bplist00new", &[instance, instance]).is_err()
        );
    }
}
