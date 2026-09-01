use std::collections::BTreeMap;

use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
    make_iref_box, make_irot_box, make_ispe_box, parse_boxes, parse_meta_box, scan_top_level_boxes,
    BoxHeader, IlocEntry, IlocExtent, IpmaAssociation, IpmaEntry, IrefEntry, ParsedMeta,
    PropertyInfo, FTYP, IDAT, IINF, ILOC, INFE, IPCO, IPMA, IPRP, IREF, MDAT, META,
};
use xdremux_format::{parse_hvcc_profile, ChromaSampling, FourCC};

use crate::direct::{DirectHevcGainMap, GainMapChannels};
use crate::error::{HeifError, Result};

const GRID: FourCC = FourCC::new(*b"grid");
const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const TMAP: FourCC = FourCC::new(*b"tmap");
const MIME: FourCC = FourCC::new(*b"mime");
const ISPE: FourCC = FourCC::new(*b"ispe");
const IROT: FourCC = FourCC::new(*b"irot");
const COLR: FourCC = FourCC::new(*b"colr");
const PIXI: FourCC = FourCC::new(*b"pixi");
const AUXC: FourCC = FourCC::new(*b"auxC");
const DIMG: FourCC = FourCC::new(*b"dimg");
const AUXL: FourCC = FourCC::new(*b"auxl");
const CDSC: FourCC = FourCC::new(*b"cdsc");

const ISO_21496_AUX_TYPE: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";

/// Fully portable input to the native Rust HEIF assembler.
///
/// The caller owns HDR metadata semantics and codec work. This layer only owns
/// deterministic ISO-BMFF graph construction around a preserved compressed base.
#[derive(Debug, Clone, Copy)]
pub struct IsoGainMapAssembly<'a> {
    pub gain_map: DirectHevcGainMap<'a>,
    pub tmap_payload: &'a [u8],
    pub xmp_payload: &'a [u8],
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn raw_box<'a>(source: &'a [u8], header: &BoxHeader, context: &str) -> Result<&'a [u8]> {
    source
        .get(header.box_range())
        .ok_or_else(|| invalid(format!("{context} box is outside source")))
}

fn raw_property<'a>(source: &'a [u8], property: &PropertyInfo) -> Result<&'a [u8]> {
    source
        .get(property.box_range.clone())
        .ok_or_else(|| invalid(format!("property {} is outside source", property.index)))
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

fn ceil_div(value: u32, divisor: u32, context: &str) -> Result<u32> {
    value
        .checked_add(divisor - 1)
        .map(|sum| sum / divisor)
        .ok_or_else(|| invalid(format!("{context} geometry overflows")))
}

fn validate_assembly(spec: &IsoGainMapAssembly<'_>) -> Result<(u32, u32)> {
    if spec.tmap_payload.is_empty() {
        return Err(invalid(
            "native ISO Gain Map assembly requires a tmap payload",
        ));
    }
    if spec.xmp_payload.is_empty() {
        return Err(invalid(
            "native ISO Gain Map assembly requires an hdrgm XMP payload",
        ));
    }

    let gain = &spec.gain_map;
    if gain.gain_map_width == 0
        || gain.gain_map_height == 0
        || gain.tile_width == 0
        || gain.tile_height == 0
    {
        return Err(invalid("native ISO Gain Map has zero geometry"));
    }
    if !(8..=15).contains(&gain.profile.luma_bit_depth)
        || !(8..=15).contains(&gain.profile.chroma_bit_depth)
    {
        return Err(invalid(
            "native ISO Gain Map bit depth must fit the hvcC 8..15 range",
        ));
    }
    match (gain.profile.channels, gain.profile.chroma) {
        (GainMapChannels::Mono, ChromaSampling::Mono400) => {}
        (
            GainMapChannels::Rgb,
            ChromaSampling::Yuv420 | ChromaSampling::Yuv422 | ChromaSampling::Yuv444,
        ) => {}
        (GainMapChannels::Mono, _) => {
            return Err(invalid(
                "monochrome Gain Map semantics require 4:0:0 storage",
            ));
        }
        (GainMapChannels::Rgb, ChromaSampling::Mono400) => {
            return Err(invalid("RGB Gain Map semantics require color HEVC storage"));
        }
    }

    let columns = ceil_div(gain.gain_map_width, gain.tile_width, "gain-map columns")?;
    let rows = ceil_div(gain.gain_map_height, gain.tile_height, "gain-map rows")?;
    let expected_count = usize::try_from(
        rows.checked_mul(columns)
            .ok_or_else(|| invalid("native ISO Gain Map tile count overflows"))?,
    )
    .map_err(|_| invalid("native ISO Gain Map tile count exceeds usize"))?;
    if gain.tiles.len() != expected_count || gain.tiles.is_empty() {
        return Err(invalid(
            "native ISO Gain Map tile count does not match geometry",
        ));
    }

    for row in 0..rows {
        for column in 0..columns {
            let index = usize::try_from(row * columns + column)
                .map_err(|_| invalid("native ISO Gain Map tile index exceeds usize"))?;
            let tile = gain.tiles[index];
            if tile.payload.is_empty() {
                return Err(invalid("native ISO Gain Map contains an empty HEVC tile"));
            }
            let expected_width = gain
                .gain_map_width
                .checked_sub(
                    column
                        .checked_mul(gain.tile_width)
                        .ok_or_else(|| invalid("native ISO Gain Map tile x overflows"))?,
                )
                .ok_or_else(|| invalid("native ISO Gain Map tile x exceeds image"))?
                .min(gain.tile_width);
            let expected_height = gain
                .gain_map_height
                .checked_sub(
                    row.checked_mul(gain.tile_height)
                        .ok_or_else(|| invalid("native ISO Gain Map tile y overflows"))?,
                )
                .ok_or_else(|| invalid("native ISO Gain Map tile y exceeds image"))?
                .min(gain.tile_height);
            let logical_edge = tile.width == expected_width && tile.height == expected_height;
            let padded_full = tile.width == gain.tile_width && tile.height == gain.tile_height;
            if !logical_edge && !padded_full {
                return Err(invalid(
                    "native ISO Gain Map edge tile geometry is inconsistent",
                ));
            }
        }
    }

    let codec = parse_hvcc_profile(gain.hvcc)
        .map_err(|error| invalid(format!("invalid native Gain Map hvcC: {error}")))?;
    if codec.chroma_sampling != gain.profile.chroma
        || codec.luma_bit_depth != gain.profile.luma_bit_depth
        || codec.chroma_bit_depth != gain.profile.chroma_bit_depth
    {
        return Err(invalid(
            "native Gain Map hvcC does not match its encode profile",
        ));
    }

    Ok((rows, columns))
}

