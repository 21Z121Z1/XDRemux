use std::collections::{BTreeMap, BTreeSet};

use xdremux_format::isobmff::{
    make_box, make_iinf_box, make_iloc_box, make_ipma_box, make_iref_box, parse_boxes,
    parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry, IlocExtent, IpmaAssociation,
    IpmaEntry, IrefEntry, ParsedMeta, PropertyInfo, IDAT, IINF, ILOC, INFE, IPCO, IPMA, IPRP, IREF,
    MDAT, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const GRID: FourCC = FourCC::new(*b"grid");
const MIME: FourCC = FourCC::new(*b"mime");
const EXIF: FourCC = FourCC::new(*b"Exif");
const TMAP: FourCC = FourCC::new(*b"tmap");
const AUXC: FourCC = FourCC::new(*b"auxC");
const AUXL: FourCC = FourCC::new(*b"auxl");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const DIMG: FourCC = FourCC::new(*b"dimg");
const COLR: FourCC = FourCC::new(*b"colr");
const IROT: FourCC = FourCC::new(*b"irot");
const IMIR: FourCC = FourCC::new(*b"imir");
const CLAP: FourCC = FourCC::new(*b"clap");
const ISPE: FourCC = FourCC::new(*b"ispe");
const PIXI: FourCC = FourCC::new(*b"pixi");
const PREM: FourCC = FourCC::new(*b"prem");

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn raw_box<'a>(data: &'a [u8], header: &BoxHeader, context: &str) -> Result<&'a [u8]> {
    data.get(header.box_range())
        .ok_or_else(|| invalid(format!("{context} box is outside input")))
}

fn raw_property<'a>(data: &'a [u8], property: &PropertyInfo) -> Result<&'a [u8]> {
    data.get(property.box_range.clone())
        .ok_or_else(|| invalid(format!("property {} is outside input", property.index)))
}

fn one_top_level<'a>(boxes: &'a [BoxHeader], kind: FourCC, context: &str) -> Result<&'a BoxHeader> {
    let mut matches = boxes.iter().filter(|header| header.kind == kind);
    let Some(first) = matches.next() else {
        return Err(invalid(format!("{context} is missing")));
    };
    if matches.next().is_some() {
        return Err(invalid(format!("{context} appears more than once")));
    }
    Ok(first)
}

fn child<'a>(children: &'a [BoxHeader], kind: FourCC, context: &str) -> Result<&'a BoxHeader> {
    children
        .iter()
        .find(|header| header.kind == kind)
        .ok_or_else(|| invalid(format!("{context}/{} is missing", kind)))
}

#[derive(Debug, Clone)]
struct Graph {
    top: xdremux_format::isobmff::TopLevelScan,
    meta_header: BoxHeader,
    meta_children: Vec<BoxHeader>,
    meta: ParsedMeta,
    mdat: BoxHeader,
}

fn parse_graph(data: &[u8], owner: &str) -> Result<Graph> {
    let top = scan_top_level_boxes(data)?;
    let meta_header = one_top_level(&top.boxes, META, &format!("{owner} meta"))?.clone();
    let mdat = one_top_level(&top.boxes, MDAT, &format!("{owner} mdat"))?.clone();
    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid(format!("{owner} meta header offset overflows")))?;
    if meta_children_start > meta_header.data_end {
        return Err(invalid(format!(
            "{owner} meta full-box header is truncated"
        )));
    }
    let meta_children = parse_boxes(data, meta_children_start..meta_header.data_end)?;
    let meta = parse_meta_box(data, &meta_header)?;
    for kind in [IINF, ILOC, IPRP] {
        let _ = child(&meta_children, kind, owner)?;
    }
    Ok(Graph {
        top,
        meta_header,
        meta_children,
        meta,
        mdat,
    })
}

fn find_single_item(meta: &ParsedMeta, item_type: FourCC, context: &str) -> Result<u32> {
    let ids = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(item_type))
        .map(|item| item.item_id)
        .collect::<Vec<_>>();
    match ids.as_slice() {
        [only] => Ok(*only),
        [] => Err(invalid(format!("{context} has no {item_type} item"))),
        _ => Err(invalid(format!("{context} has multiple {item_type} items"))),
    }
}

fn find_tmap(meta: &ParsedMeta, context: &str) -> Result<u32> {
    find_single_item(meta, TMAP, context)
}

fn item_info<'a>(
    meta: &'a ParsedMeta,
    item_id: u32,
    context: &str,
) -> Result<&'a xdremux_format::isobmff::ItemInfo> {
    meta.iinf
        .entries
        .iter()
        .find(|item| item.item_id == item_id)
        .ok_or_else(|| invalid(format!("{context} item {item_id} is missing")))
}

fn item_location<'a>(meta: &'a ParsedMeta, item_id: u32, context: &str) -> Result<&'a IlocEntry> {
    meta.iloc
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .ok_or_else(|| invalid(format!("{context} item {item_id} has no iloc entry")))
}

fn item_payload(data: &[u8], graph: &Graph, item_id: u32, context: &str) -> Result<Vec<u8>> {
    let entry = item_location(&graph.meta, item_id, context)?;
    if entry.data_reference_index != 0 {
        return Err(invalid(format!(
            "{context} item {item_id} uses data_reference_index {}",
            entry.data_reference_index
        )));
    }
    let mut payload = Vec::new();
    for extent in &entry.extents {
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "{context} item {item_id} uses a non-zero extent index"
            )));
        }
        let offset = entry.resolved_extent_offset(extent)?;
        let end = offset
            .checked_add(extent.length)
            .ok_or_else(|| invalid(format!("{context} item {item_id} extent overflows")))?;
        let range = match entry.construction_method {
            0 => {
                let start = u64::try_from(graph.mdat.data_start)
                    .map_err(|_| invalid(format!("{context} mdat offset exceeds u64")))?;
                let limit = u64::try_from(graph.mdat.data_end)
                    .map_err(|_| invalid(format!("{context} mdat end exceeds u64")))?;
                if offset < start || end > limit {
                    return Err(invalid(format!(
                        "{context} item {item_id} extent is outside mdat"
                    )));
                }
                let start = usize::try_from(offset).map_err(|_| {
                    invalid(format!("{context} item {item_id} offset exceeds usize"))
                })?;
                let end = usize::try_from(end)
                    .map_err(|_| invalid(format!("{context} item {item_id} end exceeds usize")))?;
                start..end
            }
            1 => {
                let idat = graph
                    .meta
                    .idat
                    .as_ref()
                    .ok_or_else(|| invalid(format!("{context} item {item_id} has no idat")))?;
                let idat_start = u64::try_from(idat.data_start)
                    .map_err(|_| invalid(format!("{context} idat offset exceeds u64")))?;
                let idat_length = u64::try_from(idat.data_end - idat.data_start)
                    .map_err(|_| invalid(format!("{context} idat length exceeds u64")))?;
                if end > idat_length {
                    return Err(invalid(format!(
                        "{context} item {item_id} idat extent is outside idat"
                    )));
                }
                let start = usize::try_from(idat_start.checked_add(offset).ok_or_else(|| {
                    invalid(format!("{context} item {item_id} idat offset overflows"))
                })?)
                .map_err(|_| invalid(format!("{context} item {item_id} offset exceeds usize")))?;
                let end = usize::try_from(idat_start.checked_add(end).ok_or_else(|| {
                    invalid(format!("{context} item {item_id} idat end overflows"))
                })?)
                .map_err(|_| invalid(format!("{context} item {item_id} end exceeds usize")))?;
                start..end
            }
            method => {
                return Err(invalid(format!(
                    "{context} item {item_id} uses unsupported construction_method {method}"
                )))
            }
        };
        payload.extend_from_slice(
            data.get(range).ok_or_else(|| {
                invalid(format!("{context} item {item_id} extent is outside input"))
            })?,
        );
    }
    if payload.is_empty() {
        return Err(invalid(format!(
            "{context} item {item_id} has an empty payload"
        )));
    }
    Ok(payload)
}

fn normalized_source_location(
    data: &[u8],
    graph: &Graph,
    entry: &IlocEntry,
    placeholder: bool,
) -> Result<IlocEntry> {
    if entry.data_reference_index != 0 {
        return Err(invalid(format!(
            "source item {} uses data_reference_index {}",
            entry.item_id, entry.data_reference_index
        )));
    }
    if !matches!(entry.construction_method, 0 | 1) {
        return Err(invalid(format!(
            "source item {} uses unsupported construction_method {}",
            entry.item_id, entry.construction_method
        )));
    }
    let mut extents = Vec::with_capacity(entry.extents.len());
    for extent in &entry.extents {
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "source item {} uses a non-zero extent index",
                entry.item_id
            )));
        }
        let offset = if placeholder && entry.construction_method == 0 {
            0
        } else {
            let resolved = entry.resolved_extent_offset(extent)?;
            if entry.construction_method == 0 {
                let mdat_start = u64::try_from(graph.mdat.data_start)
                    .map_err(|_| invalid("source mdat offset exceeds u64"))?;
                let mdat_end = u64::try_from(graph.mdat.data_end)
                    .map_err(|_| invalid("source mdat end exceeds u64"))?;
                let end = resolved
                    .checked_add(extent.length)
                    .ok_or_else(|| invalid("source mdat extent overflows"))?;
                if resolved < mdat_start || end > mdat_end {
                    return Err(invalid(format!(
                        "source item {} has file data outside mdat",
                        entry.item_id
                    )));
                }
            } else {
                let idat = graph
                    .meta
                    .idat
                    .as_ref()
                    .ok_or_else(|| invalid("source construction-method-1 item has no idat"))?;
                let idat_len = u64::try_from(idat.data_end - idat.data_start)
                    .map_err(|_| invalid("source idat length exceeds u64"))?;
                if resolved
                    .checked_add(extent.length)
                    .is_none_or(|end| end > idat_len)
                {
                    return Err(invalid(format!(
                        "source item {} has idat data outside idat",
                        entry.item_id
                    )));
                }
            }
            resolved
        };
        extents.push(IlocExtent {
            index: None,
            offset,
            length: extent.length,
        });
    }
    let _ = data;
    Ok(IlocEntry {
        item_id: entry.item_id,
        construction_method: entry.construction_method,
        data_reference_index: 0,
        base_offset: 0,
        extents,
    })
}

fn source_maximum_item_id(meta: &ParsedMeta) -> Result<u32> {
    meta.iinf
        .entries
        .iter()
        .map(|item| item.item_id)
        .chain(meta.iloc.entries.iter().map(|entry| entry.item_id))
        .chain(meta.iref.as_ref().into_iter().flat_map(|iref| {
            iref.entries.iter().flat_map(|reference| {
                std::iter::once(reference.from_item_id).chain(reference.to_item_ids.iter().copied())
            })
        }))
        .max()
        .ok_or_else(|| invalid("source semantic graph has no item IDs"))
}

fn semantic_items(
    data: &[u8],
    scaffold: &Graph,
    alpha: Option<&AlphaImage>,
) -> Result<(u32, Vec<u32>, Vec<u32>)> {
    let scaffold_tmap = find_tmap(&scaffold.meta, "semantic scaffold")?;
    let scaffold_primary = scaffold.meta.primary_item_id;
    let (gain_image, primary_items, gain_items) =
        hdr_image_items(&scaffold.meta, "semantic scaffold")?;
    let hdr_items = primary_items
        .into_iter()
        .chain(gain_items)
        .chain([scaffold_primary, gain_image])
        .collect::<BTreeSet<_>>();
    let mut images = Vec::new();
    let mut seen_images = BTreeSet::new();
    let mut seen_gain_auxiliary = false;
    if let Some(iref) = scaffold.meta.iref.as_ref() {
        for reference in &iref.entries {
            if reference.kind != AUXL {
                continue;
            }
            // Only an explicitly identified, source-paired alpha image is
            // excluded here. Missing semantic descriptors remain an error.
            if alpha.is_some_and(|alpha| alpha.root == reference.from_item_id) {
                continue;
            }
            if reference.to_item_ids.len() != 2
                || !reference.to_item_ids.contains(&scaffold_primary)
                || !reference.to_item_ids.contains(&scaffold_tmap)
            {
                return Err(invalid(
                    "semantic scaffold has an unsupported auxiliary relationship",
                ));
            }
            // The tmap dimg graph identifies the Gain Map even when ImageIO
            // encodes it as a direct hvc1 with the same auxl targets as mattes.
            if reference.from_item_id == gain_image {
                if seen_gain_auxiliary {
                    return Err(invalid(
                        "semantic scaffold has multiple Gain Map auxl references",
                    ));
                }
                seen_gain_auxiliary = true;
                continue;
            }
            if item_info(&scaffold.meta, reference.from_item_id, "semantic scaffold")?.item_type
                != Some(HVC1)
                || hdr_items.contains(&reference.from_item_id)
                || alpha.is_some_and(|alpha| alpha.coded_items.contains(&reference.from_item_id))
            {
                return Err(invalid(
                    "semantic scaffold has an unsupported auxiliary image",
                ));
            }
            if !seen_images.insert(reference.from_item_id) {
                return Err(invalid(
                    "semantic scaffold image has multiple auxl references",
                ));
            }
            images.push(reference.from_item_id);
        }
    }
    if images.is_empty() {
        return Err(invalid(
            "semantic scaffold has no auxiliary semantic image items",
        ));
    }
    let image_set = images.iter().copied().collect::<BTreeSet<_>>();
    let mut metadata = Vec::new();
    let mut seen_metadata = BTreeSet::new();
    let mut described_images = BTreeSet::new();
    if let Some(iref) = scaffold.meta.iref.as_ref() {
        for reference in &iref.entries {
            if reference.kind != CDSC
                || !reference
                    .to_item_ids
                    .iter()
                    .any(|id| image_set.contains(id))
            {
                continue;
            }
            if reference.to_item_ids.len() != 1
                || item_info(&scaffold.meta, reference.from_item_id, "semantic scaffold")?.item_type
                    != Some(MIME)
                || !seen_metadata.insert(reference.from_item_id)
                || !described_images.insert(reference.to_item_ids[0])
            {
                return Err(invalid(
                    "semantic scaffold requires one unique MIME descriptor per image",
                ));
            }
            metadata.push(reference.from_item_id);
        }
        for descriptor in &metadata {
            if iref
                .entries
                .iter()
                .filter(|reference| reference.kind == CDSC && reference.from_item_id == *descriptor)
                .count()
                != 1
            {
                return Err(invalid(
                    "semantic scaffold descriptor has multiple cdsc references",
                ));
            }
        }
    }
    if metadata.len() != images.len() {
        return Err(invalid(format!(
            "semantic scaffold has {} semantic images but {} metadata descriptors",
            images.len(),
            metadata.len()
        )));
    }
    for image in &images {
        if let Some(index) = property_index_of_kind(&scaffold.meta, *image, AUXC) {
            let property = scaffold
                .meta
                .properties
                .iter()
                .find(|property| property.index == u32::from(index))
                .ok_or_else(|| invalid("semantic auxiliary property is missing"))?;
            let (uri, _) = auxiliary_type_bytes(data, property)?;
            if !matches!(
                uri,
                b"urn:com:apple:photo:2018:aux:portraiteffectsmatte"
                    | b"urn:com:apple:photo:2019:aux:semanticskinmatte"
                    | b"urn:com:apple:photo:2019:aux:semantichairmatte"
                    | b"urn:com:apple:photo:2019:aux:semanticteethmatte"
                    | b"urn:com:apple:photo:2020:aux:semanticglassesmatte"
                    | b"urn:com:apple:photo:2020:aux:semanticskymatte"
            ) {
                return Err(invalid(
                    "semantic scaffold has an unsupported auxiliary role",
                ));
            }
        }
    }
    Ok((scaffold_tmap, images, metadata))
}

