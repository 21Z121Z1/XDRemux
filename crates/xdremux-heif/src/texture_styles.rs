use xdremux_format::exif::read_item_payload;
use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_ipma_box, make_iref_box,
    parse_boxes, parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry, IlocExtent,
    IpmaAssociation, IpmaEntry, IrefEntry, ParsedMeta, PropertyInfo, EXIF, FTYP, IDAT, IINF, ILOC,
    IPCO, IPMA, IPRP, IREF, MDAT, META,
};
use xdremux_format::{exif_makernote, heif_exif_tiff, replace_exif_makernote, FourCC};

use crate::error::{HeifError, Result};

const URI: FourCC = FourCC::new(*b"uri ");
const HVC1: FourCC = FourCC::new(*b"hvc1");
const MIME: FourCC = FourCC::new(*b"mime");
const AUXC: FourCC = FourCC::new(*b"auxC");
const AUXL: FourCC = FourCC::new(*b"auxl");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const TMAP: FourCC = FourCC::new(*b"tmap");

const STYLE_METADATA_URI: &[u8] = b"tag:apple.com,2023:photo:metadata:styles";
const TEXTURE_METADATA_URI: &[u8] = b"tag:apple.com,2026:photo:metadata:texture_styles";
const SKIN_AUX_TYPE: &[u8] = b"urn:com:apple:photo:2019:aux:semanticskinmatte";

const TEXTURE_ROLE_AUX_TYPES: [&[u8]; 12] = [
    b"tag:apple.com,2026:photo:aux:semanticnosematte",
    b"tag:apple.com,2026:photo:aux:semanticskinmattev2",
    b"tag:apple.com,2026:photo:aux:semanticnonfaceskinmatte",
    b"tag:apple.com,2026:photo:aux:semanticlipsmatte",
    b"tag:apple.com,2026:photo:aux:semanticteethmattev2",
    b"tag:apple.com,2026:photo:aux:semanticpersonmatte",
    b"tag:apple.com,2026:photo:aux:semanticglassesmattev2",
    b"tag:apple.com,2026:photo:aux:semanticeyebrowsmatte",
    b"tag:apple.com,2026:photo:aux:semantictattoomatte",
    b"tag:apple.com,2026:photo:aux:semantichandsmatte",
    b"tag:apple.com,2026:photo:aux:semanticearsmatte",
    b"tag:apple.com,2026:photo:aux:semanticfaceskinmatte",
];

// Clean-room metadata from the public physical-device-positive carrier. No donor image bytes.
const TEXTURE_METADATA_BPLIST_HEX: &str = concat!(
    "62706c6973743030d70102030405060708090a0b0c0d0e565072657365745b43617074757265547970655b436170747572654d6f646558506f7274547970655d48617264776172654d6f64656c",
    "5f101d546578747572655374796c6550656f706c654461746156657273696f6e5d46696c6d477261696e53656564585374616e64617264524c46555374696c6c5c506f7274547970654261636b",
    "5a6950686f6e6531392c321003100008171e2a363f4d6d7b84878d9aa5a70000000000000101000000000000000f000000000000000000000000000000a9"
);

// Native-typed Float32 tag 84 contract used by the same public positive carrier.
const TEXTURE_TAG84_BPLIST_HEX: &str = concat!(
    "62706c6973743030dd0102030405060708090a0b0c0d0e0e0f10111111121311110e0f5231305132513352313151345135523132513651375130513851315139",
    "2200000000223f80000008100110041000082326282a2d2f313436383a3c3e40454a4b4d4f0000000000000101000000000000001400000000000000000000000000000051"
);

#[derive(Debug, Clone)]
struct Graph {
    top: xdremux_format::isobmff::TopLevelScan,
    meta_header: BoxHeader,
    meta_children: Vec<BoxHeader>,
    meta: ParsedMeta,
    mdat: BoxHeader,
}

#[derive(Debug, Clone, Copy)]
struct AliasIds {
    image: u32,
    descriptor: u32,
    aux_property: u16,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn decode_hex(value: &str, context: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err(invalid(format!("{context} hex length is odd")));
    }
    let mut output = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().as_chunks::<2>().0 {
        let digit = |byte: u8| -> Result<u8> {
            match byte {
                b'0'..=b'9' => Ok(byte - b'0'),
                b'a'..=b'f' => Ok(byte - b'a' + 10),
                b'A'..=b'F' => Ok(byte - b'A' + 10),
                _ => Err(invalid(format!("{context} contains non-hex data"))),
            }
        };
        output.push((digit(pair[0])? << 4) | digit(pair[1])?);
    }
    Ok(output)
}