fn make_grid_payload(rows: u32, columns: u32, width: u32, height: u32) -> Result<Vec<u8>> {
    if !(1..=256).contains(&rows) || !(1..=256).contains(&columns) || width == 0 || height == 0 {
        return Err(invalid("invalid native HEIF grid geometry"));
    }
    let large = width > u32::from(u16::MAX) || height > u32::from(u16::MAX);
    let mut payload = vec![0, u8::from(large), (rows - 1) as u8, (columns - 1) as u8];
    if large {
        payload.extend_from_slice(&width.to_be_bytes());
        payload.extend_from_slice(&height.to_be_bytes());
    } else {
        payload.extend_from_slice(&(width as u16).to_be_bytes());
        payload.extend_from_slice(&(height as u16).to_be_bytes());
    }
    Ok(payload)
}

fn make_auxc_box() -> Result<Vec<u8>> {
    Ok(make_full_box(AUXC, 0, 0, ISO_21496_AUX_TYPE)?)
}

fn make_nclx_box(primaries: u16, transfer: u16, matrix: u16) -> Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(11);
    payload.extend_from_slice(b"nclx");
    payload.extend_from_slice(&primaries.to_be_bytes());
    payload.extend_from_slice(&transfer.to_be_bytes());
    payload.extend_from_slice(&matrix.to_be_bytes());
    payload.push(0x80);
    Ok(make_box(COLR, &payload)?)
}

fn make_gain_pixi_box(gain: &DirectHevcGainMap<'_>) -> Result<Vec<u8>> {
    let mut payload = vec![gain.profile.channels.semantic_channel_count()];
    match gain.profile.channels {
        GainMapChannels::Mono => payload.push(gain.profile.luma_bit_depth),
        GainMapChannels::Rgb => {
            payload.push(gain.profile.luma_bit_depth);
            payload.push(gain.profile.chroma_bit_depth);
            payload.push(gain.profile.chroma_bit_depth);
        }
    }
    Ok(make_full_box(PIXI, 0, 0, &payload)?)
}

fn make_tmap_pixi_box() -> Result<Vec<u8>> {
    Ok(make_full_box(PIXI, 0, 0, &[3, 10, 10, 10])?)
}

fn make_mime_infe_box(item_id: u32, flags: u32) -> Result<Vec<u8>> {
    let version = if item_id <= u32::from(u16::MAX) { 2 } else { 3 };
    let mut payload = Vec::new();
    if version == 2 {
        payload.extend_from_slice(&(item_id as u16).to_be_bytes());
    } else {
        payload.extend_from_slice(&item_id.to_be_bytes());
    }
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(MIME.as_bytes());
    payload.extend_from_slice(b"hdrgm-xmp\0");
    payload.extend_from_slice(b"application/rdf+xml\0");
    Ok(make_full_box(INFE, version, flags, &payload)?)
}

