//! Append image-derived Texture resources without re-encoding source items.
use std::collections::{BTreeMap, BTreeSet};

use crate::error::{HeifError, Result};
use xdremux_format::exif::read_item_payload;
use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_ipma_box, make_iref_box,
    parse_boxes, parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry, IlocExtent,
    IpmaAssociation, IpmaEntry, IrefEntry, ParsedMeta, PropertyInfo, FTYP, IDAT, IINF, ILOC, INFE,
    IPCO, IPRP, IREF, MDAT, META,
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

const NATIVE_SKIN_CARRIER: &[u8] = b"urn:com:apple:photo:2019:aux:semanticskinmatte";

fn codec_auxiliary_property(codec: &ParsedMeta, id: u32) -> Result<&PropertyInfo> {
    let owners = codec
        .ipma
        .entries
        .iter()
        .filter(|owner| owner.item_id == id)
        .collect::<Vec<_>>();
    let [owner] = owners.as_slice() else {
        return Err(invalid(
            "codec auxiliary properties are missing or ambiguous",
        ));
    };
    let mut auxiliary = None;
    for association in &owner.associations {
        // Match the existing codec importer: index zero has no property.
        if association.property_index == 0 {
            continue;
        }
        let property = codec
            .properties
            .iter()
            .find(|property| property.index == u32::from(association.property_index))
            .ok_or_else(|| invalid("codec auxiliary property is unresolved"))?;
        if property.kind == AUXC && auxiliary.replace(property).is_some() {
            return Err(invalid("codec auxiliary has multiple auxC properties"));
        }
    }
    auxiliary.ok_or_else(|| invalid("codec auxiliary has no auxC property"))
}

fn native_skin_carrier(data: &[u8], codec: &ParsedMeta) -> Result<(u32, BTreeSet<u32>)> {
    let references = &codec
        .iref
        .as_ref()
        .ok_or_else(|| invalid("mask has no references"))?
        .entries;
    let mut ids = BTreeSet::new();
    for item in &codec.iinf.entries {
        if !ids.insert(item.item_id) {
            return Err(invalid("codec has duplicate item IDs"));
        }
    }
    for edge in references {
        if !ids.contains(&edge.from_item_id) || edge.to_item_ids.iter().any(|id| !ids.contains(id))
        {
            return Err(invalid("codec reference has an unknown item"));
        }
    }
    let mut seen_aux = BTreeSet::new();
    let mut mask = None;
    let mut alpha = None;
    for edge in references.iter().filter(|edge| edge.kind == AUXL) {
        if !seen_aux.insert(edge.from_item_id)
            || edge.from_item_id == codec.primary_item_id
            || edge.to_item_ids != [codec.primary_item_id]
        {
            return Err(invalid(
                "codec auxiliary has duplicate or unsupported primary relationships",
            ));
        }
        let property = codec_auxiliary_property(codec, edge.from_item_id)?;
        let (uri, subtype) = crate::semantic::auxiliary_type_bytes(data, property)?;
        if uri == NATIVE_SKIN_CARRIER {
            if !subtype.is_empty() {
                return Err(invalid("native skin carrier subtype is unsupported"));
            }
            if mask.replace(edge.from_item_id).is_some() {
                return Err(invalid("codec must contain exactly one native skin mask"));
            }
        } else if crate::semantic::is_alpha_type(uri) {
            // This is the disposable codec primary's alpha, not a mask. Its
            // subtype is opaque; none of its metadata or bytes become Texture
            // resources. Product source alpha is preserved independently below.
            if alpha.replace(edge.from_item_id).is_some() {
                return Err(invalid("codec has multiple alpha resources"));
            }
        } else {
            return Err(invalid("codec auxiliary role is unsupported"));
        }
    }
    for owner in &codec.ipma.entries {
        if owner.associations.iter().any(|association| {
            codec.properties.iter().any(|property| {
                property.index == u32::from(association.property_index) && property.kind == AUXC
            })
        }) && !seen_aux.contains(&owner.item_id)
        {
            return Err(invalid("codec auxiliary has no primary relationship"));
        }
    }
    let root = mask.ok_or_else(|| invalid("codec must contain exactly one native skin mask"))?;
    let mut background = BTreeSet::from([codec.primary_item_id]);
    background.extend(crate::semantic::coded_image_items(
        codec,
        codec.primary_item_id,
        "codec primary",
    )?);
    if let Some(alpha) = alpha {
        let coded = crate::semantic::coded_image_items(codec, alpha, "codec alpha")?;
        if background.contains(&alpha) || coded.iter().any(|id| background.contains(id)) {
            return Err(invalid("codec alpha overlaps the primary graph"));
        }
        background.insert(alpha);
        background.extend(coded);
    }
    let mut selected = BTreeSet::from([root]);
    selected.extend(crate::semantic::coded_image_items(
        codec,
        root,
        "native skin mask",
    )?);
    if !selected.is_disjoint(&background) {
        return Err(invalid(
            "native skin mask overlaps a codec background resource",
        ));
    }
    for id in selected.iter().chain(background.iter()) {
        if codec
            .iloc
            .entries
            .iter()
            .filter(|entry| entry.item_id == *id)
            .count()
            != 1
            || codec
                .ipma
                .entries
                .iter()
                .filter(|entry| entry.item_id == *id)
                .count()
                != 1
        {
            return Err(invalid("codec image records are missing or ambiguous"));
        }
    }
    for id in selected.iter().chain(background.iter()) {
        let location = codec
            .iloc
            .entries
            .iter()
            .find(|entry| entry.item_id == *id)
            .ok_or_else(|| invalid("codec image has no location"))?;
        if location.data_reference_index != 0
            || location.construction_method > 1
            || read_item_payload(data, location, codec.idat.as_ref())?.is_empty()
        {
            return Err(invalid("codec image payload must be nonempty and local"));
        }
    }
    for edge in references {
        let from_mask = selected.contains(&edge.from_item_id);
        let to_mask = edge.to_item_ids.iter().any(|id| selected.contains(id));
        if (from_mask && !(edge.kind == DIMG || edge.kind == AUXL && edge.from_item_id == root))
            || (to_mask
                && !(edge.kind == DIMG && from_mask
                    || edge.kind == CDSC
                        && edge.to_item_ids == [root]
                        && codec.iinf.entries.iter().any(|item| {
                            item.item_id == edge.from_item_id && item.item_type == Some(MIME)
                        })))
        {
            return Err(invalid(
                "native skin mask has an unsupported resource relationship",
            ));
        }
    }
    Ok((root, selected))
}