fn auxiliary_type_bytes<'a>(
    data: &'a [u8],
    property: &PropertyInfo,
) -> Result<(&'a [u8], &'a [u8])> {
    let boxes = parse_boxes(data, property.box_range.clone())?;
    let [header] = boxes.as_slice() else {
        return Err(invalid("auxC property has no unique box"));
    };
    let payload = data
        .get(header.payload_range())
        .ok_or_else(|| invalid("auxC is outside input"))?;
    if header.kind != AUXC || payload.get(..4) != Some(&[0, 0, 0, 0]) {
        return Err(invalid("auxC property has an invalid full-box header"));
    }
    let bytes = &payload[4..];
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| invalid("auxC property has no terminated type"))?;
    Ok((&bytes[..end], &bytes[end + 1..]))
}

fn is_alpha_type(uri: &[u8]) -> bool {
    // HEIF/MIAF identifiers consumed as alpha by libheif's auxl reader:
    // https://github.com/strukturag/libheif/blob/v1.19.8/libheif/context.cc
    matches!(
        uri,
        b"urn:mpeg:hevc:2015:auxid:1" | b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha"
    )
}

struct AlphaImage {
    root: u32,
    coded_items: Vec<u32>,
    includes_tmap: bool,
    premultiplied: bool,
}

fn primary_alpha_image(data: &[u8], graph: &Graph, context: &str) -> Result<Option<AlphaImage>> {
    let meta = &graph.meta;
    let references = meta
        .iref
        .as_ref()
        .map_or(&[][..], |iref| iref.entries.as_slice());
    let tmap = find_tmap(meta, context)?;
    let (gain, primary_items, gain_items) = hdr_image_items(meta, context)?;
    let hdr_items = primary_items
        .into_iter()
        .chain(gain_items)
        .chain([meta.primary_item_id, gain, tmap])
        .collect::<BTreeSet<_>>();
    let mut alpha = None;
    for reference in references.iter().filter(|reference| reference.kind == AUXL) {
        let properties = associated_properties(meta, reference.from_item_id)?
            .into_iter()
            .filter_map(|association| {
                meta.properties.iter().find(|property| {
                    property.index == u32::from(association.property_index) && property.kind == AUXC
                })
            })
            .collect::<Vec<_>>();
        let property = match properties.as_slice() {
            [] => continue,
            [property] => *property,
            _ => {
                return Err(invalid(format!(
                    "{context} auxiliary image has multiple auxC properties"
                )))
            }
        };
        let (uri, subtype) = auxiliary_type_bytes(data, property)?;
        if !is_alpha_type(uri) {
            continue;
        }
        if alpha.is_some() {
            return Err(invalid(format!("{context} has multiple alpha resources")));
        }
        if !subtype.is_empty() {
            return Err(invalid(format!("{context} alpha subtype is unsupported")));
        }
        let includes_tmap = reference.to_item_ids.contains(&tmap);
        if !reference.to_item_ids.contains(&meta.primary_item_id)
            || reference.to_item_ids.len() != if includes_tmap { 2 } else { 1 }
        {
            return Err(invalid(format!(
                "{context} alpha has an unsupported auxiliary relationship"
            )));
        }
        let coded_items = coded_image_items(meta, reference.from_item_id, context)?;
        for id in std::iter::once(reference.from_item_id).chain(coded_items.iter().copied()) {
            if meta
                .iinf
                .entries
                .iter()
                .filter(|entry| entry.item_id == id)
                .count()
                != 1
                || meta
                    .ipma
                    .entries
                    .iter()
                    .filter(|entry| entry.item_id == id)
                    .count()
                    != 1
                || meta
                    .iloc
                    .entries
                    .iter()
                    .filter(|entry| entry.item_id == id)
                    .count()
                    != 1
            {
                return Err(invalid(format!(
                    "{context} alpha item records are missing or ambiguous"
                )));
            }
        }
        if hdr_items.contains(&reference.from_item_id)
            || coded_items.iter().any(|id| hdr_items.contains(id))
        {
            return Err(invalid(format!(
                "{context} alpha overlaps the HDR image graph"
            )));
        }
        let mut premultiplied = false;
        for edge in references {
            if edge.kind == PREM
                && (edge.from_item_id == meta.primary_item_id
                    || edge.to_item_ids.contains(&reference.from_item_id))
            {
                if premultiplied
                    || edge.from_item_id != meta.primary_item_id
                    || edge.to_item_ids != [reference.from_item_id]
                {
                    return Err(invalid(format!(
                        "{context} alpha has ambiguous premultiplication"
                    )));
                }
                premultiplied = true;
            } else if (edge.from_item_id == reference.from_item_id
                && edge.kind != AUXL
                && edge.kind != DIMG)
                || edge.from_item_id != reference.from_item_id
                    && coded_items.contains(&edge.from_item_id)
                || edge
                    .to_item_ids
                    .iter()
                    .any(|id| *id == reference.from_item_id || coded_items.contains(id))
                    && !(edge.kind == DIMG && edge.from_item_id == reference.from_item_id)
            {
                return Err(invalid(format!(
                    "{context} alpha has an unsupported resource relationship"
                )));
            }
        }
        alpha = Some(AlphaImage {
            root: reference.from_item_id,
            coded_items,
            includes_tmap,
            premultiplied,
        });
    }
    Ok(alpha)
}

fn alpha_properties(
    data: &[u8],
    meta: &ParsedMeta,
    id: u32,
    kinds: &[FourCC],
) -> Result<Vec<Vec<u8>>> {
    let mut properties = Vec::new();
    let mut seen = BTreeSet::new();
    for association in associated_properties(meta, id)? {
        let property = meta
            .properties
            .iter()
            .find(|property| property.index == u32::from(association.property_index))
            .ok_or_else(|| invalid("alpha property association is unresolved"))?;
        if kinds.contains(&property.kind) {
            if !seen.insert(property.kind) {
                return Err(invalid(
                    "alpha has repeated geometry or transform properties",
                ));
            }
            properties.push(raw_property(data, property)?.to_vec());
        }
    }
    Ok(properties)
}

fn paired_alpha_images(
    source_data: &[u8],
    source: &Graph,
    scaffold_data: &[u8],
    scaffold: &Graph,
) -> Result<(Option<AlphaImage>, Option<AlphaImage>)> {
    let source_alpha = primary_alpha_image(source_data, source, "source HDR")?;
    let scaffold_alpha = primary_alpha_image(scaffold_data, scaffold, "semantic scaffold")?;
    match (&source_alpha, &scaffold_alpha) {
        (None, None) => {}
        (Some(original), Some(encoded)) => {
            if original.includes_tmap != encoded.includes_tmap
                || original.premultiplied != encoded.premultiplied
                || original.coded_items.len() != encoded.coded_items.len()
                || item_info(&source.meta, original.root, "source alpha")?.item_type
                    != item_info(&scaffold.meta, encoded.root, "scaffold alpha")?.item_type
            {
                return Err(invalid(
                    "source and scaffold alpha relationships or tile graphs differ",
                ));
            }
            for (source_id, scaffold_id) in std::iter::once((original.root, encoded.root)).chain(
                original
                    .coded_items
                    .iter()
                    .copied()
                    .zip(encoded.coded_items.iter().copied()),
            ) {
                for kinds in [&[ISPE][..], &[PIXI][..], &[CLAP, IROT, IMIR][..]] {
                    if alpha_properties(source_data, &source.meta, source_id, kinds)?
                        != alpha_properties(scaffold_data, &scaffold.meta, scaffold_id, kinds)?
                    {
                        return Err(invalid(
                            "source and scaffold alpha geometry or transform order differ",
                        ));
                    }
                }
            }
            if item_is_type(&source.meta, original.root, GRID)
                && item_payload(source_data, source, original.root, "source alpha grid")?
                    != item_payload(scaffold_data, scaffold, encoded.root, "scaffold alpha grid")?
            {
                return Err(invalid("source and scaffold alpha grid geometry differs"));
            }
        }
        _ => {
            return Err(invalid(
                "source and scaffold must contain a matching alpha resource",
            ))
        }
    }
    Ok((source_alpha, scaffold_alpha))
}

// Error reports survive the runtime's temporary scaffold cleanup. Keep this
// observation bounded and restricted to structural IDs and fixed role labels:
// item payloads, metadata, paths, and unknown property strings are never logged.
fn diagnostic_auxiliary_role(uri: &[u8]) -> &'static str {
    match uri {
        b"urn:mpeg:hevc:2015:auxid:1" => "alpha-hevc",
        b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha" => "alpha-miaf",
        b"urn:iso:std:iso:ts:21496:-1" => "iso-gain-map",
        b"urn:com:apple:photo:2020:aux:hdrgainmap" => "apple-hdr-gain-map",
        b"urn:com:apple:photo:2018:aux:portraiteffectsmatte" => "portrait-effects-matte",
        b"urn:com:apple:photo:2019:aux:semanticskinmatte" => "semantic-skin",
        b"urn:com:apple:photo:2019:aux:semantichairmatte" => "semantic-hair",
        b"urn:com:apple:photo:2019:aux:semanticteethmatte" => "semantic-teeth",
        b"urn:com:apple:photo:2020:aux:semanticglassesmatte" => "semantic-glasses",
        b"urn:com:apple:photo:2020:aux:semanticskymatte" => "semantic-sky",
        _ => "unknown",
    }
}

fn diagnostic_subtype_shape(uri: &[u8], subtype: &[u8]) -> &'static str {
    if subtype.is_empty() {
        return "empty";
    }
    if subtype.len() > 65_536 {
        return "over-limit";
    }
    if subtype.iter().all(|byte| *byte == 0) {
        return "zero-filled";
    }
    if uri != b"urn:mpeg:hevc:2015:auxid:1" || subtype.len() < 4 {
        return "opaque";
    }
    // Recognize only the outer size/NAL-header envelope used by libheif's
    // decode_hevc_aux_sei_messages, not alpha SEI payload semantics:
    // https://github.com/strukturag/libheif/blob/364905fc9841bdda229e0ae2c5fda13b5fc98d1a/libheif/codecs/hevc_boxes.cc#L532
    let size = u32::from_be_bytes([subtype[0], subtype[1], subtype[2], subtype[3]]) as usize;
    if size != subtype.len() - 4 {
        return "opaque";
    }
    let mut offset = 4;
    let mut count = 0;
    while offset < subtype.len() {
        if count == 16 {
            return "over-limit";
        }
        let Some(header) = subtype.get(offset..offset + 4) else {
            return "opaque";
        };
        let size = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let start = offset + 4;
        let Some(end) = start.checked_add(size) else {
            return "opaque";
        };
        let Some(nal) = subtype.get(start..end) else {
            return "opaque";
        };
        if nal.len() < 2
            || nal[0] & 0x80 != 0
            || nal[1] & 7 == 0
            || !matches!((nal[0] >> 1) & 0x3f, 39 | 40)
        {
            return "opaque";
        }
        offset = end;
        count += 1;
    }
    "hevc-sei-envelope"
}

struct DiagnosticAlphaAuxc<'a> {
    uri: &'a [u8],
    subtype: &'a [u8],
}

fn diagnostic_alpha_auxc<'a>(
    data: &'a [u8],
    graph: &Graph,
) -> std::result::Result<Option<DiagnosticAlphaAuxc<'a>>, ()> {
    let mut alpha = None;
    let mut count = 0;
    for reference in graph
        .meta
        .iref
        .as_ref()
        .into_iter()
        .flat_map(|iref| &iref.entries)
        .filter(|reference| reference.kind == AUXL)
    {
        count += 1;
        if count > 16 {
            return Err(());
        }
        let associations =
            associated_properties(&graph.meta, reference.from_item_id).map_err(|_| ())?;
        let mut properties = associations.iter().filter_map(|association| {
            graph.meta.properties.iter().find(|property| {
                property.index == u32::from(association.property_index) && property.kind == AUXC
            })
        });
        let Some(property) = properties.next() else {
            continue;
        };
        if properties.next().is_some() {
            return Err(());
        }
        let (uri, subtype) = auxiliary_type_bytes(data, property).map_err(|_| ())?;
        if is_alpha_type(uri) {
            if alpha.is_some() || subtype.len() > 65_536 {
                return Err(());
            }
            alpha = Some(DiagnosticAlphaAuxc { uri, subtype });
        }
    }
    Ok(alpha)
}

fn diagnostic_alpha_subtype_pair(
    source_data: &[u8],
    source: &Graph,
    scaffold_data: &[u8],
    scaffold: &Graph,
) -> String {
    match (
        diagnostic_alpha_auxc(source_data, source),
        diagnostic_alpha_auxc(scaffold_data, scaffold),
    ) {
        (Ok(Some(original)), Ok(Some(encoded))) => {
            format!(
                "uri_match={},bytes_match={}",
                original.uri == encoded.uri,
                original.subtype == encoded.subtype
            )
        }
        (Ok(None), Ok(None)) => "absent".to_owned(),
        (Ok(_), Ok(_)) => "missing".to_owned(),
        _ => "unavailable".to_owned(),
    }
}

fn diagnostic_item_type(meta: &ParsedMeta, item_id: u32) -> &'static str {
    let mut infos = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_id == item_id);
    let Some(info) = infos.next() else {
        return "missing";
    };
    if infos.next().is_some() {
        return "ambiguous";
    }
    match info.item_type {
        Some(HVC1) => "hvc1",
        Some(GRID) => "grid",
        Some(TMAP) => "tmap",
        Some(MIME) => "mime",
        Some(EXIF) => "Exif",
        _ => "other",
    }
}