fn associated_property_index(meta: &ParsedMeta, item_id: u32, kind: FourCC) -> Option<u16> {
    let entry = meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)?;
    entry.associations.iter().find_map(|association| {
        meta.properties
            .iter()
            .find(|property| property.index == u32::from(association.property_index))
            .filter(|property| property.kind == kind)
            .map(|_| association.property_index)
    })
}

fn append_property(output: &mut Vec<u8>, next_index: &mut u16, raw: &[u8]) -> Result<u16> {
    if *next_index == 0 || *next_index > 0x7fff {
        return Err(invalid("native HEIF property index exceeds 15 bits"));
    }
    let index = *next_index;
    output.extend_from_slice(raw);
    *next_index = next_index
        .checked_add(1)
        .ok_or_else(|| invalid("native HEIF property index overflows"))?;
    Ok(index)
}

fn build_ftyp(source: &[u8], ftyp: &BoxHeader) -> Result<Vec<u8>> {
    let payload = source
        .get(ftyp.payload_range())
        .ok_or_else(|| invalid("ftyp payload is outside source"))?;
    if payload.len() < 8 || (payload.len() - 8) % 4 != 0 {
        return Err(invalid("ftyp payload has invalid brand layout"));
    }
    let mut output_payload = payload[..8].to_vec();
    let mut brands: Vec<[u8; 4]> =
        [*b"mif1", *b"tmap", *b"MiHE", *b"miaf", *b"MiHB", *b"heic"].to_vec();
    for chunk in payload[8..].chunks_exact(4) {
        let brand: [u8; 4] = chunk.try_into().expect("chunks_exact returns four bytes");
        if !brands.contains(&brand) {
            brands.push(brand);
        }
    }
    for brand in brands {
        output_payload.extend_from_slice(&brand);
    }
    Ok(make_box(FTYP, &output_payload)?)
}

fn build_iprp(
    source: &[u8],
    iprp_header: &BoxHeader,
    meta: &ParsedMeta,
    gain: &DirectHevcGainMap<'_>,
    tile_ids: &[u32],
    gain_id: u32,
    tmap_id: u32,
) -> Result<Vec<u8>> {
    let mut properties = meta.properties.iter().collect::<Vec<_>>();
    properties.sort_by_key(|property| property.index);
    let mut ipco_payload = Vec::new();
    for (offset, property) in properties.iter().enumerate() {
        let expected = u32::try_from(offset + 1)
            .map_err(|_| invalid("native HEIF property count exceeds u32"))?;
        if property.index != expected {
            return Err(invalid("source ipco property indices are not contiguous"));
        }
        ipco_payload.extend_from_slice(raw_property(source, property)?);
    }
    let mut next_property_index = u16::try_from(properties.len() + 1)
        .map_err(|_| invalid("source ipco property count exceeds u16"))?;

    let primary_ispe = associated_property_index(meta, meta.primary_item_id, ISPE)
        .ok_or_else(|| invalid("primary item has no ispe property association"))?;
    let irot = match associated_property_index(meta, meta.primary_item_id, IROT) {
        Some(index) => index,
        None => append_property(
            &mut ipco_payload,
            &mut next_property_index,
            &make_irot_box(0)?,
        )?,
    };
    let auxc = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_auxc_box()?,
    )?;
    let gain_colr = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_nclx_box(2, 2, 2)?,
    )?;
    let gain_pixi = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_gain_pixi_box(gain)?,
    )?;
    let gain_ispe = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_ispe_box(gain.gain_map_width, gain.gain_map_height)?,
    )?;
    let tile_hvcc = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_box(HVCC, gain.hvcc)?,
    )?;
    let tmap_colr = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_nclx_box(9, 16, 9)?,
    )?;
    let tmap_pixi = append_property(
        &mut ipco_payload,
        &mut next_property_index,
        &make_tmap_pixi_box()?,
    )?;

    let mut tile_ispe_by_size = BTreeMap::new();
    for tile in gain.tiles {
        let size = (tile.width, tile.height);
        if let std::collections::btree_map::Entry::Vacant(entry) = tile_ispe_by_size.entry(size) {
            let index = append_property(
                &mut ipco_payload,
                &mut next_property_index,
                &make_ispe_box(tile.width, tile.height)?,
            )?;
            entry.insert(index);
        }
    }

    let ipco_part = make_box(IPCO, &ipco_payload)?;
    let last_property_index = next_property_index.saturating_sub(1);
    let mut ipma_entries = meta.ipma.entries.clone();
    for (tile_id, tile) in tile_ids.iter().copied().zip(gain.tiles.iter().copied()) {
        let tile_ispe = *tile_ispe_by_size
            .get(&(tile.width, tile.height))
            .ok_or_else(|| invalid("native HEIF tile ispe mapping is missing"))?;
        ipma_entries.push(IpmaEntry {
            item_id: tile_id,
            associations: vec![
                IpmaAssociation {
                    property_index: tile_ispe,
                    essential: true,
                },
                IpmaAssociation {
                    property_index: gain_colr,
                    essential: true,
                },
                IpmaAssociation {
                    property_index: tile_hvcc,
                    essential: true,
                },
            ],
        });
    }
    ipma_entries.push(IpmaEntry {
        item_id: gain_id,
        associations: vec![
            IpmaAssociation {
                property_index: gain_colr,
                essential: true,
            },
            IpmaAssociation {
                property_index: gain_ispe,
                essential: false,
            },
            IpmaAssociation {
                property_index: gain_pixi,
                essential: false,
            },
            IpmaAssociation {
                property_index: irot,
                essential: true,
            },
            IpmaAssociation {
                property_index: auxc,
                essential: true,
            },
        ],
    });
    ipma_entries.push(IpmaEntry {
        item_id: tmap_id,
        associations: vec![
            IpmaAssociation {
                property_index: tmap_colr,
                essential: true,
            },
            IpmaAssociation {
                property_index: primary_ispe,
                essential: false,
            },
            IpmaAssociation {
                property_index: tmap_pixi,
                essential: false,
            },
            IpmaAssociation {
                property_index: irot,
                essential: true,
            },
        ],
    });

    let maximum_item_id = ipma_entries
        .iter()
        .map(|entry| entry.item_id)
        .max()
        .unwrap_or(0);
    let ipma_version = if maximum_item_id > u32::from(u16::MAX) {
        1
    } else {
        meta.ipma.version
    };
    let ipma_flags = if last_property_index > 0x7f {
        meta.ipma.flags | 1
    } else {
        meta.ipma.flags
    };
    let ipma_part = make_ipma_box(ipma_version, ipma_flags, &ipma_entries)?;

    let iprp_children = parse_boxes(source, iprp_header.payload_range())?;
    let mut payload = Vec::new();
    let mut saw_ipco = false;
    let mut saw_ipma = false;
    for header in &iprp_children {
        if header.kind == IPCO {
            if saw_ipco {
                return Err(invalid("source iprp contains more than one ipco"));
            }
            saw_ipco = true;
            payload.extend_from_slice(&ipco_part);
        } else if header.kind == IPMA {
            if saw_ipma {
                return Err(invalid("source iprp contains more than one ipma"));
            }
            saw_ipma = true;
            payload.extend_from_slice(&ipma_part);
        } else {
            payload.extend_from_slice(raw_box(source, header, "iprp child")?);
        }
    }
    if !saw_ipco || !saw_ipma {
        return Err(invalid("source iprp is missing ipco/ipma"));
    }
    Ok(make_box(IPRP, &payload)?)
}