fn codec_graph_error(error: HeifError, data: &[u8], codec: &ParsedMeta) -> HeifError {
    // Temporary codec files disappear after failure. Emit only bounded IDs,
    // fixed labels and subtype lengths; no media, metadata or unknown URIs.
    let roots = codec.iref.as_ref().map_or_else(BTreeSet::new, |iref| {
        iref.entries
            .iter()
            .filter(|edge| edge.kind == AUXL)
            .map(|edge| edge.from_item_id)
            .collect()
    });
    let resources = codec
        .iinf
        .entries
        .iter()
        .filter(|item| roots.contains(&item.item_id))
        .chain(
            codec
                .iinf
                .entries
                .iter()
                .filter(|item| !roots.contains(&item.item_id)),
        )
        .take(16)
        .map(|item| {
            let kind = match item.item_type {
                Some(kind) if kind == FourCC::new(*b"hvc1") => "hvc1",
                Some(kind) if kind == FourCC::new(*b"grid") => "grid",
                Some(kind) if kind == MIME => "mime",
                _ => "other",
            };
            let role = match codec_auxiliary_property(codec, item.item_id)
                .and_then(|property| crate::semantic::auxiliary_type_bytes(data, property))
            {
                Ok((uri, subtype)) => format!(
                    "{}:subtype_len={}",
                    if uri == NATIVE_SKIN_CARRIER {
                        "skin-carrier"
                    } else if uri == b"urn:mpeg:hevc:2015:auxid:1" {
                        "alpha-hevc"
                    } else if uri == b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha" {
                        "alpha-miaf"
                    } else {
                        "unknown"
                    },
                    subtype.len()
                ),
                Err(_) => "none-or-malformed".to_owned(),
            };
            format!("{}:{kind}:auxC=[{role}]", item.item_id)
        })
        .collect::<Vec<_>>()
        .join(",");
    let references = codec.iref.as_ref().map_or_else(Vec::new, |iref| {
        iref.entries
            .iter()
            .take(32)
            .map(|edge| {
                let kind = if edge.kind == AUXL {
                    "auxl"
                } else if edge.kind == DIMG {
                    "dimg"
                } else if edge.kind == CDSC {
                    "cdsc"
                } else {
                    "other"
                };
                format!(
                    "{kind}:{}->{:?}{}",
                    edge.from_item_id,
                    edge.to_item_ids.iter().take(8).collect::<Vec<_>>(),
                    if edge.to_item_ids.len() > 8 {
                        "+omitted"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
    });
    invalid(format!(
        "{error}; native mask codec={{primary={},items={},resources=[{resources}]{},refs=[{}]{}}}",
        codec.primary_item_id,
        codec.iinf.entries.len(),
        if codec.iinf.entries.len() > 16 {
            ",omitted-resources"
        } else {
            ""
        },
        references.join(","),
        if codec
            .iref
            .as_ref()
            .is_some_and(|iref| iref.entries.len() > 32)
        {
            ",omitted-refs"
        } else {
            ""
        }
    ))
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
        // The native producer encodes each measured L8 mask using ImageIO's
        // SemanticSegmentationSkinMatte carrier (infer.m:218-230). The codec
        // primary can separately carry alpha; only the typed skin graph is
        // imported and assigned its requested Texture role.
        let (root, selected) = native_skin_carrier(mask.codec, &codec)
            .map_err(|error| codec_graph_error(error, mask.codec, &codec))?;
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

    const SKIN: &[u8] = b"urn:com:apple:photo:2019:aux:semanticskinmatte\0";
    const ALPHA: &[u8] = b"urn:mpeg:hevc:2015:auxid:1\0";
    const ISO_GAIN: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";
    const PERSON: &str = "tag:apple.com,2026:photo:aux:semanticpersonmatte";

    fn fixture(aux: bool) -> Vec<u8> {
        fixture_roles(if aux { &[(2, SKIN)] } else { &[] }, None, &[])
    }

    fn fixture_roles(
        roles: &[(u32, &[u8])],
        grid_root: Option<u32>,
        extra: &[IrefEntry],
    ) -> Vec<u8> {
        let hvc1 = FourCC::new(*b"hvc1");
        let grid = FourCC::new(*b"grid");
        let mut items = vec![(
            1,
            if grid_root == Some(1) { grid } else { hvc1 },
            b"base".to_vec(),
        )];
        let mut properties = vec![
            make_box(
                FourCC::new(*b"hvcC"),
                &[1, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .unwrap(),
            xdremux_format::isobmff::make_ispe_box(4, 2).unwrap(),
            make_full_box(FourCC::new(*b"pixi"), 0, 0, &[1, 8]).unwrap(),
        ];
        let coded = || {
            [1, 2, 3]
                .into_iter()
                .map(|property_index| IpmaAssociation {
                    property_index,
                    essential: true,
                })
                .collect::<Vec<_>>()
        };
        let mut owners = vec![IpmaEntry {
            item_id: 1,
            associations: coded(),
        }];
        let mut refs = Vec::new();
        for (id, uri) in roles {
            let payload = if *uri == SKIN {
                b"mask".as_slice()
            } else if *uri == ISO_GAIN {
                b"gain".as_slice()
            } else {
                b"alpha".as_slice()
            };
            items.push((
                *id,
                if grid_root == Some(*id) { grid } else { hvc1 },
                payload.to_vec(),
            ));
            properties.push(make_full_box(AUXC, 0, 0, uri).unwrap());
            let mut associations = coded();
            if *uri == SKIN {
                associations.insert(
                    0,
                    IpmaAssociation {
                        property_index: 0,
                        essential: false,
                    },
                );
            }
            associations.push(IpmaAssociation {
                property_index: u16::try_from(properties.len()).unwrap(),
                essential: true,
            });
            owners.push(IpmaEntry {
                item_id: *id,
                associations,
            });
            let targets = if *uri == ISO_GAIN {
                items.push((50, FourCC::new(*b"tmap"), b"tone-map".to_vec()));
                owners.push(IpmaEntry {
                    item_id: 50,
                    associations: [2, 3]
                        .into_iter()
                        .map(|property_index| IpmaAssociation {
                            property_index,
                            essential: true,
                        })
                        .collect(),
                });
                refs.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: 50,
                    to_item_ids: vec![1, *id],
                });
                vec![1, 50]
            } else {
                vec![1]
            };
            refs.push(IrefEntry {
                kind: AUXL,
                from_item_id: *id,
                to_item_ids: targets,
            });
        }
        if let Some(root) = grid_root {
            items.iter_mut().find(|(id, _, _)| *id == root).unwrap().2 =
                vec![0, 0, 0, 1, 0, 4, 0, 2];
            for (id, payload) in [
                (100 + root, b"first-coded-tile".as_slice()),
                (200 + root, b"second-coded-tile"),
            ] {
                items.push((id, hvc1, payload.to_vec()));
                owners.push(IpmaEntry {
                    item_id: id,
                    associations: coded(),
                });
            }
            if let Some(edge) = extra
                .iter()
                .find(|edge| edge.kind == DIMG && edge.from_item_id == root)
            {
                items.iter_mut().find(|(id, _, _)| *id == root).unwrap().2[3] =
                    u8::try_from(edge.to_item_ids.len() - 1).unwrap();
            } else {
                refs.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: root,
                    to_item_ids: vec![100 + root, 200 + root],
                });
            }
        }
        refs.extend_from_slice(extra);
        let mut data = Vec::new();
        let mut infos = Vec::new();
        let mut locations = Vec::new();
        for (id, kind, payload) in items {
            infos.push(make_infe_box(id, kind, u32::from(id != 1)).unwrap());
            locations.push(IlocEntry {
                item_id: id,
                construction_method: 1,
                data_reference_index: 0,
                base_offset: 0,
                extents: vec![IlocExtent {
                    index: None,
                    offset: data.len() as u64,
                    length: payload.len() as u64,
                }],
            });
            data.extend(payload);
        }
        let iprp = make_box(
            IPRP,
            &[
                make_box(IPCO, &properties.concat()).unwrap(),
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
            make_box(MDAT, &[]).unwrap(),
        ]
        .concat()
    }

    #[test]
    fn native_skin_carrier_is_selected_among_alpha_without_changing_source_hdr_resources() {
        let source = fixture_roles(&[(2, ALPHA), (3, ISO_GAIN)], None, &[]);
        let hdr_before = crate::validate_gain_map_structure(&source).unwrap();
        for (grid, alpha) in [
            (None, ALPHA),
            (Some(1), ALPHA),
            (Some(3), ALPHA),
            (
                None,
                b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha\0".as_slice(),
            ),
            (
                None,
                b"urn:mpeg:hevc:2015:auxid:1\0opaque-codec-alpha".as_slice(),
            ),
        ] {
            let codec = fixture_roles(&[(2, alpha), (3, SKIN)], grid, &[]);
            let masks = [TextureStyleMask {
                auxiliary_type: PERSON,
                codec: &codec,
                instance_reference_key: None,
            }];
            let output =
                assemble_texture_styles_heif(&source, b"bplist00measured", &masks).unwrap();
            assert_eq!(
                crate::validate_gain_map_structure(&output).unwrap(),
                hdr_before
            );
            let (_, _, original) = meta(&source).unwrap();
            let (_, _, actual) = meta(&output).unwrap();
            for location in &original.iloc.entries {
                let after = actual
                    .iloc
                    .entries
                    .iter()
                    .find(|item| item.item_id == location.item_id)
                    .unwrap();
                assert_eq!(
                    read_item_payload(&source, location, original.idat.as_ref()).unwrap(),
                    read_item_payload(&output, after, actual.idat.as_ref()).unwrap()
                );
            }
            let original_refs = &original.iref.as_ref().unwrap().entries;
            assert_eq!(
                original_refs,
                &actual.iref.as_ref().unwrap().entries[..original_refs.len()]
            );
            for owner in &original.ipma.entries {
                let read = |data: &[u8], graph: &ParsedMeta, id: u32| {
                    graph
                        .ipma
                        .entries
                        .iter()
                        .find(|item| item.item_id == id)
                        .unwrap()
                        .associations
                        .iter()
                        .map(|a| {
                            (
                                a.essential,
                                data[graph
                                    .properties
                                    .iter()
                                    .find(|p| p.index == u32::from(a.property_index))
                                    .unwrap()
                                    .box_range
                                    .clone()]
                                .to_vec(),
                            )
                        })
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    read(&source, &original, owner.item_id),
                    read(&output, &actual, owner.item_id)
                );
            }
            let root = actual
                .iref
                .as_ref()
                .unwrap()
                .entries
                .iter()
                .find(|edge| edge.kind == AUXL && edge.from_item_id > 50)
                .unwrap()
                .from_item_id;
            let location = actual
                .iloc
                .entries
                .iter()
                .find(|item| item.item_id == root)
                .unwrap();
            assert_eq!(
                read_item_payload(&output, location, actual.idat.as_ref()).unwrap(),
                if grid == Some(3) {
                    vec![0, 0, 0, 1, 0, 4, 0, 2]
                } else {
                    b"mask".to_vec()
                }
            );
            if grid == Some(3) {
                let edge = actual
                    .iref
                    .as_ref()
                    .unwrap()
                    .entries
                    .iter()
                    .find(|edge| edge.kind == DIMG && edge.from_item_id == root)
                    .unwrap();
                for (id, expected) in edge
                    .to_item_ids
                    .iter()
                    .zip([b"first-coded-tile".as_slice(), b"second-coded-tile"])
                {
                    let location = actual
                        .iloc
                        .entries
                        .iter()
                        .find(|item| item.item_id == *id)
                        .unwrap();
                    assert_eq!(
                        read_item_payload(&output, location, actual.idat.as_ref()).unwrap(),
                        expected
                    );
                }
            }
        }
    }

    #[test]
    fn native_skin_carrier_rejects_unknown_ambiguous_or_overlapping_auxiliary_graphs() {
        let source = fixture(false);
        for case in [
            "alpha only",
            "unknown",
            "empty type",
            "unknown subtype",
            "two masks",
            "two alpha",
            "duplicate item",
            "duplicate auxl",
            "foreign primary",
            "direct derived",
            "primary overlap",
            "alpha overlap",
            "unknown reference",
        ] {
            let mut roles = vec![(2, ALPHA), (3, SKIN)];
            let mut extra = Vec::new();
            let mut grid = None;
            match case {
                "alpha only" => {
                    roles.pop();
                }
                "unknown" => roles[0].1 = b"urn:unknown:auxiliary\0",
                "empty type" => roles[1].1 = b"\0",
                "unknown subtype" => {
                    roles[1].1 = b"urn:com:apple:photo:2019:aux:semanticskinmatte\0unknown"
                }
                "two masks" => roles[0].1 = SKIN,
                "two alpha" => roles.push((4, ALPHA)),
                "duplicate item" => roles.push((2, ALPHA)),
                "duplicate auxl" => extra.push(IrefEntry {
                    kind: AUXL,
                    from_item_id: 3,
                    to_item_ids: vec![1],
                }),
                "foreign primary" => extra.push(IrefEntry {
                    kind: AUXL,
                    from_item_id: 3,
                    to_item_ids: vec![2],
                }),
                "direct derived" => extra.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: 3,
                    to_item_ids: vec![2],
                }),
                "primary overlap" | "alpha overlap" => {
                    grid = Some(3);
                    extra.push(IrefEntry {
                        kind: DIMG,
                        from_item_id: 3,
                        to_item_ids: vec![if case == "primary overlap" { 1 } else { 2 }],
                    });
                }
                "unknown reference" => extra.push(IrefEntry {
                    kind: FourCC::new(*b"thmb"),
                    from_item_id: 3,
                    to_item_ids: vec![999],
                }),
                _ => unreachable!(),
            }
            let codec = fixture_roles(&roles, grid, &extra);
            let mask = TextureStyleMask {
                auxiliary_type: PERSON,
                codec: &codec,
                instance_reference_key: None,
            };
            let expected = match case {
                "alpha only" | "two masks" => "exactly one native skin mask",
                "unknown" | "empty type" => "auxiliary role is unsupported",
                "unknown subtype" => "skin carrier subtype is unsupported",
                "two alpha" => "multiple alpha resources",
                "duplicate item" => "duplicate item IDs",
                "duplicate auxl" | "foreign primary" => "unsupported primary relationships",
                "direct derived" => "has a dimg reference",
                "primary overlap" | "alpha overlap" => "overlaps a codec background resource",
                "unknown reference" => "reference has an unknown item",
                _ => unreachable!(),
            };
            let error = assemble_texture_styles_heif(&source, b"bplist00measured", &[mask])
                .err()
                .unwrap_or_else(|| panic!("accepted {case}"))
                .to_string();
            assert!(error.contains(expected), "{case}: {error}");
        }
    }

    #[test]
    fn native_skin_carrier_rejects_malformed_or_ambiguous_auxc_properties() {
        let source = fixture(false);
        for case in [
            "version",
            "flags",
            "unterminated",
            "missing auxC",
            "duplicate auxC",
        ] {
            let mut codec = fixture_roles(&[(2, ALPHA), (3, SKIN)], None, &[]);
            let (_, header, mut parsed) = meta(&codec).unwrap();
            let property = codec_auxiliary_property(&parsed, 3).unwrap().clone();
            match case {
                "version" => codec[property.box_range.start + 8] = 1,
                "flags" => codec[property.box_range.start + 11] = 1,
                "unterminated" => codec[property.box_range.end - 1] = b'X',
                "missing auxC" => codec[property.box_range.start + 4..property.box_range.start + 8]
                    .copy_from_slice(b"free"),
                "duplicate auxC" => {
                    let owner = parsed
                        .ipma
                        .entries
                        .iter_mut()
                        .find(|owner| owner.item_id == 3)
                        .unwrap();
                    owner.associations[0].property_index = u16::try_from(property.index).unwrap();
                    let replacement = make_ipma_box(0, 0, &parsed.ipma.entries).unwrap();
                    let iprp = parse_boxes(&codec, header.data_start + 4..header.data_end)
                        .unwrap()
                        .into_iter()
                        .find(|child| child.kind == IPRP)
                        .unwrap();
                    let ipma = parse_boxes(&codec, iprp.payload_range())
                        .unwrap()
                        .into_iter()
                        .find(|child| child.kind == xdremux_format::isobmff::IPMA)
                        .unwrap();
                    assert_eq!(replacement.len(), ipma.size);
                    codec[ipma.box_range()].copy_from_slice(&replacement);
                }
                _ => unreachable!(),
            }
            let mask = TextureStyleMask {
                auxiliary_type: PERSON,
                codec: &codec,
                instance_reference_key: None,
            };
            let error = assemble_texture_styles_heif(&source, b"bplist00measured", &[mask])
                .err()
                .unwrap_or_else(|| panic!("accepted {case}"))
                .to_string();
            let expected = match case {
                "version" | "flags" => "invalid full-box header",
                "unterminated" => "no terminated type",
                "missing auxC" => "has no auxC property",
                _ => "multiple auxC properties",
            };
            assert!(error.contains(expected), "{case}: {error}");
        }
    }

    #[test]
    fn native_skin_carrier_failure_reports_bounded_roles_and_references_without_opaque_bytes() {
        let source = fixture(false);
        let mut roles = vec![
            (2, b"SECRET_UNKNOWN_URI\0SECRET_SUBTYPE".as_slice()),
            (3, SKIN),
        ];
        roles.extend((4..35).map(|id| (id, ALPHA)));
        let extra = (0..40)
            .map(|_| IrefEntry {
                kind: FourCC::new(*b"thmb"),
                from_item_id: 2,
                to_item_ids: (1..16).collect(),
            })
            .collect::<Vec<_>>();
        let codec = fixture_roles(&roles, None, &extra);
        let mask = TextureStyleMask {
            auxiliary_type: PERSON,
            codec: &codec,
            instance_reference_key: None,
        };
        let error = assemble_texture_styles_heif(&source, b"bplist00SECRET_PLIST", &[mask])
            .err()
            .unwrap()
            .to_string();
        for expected in [
            "native mask codec={primary=1",
            "2:hvc1:auxC=[unknown:subtype_len=14]",
            "3:hvc1:auxC=[skin-carrier:subtype_len=0]",
            "4:hvc1:auxC=[alpha-hevc:subtype_len=0]",
            "auxl:2->[1]",
            "omitted-resources",
            "omitted-refs",
        ] {
            assert!(error.contains(expected), "{error}");
        }
        assert!(!error.contains("SECRET"), "{error}");
        assert!(error.len() < 8192, "diagnostic has {} bytes", error.len());
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