fn diagnostic_graph(data: &[u8], graph: &Graph) -> String {
    const MAX_RESOURCES: usize = 16;
    const MAX_ROLES: usize = 4;
    const MAX_REFERENCES: usize = 12;
    const MAX_TARGETS: usize = 8;
    let meta = &graph.meta;
    let references = meta
        .iref
        .as_ref()
        .map_or(&[][..], |iref| iref.entries.as_slice());
    let mut resource_ids = BTreeSet::from([meta.primary_item_id]);
    for reference in references {
        if reference.kind == AUXL {
            resource_ids.insert(reference.from_item_id);
        } else if reference.kind == DIMG
            && diagnostic_item_type(meta, reference.from_item_id) == "tmap"
        {
            resource_ids.extend(&reference.to_item_ids);
        }
    }
    let mut resources = Vec::new();
    for item_id in resource_ids.iter().take(MAX_RESOURCES) {
        let mut roles = Vec::new();
        let mut subtypes = Vec::new();
        let mut role_count = 0;
        for association in meta
            .ipma
            .entries
            .iter()
            .filter(|entry| entry.item_id == *item_id)
            .flat_map(|entry| &entry.associations)
        {
            if let Some(property) = meta.properties.iter().find(|property| {
                property.index == u32::from(association.property_index) && property.kind == AUXC
            }) {
                role_count += 1;
                if roles.len() < MAX_ROLES {
                    match auxiliary_type_bytes(data, property) {
                        Ok((uri, subtype)) => {
                            roles.push(diagnostic_auxiliary_role(uri).to_owned());
                            subtypes.push(format!(
                                "len={},shape={}",
                                subtype.len(),
                                diagnostic_subtype_shape(uri, subtype)
                            ));
                        }
                        Err(_) => {
                            roles.push("malformed".to_owned());
                            subtypes.push("unavailable".to_owned());
                        }
                    }
                }
            }
        }
        if role_count > MAX_ROLES {
            roles.push(format!("omitted={}", role_count - MAX_ROLES));
        } else if roles.is_empty() {
            roles.push("none".to_owned());
        }
        resources.push(format!(
            "{item_id}:{}:auxC=[{}]:subtype=[{}]",
            diagnostic_item_type(meta, *item_id),
            roles.join(","),
            subtypes.join(";")
        ));
    }
    if resource_ids.len() > MAX_RESOURCES {
        resources.push(format!("omitted={}", resource_ids.len() - MAX_RESOURCES));
    }
    let mut groups = Vec::new();
    for (kind, name) in [(DIMG, "dimg"), (AUXL, "auxl"), (CDSC, "cdsc")] {
        let mut edges = Vec::new();
        let mut count = 0;
        for reference in references.iter().filter(|reference| reference.kind == kind) {
            count += 1;
            if edges.len() >= MAX_REFERENCES {
                continue;
            }
            let mut targets = reference
                .to_item_ids
                .iter()
                .take(MAX_TARGETS)
                .map(u32::to_string)
                .collect::<Vec<_>>();
            if reference.to_item_ids.len() > MAX_TARGETS {
                targets.push(format!(
                    "omitted={}",
                    reference.to_item_ids.len() - MAX_TARGETS
                ));
            }
            edges.push(format!(
                "{}->[{}]",
                reference.from_item_id,
                targets.join(",")
            ));
        }
        if count > MAX_REFERENCES {
            edges.push(format!("omitted={}", count - MAX_REFERENCES));
        }
        groups.push(format!("{name}=[{}]", edges.join(",")));
    }
    format!(
        "primary={},items={},resources=[{}],{}",
        meta.primary_item_id,
        meta.iinf.entries.len(),
        resources.join(","),
        groups.join(",")
    )
}

fn semantic_graph_error(
    error: HeifError,
    source_data: &[u8],
    source: &Graph,
    scaffold_data: &[u8],
    scaffold: &Graph,
) -> HeifError {
    let message = match error {
        HeifError::Invalid(message) => message,
        HeifError::Format(error) => error.to_string(),
    };
    invalid(format!(
        "{message}; resource graphs: source={{{}}}; scaffold={{{}}}; alpha_subtype_pair={{{}}}",
        diagnostic_graph(source_data, source),
        diagnostic_graph(scaffold_data, scaffold),
        diagnostic_alpha_subtype_pair(source_data, source, scaffold_data, scaffold)
    ))
}

fn associated_properties(meta: &ParsedMeta, item_id: u32) -> Result<Vec<IpmaAssociation>> {
    meta.ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .map(|entry| entry.associations.clone())
        .ok_or_else(|| {
            invalid(format!(
                "semantic scaffold item {item_id} has no ipma entry"
            ))
        })
}

fn property_index_of_kind(meta: &ParsedMeta, item_id: u32, kind: FourCC) -> Option<u16> {
    meta.ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .and_then(|entry| {
            entry.associations.iter().find_map(|association| {
                meta.properties
                    .iter()
                    .find(|property| property.index == u32::from(association.property_index))
                    .filter(|property| property.kind == kind)
                    .map(|_| association.property_index)
            })
        })
}

fn source_primary_color_property_index(meta: &ParsedMeta) -> Result<Option<u16>> {
    if let Some(index) = property_index_of_kind(meta, meta.primary_item_id, COLR) {
        return Ok(Some(index));
    }

    // A Rust-authored base may associate the primary color profile with the
    // primary grid's hvc1 children rather than with the grid item itself. A
    // semantic ImageIO write expects the primary association to remain
    // explicit, so recover it only from the primary grid's own dimg edge and
    // require all participating components to agree on one property.
    let mut candidates = BTreeSet::new();
    if let Some(iref) = meta.iref.as_ref() {
        for reference in &iref.entries {
            if reference.kind != DIMG || reference.from_item_id != meta.primary_item_id {
                continue;
            }
            for item_id in &reference.to_item_ids {
                let is_primary_component = meta
                    .iinf
                    .entries
                    .iter()
                    .any(|item| item.item_id == *item_id && item.item_type == Some(HVC1));
                if is_primary_component {
                    if let Some(index) = property_index_of_kind(meta, *item_id, COLR) {
                        candidates.insert(index);
                    }
                }
            }
        }
    }
    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.into_iter().next()),
        _ => Err(invalid(
            "semantic merge found ambiguous primary component color properties",
        )),
    }
}

fn first_property_index_of_kind(meta: &ParsedMeta, kind: FourCC) -> Option<u16> {
    meta.properties
        .iter()
        .find(|property| property.kind == kind)
        .and_then(|property| u16::try_from(property.index).ok())
}

fn make_iprp(
    source_data: &[u8],
    scaffold_data: &[u8],
    source: &Graph,
    scaffold: &Graph,
    imported_images: &[(u32, u32)],
) -> Result<Vec<u8>> {
    let source_iprp = child(&source.meta_children, IPRP, "source meta")?;
    let scaffold_property_by_index = scaffold
        .meta
        .properties
        .iter()
        .map(|property| (property.index, property))
        .collect::<BTreeMap<_, _>>();
    let mut properties = source.meta.properties.iter().collect::<Vec<_>>();
    properties.sort_by_key(|property| property.index);
    for (position, property) in properties.iter().enumerate() {
        let expected = u32::try_from(position + 1)
            .map_err(|_| invalid("source ipco property index exceeds u32"))?;
        if property.index != expected {
            return Err(invalid("source ipco property indices are not contiguous"));
        }
    }

    let mut ipco_payload = Vec::new();
    for property in &properties {
        ipco_payload.extend_from_slice(raw_property(source_data, property)?);
    }
    let mut next_property_index = u16::try_from(properties.len() + 1)
        .map_err(|_| invalid("source ipco property count exceeds u16"))?;
    let mut property_map = BTreeMap::new();
    let mut imported_associations = Vec::with_capacity(imported_images.len());
    for (source_id, scaffold_id) in imported_images {
        let associations = associated_properties(&scaffold.meta, *scaffold_id)?;
        let mut mapped = Vec::with_capacity(associations.len());
        for association in associations {
            let new_index = if let Some(index) = property_map.get(&association.property_index) {
                *index
            } else {
                let property = scaffold_property_by_index
                    .get(&u32::from(association.property_index))
                    .ok_or_else(|| {
                        invalid(format!(
                            "semantic scaffold property {} is missing",
                            association.property_index
                        ))
                    })?;
                let index = next_property_index;
                ipco_payload.extend_from_slice(raw_property(scaffold_data, property)?);
                next_property_index = next_property_index
                    .checked_add(1)
                    .ok_or_else(|| invalid("semantic property index overflows"))?;
                property_map.insert(association.property_index, index);
                index
            };
            mapped.push(IpmaAssociation {
                property_index: new_index,
                essential: association.essential,
            });
        }
        imported_associations.push(IpmaEntry {
            item_id: *source_id,
            associations: mapped,
        });
    }
    let last_property_index = next_property_index.saturating_sub(1);
    let mut ipma_entries = source.meta.ipma.entries.clone();
    if let Some(primary_entry) = ipma_entries
        .iter_mut()
        .find(|entry| entry.item_id == source.meta.primary_item_id)
    {
        if let Some(irot_index) = first_property_index_of_kind(&source.meta, IROT) {
            if !primary_entry
                .associations
                .iter()
                .any(|association| association.property_index == irot_index)
            {
                primary_entry.associations.push(IpmaAssociation {
                    property_index: irot_index,
                    essential: true,
                });
            }
        }
        if let Some(color_index) = source_primary_color_property_index(&source.meta)? {
            if !primary_entry
                .associations
                .iter()
                .any(|association| association.property_index == color_index)
            {
                primary_entry.associations.push(IpmaAssociation {
                    property_index: color_index,
                    essential: true,
                });
            }
        }
    }
    ipma_entries.extend(imported_associations);
    let maximum_item_id = ipma_entries
        .iter()
        .map(|entry| entry.item_id)
        .max()
        .unwrap_or(0);
    let ipma_version = if maximum_item_id > u32::from(u16::MAX) {
        1
    } else {
        source.meta.ipma.version
    };
    let ipma_flags = if last_property_index > 0x7f {
        source.meta.ipma.flags | 1
    } else {
        source.meta.ipma.flags
    };
    let ipco = make_box(IPCO, &ipco_payload)?;
    let ipma = make_ipma_box(ipma_version, ipma_flags, &ipma_entries)?;
    let iprp_children = parse_boxes(source_data, source_iprp.payload_range())?;
    let mut payload = Vec::new();
    let mut saw_ipco = false;
    let mut saw_ipma = false;
    for header in &iprp_children {
        match header.kind {
            IPCO if !saw_ipco => {
                saw_ipco = true;
                payload.extend_from_slice(&ipco);
            }
            IPCO => return Err(invalid("source iprp contains multiple ipco boxes")),
            IPMA if !saw_ipma => {
                saw_ipma = true;
                payload.extend_from_slice(&ipma);
            }
            IPMA => return Err(invalid("source iprp contains multiple ipma boxes")),
            _ => payload.extend_from_slice(raw_box(source_data, header, "source iprp child")?),
        }
    }
    if !saw_ipco || !saw_ipma {
        return Err(invalid("source iprp is missing ipco or ipma"));
    }
    Ok(make_box(IPRP, &payload)?)
}

fn remap_infe_item_id(
    data: &[u8],
    item: &xdremux_format::isobmff::ItemInfo,
    new_id: u32,
) -> Result<Vec<u8>> {
    let raw = data.get(item.box_range.clone()).ok_or_else(|| {
        invalid(format!(
            "semantic scaffold infe {} is outside input",
            item.item_id
        ))
    })?;
    if raw.get(4..8) != Some(INFE.as_bytes()) || raw.len() < 14 {
        return Err(invalid(format!(
            "semantic scaffold item {} has a malformed infe box",
            item.item_id
        )));
    }
    let version = raw[8];
    let mut output = raw.to_vec();
    match version {
        2 => {
            let id = u16::try_from(new_id).map_err(|_| {
                invalid(format!(
                    "semantic scaffold item {} cannot be remapped into infe version 2",
                    item.item_id
                ))
            })?;
            output[12..14].copy_from_slice(&id.to_be_bytes());
        }
        3 => {
            if output.len() < 16 {
                return Err(invalid(format!(
                    "semantic scaffold item {} has a truncated infe version 3 box",
                    item.item_id
                )));
            }
            output[12..16].copy_from_slice(&new_id.to_be_bytes());
        }
        other => {
            return Err(invalid(format!(
                "semantic scaffold item {} uses unsupported infe version {other}",
                item.item_id
            )))
        }
    }
    Ok(output)
}

fn make_iinf(
    source_data: &[u8],
    scaffold_data: &[u8],
    source: &Graph,
    scaffold: &Graph,
    records: &[(u32, u32)],
) -> Result<Vec<u8>> {
    let mut entries = Vec::new();
    for item in &source.meta.iinf.entries {
        entries.push(
            source_data
                .get(item.box_range.clone())
                .ok_or_else(|| invalid(format!("source infe {} is outside input", item.item_id)))?
                .to_vec(),
        );
    }
    for (new_id, old_id) in records {
        let info = item_info(&scaffold.meta, *old_id, "semantic scaffold")?;
        entries.push(remap_infe_item_id(scaffold_data, info, *new_id)?);
    }
    let version = if source.meta.iinf.version == 0 && entries.len() > usize::from(u16::MAX) {
        1
    } else {
        source.meta.iinf.version
    };
    Ok(make_iinf_box(version, &entries)?)
}

fn map_semantic_reference(
    reference: &IrefEntry,
    scaffold_primary: u32,
    scaffold_tmap: u32,
    source_primary: u32,
    source_tmap: u32,
    id_map: &BTreeMap<u32, u32>,
) -> Result<IrefEntry> {
    let from_item_id = *id_map.get(&reference.from_item_id).ok_or_else(|| {
        invalid(format!(
            "semantic reference source item {} is not imported",
            reference.from_item_id
        ))
    })?;
    let mut to_item_ids = Vec::with_capacity(reference.to_item_ids.len());
    for target in &reference.to_item_ids {
        let mapped = if *target == scaffold_primary {
            source_primary
        } else if *target == scaffold_tmap {
            source_tmap
        } else if let Some(mapped) = id_map.get(target) {
            *mapped
        } else {
            return Err(invalid(format!(
                "semantic reference target item {target} is not remappable"
            )));
        };
        to_item_ids.push(mapped);
    }
    Ok(IrefEntry {
        kind: reference.kind,
        from_item_id,
        to_item_ids,
    })
}