fn build_iinf(
    source: &[u8],
    meta: &ParsedMeta,
    tile_ids: &[u32],
    gain_id: u32,
    tmap_id: u32,
    xmp_id: u32,
) -> Result<Vec<u8>> {
    let mut entries = Vec::new();
    for item in &meta.iinf.entries {
        entries.push(
            source
                .get(item.box_range.clone())
                .ok_or_else(|| invalid(format!("infe {} is outside source", item.item_id)))?
                .to_vec(),
        );
    }
    for tile_id in tile_ids {
        entries.push(make_infe_box(*tile_id, HVC1, 1)?);
    }
    entries.push(make_infe_box(gain_id, GRID, 1)?);
    entries.push(make_infe_box(tmap_id, TMAP, 0)?);
    entries.push(make_mime_infe_box(xmp_id, 1)?);

    let version = if meta.iinf.version == 0 && entries.len() > usize::from(u16::MAX) {
        1
    } else {
        meta.iinf.version
    };
    Ok(make_iinf_box(version, &entries)?)
}

fn build_iref(
    meta: &ParsedMeta,
    tile_ids: &[u32],
    gain_id: u32,
    tmap_id: u32,
    xmp_id: u32,
) -> Result<Vec<u8>> {
    let mut entries = meta
        .iref
        .as_ref()
        .map_or_else(Vec::new, |iref| iref.entries.clone());
    entries.push(IrefEntry {
        kind: DIMG,
        from_item_id: gain_id,
        to_item_ids: tile_ids.to_vec(),
    });
    entries.push(IrefEntry {
        kind: DIMG,
        from_item_id: tmap_id,
        to_item_ids: vec![meta.primary_item_id, gain_id],
    });
    entries.push(IrefEntry {
        kind: AUXL,
        from_item_id: gain_id,
        to_item_ids: vec![meta.primary_item_id],
    });
    entries.push(IrefEntry {
        kind: CDSC,
        from_item_id: xmp_id,
        to_item_ids: vec![meta.primary_item_id, tmap_id],
    });

    let maximum_item_id = entries
        .iter()
        .flat_map(|entry| {
            std::iter::once(entry.from_item_id).chain(entry.to_item_ids.iter().copied())
        })
        .max()
        .unwrap_or(0);
    let source_version = meta.iref.as_ref().map_or(0, |iref| iref.version);
    let version = if maximum_item_id > u32::from(u16::MAX) {
        1
    } else {
        source_version
    };
    Ok(make_iref_box(version, &entries)?)
}