fn texture_metadata_bplist() -> Result<Vec<u8>> {
    let payload = decode_hex(TEXTURE_METADATA_BPLIST_HEX, "Texture Style metadata")?;
    if payload.len() != 216 || !payload.starts_with(b"bplist00") {
        return Err(invalid(
            "Texture Style metadata contract failed integrity validation",
        ));
    }
    Ok(payload)
}

fn texture_tag84_bplist() -> Result<Vec<u8>> {
    let payload = decode_hex(TEXTURE_TAG84_BPLIST_HEX, "Texture Style MakerNote tag84")?;
    if payload.len() != 133 || !payload.starts_with(b"bplist00") {
        return Err(invalid(
            "Texture Style tag84 contract failed integrity validation",
        ));
    }
    Ok(payload)
}

fn raw_box<'a>(data: &'a [u8], header: &BoxHeader, context: &str) -> Result<&'a [u8]> {
    data.get(header.box_range())
        .ok_or_else(|| invalid(format!("{context} box is outside input")))
}

fn raw_property<'a>(data: &'a [u8], property: &PropertyInfo) -> Result<&'a [u8]> {
    data.get(property.box_range.clone())
        .ok_or_else(|| invalid(format!("property {} is outside input", property.index)))
}

fn child<'a>(children: &'a [BoxHeader], kind: FourCC, context: &str) -> Result<&'a BoxHeader> {
    children
        .iter()
        .find(|header| header.kind == kind)
        .ok_or_else(|| invalid(format!("{context}/{} is missing", kind)))
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

fn parse_graph(data: &[u8]) -> Result<Graph> {
    let top = scan_top_level_boxes(data)?;
    let _ = one_top_level(&top.boxes, FTYP, "Texture Style ftyp")?;
    let meta_header = one_top_level(&top.boxes, META, "Texture Style meta")?.clone();
    let mdat = one_top_level(&top.boxes, MDAT, "Texture Style mdat")?.clone();
    let child_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("Texture Style meta child offset overflows"))?;
    if child_start > meta_header.data_end {
        return Err(invalid("Texture Style meta full-box header is truncated"));
    }
    let meta_children = parse_boxes(data, child_start..meta_header.data_end)?;
    for kind in [IINF, ILOC, IPRP, IDAT] {
        let _ = child(&meta_children, kind, "Texture Style meta")?;
    }
    let meta = parse_meta_box(data, &meta_header)?;
    Ok(Graph {
        top,
        meta_header,
        meta_children,
        meta,
        mdat,
    })
}

fn find_uri_item(data: &[u8], meta: &ParsedMeta, uri: &[u8], context: &str) -> Result<u32> {
    let mut ids = Vec::new();
    for item in &meta.iinf.entries {
        if item.item_type != Some(URI) {
            continue;
        }
        let raw = data
            .get(item.box_range.clone())
            .ok_or_else(|| invalid(format!("{context} uri item is outside input")))?;
        if raw.windows(uri.len()).any(|window| window == uri) {
            ids.push(item.item_id);
        }
    }
    match ids.as_slice() {
        [only] => Ok(*only),
        [] => Err(invalid(format!("{context} is missing"))),
        _ => Err(invalid(format!("{context} appears more than once"))),
    }
}

fn has_uri_item(data: &[u8], meta: &ParsedMeta, uri: &[u8]) -> Result<bool> {
    for item in &meta.iinf.entries {
        if item.item_type != Some(URI) {
            continue;
        }
        let raw = data
            .get(item.box_range.clone())
            .ok_or_else(|| invalid("Texture Style uri item is outside input"))?;
        if raw.windows(uri.len()).any(|window| window == uri) {
            return Ok(true);
        }
    }
    Ok(false)
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
        [] => Err(invalid(format!("{context} is missing"))),
        _ => Err(invalid(format!("{context} appears more than once"))),
    }
}

fn item_location<'a>(meta: &'a ParsedMeta, item_id: u32, context: &str) -> Result<&'a IlocEntry> {
    meta.iloc
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
        .ok_or_else(|| invalid(format!("{context} item {item_id} has no iloc entry")))
}

fn make_uri_infe(item_id: u32, name: &[u8], uri: &[u8]) -> Result<Vec<u8>> {
    let version = if item_id <= u32::from(u16::MAX) { 2 } else { 3 };
    let mut payload = Vec::new();
    if version == 2 {
        payload.extend_from_slice(&(item_id as u16).to_be_bytes());
    } else {
        payload.extend_from_slice(&item_id.to_be_bytes());
    }
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(URI.as_bytes());
    payload.extend_from_slice(name);
    payload.push(0);
    payload.extend_from_slice(uri);
    payload.push(0);
    Ok(make_full_box(FourCC::new(*b"infe"), version, 1, &payload)?)
}