fn make_meta(
    source_data: &[u8],
    source: &Graph,
    iinf: &[u8],
    iloc: &[u8],
    iprp: &[u8],
    iref: &[u8],
    idat: &[u8],
) -> Result<Vec<u8>> {
    let full_header_end = source
        .meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("source meta full-box header offset overflows"))?;
    let full_header = source_data
        .get(source.meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("source meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    let mut saw_iref = false;
    let mut saw_idat = false;
    for header in &source.meta_children {
        match header.kind {
            IINF => payload.extend_from_slice(iinf),
            ILOC => payload.extend_from_slice(iloc),
            IPRP => payload.extend_from_slice(iprp),
            IREF if !saw_iref => {
                saw_iref = true;
                payload.extend_from_slice(iref);
            }
            IREF => return Err(invalid("source meta contains multiple iref boxes")),
            IDAT if !saw_idat => {
                saw_idat = true;
                payload.extend_from_slice(idat);
            }
            IDAT => return Err(invalid("source meta contains multiple idat boxes")),
            _ => payload.extend_from_slice(raw_box(source_data, header, "source meta child")?),
        }
    }
    if !saw_iref {
        payload.extend_from_slice(iref);
    }
    if !saw_idat {
        payload.extend_from_slice(idat);
    }
    Ok(make_box(META, &payload)?)
}

fn normalized_source_final_location(
    source_data: &[u8],
    source: &Graph,
    entry: &IlocEntry,
    new_mdat_data_start: u64,
) -> Result<IlocEntry> {
    let mut normalized = normalized_source_location(source_data, source, entry, false)?;
    if normalized.construction_method == 0 {
        let old_mdat_start = u64::try_from(source.mdat.data_start)
            .map_err(|_| invalid("source mdat offset exceeds u64"))?;
        for extent in &mut normalized.extents {
            let relative = extent
                .offset
                .checked_sub(old_mdat_start)
                .ok_or_else(|| invalid("source file extent precedes mdat"))?;
            extent.offset = new_mdat_data_start
                .checked_add(relative)
                .ok_or_else(|| invalid("relocated semantic extent offset overflows"))?;
        }
    }
    Ok(normalized)
}

/// Merge semantic image resources produced by the ImageIO write primitive
/// into the Rust-owned HDR graph while retaining the source base/Gain Map
/// payloads byte-for-byte.
///
/// ImageIO is intentionally used only to encode and describe the semantic
/// resources. Re-running its destination writer over a Rust-authored ISO Gain
/// Map can hide the Gain Map from ImageIO when an auxiliary matte is added.
/// This graph merge is therefore part of the Rust container ownership layer:
/// it imports only the scaffold's semantic items, properties, and references,
/// and keeps the source primary/tmap items and mdat prefix authoritative.
pub fn merge_apple_semantic_auxiliary_heif(
    source_data: &[u8],
    scaffold_data: &[u8],
    expected_semantic_roles: usize,
) -> Result<Vec<u8>> {
    if expected_semantic_roles == 0 {
        return Err(invalid("semantic merge requires at least one role"));
    }
    let source = parse_graph(source_data, "source HDR")?;
    let scaffold = parse_graph(scaffold_data, "semantic scaffold")?;
    let source_tmap = find_tmap(&source.meta, "source HDR")?;
    let _ = hdr_image_items(&source.meta, "source HDR")?;
    let (_, scaffold_alpha) = paired_alpha_images(source_data, &source, scaffold_data, &scaffold)
        .map_err(|error| {
        semantic_graph_error(error, source_data, &source, scaffold_data, &scaffold)
    })?;
    let (scaffold_tmap, semantic_images, semantic_metadata) =
        semantic_items(scaffold_data, &scaffold, scaffold_alpha.as_ref()).map_err(|error| {
            semantic_graph_error(error, source_data, &source, scaffold_data, &scaffold)
        })?;
    if semantic_images.len() != expected_semantic_roles {
        return Err(invalid(format!(
            "semantic scaffold contains {} roles; expected {expected_semantic_roles}",
            semantic_images.len()
        )));
    }

    let source_exif = find_single_item(&source.meta, EXIF, "source HDR")?;
    let scaffold_exif = find_single_item(&scaffold.meta, EXIF, "semantic scaffold")?;
    let scaffold_exif_payload = item_payload(
        scaffold_data,
        &scaffold,
        scaffold_exif,
        "semantic scaffold Exif",
    )?;

    let mut next_item_id = source_maximum_item_id(&source.meta)?
        .checked_add(1)
        .ok_or_else(|| invalid("semantic merge item ID overflows"))?;
    let mut id_map = BTreeMap::new();
    for old_id in semantic_images
        .iter()
        .chain(semantic_metadata.iter())
        .copied()
    {
        if id_map.contains_key(&old_id) {
            continue;
        }
        id_map.insert(old_id, next_item_id);
        next_item_id = next_item_id
            .checked_add(1)
            .ok_or_else(|| invalid("semantic merge item ID overflows"))?;
    }

    let mut records = Vec::with_capacity(semantic_images.len() + semantic_metadata.len());
    for old_id in semantic_images
        .iter()
        .chain(semantic_metadata.iter())
        .copied()
    {
        let new_id = *id_map
            .get(&old_id)
            .ok_or_else(|| invalid(format!("semantic item {old_id} has no allocated ID")))?;
        records.push((new_id, old_id));
    }
    let imported_images = semantic_images
        .iter()
        .map(|old_id| {
            id_map
                .get(old_id)
                .copied()
                .map(|new_id| (new_id, *old_id))
                .ok_or_else(|| invalid(format!("semantic image {old_id} has no allocated ID")))
        })
        .collect::<Result<Vec<_>>>()?;
    let imported_payloads = semantic_images
        .iter()
        .map(|old_id| item_payload(scaffold_data, &scaffold, *old_id, "semantic image"))
        .collect::<Result<Vec<_>>>()?;

    let iinf = make_iinf(source_data, scaffold_data, &source, &scaffold, &records)?;
    let iprp = make_iprp(
        source_data,
        scaffold_data,
        &source,
        &scaffold,
        &imported_images,
    )?;

    let mut output_refs = source
        .meta
        .iref
        .as_ref()
        .map_or_else(Vec::new, |iref| iref.entries.clone());
    // The Rust native writer historically emitted an Exif descriptor for the
    // primary grid only, while ImageIO-authored HDR files describe both the
    // primary and its tone-map item.  Keep the source reference authoritative,
    // but complete that established HDR descriptor contract before adding
    // semantic resources.  ImageIO otherwise treats the resulting graph as a
    // generic HEIF once auxiliary items are present, even though the source
    // Gain Map payload and graph are unchanged.
    for reference in &mut output_refs {
        if reference.kind == CDSC
            && reference.from_item_id == source_exif
            && reference.to_item_ids.contains(&source.meta.primary_item_id)
            && !reference.to_item_ids.contains(&source_tmap)
        {
            reference.to_item_ids.push(source_tmap);
        }
    }
    if let Some(iref) = scaffold.meta.iref.as_ref() {
        for reference in &iref.entries {
            if id_map.contains_key(&reference.from_item_id) {
                output_refs.push(map_semantic_reference(
                    reference,
                    scaffold.meta.primary_item_id,
                    scaffold_tmap,
                    source.meta.primary_item_id,
                    source_tmap,
                    &id_map,
                )?);
            }
        }
    }
    let maximum_reference_id = output_refs
        .iter()
        .flat_map(|reference| {
            std::iter::once(reference.from_item_id).chain(reference.to_item_ids.iter().copied())
        })
        .max()
        .unwrap_or(0);
    let iref_version = if maximum_reference_id > u32::from(u16::MAX) {
        1
    } else {
        source.meta.iref.as_ref().map_or(0, |iref| iref.version)
    };
    let iref = make_iref_box(iref_version, &output_refs)?;

    let source_idat_payload = source
        .meta
        .idat
        .as_ref()
        .map_or_else(Vec::new, |idat| source_data[idat.payload_range()].to_vec());
    let mut idat_payload = source_idat_payload;
    let mut idat_locations = BTreeMap::new();
    for old_id in &semantic_metadata {
        let payload = item_payload(scaffold_data, &scaffold, *old_id, "semantic metadata")?;
        let offset = u64::try_from(idat_payload.len())
            .map_err(|_| invalid("semantic metadata idat offset exceeds u64"))?;
        idat_payload.extend_from_slice(&payload);
        idat_locations.insert(
            *old_id,
            (offset, u64::try_from(payload.len()).unwrap_or(u64::MAX)),
        );
    }
    let idat = make_box(IDAT, &idat_payload)?;

    let source_mdat_payload = source_data[source.mdat.payload_range()].to_vec();
    let mut placeholder_locations = Vec::new();
    for entry in &source.meta.iloc.entries {
        if entry.item_id == source_exif {
            placeholder_locations.push(IlocEntry {
                item_id: source_exif,
                construction_method: 0,
                data_reference_index: 0,
                base_offset: 0,
                extents: vec![IlocExtent {
                    index: None,
                    offset: 0,
                    length: u64::try_from(scaffold_exif_payload.len())
                        .map_err(|_| invalid("semantic Exif payload length exceeds u64"))?,
                }],
            });
        } else {
            placeholder_locations.push(normalized_source_location(
                source_data,
                &source,
                entry,
                true,
            )?);
        }
    }
    for ((new_id, old_id), payload) in imported_images.iter().zip(imported_payloads.iter()) {
        placeholder_locations.push(IlocEntry {
            item_id: *new_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: u64::try_from(payload.len())
                    .map_err(|_| invalid(format!("semantic image {old_id} length exceeds u64")))?,
            }],
        });
    }
    for old_id in &semantic_metadata {
        let new_id = *id_map
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no allocated ID")))?;
        let (offset, length) = *idat_locations
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no idat location")))?;
        placeholder_locations.push(IlocEntry {
            item_id: new_id,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length,
            }],
        });
    }
    placeholder_locations.sort_by_key(|entry| entry.item_id);
    let maximum_item_id = placeholder_locations
        .iter()
        .map(|entry| entry.item_id)
        .max()
        .unwrap_or(0);
    let iloc_version = if maximum_item_id > u32::from(u16::MAX) {
        2
    } else {
        source.meta.iloc.version.max(1)
    };
    let placeholder_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &placeholder_locations)?;
    let preliminary_meta = make_meta(
        source_data,
        &source,
        &iinf,
        &placeholder_iloc,
        &iprp,
        &iref,
        &idat,
    )?;

    let new_mdat_box_start = source
        .top
        .boxes
        .iter()
        .take_while(|header| header.box_start != source.mdat.box_start)
        .try_fold(0usize, |offset, header| {
            let replacement_len = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(replacement_len)
                .ok_or_else(|| invalid("semantic merge mdat offset overflows"))
        })?;
    let new_mdat_data_start = u64::try_from(
        new_mdat_box_start
            .checked_add(8)
            .ok_or_else(|| invalid("semantic merge mdat data offset overflows"))?,
    )
    .map_err(|_| invalid("semantic merge mdat data offset exceeds u64"))?;

    let mut final_locations = Vec::new();
    for entry in &source.meta.iloc.entries {
        if entry.item_id == source_exif {
            continue;
        }
        final_locations.push(normalized_source_final_location(
            source_data,
            &source,
            entry,
            new_mdat_data_start,
        )?);
    }
    let mut appended_mdat = Vec::new();
    let exif_offset = new_mdat_data_start
        .checked_add(u64::try_from(source_mdat_payload.len()).unwrap_or(u64::MAX))
        .ok_or_else(|| invalid("semantic Exif offset overflows"))?;
    appended_mdat.extend_from_slice(&scaffold_exif_payload);
    final_locations.push(IlocEntry {
        item_id: source_exif,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: exif_offset,
            length: u64::try_from(scaffold_exif_payload.len())
                .map_err(|_| invalid("semantic Exif payload length exceeds u64"))?,
        }],
    });
    for ((new_id, old_id), payload) in imported_images.iter().zip(imported_payloads.iter()) {
        let offset = new_mdat_data_start
            .checked_add(u64::try_from(source_mdat_payload.len()).unwrap_or(u64::MAX))
            .and_then(|value| {
                value.checked_add(u64::try_from(appended_mdat.len()).unwrap_or(u64::MAX))
            })
            .ok_or_else(|| invalid(format!("semantic image {old_id} offset overflows")))?;
        appended_mdat.extend_from_slice(payload);
        final_locations.push(IlocEntry {
            item_id: *new_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length: u64::try_from(payload.len())
                    .map_err(|_| invalid(format!("semantic image {old_id} length exceeds u64")))?,
            }],
        });
    }
    for old_id in &semantic_metadata {
        let new_id = *id_map
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no allocated ID")))?;
        let (offset, length) = *idat_locations
            .get(old_id)
            .ok_or_else(|| invalid(format!("semantic metadata {old_id} has no idat location")))?;
        final_locations.push(IlocEntry {
            item_id: new_id,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length,
            }],
        });
    }
    final_locations.sort_by_key(|entry| entry.item_id);
    let final_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &final_locations)?;
    let final_meta = make_meta(
        source_data,
        &source,
        &iinf,
        &final_iloc,
        &iprp,
        &iref,
        &idat,
    )?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid("semantic merge iloc rewrite changed meta size"));
    }

    let mut final_mdat_payload = source_mdat_payload.clone();
    final_mdat_payload.extend_from_slice(&appended_mdat);
    let final_mdat = make_box(MDAT, &final_mdat_payload)?;
    let mut output = Vec::new();
    for header in &source.top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else if header.kind == MDAT {
            output.extend_from_slice(&final_mdat);
        } else {
            output.extend_from_slice(raw_box(source_data, header, "source top-level")?);
        }
    }
    output.extend_from_slice(
        source_data
            .get(source.top.trailing_range.clone())
            .ok_or_else(|| invalid("source top-level trailing bytes are outside input"))?,
    );

    let output_graph = parse_graph(&output, "semantic merge output")?;
    if output_graph.meta.primary_item_id != source.meta.primary_item_id {
        return Err(invalid("semantic merge changed the primary item"));
    }
    if output_graph.mdat.payload_range().len() < source_mdat_payload.len()
        || output[output_graph.mdat.payload_range().start
            ..output_graph.mdat.payload_range().start + source_mdat_payload.len()]
            != source_mdat_payload
    {
        return Err(invalid("semantic merge changed the source mdat prefix"));
    }
    for new_id in id_map.values().copied() {
        let _ = item_payload(&output, &output_graph, new_id, "semantic merge output")?;
    }
    Ok(output)
}

fn required_property_index(
    meta: &ParsedMeta,
    item_id: u32,
    kind: FourCC,
    context: &str,
) -> Result<u16> {
    let matches = associated_properties(meta, item_id)?
        .into_iter()
        .filter(|association| {
            meta.properties.iter().any(|property| {
                property.index == u32::from(association.property_index) && property.kind == kind
            })
        })
        .map(|association| association.property_index)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(invalid(format!(
            "{context} item {item_id} has no {kind} property"
        ))),
        _ => Err(invalid(format!(
            "{context} item {item_id} has multiple {kind} properties"
        ))),
    }
}

fn uniform_property(
    data: &[u8],
    meta: &ParsedMeta,
    item_ids: &[u32],
    kind: FourCC,
    context: &str,
) -> Result<(u16, Vec<u8>)> {
    let mut result: Option<(u16, Vec<u8>)> = None;
    for item_id in item_ids {
        let index = required_property_index(meta, *item_id, kind, context)?;
        let property = meta
            .properties
            .iter()
            .find(|property| property.index == u32::from(index))
            .ok_or_else(|| invalid(format!("{context} property {index} is missing")))?;
        let raw = raw_property(data, property)?.to_vec();
        if let Some((expected_index, expected_raw)) = &result {
            if *expected_index != index || *expected_raw != raw {
                return Err(invalid(format!(
                    "{context} items do not share one compatible {kind} property"
                )));
            }
        } else {
            result = Some((index, raw));
        }
    }
    result.ok_or_else(|| invalid(format!("{context} has no items")))
}

fn uniform_property_index(
    meta: &ParsedMeta,
    item_ids: &[u32],
    kind: FourCC,
    context: &str,
) -> Result<u16> {
    let mut result = None;
    for item_id in item_ids {
        let index = required_property_index(meta, *item_id, kind, context)?;
        if let Some(expected) = result {
            if expected != index {
                return Err(invalid(format!(
                    "{context} items do not share one {kind} property index"
                )));
            }
        } else {
            result = Some(index);
        }
    }
    result.ok_or_else(|| invalid(format!("{context} has no items")))
}

fn dimg_targets(meta: &ParsedMeta, from_item_id: u32, context: &str) -> Result<Vec<u32>> {
    let references = meta
        .iref
        .as_ref()
        .ok_or_else(|| invalid(format!("{context} has no iref graph")))?
        .entries
        .iter()
        .filter(|reference| reference.kind == DIMG && reference.from_item_id == from_item_id)
        .collect::<Vec<_>>();
    match references.as_slice() {
        [reference] if !reference.to_item_ids.is_empty() => Ok(reference.to_item_ids.clone()),
        [] => Err(invalid(format!(
            "{context} item {from_item_id} has no dimg reference"
        ))),
        [_] => Err(invalid(format!(
            "{context} item {from_item_id} has an empty dimg reference"
        ))),
        _ => Err(invalid(format!(
            "{context} item {from_item_id} has multiple dimg references"
        ))),
    }
}