fn existing_idat_payload<'a>(source: &'a [u8], meta: &ParsedMeta) -> Result<&'a [u8]> {
    match meta.idat.as_ref() {
        Some(idat) => source
            .get(idat.payload_range())
            .ok_or_else(|| invalid("source idat payload is outside source")),
        None => Ok(&[]),
    }
}

fn normalized_existing_location(entry: &IlocEntry, placeholder: bool) -> Result<IlocEntry> {
    if entry.data_reference_index != 0 {
        return Err(invalid(format!(
            "item {} uses unsupported data_reference_index {}",
            entry.item_id, entry.data_reference_index
        )));
    }
    if !matches!(entry.construction_method, 0 | 1) {
        return Err(invalid(format!(
            "item {} uses unsupported construction_method {}",
            entry.item_id, entry.construction_method
        )));
    }
    let mut extents = Vec::with_capacity(entry.extents.len());
    for extent in &entry.extents {
        if extent.index.unwrap_or(0) != 0 {
            return Err(invalid(format!(
                "item {} uses unsupported non-zero extent_index",
                entry.item_id
            )));
        }
        let offset = if placeholder && entry.construction_method == 0 {
            0
        } else {
            entry.resolved_extent_offset(extent)?
        };
        extents.push(IlocExtent {
            index: None,
            offset,
            length: extent.length,
        });
    }
    Ok(IlocEntry {
        item_id: entry.item_id,
        construction_method: entry.construction_method,
        data_reference_index: 0,
        base_offset: 0,
        extents,
    })
}

fn ensure_u32(value: u64, context: &str) -> Result<()> {
    if value > u64::from(u32::MAX) {
        return Err(invalid(format!(
            "{context} exceeds the current 32-bit iloc contract"
        )));
    }
    Ok(())
}