fn make_mime_infe(item_id: u32) -> Result<Vec<u8>> {
    let version = if item_id <= u32::from(u16::MAX) { 2 } else { 3 };
    let mut payload = Vec::new();
    if version == 2 {
        payload.extend_from_slice(&(item_id as u16).to_be_bytes());
    } else {
        payload.extend_from_slice(&item_id.to_be_bytes());
    }
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(MIME.as_bytes());
    payload.push(0);
    payload.extend_from_slice(b"application/rdf+xml");
    payload.push(0);
    Ok(make_full_box(FourCC::new(*b"infe"), version, 1, &payload)?)
}

fn clone_infe_with_id(
    data: &[u8],
    meta: &ParsedMeta,
    source_id: u32,
    new_id: u32,
) -> Result<Vec<u8>> {
    let item = meta
        .iinf
        .entries
        .iter()
        .find(|item| item.item_id == source_id)
        .ok_or_else(|| invalid("Texture Style semantic source infe is missing"))?;
    let mut raw = data
        .get(item.box_range.clone())
        .ok_or_else(|| invalid("Texture Style semantic source infe is outside input"))?
        .to_vec();
    if raw.len() < 14 || raw.get(4..8) != Some(b"infe".as_slice()) {
        return Err(invalid(
            "Texture Style semantic source infe has unexpected layout",
        ));
    }
    match raw[8] {
        2 => {
            let value = u16::try_from(new_id)
                .map_err(|_| invalid("Texture Style alias item ID exceeds infe v2 range"))?;
            raw[12..14].copy_from_slice(&value.to_be_bytes());
        }
        3 => {
            if raw.len() < 16 {
                return Err(invalid(
                    "Texture Style semantic source infe v3 is truncated",
                ));
            }
            raw[12..16].copy_from_slice(&new_id.to_be_bytes());
        }
        version => {
            return Err(invalid(format!(
                "Texture Style semantic source uses unsupported infe version {version}"
            )))
        }
    }
    Ok(raw)
}

fn make_auxc_box(uri: &[u8]) -> Result<Vec<u8>> {
    let mut payload = uri.to_vec();
    payload.push(0);
    Ok(make_full_box(AUXC, 0, 0, &payload)?)
}

fn source_maximum_item_id(meta: &ParsedMeta) -> Result<u32> {
    meta.iinf
        .entries
        .iter()
        .map(|item| item.item_id)
        .chain(meta.iloc.entries.iter().map(|entry| entry.item_id))
        .chain(meta.iref.as_ref().into_iter().flat_map(|iref| {
            iref.entries.iter().flat_map(|entry| {
                std::iter::once(entry.from_item_id).chain(entry.to_item_ids.iter().copied())
            })
        }))
        .max()
        .ok_or_else(|| invalid("Texture Style source contains no item IDs"))
}

fn find_skin_alias_source(data: &[u8], meta: &ParsedMeta) -> Result<(u32, u32, u16)> {
    let skin_property = meta
        .properties
        .iter()
        .find(|property| {
            property.kind == AUXC
                && raw_property(data, property).is_ok_and(|raw| {
                    raw.windows(SKIN_AUX_TYPE.len())
                        .any(|window| window == SKIN_AUX_TYPE)
                })
        })
        .ok_or_else(|| invalid("Texture Style source has no 2019 semantic skin matte"))?;
    let skin_property_index = u16::try_from(skin_property.index)
        .map_err(|_| invalid("Texture Style skin property index exceeds u16"))?;
    let image_ids = meta
        .ipma
        .entries
        .iter()
        .filter(|entry| {
            entry
                .associations
                .iter()
                .any(|association| association.property_index == skin_property_index)
        })
        .map(|entry| entry.item_id)
        .filter(|item_id| {
            meta.iinf
                .entries
                .iter()
                .any(|item| item.item_id == *item_id && item.item_type == Some(HVC1))
        })
        .collect::<Vec<_>>();
    let image_id = match image_ids.as_slice() {
        [only] => *only,
        [] => return Err(invalid("Texture Style skin property has no hvc1 owner")),
        _ => {
            return Err(invalid(
                "Texture Style skin property has multiple hvc1 owners",
            ))
        }
    };
    let descriptor_ids =
        meta.iref
            .as_ref()
            .into_iter()
            .flat_map(|iref| iref.entries.iter())
            .filter(|entry| {
                entry.kind == CDSC
                    && entry.to_item_ids == vec![image_id]
                    && meta.iinf.entries.iter().any(|item| {
                        item.item_id == entry.from_item_id && item.item_type == Some(MIME)
                    })
            })
            .map(|entry| entry.from_item_id)
            .collect::<Vec<_>>();
    let descriptor_id = match descriptor_ids.as_slice() {
        [only] => *only,
        [] => return Err(invalid("Texture Style skin matte has no MIME descriptor")),
        _ => {
            return Err(invalid(
                "Texture Style skin matte has multiple MIME descriptors",
            ))
        }
    };
    Ok((image_id, descriptor_id, skin_property_index))
}