fn item_is_type(meta: &ParsedMeta, item_id: u32, item_type: FourCC) -> bool {
    meta.iinf
        .entries
        .iter()
        .any(|item| item.item_id == item_id && item.item_type == Some(item_type))
}

fn coded_image_items(meta: &ParsedMeta, image_id: u32, context: &str) -> Result<Vec<u32>> {
    let targets = match item_info(meta, image_id, context)?.item_type {
        Some(HVC1) => vec![image_id],
        Some(GRID) => dimg_targets(meta, image_id, context)?,
        _ => {
            return Err(invalid(format!(
                "{context} image {image_id} is neither hvc1 nor grid"
            )))
        }
    };
    let mut seen = BTreeSet::new();
    for item_id in &targets {
        if !seen.insert(*item_id) || !item_is_type(meta, *item_id, HVC1) {
            return Err(invalid(format!(
                "{context} image {image_id} has invalid or repeated hvc1 items"
            )));
        }
        if meta.iref.as_ref().is_some_and(|iref| {
            iref.entries
                .iter()
                .any(|reference| reference.kind == DIMG && reference.from_item_id == *item_id)
        }) {
            return Err(invalid(format!(
                "{context} hvc1 item {item_id} has a dimg reference"
            )));
        }
    }
    Ok(targets)
}

fn hdr_image_items(meta: &ParsedMeta, context: &str) -> Result<(u32, Vec<u32>, Vec<u32>)> {
    let tmap_id = find_tmap(meta, context)?;
    let tmap_targets = dimg_targets(meta, tmap_id, context)?;
    if tmap_targets.len() != 2
        || !tmap_targets.contains(&meta.primary_item_id)
        || tmap_targets[0] == tmap_targets[1]
    {
        return Err(invalid(format!(
            "{context} tmap must reference the primary and one Gain Map image"
        )));
    }
    let gain_image = tmap_targets
        .into_iter()
        .find(|id| *id != meta.primary_item_id)
        .ok_or_else(|| invalid(format!("{context} tmap has no Gain Map image")))?;
    let primary_items = coded_image_items(meta, meta.primary_item_id, context)?;
    let gain_items = coded_image_items(meta, gain_image, context)?;
    if gain_items.iter().any(|id| primary_items.contains(id)) {
        return Err(invalid(format!(
            "{context} primary and Gain Map share coded image items"
        )));
    }
    Ok((gain_image, primary_items, gain_items))
}

fn rebuild_iprp_properties(
    data: &[u8],
    graph: &Graph,
    replacements: &BTreeMap<u32, Vec<u8>>,
) -> Result<Vec<u8>> {
    let iprp = child(&graph.meta_children, IPRP, "semantic scaffold meta")?;
    let children = parse_boxes(data, iprp.payload_range())?;
    let mut payload = Vec::new();
    let mut saw_ipco = false;
    for header in &children {
        if header.kind != IPCO {
            payload.extend_from_slice(raw_box(data, header, "semantic scaffold iprp child")?);
            continue;
        }
        if saw_ipco {
            return Err(invalid(
                "semantic scaffold iprp contains multiple ipco boxes",
            ));
        }
        saw_ipco = true;
        let properties = parse_boxes(data, header.payload_range())?;
        let mut ipco_payload = Vec::new();
        for (position, property) in properties.iter().enumerate() {
            let index = u32::try_from(position + 1)
                .map_err(|_| invalid("semantic scaffold property index overflows"))?;
            if let Some(replacement) = replacements.get(&index) {
                ipco_payload.extend_from_slice(replacement);
            } else {
                ipco_payload.extend_from_slice(raw_box(
                    data,
                    property,
                    "semantic scaffold ipco property",
                )?);
            }
        }
        payload.extend_from_slice(&make_box(IPCO, &ipco_payload)?);
    }
    if !saw_ipco {
        return Err(invalid("semantic scaffold iprp has no ipco box"));
    }
    Ok(make_box(IPRP, &payload)?)
}

fn rebuild_meta_children(data: &[u8], graph: &Graph, iloc: &[u8], iprp: &[u8]) -> Result<Vec<u8>> {
    let full_header_end = graph
        .meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("semantic scaffold meta full-box header overflows"))?;
    let full_header = data
        .get(graph.meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("semantic scaffold meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    let mut saw_iloc = false;
    let mut saw_iprp = false;
    for header in &graph.meta_children {
        match header.kind {
            ILOC if !saw_iloc => {
                saw_iloc = true;
                payload.extend_from_slice(iloc);
            }
            ILOC => {
                return Err(invalid(
                    "semantic scaffold meta contains multiple iloc boxes",
                ))
            }
            IPRP if !saw_iprp => {
                saw_iprp = true;
                payload.extend_from_slice(iprp);
            }
            IPRP => {
                return Err(invalid(
                    "semantic scaffold meta contains multiple iprp boxes",
                ))
            }
            _ => payload.extend_from_slice(raw_box(data, header, "semantic scaffold meta child")?),
        }
    }
    if !saw_iloc || !saw_iprp {
        return Err(invalid("semantic scaffold meta is missing iloc or iprp"));
    }
    Ok(make_box(META, &payload)?)
}

#[derive(Debug)]
struct PayloadReplacement {
    item_id: u32,
    old_start: u64,
    old_end: u64,
    payload: Vec<u8>,
}

fn payload_replacements(
    source_data: &[u8],
    source: &Graph,
    scaffold: &Graph,
    source_item_ids: &[u32],
    scaffold_item_ids: &[u32],
    context: &str,
) -> Result<Vec<PayloadReplacement>> {
    if source_item_ids.len() != scaffold_item_ids.len() || source_item_ids.is_empty() {
        return Err(invalid(format!(
            "{context} source/scaffold tile counts differ"
        )));
    }
    let mdat_start = u64::try_from(scaffold.mdat.data_start)
        .map_err(|_| invalid("semantic scaffold mdat offset exceeds u64"))?;
    let mdat_end = u64::try_from(scaffold.mdat.data_end)
        .map_err(|_| invalid("semantic scaffold mdat end exceeds u64"))?;
    let mut replacements = Vec::with_capacity(source_item_ids.len());
    for (source_id, scaffold_id) in source_item_ids.iter().zip(scaffold_item_ids) {
        let source_payload = item_payload(source_data, source, *source_id, context)?;
        let entry = item_location(&scaffold.meta, *scaffold_id, context)?;
        if entry.construction_method != 0 || entry.data_reference_index != 0 {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} is not a local file item"
            )));
        }
        if entry.extents.len() != 1 {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} does not have one extent"
            )));
        }
        let extent = &entry.extents[0];
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} has a non-zero extent index"
            )));
        }
        let old_start = entry.resolved_extent_offset(extent)?;
        let old_end = old_start.checked_add(extent.length).ok_or_else(|| {
            invalid(format!(
                "{context} scaffold item {scaffold_id} extent overflows"
            ))
        })?;
        if old_start < mdat_start || old_end > mdat_end {
            return Err(invalid(format!(
                "{context} scaffold item {scaffold_id} extent is outside mdat"
            )));
        }
        replacements.push(PayloadReplacement {
            item_id: *scaffold_id,
            old_start,
            old_end,
            payload: source_payload,
        });
    }
    replacements.sort_by_key(|replacement| replacement.old_start);
    for pair in replacements.windows(2) {
        if pair[0].old_end > pair[1].old_start {
            return Err(invalid(format!("{context} scaffold tile extents overlap")));
        }
    }
    Ok(replacements)
}

fn apply_replacement_delta(value: u64, old_length: u64, new_length: u64) -> Result<u64> {
    if new_length >= old_length {
        value
            .checked_add(new_length - old_length)
            .ok_or_else(|| invalid("semantic scaffold mdat offset overflows"))
    } else {
        value
            .checked_sub(old_length - new_length)
            .ok_or_else(|| invalid("semantic scaffold mdat offset underflows"))
    }
}

fn translated_mdat_offset(
    old_offset: u64,
    old_mdat_start: u64,
    replacements: &[PayloadReplacement],
) -> Result<u64> {
    let relative = old_offset
        .checked_sub(old_mdat_start)
        .ok_or_else(|| invalid("semantic scaffold extent precedes mdat"))?;
    let mut translated = relative;
    for replacement in replacements {
        if replacement.old_end <= old_offset {
            translated = apply_replacement_delta(
                translated,
                replacement.old_end - replacement.old_start,
                u64::try_from(replacement.payload.len())
                    .map_err(|_| invalid("semantic replacement payload exceeds u64"))?,
            )?;
        } else if replacement.old_start < old_offset {
            return Err(invalid(
                "semantic scaffold extent begins inside a replaced tile payload",
            ));
        }
    }
    Ok(translated)
}

fn replaced_mdat_payload(
    data: &[u8],
    graph: &Graph,
    replacements: &[PayloadReplacement],
) -> Result<Vec<u8>> {
    let original = data
        .get(graph.mdat.payload_range())
        .ok_or_else(|| invalid("semantic scaffold mdat payload is outside input"))?;
    let mdat_start = u64::try_from(graph.mdat.data_start)
        .map_err(|_| invalid("semantic scaffold mdat offset exceeds u64"))?;
    let mut output = Vec::new();
    let mut cursor = 0usize;
    for replacement in replacements {
        let relative_start = usize::try_from(
            replacement
                .old_start
                .checked_sub(mdat_start)
                .ok_or_else(|| invalid("semantic replacement precedes mdat"))?,
        )
        .map_err(|_| invalid("semantic replacement offset exceeds usize"))?;
        let relative_end = usize::try_from(
            replacement
                .old_end
                .checked_sub(mdat_start)
                .ok_or_else(|| invalid("semantic replacement end precedes mdat"))?,
        )
        .map_err(|_| invalid("semantic replacement end exceeds usize"))?;
        if relative_start < cursor || relative_end > original.len() {
            return Err(invalid(
                "semantic replacement range is outside mdat payload",
            ));
        }
        output.extend_from_slice(&original[cursor..relative_start]);
        output.extend_from_slice(&replacement.payload);
        cursor = relative_end;
    }
    output.extend_from_slice(&original[cursor..]);
    Ok(output)
}

fn transplant_iloc_locations(
    scaffold: &Graph,
    replacements: &[PayloadReplacement],
    new_mdat_data_start: u64,
) -> Result<Vec<IlocEntry>> {
    let old_mdat_data_start = u64::try_from(scaffold.mdat.data_start)
        .map_err(|_| invalid("semantic scaffold mdat offset exceeds u64"))?;
    let mut entries = Vec::with_capacity(scaffold.meta.iloc.entries.len());
    for entry in &scaffold.meta.iloc.entries {
        let mut output = entry.clone();
        if entry.construction_method == 0 {
            if entry.data_reference_index != 0 {
                return Err(invalid(format!(
                    "semantic scaffold item {} uses a data reference",
                    entry.item_id
                )));
            }
            output.base_offset = 0;
            for extent in &mut output.extents {
                if extent.index.unwrap_or(0) != 0 {
                    return Err(invalid(format!(
                        "semantic scaffold item {} uses a non-zero extent index",
                        entry.item_id
                    )));
                }
                let old_start = entry
                    .resolved_extent_offset(extent)
                    .map_err(|_| invalid("semantic scaffold extent offset overflows"))?;
                let new_relative =
                    translated_mdat_offset(old_start, old_mdat_data_start, replacements)?;
                extent.offset = new_mdat_data_start
                    .checked_add(new_relative)
                    .ok_or_else(|| invalid("semantic transplanted extent offset overflows"))?;
                if let Some(replacement) = replacements
                    .iter()
                    .find(|replacement| replacement.item_id == entry.item_id)
                {
                    let old_end = old_start
                        .checked_add(extent.length)
                        .ok_or_else(|| invalid("semantic scaffold tile extent overflows"))?;
                    if old_start != replacement.old_start || old_end != replacement.old_end {
                        return Err(invalid(format!(
                            "semantic scaffold item {} has an unexpected replacement extent",
                            entry.item_id
                        )));
                    }
                    extent.length = u64::try_from(replacement.payload.len())
                        .map_err(|_| invalid("semantic replacement payload exceeds u64"))?;
                }
            }
        }
        entries.push(output);
    }
    Ok(entries)
}

