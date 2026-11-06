//! Source-declared HDR metadata attached to the derived HDR image.
//!
//! The derived HDR image of a dual-layer file is the `tmap` item. Only
//! properties and descriptive items that the source associates with that item
//! describe the reconstructed image. Carriers that describe another image are
//! reported as ignored instead of being copied onto the alternate.
use xdremux_format::isobmff::{
    parse_boxes, parse_meta_box, scan_top_level_boxes, ParsedMeta, PropertyInfo, META,
};
use xdremux_format::FourCC;
use xdremux_metadata::{
    ContentLightLevel, HdrStaticMetadata, MasteringDisplay, RegisteredHdrMetadata,
};

use crate::error::{HeifError, Result};
use crate::validation::{item_payload, one_top_level, validate_meta_integrity, MetaIndex};

const TMAP: FourCC = FourCC::new(*b"tmap");
const DIMG: FourCC = FourCC::new(*b"dimg");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const IT35: FourCC = FourCC::new(*b"it35");

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

/// Where each declared value came from, so a report can distinguish a source
/// declaration from an absent one and from an ignored carrier.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HdrMetadataProvenance {
    /// Item that represents the reconstructed HDR image, when the source has one.
    pub derived_hdr_item_id: Option<u32>,
    /// Property index of the `mdcv` associated with the derived HDR image.
    pub mastering_display_property: Option<u32>,
    /// Property index of the `clli` associated with the derived HDR image.
    pub content_light_level_property: Option<u32>,
    /// Item that carried registered HDR metadata for the derived HDR image.
    pub registered_item_id: Option<u32>,
    /// The registered static record supplied a component that the source did
    /// not also declare as a HEIF property. The values still come from the
    /// source; no value is generated.
    pub properties_from_registered: bool,
    /// Carriers found in the file that describe a different image.
    pub ignored_carrier_count: usize,
}

/// HDR metadata a source declares for its derived HDR image.
///
/// Empty fields stay empty. Nothing here is inferred from the codec, the
/// container defaults, or the conversion settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceHdrMetadata {
    pub static_metadata: HdrStaticMetadata,
    pub registered_metadata: Option<RegisteredHdrMetadata>,
    pub provenance: HdrMetadataProvenance,
}

impl SourceHdrMetadata {
    pub fn is_empty(&self) -> bool {
        self.static_metadata.is_empty() && self.registered_metadata.is_none()
    }

    /// True when the source declared dynamic metadata for the derived image.
    pub fn has_dynamic_metadata(&self) -> bool {
        self.registered_metadata
            .as_ref()
            .is_some_and(|m| m.dynamic_metadata.is_some())
    }
}

fn property_payload<'a>(source: &'a [u8], property: &PropertyInfo) -> Result<&'a [u8]> {
    let raw = source
        .get(property.box_range.clone())
        .ok_or_else(|| invalid(format!("property {} is outside the HEIF", property.index)))?;
    let headers = parse_boxes(raw, 0..raw.len())?;
    let [header] = headers.as_slice() else {
        return Err(invalid(format!(
            "property {} does not contain exactly one box",
            property.index
        )));
    };
    Ok(&raw[header.payload_range()])
}

fn dimg_targets_primary(meta: &ParsedMeta, item_id: u32) -> bool {
    meta.iref.as_ref().is_some_and(|iref| {
        iref.entries.iter().any(|reference| {
            reference.kind == DIMG
                && reference.from_item_id == item_id
                && reference.to_item_ids.contains(&meta.primary_item_id)
        })
    })
}

fn cdsc_targets(meta: &ParsedMeta, item_id: u32, target: u32) -> bool {
    meta.iref.as_ref().is_some_and(|iref| {
        iref.entries.iter().any(|reference| {
            reference.kind == CDSC
                && reference.from_item_id == item_id
                && reference.to_item_ids.contains(&target)
        })
    })
}

fn tmap_items(meta: &ParsedMeta) -> Vec<u32> {
    meta.iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(TMAP))
        .map(|item| item.item_id)
        .collect()
}

fn it35_items(meta: &ParsedMeta) -> Vec<u32> {
    meta.iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(IT35))
        .map(|item| item.item_id)
        .collect()
}

/// Count carriers that exist but do not describe the derived HDR image.
fn ignored_carriers(meta: &ParsedMeta, derived: Option<u32>) -> usize {
    let property_is_used = |property: &PropertyInfo| {
        derived.is_some_and(|item_id| {
            meta.ipma.entries.iter().any(|entry| {
                entry.item_id == item_id
                    && entry
                        .associations
                        .iter()
                        .any(|a| u32::from(a.property_index) == property.index)
            })
        })
    };
    let properties = meta
        .properties
        .iter()
        .filter(|p| matches!(p.kind.as_bytes(), b"mdcv" | b"clli"))
        .filter(|p| !property_is_used(p))
        .count();
    let items = it35_items(meta)
        .into_iter()
        .filter(|id| derived.is_none_or(|derived| !cdsc_targets(meta, *id, derived)))
        .count();
    properties + items
}

