//! Opaque Texture Styles transport. This module does not produce capture facts.
use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use xdremux_format::exif::read_item_payload;
use xdremux_format::isobmff::{
    parse_meta_box, scan_top_level_boxes, BoxHeader, IlocEntry, ItemInfo, ParsedMeta, FTYP, MDAT,
    META,
};
use xdremux_format::{Cursor, Endian, FourCC};

use crate::error::{HeifError, Result};

const URI: FourCC = FourCC::new(*b"uri ");
const AUXC: FourCC = FourCC::new(*b"auxC");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const TEXTURE_URI: &[u8] = b"tag:apple.com,2026:photo:metadata:texture_styles";
const TEXTURE_AUX_PREFIX: &[u8] = b"tag:apple.com,2026:photo:aux:";

/// Container observations only. These fields do not certify plist semantics,
/// capture provenance, a native producer, rendering, or Photos editability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureStyleCarrierFacts {
    pub primary_item_id: u32,
    pub metadata_item_id: u32,
    pub metadata_target_item_ids: Vec<u32>,
    pub metadata_byte_count: usize,
    pub metadata_sha256: [u8; 32],
    pub metadata_has_binary_plist_header: bool,
    pub auxiliary_type_claims: Vec<String>,
}

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn one_box(boxes: &[BoxHeader], kind: FourCC) -> Result<&BoxHeader> {
    let mut matching = boxes.iter().filter(|b| b.kind == kind);
    let value = matching
        .next()
        .ok_or_else(|| invalid(format!("Texture transport: missing {kind}")))?;
    if matching.next().is_some() {
        return Err(invalid(format!("Texture transport: duplicate {kind}")));
    }
    Ok(value)
}

// Read the URI field itself. A matching substring in the name or another
// property must not authorize Texture transport.
fn item_uri<'a>(data: &'a [u8], item: &ItemInfo) -> Result<Option<&'a [u8]>> {
    if item.item_type != Some(URI) {
        return Ok(None);
    }
    let mut cursor = Cursor::bounded(
        data,
        item.box_range.clone(),
        Endian::Big,
        "Texture URI infe",
    )?;
    let size = cursor.read_u32()?;
    cursor.skip(4)?;
    if size == 1 {
        cursor.skip(8)?;
    }
    let version = cursor.read_u8()?;
    cursor.skip(3)?;
    cursor.skip(match version {
        2 => 2,
        3 => 4,
        _ => return Err(invalid("Texture URI uses unsupported infe version")),
    })?;
    cursor.skip(2 + 4)?;
    let _name = cursor.read_c_string()?;
    let uri = cursor.read_c_string()?;
    if !cursor.is_empty() {
        return Err(invalid("Texture URI infe has trailing fields"));
    }
    Ok(Some(uri))
}

fn check_location(
    data: &[u8],
    meta: &ParsedMeta,
    entry: &IlocEntry,
    mdats: &[&BoxHeader],
) -> Result<()> {
    if entry.data_reference_index != 0 || !matches!(entry.construction_method, 0 | 1) {
        return Err(invalid(format!(
            "Texture transport: item {} needs an unsupported external or derived location",
            entry.item_id
        )));
    }
    if entry.extents.is_empty() {
        return Err(invalid("Texture transport: item has no extents"));
    }
    for extent in &entry.extents {
        let start = entry
            .base_offset
            .checked_add(extent.offset)
            .ok_or_else(|| invalid("Texture transport: extent offset overflow"))?;
        let end = start
            .checked_add(extent.length)
            .ok_or_else(|| invalid("Texture transport: extent end overflow"))?;
        if extent.length == 0 {
            return Err(invalid(
                "Texture transport: zero-length extent is unsupported",
            ));
        }
        let in_payload = if entry.construction_method == 1 {
            meta.idat
                .as_ref()
                .is_some_and(|idat| end <= (idat.data_end - idat.data_start) as u64)
        } else {
            mdats
                .iter()
                .any(|mdat| start >= mdat.data_start as u64 && end <= mdat.data_end as u64)
        };
        if !in_payload || end > data.len() as u64 {
            return Err(invalid(format!(
                "Texture transport: item {} extent escapes its item-data box",
                entry.item_id
            )));
        }
    }
    Ok(())
}