fn build_placeholder_locations(
    meta: &ParsedMeta,
    tile_ids: &[u32],
    gain_id: u32,
    tmap_id: u32,
    xmp_id: u32,
    gain_grid_offset: u64,
    gain_grid_length: u64,
    tmap_offset: u64,
    tmap_length: u64,
    xmp_offset: u64,
    xmp_length: u64,
    gain: &DirectHevcGainMap<'_>,
) -> Result<Vec<IlocEntry>> {
    let mut entries = meta
        .iloc
        .entries
        .iter()
        .map(|entry| normalized_existing_location(entry, true))
        .collect::<Result<Vec<_>>>()?;
    for (tile_id, tile) in tile_ids.iter().copied().zip(gain.tiles.iter().copied()) {
        entries.push(IlocEntry {
            item_id: tile_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: u64::try_from(tile.payload.len())
                    .map_err(|_| invalid("HEVC tile length exceeds u64"))?,
            }],
        });
    }
    for (item_id, offset, length) in [
        (gain_id, gain_grid_offset, gain_grid_length),
        (tmap_id, tmap_offset, tmap_length),
        (xmp_id, xmp_offset, xmp_length),
    ] {
        entries.push(IlocEntry {
            item_id,
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
    entries.sort_by_key(|entry| entry.item_id);
    Ok(entries)
}

#[allow(clippy::too_many_arguments)]
fn build_final_locations(
    source: &[u8],
    meta: &ParsedMeta,
    mdat: &BoxHeader,
    new_mdat_data_start: u64,
    original_mdat_payload_len: usize,
    tile_ids: &[u32],
    gain_id: u32,
    tmap_id: u32,
    xmp_id: u32,
    gain_grid_offset: u64,
    gain_grid_length: u64,
    tmap_offset: u64,
    tmap_length: u64,
    xmp_offset: u64,
    xmp_length: u64,
    gain: &DirectHevcGainMap<'_>,
) -> Result<Vec<IlocEntry>> {
    let old_mdat_start =
        u64::try_from(mdat.data_start).map_err(|_| invalid("source mdat offset exceeds u64"))?;
    let old_mdat_end =
        u64::try_from(mdat.data_end).map_err(|_| invalid("source mdat end exceeds u64"))?;
    let idat_len = u64::try_from(existing_idat_payload(source, meta)?.len())
        .map_err(|_| invalid("source idat length exceeds u64"))?;

    let mut entries = Vec::new();
    for entry in &meta.iloc.entries {
        let mut normalized = normalized_existing_location(entry, false)?;
        for extent in &mut normalized.extents {
            if normalized.construction_method == 0 {
                let old_start = extent.offset;
                let old_end = old_start.checked_add(extent.length).ok_or_else(|| {
                    invalid(format!("item {} extent end overflows", entry.item_id))
                })?;
                if old_start < old_mdat_start || old_end > old_mdat_end {
                    return Err(invalid(format!(
                        "item {} has file-backed data outside the primary mdat; native augmentation refuses to guess",
                        entry.item_id
                    )));
                }
                let relative = old_start - old_mdat_start;
                extent.offset = new_mdat_data_start.checked_add(relative).ok_or_else(|| {
                    invalid(format!("item {} relocated offset overflows", entry.item_id))
                })?;
            } else {
                let end = extent.offset.checked_add(extent.length).ok_or_else(|| {
                    invalid(format!("item {} idat extent end overflows", entry.item_id))
                })?;
                if end > idat_len {
                    return Err(invalid(format!(
                        "item {} has an idat extent outside the preserved source idat",
                        entry.item_id
                    )));
                }
            }
            ensure_u32(extent.offset, "existing iloc offset")?;
            ensure_u32(extent.length, "existing iloc length")?;
        }
        entries.push(normalized);
    }

    let mut appended = 0u64;
    let original_payload_len = u64::try_from(original_mdat_payload_len)
        .map_err(|_| invalid("source mdat payload length exceeds u64"))?;
    for (tile_id, tile) in tile_ids.iter().copied().zip(gain.tiles.iter().copied()) {
        let offset = new_mdat_data_start
            .checked_add(original_payload_len)
            .and_then(|value| value.checked_add(appended))
            .ok_or_else(|| invalid("native HEVC tile file offset overflows"))?;
        let length = u64::try_from(tile.payload.len())
            .map_err(|_| invalid("native HEVC tile length exceeds u64"))?;
        ensure_u32(offset, "native HEVC tile offset")?;
        ensure_u32(length, "native HEVC tile length")?;
        entries.push(IlocEntry {
            item_id: tile_id,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset,
                length,
            }],
        });
        appended = appended
            .checked_add(length)
            .ok_or_else(|| invalid("native HEVC tile payload length overflows"))?;
    }

    for (item_id, offset, length) in [
        (gain_id, gain_grid_offset, gain_grid_length),
        (tmap_id, tmap_offset, tmap_length),
        (xmp_id, xmp_offset, xmp_length),
    ] {
        ensure_u32(offset, "native idat offset")?;
        ensure_u32(length, "native idat length")?;
        entries.push(IlocEntry {
            item_id,
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
    entries.sort_by_key(|entry| entry.item_id);
    Ok(entries)
}

#[allow(clippy::too_many_arguments)]
fn build_meta(
    source: &[u8],
    meta_header: &BoxHeader,
    meta_children: &[BoxHeader],
    iinf: &[u8],
    iloc: &[u8],
    iprp: &[u8],
    iref: &[u8],
    idat: &[u8],
) -> Result<Vec<u8>> {
    let full_header_end = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta full-box header offset overflows"))?;
    let full_header = source
        .get(meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("meta full-box header is truncated"))?;
    let mut payload = full_header.to_vec();
    let mut saw_iref = false;
    let mut saw_idat = false;
    for header in meta_children {
        if header.kind == IINF {
            payload.extend_from_slice(iinf);
        } else if header.kind == ILOC {
            payload.extend_from_slice(iloc);
        } else if header.kind == IPRP {
            payload.extend_from_slice(iprp);
        } else if header.kind == IREF {
            if saw_iref {
                return Err(invalid("source meta contains more than one iref"));
            }
            saw_iref = true;
            payload.extend_from_slice(iref);
        } else if header.kind == IDAT {
            if saw_idat {
                return Err(invalid("source meta contains more than one idat"));
            }
            saw_idat = true;
            payload.extend_from_slice(idat);
        } else {
            payload.extend_from_slice(raw_box(source, header, "meta child")?);
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

/// Construct an ISO 21496 tiled HEVC Gain Map graph directly from an ordinary
/// source HEIF. No Python, Swift, ImageIO, or temporary JPEG Gain Map graph is
/// required. The compressed base item data is preserved byte-for-byte.
pub fn assemble_iso_gain_map_heif(
    source: &[u8],
    assembly: &IsoGainMapAssembly<'_>,
) -> Result<Vec<u8>> {
    let (rows, columns) = validate_assembly(assembly)?;
    let gain = &assembly.gain_map;

    let top = scan_top_level_boxes(source)?;
    let ftyp = one_top_level(&top.boxes, FTYP, "ftyp")?;
    let meta_header = one_top_level(&top.boxes, META, "meta")?;
    let mdat = one_top_level(&top.boxes, MDAT, "mdat")?;
    if ftyp.box_start != 0
        || ftyp.data_end > meta_header.box_start
        || meta_header.data_end > mdat.box_start
    {
        return Err(invalid(
            "native HEIF augmentation requires ftyp -> meta -> mdat top-level order",
        ));
    }

    let meta = parse_meta_box(source, meta_header)?;
    if meta
        .iinf
        .entries
        .iter()
        .any(|item| item.item_type == Some(TMAP))
    {
        return Err(invalid(
            "source already contains a tmap graph; canonical graph replacement is a separate operation",
        ));
    }
    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta child offset overflows"))?;
    if meta_children_start > meta_header.data_end {
        return Err(invalid("meta full-box header is truncated"));
    }
    let meta_children = parse_boxes(source, meta_children_start..meta_header.data_end)?;
    let iprp_header = child(&meta_children, IPRP, "meta")?;
    let _ = child(&meta_children, IINF, "meta")?;
    let _ = child(&meta_children, ILOC, "meta")?;

    let original_max_item_id = meta
        .iinf
        .entries
        .iter()
        .map(|item| item.item_id)
        .max()
        .ok_or_else(|| invalid("source HEIF has no items"))?;
    let mut next_item_id = original_max_item_id
        .checked_add(1)
        .ok_or_else(|| invalid("native HEIF item ID overflows"))?;
    let mut allocate = || -> Result<u32> {
        let value = next_item_id;
        next_item_id = next_item_id
            .checked_add(1)
            .ok_or_else(|| invalid("native HEIF item ID overflows"))?;
        Ok(value)
    };

    let mut tile_ids = Vec::with_capacity(gain.tiles.len());
    for _ in gain.tiles {
        tile_ids.push(allocate()?);
    }
    let gain_id = allocate()?;
    let tmap_id = allocate()?;
    let xmp_id = allocate()?;
    let maximum_item_id = xmp_id;

    let iinf = build_iinf(source, &meta, &tile_ids, gain_id, tmap_id, xmp_id)?;
    let iprp = build_iprp(
        source,
        iprp_header,
        &meta,
        gain,
        &tile_ids,
        gain_id,
        tmap_id,
    )?;
    let iref = build_iref(&meta, &tile_ids, gain_id, tmap_id, xmp_id)?;

    let mut idat_payload = existing_idat_payload(source, &meta)?.to_vec();
    let gain_grid_offset =
        u64::try_from(idat_payload.len()).map_err(|_| invalid("native idat offset exceeds u64"))?;
    let gain_grid_payload =
        make_grid_payload(rows, columns, gain.gain_map_width, gain.gain_map_height)?;
    idat_payload.extend_from_slice(&gain_grid_payload);
    let tmap_offset =
        u64::try_from(idat_payload.len()).map_err(|_| invalid("native tmap offset exceeds u64"))?;
    idat_payload.extend_from_slice(assembly.tmap_payload);
    let xmp_offset =
        u64::try_from(idat_payload.len()).map_err(|_| invalid("native XMP offset exceeds u64"))?;
    idat_payload.extend_from_slice(assembly.xmp_payload);
    let idat = make_box(IDAT, &idat_payload)?;

    let gain_grid_length = u64::try_from(gain_grid_payload.len())
        .map_err(|_| invalid("gain-grid payload length exceeds u64"))?;
    let tmap_length = u64::try_from(assembly.tmap_payload.len())
        .map_err(|_| invalid("tmap payload length exceeds u64"))?;
    let xmp_length = u64::try_from(assembly.xmp_payload.len())
        .map_err(|_| invalid("XMP payload length exceeds u64"))?;

    let iloc_version = if maximum_item_id > u32::from(u16::MAX) {
        2
    } else {
        meta.iloc.version.max(1)
    };
    let placeholder_locations = build_placeholder_locations(
        &meta,
        &tile_ids,
        gain_id,
        tmap_id,
        xmp_id,
        gain_grid_offset,
        gain_grid_length,
        tmap_offset,
        tmap_length,
        xmp_offset,
        xmp_length,
        gain,
    )?;
    let placeholder_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &placeholder_locations)?;
    let preliminary_meta = build_meta(
        source,
        meta_header,
        &meta_children,
        &iinf,
        &placeholder_iloc,
        &iprp,
        &iref,
        &idat,
    )?;

    let ftyp_part = build_ftyp(source, ftyp)?;
    let between_ftyp_meta = source
        .get(ftyp.data_end..meta_header.box_start)
        .ok_or_else(|| invalid("bytes between ftyp and meta are outside source"))?;
    let between_meta_mdat = source
        .get(meta_header.data_end..mdat.box_start)
        .ok_or_else(|| invalid("bytes between meta and mdat are outside source"))?;
    let original_mdat_payload = source
        .get(mdat.payload_range())
        .ok_or_else(|| invalid("source mdat payload is outside source"))?;

    let new_mdat_box_start = ftyp_part
        .len()
        .checked_add(between_ftyp_meta.len())
        .and_then(|value| value.checked_add(preliminary_meta.len()))
        .and_then(|value| value.checked_add(between_meta_mdat.len()))
        .ok_or_else(|| invalid("native mdat file offset overflows"))?;
    let new_mdat_data_start = new_mdat_box_start
        .checked_add(8)
        .ok_or_else(|| invalid("native mdat data offset overflows"))?;
    let new_mdat_data_start_u64 = u64::try_from(new_mdat_data_start)
        .map_err(|_| invalid("native mdat data offset exceeds u64"))?;
    ensure_u32(new_mdat_data_start_u64, "native mdat data offset")?;

    let final_locations = build_final_locations(
        source,
        &meta,
        mdat,
        new_mdat_data_start_u64,
        original_mdat_payload.len(),
        &tile_ids,
        gain_id,
        tmap_id,
        xmp_id,
        gain_grid_offset,
        gain_grid_length,
        tmap_offset,
        tmap_length,
        xmp_offset,
        xmp_length,
        gain,
    )?;
    let final_iloc = make_iloc_box(iloc_version, 4, 4, 0, 0, &final_locations)?;
    let final_meta = build_meta(
        source,
        meta_header,
        &meta_children,
        &iinf,
        &final_iloc,
        &iprp,
        &iref,
        &idat,
    )?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid(
            "native HEIF iloc rewrite changed meta size unexpectedly",
        ));
    }

    let appended_tile_bytes = gain.tiles.iter().try_fold(0usize, |total, tile| {
        total
            .checked_add(tile.payload.len())
            .ok_or_else(|| invalid("native mdat payload length overflows"))
    })?;
    let final_mdat_payload_len = original_mdat_payload
        .len()
        .checked_add(appended_tile_bytes)
        .ok_or_else(|| invalid("native mdat payload length overflows"))?;
    if final_mdat_payload_len > (u32::MAX as usize).saturating_sub(8) {
        return Err(invalid(
            "native HEIF writer currently requires an mdat that fits a 32-bit box size",
        ));
    }
    let mut final_mdat_payload = Vec::with_capacity(final_mdat_payload_len);
    final_mdat_payload.extend_from_slice(original_mdat_payload);
    for tile in gain.tiles {
        final_mdat_payload.extend_from_slice(tile.payload);
    }
    let final_mdat = make_box(MDAT, &final_mdat_payload)?;
    if final_mdat.len() != final_mdat_payload.len() + 8 {
        return Err(invalid(
            "native HEIF mdat unexpectedly used a large-size header",
        ));
    }

    let after_mdat = source
        .get(mdat.data_end..)
        .ok_or_else(|| invalid("bytes after mdat are outside source"))?;
    let mut output = Vec::with_capacity(
        ftyp_part.len()
            + between_ftyp_meta.len()
            + final_meta.len()
            + between_meta_mdat.len()
            + final_mdat.len()
            + after_mdat.len(),
    );
    output.extend_from_slice(&ftyp_part);
    output.extend_from_slice(between_ftyp_meta);
    output.extend_from_slice(&final_meta);
    output.extend_from_slice(between_meta_mdat);
    output.extend_from_slice(&final_mdat);
    output.extend_from_slice(after_mdat);

    let structure = crate::validation::validate_gain_map_structure(&output)?;
    if structure.primary_item_id != meta.primary_item_id
        || structure.gain_map_item_id != gain_id
        || structure.tmap_item_id != tmap_id
        || structure.tile_item_ids != tile_ids
    {
        return Err(invalid(
            "native HEIF post-write structure does not match the assembly plan",
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_rejects_empty_metadata_before_parsing_source() {
        let tile = crate::GainMapTile {
            payload: &[1],
            width: 4,
            height: 4,
        };
        let mut hvcc = vec![0u8; 19];
        hvcc[0] = 1;
        hvcc[1] = 4;
        hvcc[16] = 3;
        let gain = DirectHevcGainMap {
            gain_map_width: 4,
            gain_map_height: 4,
            tile_width: 4,
            tile_height: 4,
            tiles: std::slice::from_ref(&tile),
            hvcc: &hvcc,
            profile: crate::GainMapEncodeProfile {
                channels: GainMapChannels::Rgb,
                chroma: ChromaSampling::Yuv444,
                luma_bit_depth: 8,
                chroma_bit_depth: 8,
            },
        };
        assert!(assemble_iso_gain_map_heif(
            &[],
            &IsoGainMapAssembly {
                gain_map: gain,
                tmap_payload: &[],
                xmp_payload: b"xmp",
            },
        )
        .is_err());
    }
}
