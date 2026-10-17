//! Read embedded Styles resources without changing their payloads or the source.
use std::collections::BTreeSet;
use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
    make_iref_box, make_pitm_box, parse_boxes, parse_meta_box, IlocEntry, IlocExtent, ParsedMeta,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const STYLE_URI: &[u8] = b"tag:apple.com,2023:photo:metadata:styles\0";
const LINEAR_URI: &[u8] = b"tag:apple.com,2023:photo:aux:linearthumbnail\0";
const DELTA_URI: &[u8] = b"tag:apple.com,2023:photo:aux:styledeltamap\0";

#[derive(Debug)]
pub struct StyleReconstructionResources {
    pub metadata: Vec<u8>,
    /// Standalone auxiliary image graph, with original coded bytes and colour
    /// properties. Other images and alternative groups cannot override pitm.
    pub linear_thumbnail_heif: Vec<u8>,
    pub delta_map_heif: Vec<u8>,
    pub metadata_item_id: u32,
    pub linear_thumbnail_item_id: u32,
    pub delta_map_item_id: u32,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn unique(mut ids: impl Iterator<Item = u32>, context: &str) -> Result<u32> {
    let id = ids
        .next()
        .ok_or_else(|| invalid(format!("missing {context}")))?;
    if ids.next().is_some() {
        return Err(invalid(format!("ambiguous {context}")));
    }
    Ok(id)
}

fn linked(meta: &ParsedMeta, id: u32, kind: &[u8; 4]) -> bool {
    meta.iref.as_ref().is_some_and(|refs| {
        refs.entries.iter().any(|r| {
            r.kind == FourCC::new(*kind)
                && r.from_item_id == id
                && r.to_item_ids.contains(&meta.primary_item_id)
        })
    })
}

fn auxiliary(source: &[u8], meta: &ParsedMeta, uri: &[u8]) -> Result<u32> {
    unique(
        meta.ipma.entries.iter().filter_map(|entry| {
            let matches = entry.associations.iter().any(|a| {
                meta.properties.iter().any(|p| {
                    p.index == u32::from(a.property_index)
                        && p.kind == FourCC::new(*b"auxC")
                        && source[p.box_range.clone()].ends_with(uri)
                })
            });
            (matches && linked(meta, entry.item_id, b"auxl")).then_some(entry.item_id)
        }),
        std::str::from_utf8(uri).unwrap_or("Styles auxiliary image"),
    )
}

fn image_ids(meta: &ParsedMeta, root: u32) -> BTreeSet<u32> {
    let mut ids = BTreeSet::from([root]);
    loop {
        let before = ids.len();
        if let Some(refs) = &meta.iref {
            for r in &refs.entries {
                if r.kind == FourCC::new(*b"dimg") && ids.contains(&r.from_item_id) {
                    ids.extend(&r.to_item_ids);
                }
            }
        }
        if before == ids.len() {
            return ids;
        }
    }
}

fn standalone_auxiliary(source: &[u8], meta: &ParsedMeta, id: u32, hdlr: &[u8]) -> Result<Vec<u8>> {
    let ids = image_ids(meta, id);
    let mut mdat = Vec::new();
    let mut entries = Vec::new();
    let mut infos = Vec::new();
    for item_id in &ids {
        let item = meta
            .iinf
            .entries
            .iter()
            .find(|i| i.item_id == *item_id)
            .ok_or_else(|| invalid("auxiliary references a missing image"))?;
        let location = meta
            .iloc
            .entries
            .iter()
            .find(|i| i.item_id == *item_id)
            .ok_or_else(|| invalid("auxiliary image has no location"))?;
        let bytes = crate::validation::item_payload(source, meta.idat.as_ref(), location)?;
        entries.push(IlocEntry {
            item_id: *item_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: mdat.len() as u64,
                length: bytes.len() as u64,
            }],
        });
        mdat.extend(bytes);
        infos.push(make_infe_box(
            *item_id,
            item.item_type
                .ok_or_else(|| invalid("unknown auxiliary item type"))?,
            u32::from(*item_id != id),
        )?);
    }
    let mut associations = meta
        .ipma
        .entries
        .iter()
        .filter(|e| ids.contains(&e.item_id))
        .cloned()
        .collect::<Vec<_>>();
    for entry in &mut associations {
        entry.associations.retain(|a| {
            !meta.properties.iter().any(|p| {
                p.index == u32::from(a.property_index)
                    && [*b"auxC", *b"irot", *b"imir"].contains(p.kind.as_bytes())
            })
        });
    }
    let refs = meta
        .iref
        .as_ref()
        .map(|r| {
            r.entries
                .iter()
                .filter(|r| r.kind == FourCC::new(*b"dimg") && ids.contains(&r.from_item_id))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let props = meta
        .properties
        .iter()
        .flat_map(|p| source[p.box_range.clone()].iter().copied())
        .collect::<Vec<_>>();
    let mut iprp = make_box(FourCC::new(*b"ipco"), &props)?;
    iprp.extend(make_ipma_box(1, 1, &associations)?);
    let iprp = make_box(FourCC::new(*b"iprp"), &iprp)?;
    let ftyp = make_box(FourCC::new(*b"ftyp"), b"heic\0\0\0\0mif1heic")?;
    let build = |locations: &[IlocEntry]| -> Result<Vec<u8>> {
        let mut p = hdlr.to_vec();
        p.extend(make_pitm_box(1, id)?);
        p.extend(make_iloc_box(2, 8, 8, 0, 0, locations)?);
        p.extend(make_iinf_box(1, &infos)?);
        p.extend(&iprp);
        p.extend(make_iref_box(1, &refs)?);
        Ok(make_full_box(FourCC::new(*b"meta"), 0, 0, &p)?)
    };
    let offset = (ftyp.len() + build(&entries)?.len() + 8) as u64;
    for entry in &mut entries {
        entry.extents[0].offset += offset;
    }
    let mut result = ftyp;
    result.extend(build(&entries)?);
    result.extend(make_box(FourCC::new(*b"mdat"), &mdat)?);
    Ok(result)
}

/// Source-independent byte representation of the compressed SDR image tree and
/// its properties. Location offsets and association flags do not affect pixels.
pub fn primary_image_signature(source: &[u8]) -> Result<Vec<u8>> {
    let roots = parse_boxes(source, 0..source.len())?;
    let root = roots
        .iter()
        .find(|r| r.kind == FourCC::new(*b"meta"))
        .ok_or_else(|| invalid("missing meta"))?;
    let meta = parse_meta_box(source, root)?;
    let mut result = Vec::new();
    for id in image_ids(&meta, meta.primary_item_id) {
        result.extend(id.to_be_bytes());
        let e = meta
            .iloc
            .entries
            .iter()
            .find(|e| e.item_id == id)
            .ok_or_else(|| invalid("missing primary image location"))?;
        let data = crate::validation::item_payload(source, meta.idat.as_ref(), e)?;
        result.extend((data.len() as u64).to_be_bytes());
        result.extend(data);
        if let Some(a) = meta.ipma.entries.iter().find(|a| a.item_id == id) {
            for a in &a.associations {
                let p = meta
                    .properties
                    .iter()
                    .find(|p| p.index == u32::from(a.property_index))
                    .ok_or_else(|| invalid("missing primary property"))?;
                result.extend(&source[p.box_range.clone()]);
            }
        }
    }
    Ok(result)
}

pub fn extract_style_reconstruction_resources(
    source: &[u8],
) -> Result<StyleReconstructionResources> {
    let roots = parse_boxes(source, 0..source.len())?;
    let mut metas = roots.iter().filter(|b| b.kind == FourCC::new(*b"meta"));
    let root = metas
        .next()
        .ok_or_else(|| invalid("missing HEIF meta box"))?;
    if metas.next().is_some() {
        return Err(invalid("ambiguous HEIF meta box"));
    }
    let meta = parse_meta_box(source, root)?;
    let metadata_item_id = unique(
        meta.iinf.entries.iter().filter_map(|item| {
            (item.item_type == Some(FourCC::new(*b"uri "))
                && source[item.box_range.clone()].ends_with(STYLE_URI)
                && linked(&meta, item.item_id, b"cdsc"))
            .then_some(item.item_id)
        }),
        "Photographic Styles metadata linked to the primary",
    )?;
    let mut entries = meta
        .iloc
        .entries
        .iter()
        .filter(|e| e.item_id == metadata_item_id);
    let entry = entries
        .next()
        .ok_or_else(|| invalid("Styles metadata has no location"))?;
    if entries.next().is_some() {
        return Err(invalid("duplicate Styles metadata location"));
    }
    let metadata = crate::validation::item_payload(source, meta.idat.as_ref(), entry)?;
    let children = parse_boxes(source, root.data_start + 4..root.data_end)?;
    let hdlr = children
        .iter()
        .find(|b| b.kind == FourCC::new(*b"hdlr"))
        .ok_or_else(|| invalid("missing HEIF handler"))?;
    let linear_thumbnail_item_id = auxiliary(source, &meta, LINEAR_URI)?;
    let delta_map_item_id = auxiliary(source, &meta, DELTA_URI)?;
    Ok(StyleReconstructionResources {
        metadata,
        linear_thumbnail_heif: standalone_auxiliary(
            source,
            &meta,
            linear_thumbnail_item_id,
            &source[hdlr.box_range()],
        )?,
        delta_map_heif: standalone_auxiliary(
            source,
            &meta,
            delta_map_item_id,
            &source[hdlr.box_range()],
        )?,
        metadata_item_id,
        linear_thumbnail_item_id,
        delta_map_item_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::{IpmaAssociation, IpmaEntry, IrefEntry};
    fn fixture() -> Vec<u8> {
        let cc = |s: &[u8; 4]| FourCC::new(*s);
        let mut metadata = 4u16.to_be_bytes().to_vec();
        metadata.extend([0, 0]);
        metadata.extend(b"uri \0");
        metadata.extend(STYLE_URI);
        let infos = vec![
            make_infe_box(1, cc(b"hvc1"), 0).unwrap(),
            make_infe_box(2, cc(b"hvc1"), 1).unwrap(),
            make_infe_box(3, cc(b"hvc1"), 1).unwrap(),
            make_full_box(cc(b"infe"), 2, 1, &metadata).unwrap(),
        ];
        let mut props = xdremux_format::isobmff::make_ispe_box(2, 2).unwrap();
        props.extend(make_full_box(cc(b"auxC"), 0, 0, LINEAR_URI).unwrap());
        props.extend(make_full_box(cc(b"auxC"), 0, 0, DELTA_URI).unwrap());
        let entries = (1..=3)
            .map(|id| IpmaEntry {
                item_id: id,
                associations: if id == 1 {
                    vec![IpmaAssociation {
                        property_index: 1,
                        essential: true,
                    }]
                } else {
                    vec![
                        IpmaAssociation {
                            property_index: 1,
                            essential: true,
                        },
                        IpmaAssociation {
                            property_index: id as u16,
                            essential: true,
                        },
                    ]
                },
            })
            .collect::<Vec<_>>();
        let mut iprp = make_box(cc(b"ipco"), &props).unwrap();
        iprp.extend(make_ipma_box(0, 0, &entries).unwrap());
        let payloads = [
            b"SDR".as_slice(),
            b"Log".as_slice(),
            b"Delta".as_slice(),
            b"bplist00fake".as_slice(),
        ];
        let mut idat = Vec::new();
        let mut locations = Vec::new();
        for (n, p) in payloads.iter().enumerate() {
            locations.push(IlocEntry {
                item_id: n as u32 + 1,
                construction_method: 1,
                data_reference_index: 0,
                base_offset: 0,
                extents: vec![IlocExtent {
                    index: None,
                    offset: idat.len() as u64,
                    length: p.len() as u64,
                }],
            });
            idat.extend(*p);
        }
        let refs = vec![
            IrefEntry {
                kind: cc(b"auxl"),
                from_item_id: 2,
                to_item_ids: vec![1],
            },
            IrefEntry {
                kind: cc(b"auxl"),
                from_item_id: 3,
                to_item_ids: vec![1],
            },
            IrefEntry {
                kind: cc(b"cdsc"),
                from_item_id: 4,
                to_item_ids: vec![1],
            },
        ];
        let mut m =
            make_full_box(cc(b"hdlr"), 0, 0, b"\0\0\0\0pict\0\0\0\0\0\0\0\0\0\0\0\0\0").unwrap();
        m.extend(make_pitm_box(0, 1).unwrap());
        m.extend(make_iinf_box(0, &infos).unwrap());
        m.extend(make_iloc_box(1, 4, 4, 0, 0, &locations).unwrap());
        m.extend(make_box(cc(b"iprp"), &iprp).unwrap());
        m.extend(make_iref_box(0, &refs).unwrap());
        m.extend(make_box(cc(b"idat"), &idat).unwrap());
        let mut result = make_box(cc(b"ftyp"), b"heic\0\0\0\0mif1heic").unwrap();
        result.extend(make_full_box(cc(b"meta"), 0, 0, &m).unwrap());
        result
    }
    #[test]
    fn extracts_closed_auxiliary_graphs_without_selecting_the_original_primary() {
        let source = fixture();
        let before = source.clone();
        let r = extract_style_reconstruction_resources(&source).unwrap();
        assert_eq!(r.metadata, b"bplist00fake");
        assert_eq!(source, before);
        for (data, expected, id) in [
            (&r.linear_thumbnail_heif, b"Log".as_slice(), 2),
            (&r.delta_map_heif, b"Delta".as_slice(), 3),
        ] {
            let boxes = parse_boxes(data, 0..data.len()).unwrap();
            let root = boxes
                .iter()
                .find(|b| b.kind == FourCC::new(*b"meta"))
                .unwrap();
            let m = parse_meta_box(data, root).unwrap();
            assert_eq!(m.primary_item_id, id);
            assert_eq!(m.iinf.entries.len(), 1);
            assert_eq!(
                crate::validation::item_payload(data, m.idat.as_ref(), &m.iloc.entries[0]).unwrap(),
                expected
            );
            assert!(m.ipma.entries[0]
                .associations
                .iter()
                .all(|a| a.property_index == 1));
        }
        assert!(!primary_image_signature(&source).unwrap().is_empty());
    }
    #[test]
    fn rejects_missing_links_and_truncated_resources() {
        let source = fixture();
        for n in [0, 4, 20, source.len() - 1] {
            assert!(extract_style_reconstruction_resources(&source[..n]).is_err());
        }
        let mut missing = source.clone();
        let at = missing.windows(4).position(|s| s == b"auxl").unwrap();
        missing[at..at + 4].copy_from_slice(b"junk");
        assert!(extract_style_reconstruction_resources(&missing).is_err());
        let mut changed = source;
        let at = changed.windows(3).position(|s| s == b"SDR").unwrap();
        changed[at] = b'X';
        assert_ne!(
            primary_image_signature(&changed).unwrap(),
            primary_image_signature(&fixture()).unwrap()
        );
    }
}