fn build_iinf(
    data: &[u8],
    meta: &ParsedMeta,
    style_id: u32,
    texture_id: u32,
    source_skin_id: u32,
    aliases: &[AliasIds],
) -> Result<Vec<u8>> {
    let mut entries = Vec::new();
    for item in &meta.iinf.entries {
        if item.item_id == style_id {
            entries.push(make_uri_infe(style_id, b"metadata", STYLE_METADATA_URI)?);
        } else {
            entries.push(
                data.get(item.box_range.clone())
                    .ok_or_else(|| invalid("Texture Style source infe is outside input"))?
                    .to_vec(),
            );
        }
    }
    entries.push(make_uri_infe(
        texture_id,
        b"metadata",
        TEXTURE_METADATA_URI,
    )?);
    for alias in aliases {
        entries.push(clone_infe_with_id(data, meta, source_skin_id, alias.image)?);
        entries.push(make_mime_infe(alias.descriptor)?);
    }
    let version = if meta.iinf.version == 0 && entries.len() > usize::from(u16::MAX) {
        1
    } else {
        meta.iinf.version
    };
    Ok(make_iinf_box(version, &entries)?)
}

fn build_iprp(
    data: &[u8],
    graph: &Graph,
    source_skin_id: u32,
    source_skin_property: u16,
    aliases: &[AliasIds],
) -> Result<Vec<u8>> {
    let iprp_header = child(&graph.meta_children, IPRP, "Texture Style meta")?;
    let children = parse_boxes(data, iprp_header.payload_range())?;
    let mut properties = graph.meta.properties.iter().collect::<Vec<_>>();
    properties.sort_by_key(|property| property.index);
    for (offset, property) in properties.iter().enumerate() {
        if property.index
            != u32::try_from(offset + 1).map_err(|_| invalid("property count overflows"))?
        {
            return Err(invalid(
                "Texture Style source ipco property indices are not contiguous",
            ));
        }
    }
    let mut ipco_payload = Vec::new();
    for property in &properties {
        ipco_payload.extend_from_slice(raw_property(data, property)?);
    }
    for uri in TEXTURE_ROLE_AUX_TYPES {
        ipco_payload.extend_from_slice(&make_auxc_box(uri)?);
    }
    let ipco = make_box(IPCO, &ipco_payload)?;

    let source_associations = graph
        .meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == source_skin_id)
        .map(|entry| entry.associations.clone())
        .ok_or_else(|| invalid("Texture Style skin matte has no ipma entry"))?;
    if !source_associations
        .iter()
        .any(|association| association.property_index == source_skin_property)
    {
        return Err(invalid(
            "Texture Style skin ipma does not carry its auxC property",
        ));
    }
    let mut ipma_entries = graph.meta.ipma.entries.clone();
    for alias in aliases {
        let associations = source_associations
            .iter()
            .map(|association| IpmaAssociation {
                property_index: if association.property_index == source_skin_property {
                    alias.aux_property
                } else {
                    association.property_index
                },
                essential: association.essential,
            })
            .collect();
        ipma_entries.push(IpmaEntry {
            item_id: alias.image,
            associations,
        });
    }
    let maximum_property = aliases
        .iter()
        .map(|alias| alias.aux_property)
        .max()
        .unwrap_or(source_skin_property);
    let maximum_item = ipma_entries
        .iter()
        .map(|entry| entry.item_id)
        .max()
        .unwrap_or(0);
    let version = if maximum_item > u32::from(u16::MAX) {
        1
    } else {
        graph.meta.ipma.version
    };
    let flags = if maximum_property > 0x7f {
        graph.meta.ipma.flags | 1
    } else {
        graph.meta.ipma.flags
    };
    let ipma = make_ipma_box(version, flags, &ipma_entries)?;

    let mut payload = Vec::new();
    let mut saw_ipco = false;
    let mut saw_ipma = false;
    for header in &children {
        match header.kind {
            IPCO if !saw_ipco => {
                saw_ipco = true;
                payload.extend_from_slice(&ipco);
            }
            IPCO => return Err(invalid("Texture Style source iprp has multiple ipco boxes")),
            IPMA if !saw_ipma => {
                saw_ipma = true;
                payload.extend_from_slice(&ipma);
            }
            IPMA => return Err(invalid("Texture Style source iprp has multiple ipma boxes")),
            _ => payload.extend_from_slice(raw_box(data, header, "Texture Style iprp child")?),
        }
    }
    if !saw_ipco || !saw_ipma {
        return Err(invalid("Texture Style source iprp is missing ipco/ipma"));
    }
    Ok(make_box(IPRP, &payload)?)
}