fn item_properties<'a>(
    index: &'a MetaIndex<'a>,
    item_id: u32,
) -> Result<Vec<(&'a PropertyInfo, bool)>> {
    let entry = index.ipma_by_item.get(&item_id).ok_or_else(|| {
        invalid(format!(
            "derived HDR item {item_id} has no property associations"
        ))
    })?;
    entry
        .associations
        .iter()
        .map(|association| {
            index
                .properties
                .get(&u32::from(association.property_index))
                .map(|property| (*property, association.essential))
                .ok_or_else(|| invalid("association references a missing property"))
        })
        .collect()
}

/// Read the HDR metadata a file declares for its derived HDR image.
///
/// The same function reads a source and a finished output, so a caller can
/// compare what was planned with what the published bytes actually declare.
pub fn extract_hdr_metadata(source: &[u8]) -> Result<SourceHdrMetadata> {
    let top = scan_top_level_boxes(source)?;
    let header = one_top_level(&top.boxes, META, "meta")?;
    let meta = parse_meta_box(source, header)?;
    let candidates = tmap_items(&meta);
    if candidates.is_empty() {
        // Without a derived HDR item no declared value describes the
        // reconstructed image. Report the carriers that were not applicable.
        return Ok(SourceHdrMetadata {
            static_metadata: HdrStaticMetadata::default(),
            registered_metadata: None,
            provenance: HdrMetadataProvenance {
                derived_hdr_item_id: None,
                ignored_carrier_count: ignored_carriers(&meta, None),
                ..HdrMetadataProvenance::default()
            },
        });
    }
    let roots = candidates
        .iter()
        .copied()
        .filter(|item_id| dimg_targets_primary(&meta, *item_id))
        .collect::<Vec<_>>();
    let derived = match roots.as_slice() {
        [only] => *only,
        [] => {
            return Err(invalid(
                "the source has a tmap item but none describes the primary image",
            ))
        }
        _ => {
            return Err(invalid(
                "the source declares more than one derived HDR image for the primary image",
            ))
        }
    };
    let index = validate_meta_integrity(source, &meta)?;

    let mut declared = HdrStaticMetadata::default();
    let mut provenance = HdrMetadataProvenance {
        derived_hdr_item_id: Some(derived),
        ignored_carrier_count: ignored_carriers(&meta, Some(derived)),
        ..HdrMetadataProvenance::default()
    };
    for (property, _) in item_properties(&index, derived)? {
        match property.kind.as_bytes() {
            b"mdcv" => {
                let value =
                    MasteringDisplay::from_mdcv_payload(property_payload(source, property)?)
                        .map_err(|e| invalid(e.to_string()))?;
                if declared.mastering_display.replace(value).is_some() {
                    return Err(invalid(format!(
                        "derived HDR item {derived} has more than one mastering display property"
                    )));
                }
                provenance.mastering_display_property = Some(property.index);
            }
            b"clli" => {
                let value =
                    ContentLightLevel::from_clli_payload(property_payload(source, property)?)
                        .map_err(|e| invalid(e.to_string()))?;
                if declared.content_light_level.replace(value).is_some() {
                    return Err(invalid(format!(
                        "derived HDR item {derived} has more than one content light property"
                    )));
                }
                provenance.content_light_level_property = Some(property.index);
            }
            _ => {}
        }
    }

    let mut registered_metadata = None;
    for item_id in it35_items(&meta).into_iter() {
        if !cdsc_targets(&meta, item_id, derived) {
            continue;
        }
        if registered_metadata.is_some() {
            return Err(invalid(
                "the derived HDR image has more than one registered metadata item",
            ));
        }
        let location = index
            .locations
            .get(&item_id)
            .ok_or_else(|| invalid(format!("registered HDR item {item_id} has no location")))?;
        let payload = item_payload(source, meta.idat.as_ref(), location)?;
        let data = RegisteredHdrMetadata::parse_it35(&payload)
            .map_err(|e| invalid(format!("registered HDR item {item_id}: {e}")))?;
        provenance.registered_item_id = Some(item_id);
        registered_metadata = Some(data);
    }

    if let Some((mastering, light)) = registered_metadata
        .as_ref()
        .and_then(|registered| registered.static_metadata.clone())
    {
        let from_registered = HdrStaticMetadata {
            mastering_display: Some(mastering),
            content_light_level: Some(light),
        };
        let merged = declared
            .merge(&from_registered)
            .map_err(|e| invalid(e.to_string()))?;
        provenance.properties_from_registered =
            declared.mastering_display.is_none() || declared.content_light_level.is_none();
        declared = merged;
    }

    Ok(SourceHdrMetadata {
        static_metadata: declared,
        registered_metadata,
        provenance,
    })
}