/// Inspect an existing HEIF without changing its media or metadata.
///
/// `None` means there is no exact 2026 Texture metadata URI. A returned
/// carrier is an opaque, locally addressable item graph. Apple must separately
/// validate the metadata, resource semantics, and complete rendering context.
pub fn inspect_texture_style_heif(source: &[u8]) -> Result<Option<TextureStyleCarrierFacts>> {
    let top = scan_top_level_boxes(source)?;
    one_box(&top.boxes, FTYP)?;
    let meta = parse_meta_box(source, one_box(&top.boxes, META)?)?;
    let mut texture_ids = Vec::new();
    let mut ids = BTreeSet::new();
    for item in &meta.iinf.entries {
        if !ids.insert(item.item_id) {
            return Err(invalid("Texture transport: duplicate item ID"));
        }
        if item_uri(source, item)? == Some(TEXTURE_URI) {
            texture_ids.push(item.item_id);
        }
    }
    let texture_id = match texture_ids.as_slice() {
        [] => return Ok(None),
        [id] => *id,
        _ => {
            return Err(invalid(
                "Texture transport: ambiguous Texture metadata items",
            ))
        }
    };
    if !ids.contains(&meta.primary_item_id) {
        return Err(invalid("Texture transport: primary item is missing"));
    }
    let mdats: Vec<_> = top.boxes.iter().filter(|b| b.kind == MDAT).collect();
    let mut locations = BTreeSet::new();
    for entry in &meta.iloc.entries {
        if !ids.contains(&entry.item_id) || !locations.insert(entry.item_id) {
            return Err(invalid(
                "Texture transport: unknown or duplicate item location",
            ));
        }
        check_location(source, &meta, entry, &mdats)?;
    }
    if ids != locations {
        return Err(invalid("Texture transport: an item has no local location"));
    }
    let properties: BTreeSet<_> = meta.properties.iter().map(|p| p.index).collect();
    let mut owners = BTreeSet::new();
    for owner in &meta.ipma.entries {
        if !ids.contains(&owner.item_id) || !owners.insert(owner.item_id) {
            return Err(invalid(
                "Texture transport: unknown or duplicate property owner",
            ));
        }
        for a in &owner.associations {
            if a.property_index != 0 && !properties.contains(&u32::from(a.property_index)) {
                return Err(invalid("Texture transport: dangling property index"));
            }
        }
    }
    let mut targets = Vec::new();
    let mut descriptors = 0;
    for entry in meta.iref.as_ref().into_iter().flat_map(|r| &r.entries) {
        if !ids.contains(&entry.from_item_id)
            || entry.to_item_ids.is_empty()
            || entry.to_item_ids.iter().any(|id| !ids.contains(id))
        {
            return Err(invalid(
                "Texture transport: dangling or empty item reference",
            ));
        }
        if entry.kind == CDSC && entry.from_item_id == texture_id {
            descriptors += 1;
            targets = entry.to_item_ids.clone();
        }
    }
    if descriptors != 1 || !targets.contains(&meta.primary_item_id) {
        return Err(invalid(
            "Texture transport: metadata must describe the primary image",
        ));
    }
    let location = meta
        .iloc
        .entries
        .iter()
        .find(|e| e.item_id == texture_id)
        .ok_or_else(|| invalid("Texture transport: metadata location is missing"))?;
    let payload = read_item_payload(source, location, meta.idat.as_ref())?;
    let mut aux = BTreeSet::new();
    for property in meta.properties.iter().filter(|p| p.kind == AUXC) {
        let bytes = &source[property.box_range.clone()];
        let mut cursor = Cursor::new(bytes, Endian::Big, "Texture auxC");
        let size = cursor.read_u32()?;
        cursor.skip(if size == 1 { 16 } else { 8 })?;
        let uri = cursor.read_c_string()?;
        if uri.starts_with(TEXTURE_AUX_PREFIX) {
            let associated = meta.ipma.entries.iter().any(|owner| {
                owner
                    .associations
                    .iter()
                    .any(|a| u32::from(a.property_index) == property.index)
            });
            if associated {
                aux.insert(String::from_utf8_lossy(uri).into_owned());
            }
        }
    }
    Ok(Some(TextureStyleCarrierFacts {
        primary_item_id: meta.primary_item_id,
        metadata_item_id: texture_id,
        metadata_target_item_ids: targets,
        metadata_byte_count: payload.len(),
        metadata_sha256: Sha256::digest(&payload).into(),
        metadata_has_binary_plist_header: payload.starts_with(b"bplist00"),
        auxiliary_type_claims: aux.into_iter().collect(),
    }))
}

/// Preserve an existing opaque Texture carrier byte for byte.
///
/// This does not certify the original producer or native Photos acceptance.
/// Missing Texture data is an error. No capture metadata, MakerNote, semantic
/// role, LinearThumbnail, or RAW information is synthesized.
pub fn preserve_texture_style_heif(source: &[u8]) -> Result<Vec<u8>> {
    if inspect_texture_style_heif(source)?.is_none() {
        return Err(invalid(
            "Texture Styles generation unsupported: no image-bound native producer contract is verified; existing Texture metadata is required for preservation",
        ));
    }
    Ok(source.to_vec())
}

/// Compatibility entry point. New Texture generation is unsupported.
/// Existing carriers are preserved through the same opaque transport contract.
pub fn augment_texture_style_heif(source: &[u8]) -> Result<Vec<u8>> {
    preserve_texture_style_heif(source)
}