fn build_iref(meta: &ParsedMeta, texture_id: u32, aliases: &[AliasIds]) -> Result<Vec<u8>> {
    let tmap = find_single_item(meta, TMAP, "Texture Style tmap")?;
    let targets = vec![meta.primary_item_id, tmap];
    let mut entries = meta
        .iref
        .as_ref()
        .map_or_else(Vec::new, |iref| iref.entries.clone());
    entries.push(IrefEntry {
        kind: CDSC,
        from_item_id: texture_id,
        to_item_ids: targets.clone(),
    });
    for alias in aliases {
        entries.push(IrefEntry {
            kind: AUXL,
            from_item_id: alias.image,
            to_item_ids: targets.clone(),
        });
        entries.push(IrefEntry {
            kind: CDSC,
            from_item_id: alias.descriptor,
            to_item_ids: vec![alias.image],
        });
    }
    let max_id = entries
        .iter()
        .flat_map(|entry| {
            std::iter::once(entry.from_item_id).chain(entry.to_item_ids.iter().copied())
        })
        .max()
        .unwrap_or(0);
    let version = if max_id > u32::from(u16::MAX) {
        1
    } else {
        meta.iref.as_ref().map_or(0, |iref| iref.version)
    };
    Ok(make_iref_box(version, &entries)?)
}

fn normalized_location(
    graph: &Graph,
    entry: &IlocEntry,
    new_item_id: u32,
    placeholder: bool,
    new_mdat_data_start: u64,
    prefix_len: u64,
) -> Result<IlocEntry> {
    if entry.data_reference_index != 0 || !matches!(entry.construction_method, 0 | 1) {
        return Err(invalid(format!(
            "Texture Style source item {} uses unsupported location semantics",
            entry.item_id
        )));
    }
    let old_mdat_start = u64::try_from(graph.mdat.data_start)
        .map_err(|_| invalid("Texture Style source mdat offset exceeds u64"))?;
    let old_mdat_end = u64::try_from(graph.mdat.data_end)
        .map_err(|_| invalid("Texture Style source mdat end exceeds u64"))?;
    let idat_len = graph
        .meta
        .idat
        .as_ref()
        .map(|idat| idat.data_end - idat.data_start)
        .unwrap_or(0);
    let idat_len = u64::try_from(idat_len)
        .map_err(|_| invalid("Texture Style source idat length exceeds u64"))?;
    let mut extents = Vec::with_capacity(entry.extents.len());
    for extent in &entry.extents {
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid("Texture Style source uses a non-zero extent index"));
        }
        let resolved = entry.resolved_extent_offset(extent)?;
        let end = resolved
            .checked_add(extent.length)
            .ok_or_else(|| invalid("Texture Style source extent overflows"))?;
        let offset = if entry.construction_method == 0 {
            if resolved < old_mdat_start || end > old_mdat_end {
                return Err(invalid("Texture Style source file extent is outside mdat"));
            }
            if placeholder {
                0
            } else {
                new_mdat_data_start
                    .checked_add(prefix_len)
                    .and_then(|value| value.checked_add(resolved - old_mdat_start))
                    .ok_or_else(|| invalid("Texture Style relocated mdat offset overflows"))?
            }
        } else {
            if end > idat_len {
                return Err(invalid("Texture Style source idat extent is outside idat"));
            }
            resolved
        };
        extents.push(IlocExtent {
            index: None,
            offset,
            length: extent.length,
        });
    }
    Ok(IlocEntry {
        item_id: new_item_id,
        construction_method: entry.construction_method,
        data_reference_index: 0,
        base_offset: 0,
        extents,
    })
}

fn direct_location(item_id: u32, offset: u64, length: usize) -> Result<IlocEntry> {
    Ok(IlocEntry {
        item_id,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset,
            length: u64::try_from(length)
                .map_err(|_| invalid("Texture Style direct item length exceeds u64"))?,
        }],
    })
}