/// Build the ImageIO-compatible semantic scaffold through the platform
/// primitive, then transplant source primary, Gain Map, and paired alpha payloads
/// and their matching codec configurations into that graph.  The scaffold's
/// item/property/reference model remains authoritative for Apple semantic
/// resources; Rust remains authoritative for the compressed HDR payloads.
pub fn transplant_apple_semantic_auxiliary_heif(
    source_data: &[u8],
    scaffold_data: &[u8],
    expected_semantic_roles: usize,
) -> Result<Vec<u8>> {
    if expected_semantic_roles == 0 {
        return Err(invalid("semantic transplant requires at least one role"));
    }
    let source = parse_graph(source_data, "source HDR")?;
    let scaffold = parse_graph(scaffold_data, "semantic scaffold")?;
    let (source_alpha, scaffold_alpha) =
        paired_alpha_images(source_data, &source, scaffold_data, &scaffold).map_err(|error| {
            semantic_graph_error(error, source_data, &source, scaffold_data, &scaffold)
        })?;
    let (_, semantic_images, semantic_metadata) =
        semantic_items(scaffold_data, &scaffold, scaffold_alpha.as_ref()).map_err(|error| {
            semantic_graph_error(error, source_data, &source, scaffold_data, &scaffold)
        })?;
    if semantic_images.len() != expected_semantic_roles {
        return Err(invalid(format!(
            "semantic scaffold contains {} roles; expected {expected_semantic_roles}",
            semantic_images.len()
        )));
    }

    let (source_gain_image, source_primary_tiles, source_gain_tiles) =
        hdr_image_items(&source.meta, "source HDR")?;
    let (_, scaffold_primary_tiles, scaffold_gain_tiles) =
        hdr_image_items(&scaffold.meta, "semantic scaffold")?;
    if source.meta.iref.as_ref().is_some_and(|iref| {
        iref.entries.iter().any(|reference| {
            reference.kind == AUXL
                && reference.from_item_id != source_gain_image
                && !source_alpha
                    .as_ref()
                    .is_some_and(|alpha| alpha.root == reference.from_item_id)
        })
    }) {
        return Err(semantic_graph_error(
            invalid("semantic transplant cannot discard an unpaired source auxiliary"),
            source_data,
            &source,
            scaffold_data,
            &scaffold,
        ));
    }
    let source_alpha_tiles = source_alpha
        .as_ref()
        .map_or(&[][..], |alpha| alpha.coded_items.as_slice());
    let scaffold_alpha_tiles = scaffold_alpha
        .as_ref()
        .map_or(&[][..], |alpha| alpha.coded_items.as_slice());
    if source_primary_tiles.len() != scaffold_primary_tiles.len()
        || source_gain_tiles.len() != scaffold_gain_tiles.len()
    {
        return Err(invalid(
            "source and semantic scaffold tile graph counts differ",
        ));
    }

    let mut codec_replacements = BTreeMap::new();
    for (source_ids, scaffold_ids, context) in [
        (
            source_primary_tiles.as_slice(),
            scaffold_primary_tiles.as_slice(),
            "primary tiles",
        ),
        (
            source_gain_tiles.as_slice(),
            scaffold_gain_tiles.as_slice(),
            "Gain Map tiles",
        ),
        (source_alpha_tiles, scaffold_alpha_tiles, "alpha tiles"),
    ] {
        if source_ids.is_empty() {
            continue;
        }
        let (_, source_raw) =
            uniform_property(source_data, &source.meta, source_ids, HVCC, context)?;
        let scaffold_index = uniform_property_index(&scaffold.meta, scaffold_ids, HVCC, context)?;
        let property_index = u32::from(scaffold_index);
        if let Some(existing) = codec_replacements.get(&property_index) {
            if *existing != source_raw {
                return Err(invalid(format!(
                    "semantic scaffold reuses hvcC property {} for incompatible codecs",
                    scaffold_index
                )));
            }
        } else {
            codec_replacements.insert(property_index, source_raw);
        }
    }
    let replaced_items = scaffold_primary_tiles
        .iter()
        .chain(&scaffold_gain_tiles)
        .chain(scaffold_alpha_tiles)
        .copied()
        .collect::<BTreeSet<_>>();
    for entry in &scaffold.meta.ipma.entries {
        if replaced_items.contains(&entry.item_id) {
            continue;
        }
        for association in &entry.associations {
            if let Some(replacement) =
                codec_replacements.get(&u32::from(association.property_index))
            {
                let property = scaffold
                    .meta
                    .properties
                    .iter()
                    .find(|property| property.index == u32::from(association.property_index))
                    .ok_or_else(|| invalid("shared codec property is missing"))?;
                if replacement.as_slice() != raw_property(scaffold_data, property)? {
                    return Err(invalid(
                        "semantic transplant would change an unreplaced item's shared codec",
                    ));
                }
            }
        }
    }
    let iprp = rebuild_iprp_properties(scaffold_data, &scaffold, &codec_replacements)?;
    let preliminary_iloc = make_iloc_box(
        scaffold.meta.iloc.version,
        scaffold.meta.iloc.offset_size,
        scaffold.meta.iloc.length_size,
        scaffold.meta.iloc.base_offset_size,
        scaffold.meta.iloc.index_size,
        &scaffold.meta.iloc.entries,
    )?;
    let preliminary_meta =
        rebuild_meta_children(scaffold_data, &scaffold, &preliminary_iloc, &iprp)?;

    let new_mdat_box_start = scaffold
        .top
        .boxes
        .iter()
        .take_while(|header| header.box_start != scaffold.mdat.box_start)
        .try_fold(0usize, |offset, header| {
            let replacement_len = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(replacement_len)
                .ok_or_else(|| invalid("semantic transplanted mdat offset overflows"))
        })?;
    let new_mdat_data_start = u64::try_from(
        new_mdat_box_start
            .checked_add(8)
            .ok_or_else(|| invalid("semantic transplanted mdat data offset overflows"))?,
    )
    .map_err(|_| invalid("semantic transplanted mdat data offset exceeds u64"))?;

    let mut replacements = payload_replacements(
        source_data,
        &source,
        &scaffold,
        &source_primary_tiles,
        &scaffold_primary_tiles,
        "primary tiles",
    )?;
    replacements.extend(payload_replacements(
        source_data,
        &source,
        &scaffold,
        &source_gain_tiles,
        &scaffold_gain_tiles,
        "Gain Map tiles",
    )?);
    if !source_alpha_tiles.is_empty() {
        replacements.extend(payload_replacements(
            source_data,
            &source,
            &scaffold,
            source_alpha_tiles,
            scaffold_alpha_tiles,
            "alpha tiles",
        )?);
    }
    replacements.sort_by_key(|replacement| replacement.old_start);
    for pair in replacements.windows(2) {
        if pair[0].old_end > pair[1].old_start {
            return Err(invalid("semantic scaffold replacement extents overlap"));
        }
    }

    let final_iloc_entries =
        transplant_iloc_locations(&scaffold, &replacements, new_mdat_data_start)?;
    let final_iloc = make_iloc_box(
        scaffold.meta.iloc.version,
        scaffold.meta.iloc.offset_size,
        scaffold.meta.iloc.length_size,
        scaffold.meta.iloc.base_offset_size,
        scaffold.meta.iloc.index_size,
        &final_iloc_entries,
    )?;
    let final_meta = rebuild_meta_children(scaffold_data, &scaffold, &final_iloc, &iprp)?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid(
            "semantic transplant iloc rewrite changed meta size",
        ));
    }
    let final_mdat_payload = replaced_mdat_payload(scaffold_data, &scaffold, &replacements)?;
    let final_mdat = make_box(MDAT, &final_mdat_payload)?;

    let mut output = Vec::new();
    for header in &scaffold.top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else if header.kind == MDAT {
            output.extend_from_slice(&final_mdat);
        } else {
            output.extend_from_slice(raw_box(
                scaffold_data,
                header,
                "semantic scaffold top-level",
            )?);
        }
    }
    output.extend_from_slice(
        scaffold_data
            .get(scaffold.top.trailing_range.clone())
            .ok_or_else(|| invalid("semantic scaffold trailing bytes are outside input"))?,
    );

    let output_graph = parse_graph(&output, "semantic transplant output")?;
    if output_graph.meta.primary_item_id != scaffold.meta.primary_item_id {
        return Err(invalid("semantic transplant changed the primary item"));
    }
    for (source_id, scaffold_id) in source_primary_tiles
        .iter()
        .chain(source_gain_tiles.iter())
        .chain(source_alpha_tiles.iter())
        .zip(
            scaffold_primary_tiles
                .iter()
                .chain(scaffold_gain_tiles.iter())
                .chain(scaffold_alpha_tiles.iter()),
        )
    {
        let source_payload = item_payload(source_data, &source, *source_id, "source tile")?;
        let output_payload = item_payload(&output, &output_graph, *scaffold_id, "output tile")?;
        if source_payload != output_payload {
            return Err(invalid(format!(
                "semantic transplant changed Rust tile payload {source_id} -> {scaffold_id}"
            )));
        }
    }
    for item_id in semantic_images.iter().chain(semantic_metadata.iter()) {
        let expected = item_payload(scaffold_data, &scaffold, *item_id, "semantic scaffold item")?;
        let actual = item_payload(&output, &output_graph, *item_id, "semantic output item")?;
        if expected != actual {
            return Err(invalid(format!(
                "semantic transplant changed scaffold item payload {item_id}"
            )));
        }
    }
    for (index, raw) in &codec_replacements {
        let property = output_graph
            .meta
            .properties
            .iter()
            .find(|property| property.index == *index)
            .ok_or_else(|| invalid(format!("semantic output hvcC property {index} is missing")))?;
        if raw_property(&output, property)? != raw {
            return Err(invalid(format!(
                "semantic output hvcC property {index} differs from Rust codec"
            )));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::{make_infe_box, make_pitm_box};

    const PRIMARY: u32 = 1;
    const GAIN: u32 = 2;
    const TONE_MAP: u32 = 3;
    const EXIF_ID: u32 = 4;
    const SEMANTICS: [u32; 3] = [10, 11, 12];
    const DESCRIPTORS: [u32; 3] = [20, 21, 22];
    const PRIMARY_TILES: [u32; 2] = [100, 101];
    const GAIN_TILES: [u32; 2] = [110, 111];

    // These are synthetic container graphs and opaque compressed payloads;
    // they exercise the public assembly contract, not native HEVC decoding.
    struct Fixture {
        items: Vec<(u32, FourCC, Vec<u8>)>,
        references: Vec<IrefEntry>,
        codec_marker: u8,
        auxiliary_properties: Vec<(u32, Vec<u8>)>,
    }

    impl Fixture {
        fn new(grid: bool, semantic: bool, label: &str) -> Self {
            let root_type = if grid { GRID } else { HVC1 };
            let mut items = vec![
                (PRIMARY, root_type, format!("{label}-primary").into_bytes()),
                (GAIN, root_type, format!("{label}-gain").into_bytes()),
                (TONE_MAP, TMAP, b"tone-map-metadata".to_vec()),
                (EXIF_ID, EXIF, format!("{label}-exif").into_bytes()),
            ];
            let mut references = vec![
                IrefEntry {
                    kind: DIMG,
                    from_item_id: TONE_MAP,
                    to_item_ids: vec![PRIMARY, GAIN],
                },
                IrefEntry {
                    kind: AUXL,
                    from_item_id: GAIN,
                    to_item_ids: vec![PRIMARY, TONE_MAP],
                },
                IrefEntry {
                    kind: CDSC,
                    from_item_id: EXIF_ID,
                    to_item_ids: vec![PRIMARY, TONE_MAP],
                },
            ];
            if grid {
                for (root, tiles, role) in [
                    (PRIMARY, PRIMARY_TILES, "primary"),
                    (GAIN, GAIN_TILES, "gain"),
                ] {
                    references.push(IrefEntry {
                        kind: DIMG,
                        from_item_id: root,
                        to_item_ids: tiles.to_vec(),
                    });
                    for (index, item_id) in tiles.into_iter().enumerate() {
                        items.push((
                            item_id,
                            HVC1,
                            format!("{label}-{role}-tile-{index}").into_bytes(),
                        ));
                    }
                }
            }
            if semantic {
                for (index, (image, descriptor)) in
                    SEMANTICS.into_iter().zip(DESCRIPTORS).enumerate()
                {
                    items.push((
                        image,
                        HVC1,
                        format!("scaffold-semantic-{index}").into_bytes(),
                    ));
                    items.push((descriptor, MIME, format!("descriptor-{index}").into_bytes()));
                    references.extend([
                        IrefEntry {
                            kind: AUXL,
                            from_item_id: image,
                            to_item_ids: vec![PRIMARY, TONE_MAP],
                        },
                        IrefEntry {
                            kind: CDSC,
                            from_item_id: descriptor,
                            to_item_ids: vec![image],
                        },
                    ]);
                }
            }
            Self {
                items,
                references,
                codec_marker: if semantic { 7 } else { 9 },
                auxiliary_properties: Vec::new(),
            }
        }

        fn bytes(&self) -> Vec<u8> {
            let infos = self
                .items
                .iter()
                .map(|(id, kind, _)| make_infe_box(*id, *kind, 0).unwrap())
                .collect::<Vec<_>>();
            let iinf = make_iinf_box(0, &infos).unwrap();
            let mut properties = [
                make_box(HVCC, &[self.codec_marker; 19]).unwrap(),
                make_box(HVCC, &[self.codec_marker + 1; 21]).unwrap(),
                make_box(HVCC, &[5; 19]).unwrap(),
            ]
            .concat();
            for (_, property) in &self.auxiliary_properties {
                properties.extend_from_slice(property);
            }
            let mut associations = self
                .items
                .iter()
                .map(|(id, kind, _)| IpmaEntry {
                    item_id: *id,
                    associations: if *kind == HVC1 {
                        vec![IpmaAssociation {
                            property_index: if *id == PRIMARY || PRIMARY_TILES.contains(id) {
                                1
                            } else if *id == GAIN || GAIN_TILES.contains(id) {
                                2
                            } else {
                                3
                            },
                            essential: true,
                        }]
                    } else {
                        Vec::new()
                    },
                })
                .collect::<Vec<_>>();
            for (index, (item_id, property)) in self.auxiliary_properties.iter().enumerate() {
                let entry = associations
                    .iter_mut()
                    .find(|entry| entry.item_id == *item_id)
                    .unwrap();
                if property.get(4..8) == Some(HVCC.as_bytes()) {
                    entry
                        .associations
                        .retain(|association| association.property_index > 3);
                }
                entry.associations.push(IpmaAssociation {
                    property_index: u16::try_from(index + 4).unwrap(),
                    essential: true,
                });
            }
            let iprp = make_box(
                IPRP,
                &[
                    make_box(IPCO, &properties).unwrap(),
                    make_ipma_box(0, 0, &associations).unwrap(),
                ]
                .concat(),
            )
            .unwrap();
            let iref = make_iref_box(0, &self.references).unwrap();
            let ftyp = make_box(FourCC::new(*b"ftyp"), b"mif1\0\0\0\0").unwrap();
            let build_meta = |mdat_start: u64| {
                let mut offset = mdat_start;
                let locations = self
                    .items
                    .iter()
                    .map(|(id, _, payload)| {
                        let entry = IlocEntry {
                            item_id: *id,
                            construction_method: 0,
                            data_reference_index: 0,
                            base_offset: 0,
                            extents: vec![IlocExtent {
                                index: None,
                                offset,
                                length: payload.len() as u64,
                            }],
                        };
                        offset += payload.len() as u64;
                        entry
                    })
                    .collect::<Vec<_>>();
                make_box(
                    META,
                    &[
                        vec![0; 4],
                        make_pitm_box(0, PRIMARY).unwrap(),
                        iinf.clone(),
                        make_iloc_box(1, 4, 4, 0, 0, &locations).unwrap(),
                        iprp.clone(),
                        iref.clone(),
                    ]
                    .concat(),
                )
                .unwrap()
            };
            let preliminary = build_meta(0);
            let meta = build_meta((ftyp.len() + preliminary.len() + 8) as u64);
            let payload = self
                .items
                .iter()
                .flat_map(|(_, _, bytes)| bytes.iter().copied())
                .collect::<Vec<_>>();
            [ftyp, meta, make_box(MDAT, &payload).unwrap()].concat()
        }
    }

    fn assert_payload(data: &[u8], item_id: u32, expected: &[u8]) {
        let graph = parse_graph(data, "test output").unwrap();
        assert_eq!(
            item_payload(data, &graph, item_id, "test item").unwrap(),
            expected
        );
    }

    #[test]
    fn transplant_direct_hdr_keeps_gain_out_of_three_semantic_roles() {
        let source = Fixture::new(false, false, "rust-owned-hdr").bytes();
        let scaffold = Fixture::new(false, true, "framework").bytes();
        let output = transplant_apple_semantic_auxiliary_heif(&source, &scaffold, 3).unwrap();
        assert_payload(&output, PRIMARY, b"rust-owned-hdr-primary");
        assert_payload(&output, GAIN, b"rust-owned-hdr-gain");
        for (id, expected) in [
            (10, b"scaffold-semantic-0".as_slice()),
            (11, b"scaffold-semantic-1".as_slice()),
            (12, b"scaffold-semantic-2".as_slice()),
            (20, b"descriptor-0".as_slice()),
            (21, b"descriptor-1".as_slice()),
            (22, b"descriptor-2".as_slice()),
        ] {
            assert_payload(&output, id, expected);
        }
        let graph = parse_graph(&output, "test output").unwrap();
        assert_eq!(
            graph.meta.iref.unwrap().entries,
            parse_graph(&scaffold, "test scaffold")
                .unwrap()
                .meta
                .iref
                .unwrap()
                .entries
        );
        for (index, expected) in [
            (1, make_box(HVCC, &[9; 19]).unwrap()),
            (2, make_box(HVCC, &[10; 21]).unwrap()),
            (3, make_box(HVCC, &[5; 19]).unwrap()),
        ] {
            let property = graph
                .meta
                .properties
                .iter()
                .find(|p| p.index == index)
                .unwrap();
            assert_eq!(raw_property(&output, property).unwrap(), expected);
        }
    }

    #[test]
    fn merge_direct_hdr_imports_three_described_semantics_without_importing_gain() {
        let source = Fixture::new(false, false, "rust-owned-hdr").bytes();
        let scaffold = Fixture::new(false, true, "framework").bytes();
        let output = merge_apple_semantic_auxiliary_heif(&source, &scaffold, 3).unwrap();
        assert_payload(&output, PRIMARY, b"rust-owned-hdr-primary");
        assert_payload(&output, GAIN, b"rust-owned-hdr-gain");
        let graph = parse_graph(&output, "test output").unwrap();
        assert_eq!(graph.meta.iinf.entries.len(), 10);
        for (image, descriptor, image_bytes, descriptor_bytes) in [
            (
                5,
                8,
                b"scaffold-semantic-0".as_slice(),
                b"descriptor-0".as_slice(),
            ),
            (
                6,
                9,
                b"scaffold-semantic-1".as_slice(),
                b"descriptor-1".as_slice(),
            ),
            (
                7,
                10,
                b"scaffold-semantic-2".as_slice(),
                b"descriptor-2".as_slice(),
            ),
        ] {
            assert_payload(&output, image, image_bytes);
            assert_payload(&output, descriptor, descriptor_bytes);
            assert!(graph
                .meta
                .iref
                .as_ref()
                .unwrap()
                .entries
                .iter()
                .any(|r| r.kind == CDSC
                    && r.from_item_id == descriptor
                    && r.to_item_ids == [image]));
        }
    }

    #[test]
    fn transplant_preserves_multi_tile_grid_payload_order_and_semantics() {
        let source = Fixture::new(true, false, "rust-owned-hdr").bytes();
        let scaffold = Fixture::new(true, true, "framework").bytes();
        let output = transplant_apple_semantic_auxiliary_heif(&source, &scaffold, 3).unwrap();
        for (id, expected) in [
            (100, b"rust-owned-hdr-primary-tile-0".as_slice()),
            (101, b"rust-owned-hdr-primary-tile-1".as_slice()),
            (110, b"rust-owned-hdr-gain-tile-0".as_slice()),
            (111, b"rust-owned-hdr-gain-tile-1".as_slice()),
            (1, b"framework-primary".as_slice()),
            (2, b"framework-gain".as_slice()),
            (10, b"scaffold-semantic-0".as_slice()),
        ] {
            assert_payload(&output, id, expected);
        }
    }

    #[test]
    fn semantic_descriptors_must_cover_each_image_once_even_when_counts_match() {
        let source = Fixture::new(true, false, "rust").bytes();
        let mut scaffold = Fixture::new(true, true, "framework");
        scaffold
            .references
            .iter_mut()
            .find(|r| r.from_item_id == 22)
            .unwrap()
            .to_item_ids = vec![11];
        for assemble in [
            transplant_apple_semantic_auxiliary_heif,
            merge_apple_semantic_auxiliary_heif,
        ] {
            assert!(
                assemble(&source, &scaffold.bytes(), 3).is_err(),
                "two descriptors for image 11 must not hide missing descriptor for image 12"
            );
        }
    }

    #[test]
    fn semantic_assembly_rejects_unknown_auxiliary_instead_of_dropping_it() {
        let source = Fixture::new(true, false, "rust").bytes();
        let mut scaffold = Fixture::new(true, true, "framework");
        scaffold
            .items
            .push((30, FourCC::new(*b"jpeg"), b"unknown-image".to_vec()));
        scaffold.references.push(IrefEntry {
            kind: AUXL,
            from_item_id: 30,
            to_item_ids: vec![PRIMARY, TONE_MAP],
        });
        for assemble in [
            transplant_apple_semantic_auxiliary_heif,
            merge_apple_semantic_auxiliary_heif,
        ] {
            assert!(
                assemble(&source, &scaffold.bytes(), 3).is_err(),
                "an unsupported auxiliary image must not disappear behind the expected count"
            );
        }
    }

    #[test]
    fn semantic_assembly_rejects_ambiguous_or_incomplete_hdr_graphs() {
        let source = Fixture::new(true, false, "rust").bytes();
        for case in [
            "missing tmap dimg",
            "empty tmap dimg",
            "missing primary",
            "missing gain",
            "extra target",
            "missing target item",
            "duplicate tmap dimg",
            "duplicate target",
            "unknown gain type",
            "missing grid dimg",
            "duplicate grid dimg",
            "duplicate tile",
            "missing tile",
            "direct image with dimg",
            "shared primary and gain tiles",
        ] {
            let mut scaffold = Fixture::new(true, true, "framework");
            let tmap = scaffold
                .references
                .iter_mut()
                .find(|r| r.kind == DIMG && r.from_item_id == TONE_MAP)
                .unwrap();
            match case {
                "empty tmap dimg" => tmap.to_item_ids.clear(),
                "missing primary" => tmap.to_item_ids = vec![GAIN],
                "missing gain" => tmap.to_item_ids = vec![PRIMARY],
                "extra target" => tmap.to_item_ids.push(100),
                "missing target item" => tmap.to_item_ids = vec![PRIMARY, 999],
                "duplicate target" => tmap.to_item_ids = vec![PRIMARY, GAIN, GAIN],
                _ => {}
            }
            match case {
                "missing tmap dimg" => scaffold
                    .references
                    .retain(|r| r.kind != DIMG || r.from_item_id != TONE_MAP),
                "duplicate tmap dimg" => scaffold.references.push(scaffold.references[0].clone()),
                "unknown gain type" => {
                    scaffold.items.iter_mut().find(|i| i.0 == GAIN).unwrap().1 = MIME
                }
                "missing grid dimg" => scaffold
                    .references
                    .retain(|r| r.kind != DIMG || r.from_item_id != GAIN),
                "duplicate grid dimg" => scaffold.references.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: GAIN,
                    to_item_ids: GAIN_TILES.to_vec(),
                }),
                "duplicate tile" | "missing tile" | "shared primary and gain tiles" => {
                    scaffold
                        .references
                        .iter_mut()
                        .find(|r| r.kind == DIMG && r.from_item_id == GAIN)
                        .unwrap()
                        .to_item_ids = match case {
                        "duplicate tile" => vec![110, 110],
                        "missing tile" => vec![110, 999],
                        _ => PRIMARY_TILES.to_vec(),
                    };
                }
                "direct image with dimg" => {
                    scaffold
                        .items
                        .iter_mut()
                        .find(|i| i.0 == PRIMARY)
                        .unwrap()
                        .1 = HVC1
                }
                _ => {}
            }
            for assemble in [
                transplant_apple_semantic_auxiliary_heif,
                merge_apple_semantic_auxiliary_heif,
            ] {
                assert!(
                    assemble(&source, &scaffold.bytes(), 3).is_err(),
                    "accepted {case}"
                );
            }
        }
    }

    #[test]
    fn semantic_assembly_rejects_ambiguous_auxiliary_and_descriptor_edges() {
        let source = Fixture::new(true, false, "rust").bytes();
        for case in [
            "extra aux target",
            "duplicate aux edge",
            "multiple descriptor targets",
            "duplicate descriptor edge",
            "non-mime descriptor",
        ] {
            let mut scaffold = Fixture::new(true, true, "framework");
            match case {
                "extra aux target" => scaffold
                    .references
                    .iter_mut()
                    .find(|r| r.kind == AUXL && r.from_item_id == 10)
                    .unwrap()
                    .to_item_ids
                    .push(999),
                "duplicate aux edge" => scaffold.references.push(IrefEntry {
                    kind: AUXL,
                    from_item_id: 10,
                    to_item_ids: vec![PRIMARY, TONE_MAP],
                }),
                "multiple descriptor targets" => scaffold
                    .references
                    .iter_mut()
                    .find(|r| r.from_item_id == 20)
                    .unwrap()
                    .to_item_ids
                    .push(11),
                "duplicate descriptor edge" => scaffold.references.push(IrefEntry {
                    kind: CDSC,
                    from_item_id: 20,
                    to_item_ids: vec![10],
                }),
                "non-mime descriptor" => {
                    scaffold.items.iter_mut().find(|i| i.0 == 20).unwrap().1 = EXIF
                }
                _ => unreachable!(),
            }
            for assemble in [
                transplant_apple_semantic_auxiliary_heif,
                merge_apple_semantic_auxiliary_heif,
            ] {
                assert!(
                    assemble(&source, &scaffold.bytes(), 3).is_err(),
                    "accepted {case}"
                );
            }
        }
    }

    #[test]
    fn semantic_merge_rejects_ambiguous_source_gain_map_graph() {
        let mut source = Fixture::new(true, false, "rust");
        source.references[0].to_item_ids.push(PRIMARY_TILES[0]);
        let scaffold = Fixture::new(true, true, "framework").bytes();
        assert!(merge_apple_semantic_auxiliary_heif(&source.bytes(), &scaffold, 3).is_err());
    }

    #[test]
    fn transplant_identifies_gain_map_by_relationship_with_reversed_tmap_targets() {
        let mut source = Fixture::new(false, false, "rust");
        let mut scaffold = Fixture::new(false, true, "framework");
        source.references[0].to_item_ids.reverse();
        scaffold.references[0].to_item_ids.reverse();
        let output =
            transplant_apple_semantic_auxiliary_heif(&source.bytes(), &scaffold.bytes(), 3)
                .unwrap();
        assert_payload(&output, PRIMARY, b"rust-primary");
        assert_payload(&output, GAIN, b"rust-gain");
        assert_payload(&output, 10, b"scaffold-semantic-0");
    }

    fn auxiliary_property(uri: &[u8]) -> Vec<u8> {
        make_box(FourCC::new(*b"auxC"), &[vec![0; 4], uri.to_vec()].concat()).unwrap()
    }

    #[test]
    fn unclassified_auxiliary_failure_diagnostic_reports_roles_and_edges_without_media() {
        let mut source = Fixture::new(false, false, "DO_NOT_REPORT_SOURCE_PIXELS");
        let mut scaffold = Fixture::new(false, true, "DO_NOT_REPORT_SCAFFOLD_PIXELS");
        for (fixture, extra_id) in [(&mut source, 30), (&mut scaffold, 31)] {
            fixture
                .items
                .push((extra_id, HVC1, b"DO_NOT_REPORT_AUXILIARY_PIXELS".to_vec()));
            fixture.references.push(IrefEntry {
                kind: AUXL,
                from_item_id: extra_id,
                to_item_ids: vec![PRIMARY, TONE_MAP],
            });
            fixture.auxiliary_properties.extend([
                (GAIN, auxiliary_property(b"urn:iso:std:iso:ts:21496:-1\0")),
                (
                    extra_id,
                    auxiliary_property(b"urn:com:apple:photo:2020:aux:hdrgainmap\0"),
                ),
            ]);
        }
        scaffold.auxiliary_properties.extend([
            (
                10,
                auxiliary_property(b"urn:com:apple:photo:2018:aux:portraiteffectsmatte\0"),
            ),
            (
                11,
                auxiliary_property(b"urn:com:apple:photo:2019:aux:semanticskinmatte\0"),
            ),
            (
                12,
                auxiliary_property(b"urn:com:apple:photo:2020:aux:semanticskymatte\0"),
            ),
        ]);
        for assemble in [
            transplant_apple_semantic_auxiliary_heif,
            merge_apple_semantic_auxiliary_heif,
        ] {
            let error = assemble(&source.bytes(), &scaffold.bytes(), 3)
                .unwrap_err()
                .to_string();
            assert!(error.contains("4 semantic images but 3 metadata descriptors"));
            assert!(error.contains("source={primary=1"), "{error}");
            assert!(error.contains("scaffold={primary=1"), "{error}");
            for expected in [
                "2:hvc1:auxC=[iso-gain-map]",
                "30:hvc1:auxC=[apple-hdr-gain-map]",
                "31:hvc1:auxC=[apple-hdr-gain-map]",
                "10:hvc1:auxC=[portrait-effects-matte]",
                "11:hvc1:auxC=[semantic-skin]",
                "12:hvc1:auxC=[semantic-sky]",
                "dimg=[3->[1,2]]",
                "31->[1,3]",
                "20->[10]",
            ] {
                assert!(error.contains(expected), "missing {expected} in {error}");
            }
            assert!(!error.contains("DO_NOT_REPORT"), "{error}");
        }
    }

    #[test]
    fn auxiliary_graph_diagnostic_bounds_unknown_and_malformed_properties() {
        let source = Fixture::new(false, false, "DO_NOT_REPORT_SOURCE_PIXELS").bytes();
        let mut scaffold = Fixture::new(false, true, "DO_NOT_REPORT_SCAFFOLD_PIXELS");
        scaffold.auxiliary_properties.extend([
            (
                10,
                auxiliary_property(b"DO_NOT_REPORT_UNKNOWN_AUXILIARY_URI\0"),
            ),
            (11, auxiliary_property(b"DO_NOT_REPORT_UNTERMINATED_URI")),
            (12, make_box(FourCC::new(*b"auxC"), b"\0\0").unwrap()),
        ]);
        for item_id in 1000..2000 {
            scaffold
                .items
                .push((item_id, HVC1, b"DO_NOT_REPORT_PIXELS".to_vec()));
            scaffold.references.push(IrefEntry {
                kind: AUXL,
                from_item_id: item_id,
                to_item_ids: (1..100).collect(),
            });
        }
        for assemble in [
            transplant_apple_semantic_auxiliary_heif,
            merge_apple_semantic_auxiliary_heif,
        ] {
            let error = assemble(&source, &scaffold.bytes(), 3)
                .unwrap_err()
                .to_string();
            assert!(error.contains("10:hvc1:auxC=[unknown]"), "{error}");
            assert!(error.contains("11:hvc1:auxC=[malformed]"), "{error}");
            assert!(error.contains("12:hvc1:auxC=[malformed]"), "{error}");
            assert!(error.contains("omitted="), "{error}");
            assert!(error.len() < 32_768, "diagnostic has {} bytes", error.len());
            assert!(!error.contains("DO_NOT_REPORT"), "{error}");
        }
    }

    const HEVC_ALPHA: &[u8] = b"urn:mpeg:hevc:2015:auxid:1\0";
    const MIAF_ALPHA: &[u8] = b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha\0";

    fn add_alpha(fixture: &mut Fixture, id: u32, uri: &[u8], grid: bool, source: bool) {
        fixture.items.push((
            id,
            if grid { GRID } else { HVC1 },
            if grid {
                b"alpha-grid-geometry".to_vec()
            } else if source {
                b"original-alpha-payload".to_vec()
            } else {
                b"framework-reencoded-alpha-payload".to_vec()
            },
        ));
        fixture.references.push(IrefEntry {
            kind: AUXL,
            from_item_id: id,
            to_item_ids: vec![PRIMARY, TONE_MAP],
        });
        fixture
            .auxiliary_properties
            .push((id, auxiliary_property(uri)));
        if grid {
            fixture.references.push(IrefEntry {
                kind: DIMG,
                from_item_id: id,
                to_item_ids: vec![id + 1000, id + 1001],
            });
            for (index, tile) in [id + 1000, id + 1001].into_iter().enumerate() {
                fixture.items.push((
                    tile,
                    HVC1,
                    format!(
                        "{}-alpha-tile-{index}",
                        if source { "original" } else { "framework" }
                    )
                    .into_bytes(),
                ));
            }
        } else {
            fixture.auxiliary_properties.push((
                id,
                make_box(HVCC, &[if source { 41 } else { 42 }; 19]).unwrap(),
            ));
        }
    }

    #[test]
    fn alpha_transplant_preserves_source_payload_codec_and_tile_order() {
        for uri in [HEVC_ALPHA, MIAF_ALPHA] {
            for grid in [false, true] {
                let mut source = Fixture::new(false, false, "source");
                let mut scaffold = Fixture::new(false, true, "framework");
                add_alpha(&mut source, 30, uri, grid, true);
                add_alpha(&mut scaffold, 31, uri, grid, false);
                let scaffold_bytes = scaffold.bytes();
                let output =
                    transplant_apple_semantic_auxiliary_heif(&source.bytes(), &scaffold_bytes, 3)
                        .unwrap();
                if grid {
                    assert_payload(&output, 31, b"alpha-grid-geometry");
                    assert_payload(&output, 1031, b"original-alpha-tile-0");
                    assert_payload(&output, 1032, b"original-alpha-tile-1");
                } else {
                    assert_payload(&output, 31, b"original-alpha-payload");
                    let graph = parse_graph(&output, "output").unwrap();
                    let (_, codec) =
                        uniform_property(&output, &graph.meta, &[31], HVCC, "alpha").unwrap();
                    assert_eq!(codec, make_box(HVCC, &[41; 19]).unwrap());
                }
                assert_payload(&output, PRIMARY, b"source-primary");
                assert_payload(&output, GAIN, b"source-gain");
                assert_payload(&output, 10, b"scaffold-semantic-0");
                let actual = parse_graph(&output, "output").unwrap();
                let expected = parse_graph(&scaffold_bytes, "scaffold").unwrap();
                assert_eq!(
                    actual.meta.iref.unwrap().entries,
                    expected.meta.iref.unwrap().entries
                );
                assert_eq!(actual.meta.ipma.entries, expected.meta.ipma.entries);
            }
        }
    }

    #[test]
    fn alpha_merge_preserves_source_auxiliary_instead_of_importing_reencoded_alpha() {
        let mut source = Fixture::new(false, false, "source");
        let mut scaffold = Fixture::new(false, true, "framework");
        add_alpha(&mut source, 30, HEVC_ALPHA, false, true);
        add_alpha(&mut scaffold, 31, HEVC_ALPHA, false, false);
        let output =
            merge_apple_semantic_auxiliary_heif(&source.bytes(), &scaffold.bytes(), 3).unwrap();
        assert_payload(&output, 30, b"original-alpha-payload");
        assert_payload(&output, 31, b"scaffold-semantic-0");
        let graph = parse_graph(&output, "output").unwrap();
        assert!(graph
            .meta
            .iref
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .any(|r| r.kind == AUXL
                && r.from_item_id == 30
                && r.to_item_ids == [PRIMARY, TONE_MAP]));
        let (_, codec) = uniform_property(&output, &graph.meta, &[30], HVCC, "alpha").unwrap();
        assert_eq!(codec, make_box(HVCC, &[41; 19]).unwrap());
    }

    #[test]
    fn alpha_assembly_rejects_unknown_missing_ambiguous_or_conflicting_pairs() {
        for case in [
            "source only",
            "scaffold only",
            "unknown role",
            "malformed role",
            "multiple roles",
            "multiple alpha images",
            "duplicate auxl",
            "bad relationship",
            "different relationship",
            "overlapping HDR",
            "direct dimg",
            "shared semantic codec",
            "different transform order",
            "different geometry",
            "different premultiplication",
        ] {
            let mut source = Fixture::new(false, false, "source");
            let mut scaffold = Fixture::new(false, true, "framework");
            if case != "scaffold only" {
                add_alpha(&mut source, 30, HEVC_ALPHA, false, true);
            }
            if case != "source only" {
                add_alpha(&mut scaffold, 31, HEVC_ALPHA, false, false);
            }
            match case {
                "unknown role" | "malformed role" => {
                    scaffold.auxiliary_properties[0].1 =
                        auxiliary_property(if case == "unknown role" {
                            b"urn:unrecognized:alpha\0"
                        } else {
                            b"unterminated"
                        })
                }
                "multiple roles" => scaffold
                    .auxiliary_properties
                    .push((31, auxiliary_property(MIAF_ALPHA))),
                "multiple alpha images" => add_alpha(&mut scaffold, 32, MIAF_ALPHA, false, false),
                "duplicate auxl" => scaffold.references.push(IrefEntry {
                    kind: AUXL,
                    from_item_id: 31,
                    to_item_ids: vec![PRIMARY, TONE_MAP],
                }),
                "bad relationship" | "different relationship" => {
                    scaffold
                        .references
                        .iter_mut()
                        .find(|r| r.kind == AUXL && r.from_item_id == 31)
                        .unwrap()
                        .to_item_ids = if case == "bad relationship" {
                        vec![PRIMARY, TONE_MAP, 999]
                    } else {
                        vec![PRIMARY]
                    }
                }
                "overlapping HDR" => scaffold
                    .auxiliary_properties
                    .push((GAIN, auxiliary_property(MIAF_ALPHA))),
                "direct dimg" => scaffold.references.push(IrefEntry {
                    kind: DIMG,
                    from_item_id: 31,
                    to_item_ids: vec![10],
                }),
                "shared semantic codec" => scaffold
                    .auxiliary_properties
                    .retain(|(_, property)| property.get(4..8) != Some(HVCC.as_bytes())),
                "different transform order" => {
                    source.auxiliary_properties.extend([
                        (30, make_box(IROT, &[3]).unwrap()),
                        (30, make_box(FourCC::new(*b"imir"), &[1]).unwrap()),
                    ]);
                    scaffold.auxiliary_properties.extend([
                        (31, make_box(FourCC::new(*b"imir"), &[1]).unwrap()),
                        (31, make_box(IROT, &[3]).unwrap()),
                    ]);
                }
                "different geometry" => {
                    source
                        .auxiliary_properties
                        .push((30, xdremux_format::isobmff::make_ispe_box(4, 4).unwrap()));
                    scaffold
                        .auxiliary_properties
                        .push((31, xdremux_format::isobmff::make_ispe_box(6, 4).unwrap()));
                }
                "different premultiplication" => source.references.push(IrefEntry {
                    kind: FourCC::new(*b"prem"),
                    from_item_id: PRIMARY,
                    to_item_ids: vec![30],
                }),
                _ => {}
            }
            if case == "shared semantic codec" {
                assert!(transplant_apple_semantic_auxiliary_heif(
                    &source.bytes(),
                    &scaffold.bytes(),
                    3
                )
                .is_err());
                // Merge retains the original source alpha and never rewrites
                // the scaffold codec shared with its semantic images.
                assert!(
                    merge_apple_semantic_auxiliary_heif(&source.bytes(), &scaffold.bytes(), 3)
                        .is_ok()
                );
                continue;
            }
            for assemble in [
                transplant_apple_semantic_auxiliary_heif,
                merge_apple_semantic_auxiliary_heif,
            ] {
                assert!(
                    assemble(&source.bytes(), &scaffold.bytes(), 3).is_err(),
                    "accepted {case}"
                );
            }
        }
    }

    #[test]
    fn auxiliary_identity_rejects_duplicate_alpha_item_records() {
        let mut source = Fixture::new(false, false, "source");
        let mut scaffold = Fixture::new(false, true, "framework");
        add_alpha(&mut source, 30, HEVC_ALPHA, false, true);
        add_alpha(&mut scaffold, 31, HEVC_ALPHA, false, false);
        scaffold
            .items
            .push((31, HVC1, b"ambiguous-alpha-payload".to_vec()));
        for assemble in [
            transplant_apple_semantic_auxiliary_heif,
            merge_apple_semantic_auxiliary_heif,
        ] {
            assert!(assemble(&source.bytes(), &scaffold.bytes(), 3).is_err());
        }
    }

    #[test]
    fn auxiliary_identity_rejects_unknown_semantic_role_even_with_matching_descriptors() {
        let source = Fixture::new(false, false, "source").bytes();
        let mut scaffold = Fixture::new(false, true, "framework");
        scaffold
            .auxiliary_properties
            .push((10, auxiliary_property(b"urn:unknown:auxiliary\0")));
        for assemble in [
            transplant_apple_semantic_auxiliary_heif,
            merge_apple_semantic_auxiliary_heif,
        ] {
            assert!(assemble(&source, &scaffold.bytes(), 3).is_err());
        }
    }

    #[test]
    fn auxiliary_identity_rejects_unhandled_alpha_tile_outgoing_references() {
        for bad_source in [false, true] {
            let mut source = Fixture::new(false, false, "source");
            let mut scaffold = Fixture::new(false, true, "framework");
            add_alpha(&mut source, 30, HEVC_ALPHA, true, true);
            add_alpha(&mut scaffold, 31, HEVC_ALPHA, true, false);
            let (fixture, tile) = if bad_source {
                (&mut source, 1030)
            } else {
                (&mut scaffold, 1031)
            };
            fixture.references.push(IrefEntry {
                kind: FourCC::new(*b"thmb"),
                from_item_id: tile,
                to_item_ids: vec![PRIMARY],
            });
            for assemble in [
                transplant_apple_semantic_auxiliary_heif,
                merge_apple_semantic_auxiliary_heif,
            ] {
                assert!(assemble(&source.bytes(), &scaffold.bytes(), 3).is_err());
            }
        }
    }

    #[test]
    fn auxiliary_identity_transplant_rejects_unpaired_unknown_source_auxiliary() {
        let mut source = Fixture::new(false, false, "source");
        let scaffold = Fixture::new(false, true, "framework").bytes();
        add_alpha(&mut source, 30, b"urn:unknown:auxiliary\0", false, true);
        assert!(transplant_apple_semantic_auxiliary_heif(&source.bytes(), &scaffold, 3).is_err());
    }

    #[test]
    fn alpha_primary_only_and_premultiplied_relationships_preserve_transform_order() {
        let mut source = Fixture::new(false, false, "source");
        let mut scaffold = Fixture::new(false, true, "framework");
        add_alpha(&mut source, 30, HEVC_ALPHA, false, true);
        add_alpha(&mut scaffold, 31, MIAF_ALPHA, false, false);
        for (fixture, alpha_id) in [(&mut source, 30), (&mut scaffold, 31)] {
            fixture
                .references
                .iter_mut()
                .find(|reference| reference.kind == AUXL && reference.from_item_id == alpha_id)
                .unwrap()
                .to_item_ids = vec![PRIMARY];
            fixture.references.push(IrefEntry {
                kind: PREM,
                from_item_id: PRIMARY,
                to_item_ids: vec![alpha_id],
            });
            fixture.auxiliary_properties.extend([
                (
                    alpha_id,
                    xdremux_format::isobmff::make_ispe_box(4, 4).unwrap(),
                ),
                (
                    alpha_id,
                    xdremux_format::isobmff::make_full_box(PIXI, 0, 0, &[1, 8]).unwrap(),
                ),
                (alpha_id, make_box(IROT, &[3]).unwrap()),
                (alpha_id, make_box(IMIR, &[1]).unwrap()),
            ]);
        }
        let scaffold_bytes = scaffold.bytes();
        let output =
            transplant_apple_semantic_auxiliary_heif(&source.bytes(), &scaffold_bytes, 3).unwrap();
        assert_payload(&output, 31, b"original-alpha-payload");
        let actual = parse_graph(&output, "output").unwrap();
        let expected = parse_graph(&scaffold_bytes, "scaffold").unwrap();
        assert_eq!(
            actual.meta.iref.unwrap().entries,
            expected.meta.iref.unwrap().entries
        );
        assert_eq!(actual.meta.ipma.entries, expected.meta.ipma.entries);
    }

    fn add_alpha_subtype(fixture: &mut Fixture, id: u32, uri: &[u8], subtype: &[u8], source: bool) {
        add_alpha(fixture, id, uri, false, source);
        fixture
            .auxiliary_properties
            .iter_mut()
            .find(|(item_id, property)| {
                *item_id == id && property.get(4..8) == Some(AUXC.as_bytes())
            })
            .unwrap()
            .1 = make_box(AUXC, &[vec![0; 4], uri.to_vec(), subtype.to_vec()].concat()).unwrap();
    }

    #[test]
    fn alpha_subtype_diagnostic_reports_bounded_structure_and_keeps_unsupported_bytes_rejected() {
        // Only the outer length/NAL-header envelope is recognizable here.
        // This deliberately does not establish valid alpha SEI semantics.
        let framed = [0, 0, 0, 8, 0, 0, 0, 4, 0x4e, 1, 165, 0];
        for (subtype, expected) in [
            (b"".as_slice(), "len=0,shape=empty"),
            ([0, 0, 0, 0].as_slice(), "len=4,shape=zero-filled"),
            (framed.as_slice(), "len=12,shape=hevc-sei-envelope"),
            (&framed[..11], "len=11,shape=opaque"),
            (b"SECRET_SUBTYPE".as_slice(), "len=14,shape=opaque"),
        ] {
            let mut source = Fixture::new(false, false, "SECRET_SOURCE_PIXELS");
            let mut scaffold = Fixture::new(false, true, "SECRET_SCAFFOLD_PIXELS");
            add_alpha_subtype(&mut source, 30, HEVC_ALPHA, subtype, true);
            add_alpha_subtype(&mut scaffold, 31, HEVC_ALPHA, subtype, false);
            scaffold
                .references
                .iter_mut()
                .find(|reference| reference.from_item_id == 22)
                .unwrap()
                .to_item_ids = vec![11];
            for assemble in [
                transplant_apple_semantic_auxiliary_heif,
                merge_apple_semantic_auxiliary_heif,
            ] {
                let error = assemble(&source.bytes(), &scaffold.bytes(), 3)
                    .unwrap_err()
                    .to_string();
                assert!(error.contains(expected), "{error}");
                assert!(
                    error.contains("alpha_subtype_pair={uri_match=true,bytes_match=true}"),
                    "{error}"
                );
                if !subtype.is_empty() {
                    assert!(
                        error.contains("source HDR alpha subtype is unsupported"),
                        "{error}"
                    );
                }
                assert!(!error.contains("SECRET"), "{error}");
                assert!(error.len() < 32_768);
            }
        }
    }

    #[test]
    fn alpha_subtype_diagnostic_distinguishes_uri_and_byte_mismatches_without_dumping_them() {
        for (scaffold_uri, scaffold_subtype, expected) in [
            (
                HEVC_ALPHA,
                b"SECRET_OTHER".as_slice(),
                "uri_match=true,bytes_match=false",
            ),
            (
                MIAF_ALPHA,
                b"SECRET_SUBTYPE".as_slice(),
                "uri_match=false,bytes_match=true",
            ),
        ] {
            let mut source = Fixture::new(false, false, "source");
            let mut scaffold = Fixture::new(false, true, "scaffold");
            add_alpha_subtype(&mut source, 30, HEVC_ALPHA, b"SECRET_SUBTYPE", true);
            add_alpha_subtype(&mut scaffold, 31, scaffold_uri, scaffold_subtype, false);
            for assemble in [
                transplant_apple_semantic_auxiliary_heif,
                merge_apple_semantic_auxiliary_heif,
            ] {
                let error = assemble(&source.bytes(), &scaffold.bytes(), 3)
                    .unwrap_err()
                    .to_string();
                assert!(error.contains(expected), "{error}");
                assert!(!error.contains("SECRET"), "{error}");
            }
        }
    }
}