#[allow(clippy::too_many_arguments)]
fn build_locations(
    graph: &Graph,
    style_id: u32,
    texture_id: u32,
    exif_id: u32,
    source_skin_id: u32,
    source_descriptor_id: u32,
    aliases: &[AliasIds],
    style_payload: &[u8],
    texture_payload: &[u8],
    exif_payload: &[u8],
    placeholder: bool,
    new_mdat_data_start: u64,
) -> Result<Vec<IlocEntry>> {
    let prefix_len_usize = style_payload
        .len()
        .checked_add(texture_payload.len())
        .and_then(|value| value.checked_add(exif_payload.len()))
        .ok_or_else(|| invalid("Texture Style mdat prefix length overflows"))?;
    let prefix_len = u64::try_from(prefix_len_usize)
        .map_err(|_| invalid("Texture Style mdat prefix length exceeds u64"))?;
    let mut entries = Vec::new();
    for entry in &graph.meta.iloc.entries {
        if entry.item_id == style_id || entry.item_id == exif_id {
            continue;
        }
        entries.push(normalized_location(
            graph,
            entry,
            entry.item_id,
            placeholder,
            new_mdat_data_start,
            prefix_len,
        )?);
    }

    let style_offset = if placeholder { 0 } else { new_mdat_data_start };
    let texture_offset = if placeholder {
        0
    } else {
        new_mdat_data_start
            .checked_add(
                u64::try_from(style_payload.len())
                    .map_err(|_| invalid("style length exceeds u64"))?,
            )
            .ok_or_else(|| invalid("Texture Style metadata offset overflows"))?
    };
    let exif_offset = if placeholder {
        0
    } else {
        texture_offset
            .checked_add(
                u64::try_from(texture_payload.len())
                    .map_err(|_| invalid("texture length exceeds u64"))?,
            )
            .ok_or_else(|| invalid("Texture Style Exif offset overflows"))?
    };
    entries.push(direct_location(
        style_id,
        style_offset,
        style_payload.len(),
    )?);
    entries.push(direct_location(
        texture_id,
        texture_offset,
        texture_payload.len(),
    )?);
    entries.push(direct_location(exif_id, exif_offset, exif_payload.len())?);

    let skin_location = item_location(&graph.meta, source_skin_id, "Texture Style skin source")?;
    let descriptor_location = item_location(
        &graph.meta,
        source_descriptor_id,
        "Texture Style skin descriptor source",
    )?;
    for alias in aliases {
        entries.push(normalized_location(
            graph,
            skin_location,
            alias.image,
            placeholder,
            new_mdat_data_start,
            prefix_len,
        )?);
        entries.push(normalized_location(
            graph,
            descriptor_location,
            alias.descriptor,
            placeholder,
            new_mdat_data_start,
            prefix_len,
        )?);
    }
    entries.sort_by_key(|entry| entry.item_id);
    Ok(entries)
}

fn build_meta(
    data: &[u8],
    graph: &Graph,
    iinf: &[u8],
    iloc: &[u8],
    iprp: &[u8],
    iref: &[u8],
) -> Result<Vec<u8>> {
    let full_header_end = graph
        .meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("Texture Style meta full-box header overflows"))?;
    let full_header = data
        .get(graph.meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("Texture Style meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    // The physical-device-positive H carrier preserves the source meta-child
    // order. Replacing boxes in place keeps this postprocessor from adding a
    // second, unverified container-layout transformation.
    let mut saw_iref = false;
    for header in &graph.meta_children {
        match header.kind {
            IINF => payload.extend_from_slice(iinf),
            ILOC => payload.extend_from_slice(iloc),
            IPRP => payload.extend_from_slice(iprp),
            IREF if !saw_iref => {
                saw_iref = true;
                payload.extend_from_slice(iref);
            }
            IREF => return Err(invalid("Texture Style source has multiple iref boxes")),
            _ => payload.extend_from_slice(raw_box(data, header, "Texture Style meta child")?),
        }
    }
    if !saw_iref {
        payload.extend_from_slice(iref);
    }
    Ok(make_box(META, &payload)?)
}

fn texture_makernote() -> Result<Vec<u8>> {
    let tag84 = texture_tag84_bplist()?;
    let mut note = b"Apple iOS\0\0\x01MM".to_vec();
    note.extend_from_slice(&1u16.to_be_bytes());
    note.extend_from_slice(&84u16.to_be_bytes());
    note.extend_from_slice(&7u16.to_be_bytes());
    note.extend_from_slice(
        &u32::try_from(tag84.len())
            .map_err(|_| invalid("Texture Style tag84 length exceeds u32"))?
            .to_be_bytes(),
    );
    note.extend_from_slice(&28u32.to_be_bytes());
    note.extend_from_slice(&tag84);
    Ok(note)
}

fn texture_exif_payload(data: &[u8]) -> Result<Vec<u8>> {
    let tiff =
        heif_exif_tiff(data)?.ok_or_else(|| invalid("Texture Style source has no Exif item"))?;
    let note = texture_makernote()?;
    let patched = replace_exif_makernote(Some(&tiff), &note)?;
    let mut output = Vec::with_capacity(10 + patched.len());
    output.extend_from_slice(&6u32.to_be_bytes());
    output.extend_from_slice(b"Exif\0\0");
    output.extend_from_slice(&patched);
    Ok(output)
}

fn validate_output(data: &[u8], expected_alias_count: usize) -> Result<()> {
    let graph = parse_graph(data)?;
    let style_id = find_uri_item(
        data,
        &graph.meta,
        STYLE_METADATA_URI,
        "Texture Style style item",
    )?;
    let texture_id = find_uri_item(
        data,
        &graph.meta,
        TEXTURE_METADATA_URI,
        "Texture Style metadata item",
    )?;
    for (item_id, label, uri) in [
        (style_id, "style", STYLE_METADATA_URI),
        (texture_id, "texture", TEXTURE_METADATA_URI),
    ] {
        let item = graph
            .meta
            .iinf
            .entries
            .iter()
            .find(|item| item.item_id == item_id)
            .ok_or_else(|| invalid(format!("Texture Style {label} infe disappeared")))?;
        let raw = data
            .get(item.box_range.clone())
            .ok_or_else(|| invalid(format!("Texture Style {label} infe is outside output")))?;
        let mut needle = b"metadata\0".to_vec();
        needle.extend_from_slice(uri);
        if !raw.windows(needle.len()).any(|window| window == needle) {
            return Err(invalid(format!(
                "Texture Style {label} infe does not use the native metadata item name"
            )));
        }
        if item_location(&graph.meta, item_id, "Texture Style output")?.construction_method != 0 {
            return Err(invalid(format!(
                "Texture Style {label} metadata is not method-0"
            )));
        }
    }
    let role_count = graph
        .meta
        .properties
        .iter()
        .filter(|property| {
            property.kind == AUXC
                && raw_property(data, property).is_ok_and(|raw| {
                    TEXTURE_ROLE_AUX_TYPES
                        .iter()
                        .any(|uri| raw.windows(uri.len()).any(|window| window == *uri))
                })
        })
        .count();
    if role_count != expected_alias_count {
        return Err(invalid(format!(
            "Texture Style output has {role_count} 2026 semantic roles, expected {expected_alias_count}"
        )));
    }
    let tiff = heif_exif_tiff(data)?.ok_or_else(|| invalid("Texture Style output lost Exif"))?;
    let note =
        exif_makernote(&tiff)?.ok_or_else(|| invalid("Texture Style output lost MakerNote"))?;
    let tag84 = texture_tag84_bplist()?;
    if !note.starts_with(b"Apple iOS\0\0\x01")
        || !note
            .windows(tag84.len())
            .any(|window| window == tag84.as_slice())
    {
        return Err(invalid(
            "Texture Style output MakerNote tag84 contract is missing",
        ));
    }
    Ok(())
}

/// Upgrade a Rust-authored Photographic Styles HEIF to the clean-room iOS 27
/// Texture Style carrier contract. This mirrors the physical-device-positive H
/// carrier: style/Texture/Exif are method-0 and packed first in the single mdat;
/// the twelve 2026 semantic roles alias the current photo's own skin matte and
/// descriptor, so no donor image data enters the file.
pub fn augment_texture_style_heif(source: &[u8]) -> Result<Vec<u8>> {
    let graph = parse_graph(source)?;
    let style_id = find_uri_item(
        source,
        &graph.meta,
        STYLE_METADATA_URI,
        "Texture Style source style metadata",
    )?;
    if has_uri_item(source, &graph.meta, TEXTURE_METADATA_URI)? {
        return Err(invalid("source already contains Texture Style metadata"));
    }
    let exif_id = find_single_item(&graph.meta, EXIF, "Texture Style Exif item")?;
    let (source_skin_id, source_descriptor_id, source_skin_property) =
        find_skin_alias_source(source, &graph.meta)?;

    let style_location = item_location(&graph.meta, style_id, "Texture Style style metadata")?;
    let style_payload = read_item_payload(source, style_location, graph.meta.idat.as_ref())?;
    if !style_payload.starts_with(b"bplist00") {
        return Err(invalid(
            "Texture Style style metadata is not a binary plist",
        ));
    }
    let texture_payload = texture_metadata_bplist()?;
    let exif_payload = texture_exif_payload(source)?;

    let mut next_id = source_maximum_item_id(&graph.meta)?
        .checked_add(1)
        .ok_or_else(|| invalid("Texture Style item ID overflows"))?;
    let texture_id = next_id;
    next_id = next_id
        .checked_add(1)
        .ok_or_else(|| invalid("Texture Style item ID overflows"))?;
    let first_property = graph
        .meta
        .properties
        .len()
        .checked_add(1)
        .ok_or_else(|| invalid("Texture Style property index overflows"))?;
    let mut aliases = Vec::with_capacity(TEXTURE_ROLE_AUX_TYPES.len());
    for index in 0..TEXTURE_ROLE_AUX_TYPES.len() {
        let image = next_id;
        let descriptor = image
            .checked_add(1)
            .ok_or_else(|| invalid("Texture Style descriptor item ID overflows"))?;
        next_id = descriptor
            .checked_add(1)
            .ok_or_else(|| invalid("Texture Style item ID overflows"))?;
        let aux_property = u16::try_from(
            first_property
                .checked_add(index)
                .ok_or_else(|| invalid("Texture Style property index overflows"))?,
        )
        .map_err(|_| invalid("Texture Style property index exceeds u16"))?;
        aliases.push(AliasIds {
            image,
            descriptor,
            aux_property,
        });
    }

    let iinf = build_iinf(
        source,
        &graph.meta,
        style_id,
        texture_id,
        source_skin_id,
        &aliases,
    )?;
    let iprp = build_iprp(
        source,
        &graph,
        source_skin_id,
        source_skin_property,
        &aliases,
    )?;
    let iref = build_iref(&graph.meta, texture_id, &aliases)?;
    let maximum_id = aliases
        .last()
        .map(|alias| alias.descriptor)
        .unwrap_or(texture_id);
    let iloc_version = if maximum_id > u32::from(u16::MAX) {
        2
    } else {
        graph.meta.iloc.version.max(1)
    };

    let placeholders = build_locations(
        &graph,
        style_id,
        texture_id,
        exif_id,
        source_skin_id,
        source_descriptor_id,
        &aliases,
        &style_payload,
        &texture_payload,
        &exif_payload,
        true,
        0,
    )?;
    let placeholder_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &placeholders)?;
    let preliminary_meta = build_meta(source, &graph, &iinf, &placeholder_iloc, &iprp, &iref)?;

    let new_mdat_box_start = graph
        .top
        .boxes
        .iter()
        .take_while(|header| header.box_start != graph.mdat.box_start)
        .try_fold(0usize, |offset, header| {
            let length = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(length)
                .ok_or_else(|| invalid("Texture Style mdat offset overflows"))
        })?;
    let new_mdat_data_start = u64::try_from(
        new_mdat_box_start
            .checked_add(8)
            .ok_or_else(|| invalid("Texture Style mdat data offset overflows"))?,
    )
    .map_err(|_| invalid("Texture Style mdat data offset exceeds u64"))?;

    let final_locations = build_locations(
        &graph,
        style_id,
        texture_id,
        exif_id,
        source_skin_id,
        source_descriptor_id,
        &aliases,
        &style_payload,
        &texture_payload,
        &exif_payload,
        false,
        new_mdat_data_start,
    )?;
    let final_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &final_locations)?;
    let final_meta = build_meta(source, &graph, &iinf, &final_iloc, &iprp, &iref)?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid(
            "Texture Style iloc rewrite changed meta size unexpectedly",
        ));
    }

    let source_mdat_payload = source
        .get(graph.mdat.payload_range())
        .ok_or_else(|| invalid("Texture Style source mdat payload is outside input"))?;
    let prefix_len = style_payload
        .len()
        .checked_add(texture_payload.len())
        .and_then(|value| value.checked_add(exif_payload.len()))
        .ok_or_else(|| invalid("Texture Style final mdat length overflows"))?;
    let final_len = prefix_len
        .checked_add(source_mdat_payload.len())
        .ok_or_else(|| invalid("Texture Style final mdat length overflows"))?;
    if final_len > (u32::MAX as usize).saturating_sub(8) {
        return Err(invalid("Texture Style writer requires a 32-bit mdat size"));
    }
    let mut mdat_payload = Vec::with_capacity(final_len);
    mdat_payload.extend_from_slice(&style_payload);
    mdat_payload.extend_from_slice(&texture_payload);
    mdat_payload.extend_from_slice(&exif_payload);
    mdat_payload.extend_from_slice(source_mdat_payload);
    let final_mdat = make_box(MDAT, &mdat_payload)?;

    let trailing = source
        .get(graph.top.trailing_range.clone())
        .ok_or_else(|| invalid("Texture Style trailing bytes are outside input"))?;
    let mut output = Vec::new();
    for header in &graph.top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else if header.kind == MDAT {
            output.extend_from_slice(&final_mdat);
        } else {
            output.extend_from_slice(raw_box(source, header, "Texture Style top-level")?);
        }
    }
    output.extend_from_slice(trailing);
    validate_output(&output, aliases.len())?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_texture_contract_constants_are_stable() {
        assert_eq!(TEXTURE_ROLE_AUX_TYPES.len(), 12);
        assert_eq!(texture_metadata_bplist().unwrap().len(), 216);
        assert_eq!(texture_tag84_bplist().unwrap().len(), 133);
        let note = texture_makernote().unwrap();
        assert!(note.starts_with(b"Apple iOS\0\0\x01MM"));
        assert!(note.ends_with(&texture_tag84_bplist().unwrap()));
    }
}
