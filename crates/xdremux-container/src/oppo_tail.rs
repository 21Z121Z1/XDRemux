use std::collections::BTreeMap;

use serde_json::Value;

use crate::{ContainerError, ManifestEntry, ManifestInfo, Result};

pub const OPPO_CAMERA_WATERMARK_AUXILIARY_ENTRY_NAMES: &[&str] = &[
    "color.space",
    "gr.effect.info",
    "master.mode.preset.info",
    "private.emptyspace",
];

pub const OPPO_CAMERA_PORTRAIT_EDITING_ENTRY_NAMES: &[&str] = &[
    "crop.region",
    "front.depth",
    "front.depth.config",
    "front.hair.mask",
    "front.matter.info",
    "front.negevimg",
    "front.segment",
    "mesh.coord",
    "mesh.coord.config",
    "rear.depth",
    "rear.depth.config",
    "rear.spotlight",
    "src.image",
    "src.image.block",
];

pub const OPPO_PRIVATE_UHDR_TAIL_ENTRY_NAMES: &[&str] =
    &["local.uhdr.gainmap.data", "local.uhdr.gainmap.info"];

pub fn is_oppo_private_uhdr_tail_entry(name: &str) -> bool {
    OPPO_PRIVATE_UHDR_TAIL_ENTRY_NAMES.contains(&name)
}

pub fn is_oppo_private_hdr_tail_entry(name: &str) -> bool {
    is_oppo_private_uhdr_tail_entry(name)
        || name.starts_with("hdr.")
        || name.starts_with("local.hdr.")
        || name.starts_with("src.local.hdr.")
}

pub fn is_oppo_portrait_editing_tail_entry(name: &str) -> bool {
    OPPO_CAMERA_PORTRAIT_EDITING_ENTRY_NAMES.contains(&name)
}

pub fn is_oppo_watermark_tail_entry(name: &str) -> bool {
    name.starts_with("watermark.") || OPPO_CAMERA_WATERMARK_AUXILIARY_ENTRY_NAMES.contains(&name)
}

pub fn is_oppo_compact_tail_entry(name: &str) -> bool {
    is_oppo_watermark_tail_entry(name)
        || is_oppo_portrait_editing_tail_entry(name)
        || matches!(name, "hdr.transform.data" | "src.local.hdr.linear.mask")
}

fn manifest_version(manifest: &[Value], entry: &ManifestEntry) -> i64 {
    manifest
        .get(entry.json_order)
        .and_then(Value::as_object)
        .and_then(|object| object.get("version"))
        .and_then(|value| match value {
            Value::Number(number) => number.as_i64(),
            Value::String(string) => string.parse().ok(),
            _ => None,
        })
        .unwrap_or(1)
}

fn checked_source_range(
    source_len: usize,
    start: i64,
    length: i64,
) -> Option<std::ops::Range<usize>> {
    if start < 0 || length < 0 {
        return None;
    }
    let start = usize::try_from(start).ok()?;
    let length = usize::try_from(length).ok()?;
    let end = start.checked_add(length)?;
    (end <= source_len).then_some(start..end)
}

fn source_range_for_entry(
    source: &[u8],
    manifest_info: &ManifestInfo,
    data_base: usize,
    entry: &ManifestEntry,
) -> Result<std::ops::Range<usize>> {
    let json_start = i64::try_from(manifest_info.json_start)
        .map_err(|_| ContainerError::invalid("OPPO tail", "JSON offset exceeds i64"))?;
    if let Some(start) = json_start.checked_sub(entry.offset) {
        if let Some(range) = checked_source_range(source.len(), start, entry.length) {
            return Ok(range);
        }
    }

    let data_base = i64::try_from(data_base)
        .map_err(|_| ContainerError::invalid("OPPO tail", "data base exceeds i64"))?;
    let start = data_base
        .checked_add(entry.start)
        .ok_or_else(|| ContainerError::invalid("OPPO tail", "entry source offset overflows"))?;
    checked_source_range(source.len(), start, entry.length).ok_or_else(|| {
        ContainerError::invalid(
            "OPPO tail",
            format!("entry {} payload is outside source", entry.name),
        )
    })
}

fn source_tail_tag(source: &[u8]) -> [u8; 4] {
    if source.len() >= 9 && source[source.len() - 9] == 0 {
        let start = source.len() - 8;
        let tag = &source[start..start + 4];
        if tag.iter().all(|byte| (32..=126).contains(byte)) {
            return [tag[0], tag[1], tag[2], tag[3]];
        }
    }
    *b"jxrs"
}

/// Return the exact source bytes after the standard HEIF body.
///
/// This is the canonical implementation of the product's `preserve` policy:
/// unknown vendor manifest fields and byte representation are retained rather
/// than normalized through a decode/re-encode cycle.
pub fn complete_oppo_camera_tail(source: &[u8], manifest_info: &ManifestInfo) -> Result<Vec<u8>> {
    source
        .get(manifest_info.extension_start..)
        .map(ToOwned::to_owned)
        .ok_or_else(|| ContainerError::invalid("OPPO tail", "extension start is outside source"))
}

fn find_subslice(haystack: &[u8], needle: &[u8], range: std::ops::Range<usize>) -> Option<usize> {
    if needle.is_empty() || range.start > range.end || range.end > haystack.len() {
        return None;
    }
    haystack[range.clone()]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| range.start + offset)
}

/// Preserve the complete source tail but neutralize selected manifest entry names.
///
/// The payload bytes and footer stay byte-identical. Only the first byte inside
/// the quoted JSON name token changes to `x`, matching the legacy Swift product
/// contract for `preserve-no-uhdr` / `preserve-no-hdr`. Searching for the quoted
/// token avoids patching a nested-name substring such as `local.hdr.*` inside
/// `src.local.hdr.*`.
pub fn neutralize_oppo_camera_tail_entries<F>(
    source: &[u8],
    manifest_info: &ManifestInfo,
    mut neutralize: F,
) -> Result<Vec<u8>>
where
    F: FnMut(&ManifestEntry) -> bool,
{
    let mut tail = complete_oppo_camera_tail(source, manifest_info)?;
    let tail_start = manifest_info.extension_start;
    let json_start = manifest_info
        .json_start
        .checked_sub(tail_start)
        .ok_or_else(|| ContainerError::invalid("OPPO tail", "manifest starts before tail"))?;
    let json_end = manifest_info
        .json_end
        .checked_sub(tail_start)
        .ok_or_else(|| ContainerError::invalid("OPPO tail", "manifest ends before tail"))?;
    if json_start >= json_end || json_end > tail.len() {
        return Err(ContainerError::invalid(
            "OPPO tail",
            "manifest range is outside preserved tail",
        ));
    }

    for entry in manifest_info
        .entries
        .iter()
        .filter(|entry| neutralize(entry))
    {
        let quoted = format!("\"{}\"", entry.name).into_bytes();
        let start = find_subslice(&tail, &quoted, json_start..json_end).ok_or_else(|| {
            ContainerError::invalid(
                "OPPO tail",
                format!("unable to neutralize manifest entry {}", entry.name),
            )
        })?;
        let first_name_byte = start
            .checked_add(1)
            .ok_or_else(|| ContainerError::invalid("OPPO tail", "entry name offset overflows"))?;
        let byte = tail
            .get_mut(first_name_byte)
            .ok_or_else(|| ContainerError::invalid("OPPO tail", "entry name token is truncated"))?;
        *byte = b'x';
    }
    Ok(tail)
}

/// Repack a filtered OPPO camera metadata tail without imposing product policy.
///
/// The caller owns the selection predicate. The container layer only preserves
/// source payload bytes, original manifest order/version semantics, sorted JSON
/// keys, and the vendor footer representation used by the existing product.
pub fn pack_filtered_oppo_camera_tail<F>(
    source: &[u8],
    manifest_info: &ManifestInfo,
    data_base: usize,
    mut preserve: F,
) -> Result<Vec<u8>>
where
    F: FnMut(&ManifestEntry) -> bool,
{
    let manifest_bytes = source
        .get(manifest_info.json_start..manifest_info.json_end)
        .ok_or_else(|| ContainerError::invalid("OPPO tail", "manifest JSON is outside source"))?;
    let manifest = serde_json::from_slice::<Value>(manifest_bytes).map_err(|error| {
        ContainerError::invalid("OPPO tail", format!("invalid manifest JSON: {error}"))
    })?;
    let manifest = manifest
        .as_array()
        .ok_or_else(|| ContainerError::invalid("OPPO tail", "manifest root is not an array"))?;

    let selected = manifest_info
        .entries
        .iter()
        .filter(|entry| preserve(entry))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Ok(Vec::new());
    }

    let mut physical = selected
        .iter()
        .map(|entry| {
            let range = source_range_for_entry(source, manifest_info, data_base, entry)?;
            Ok((*entry, range))
        })
        .collect::<Result<Vec<_>>>()?;
    physical.sort_by_key(|(_, range)| range.start);

    let mut payload = Vec::new();
    let mut payload_start_by_json_order = BTreeMap::new();
    for (entry, range) in physical {
        let payload_start = payload.len();
        payload.extend_from_slice(&source[range]);
        payload_start_by_json_order.insert(entry.json_order, payload_start);
    }

    let payload_len = payload.len();
    let mut ordered = selected;
    ordered.sort_by_key(|entry| entry.json_order);
    let mut records = Vec::with_capacity(ordered.len());
    for entry in ordered {
        let payload_start = *payload_start_by_json_order
            .get(&entry.json_order)
            .ok_or_else(|| ContainerError::invalid("OPPO tail", "packed entry is missing"))?;
        let offset = payload_len
            .checked_sub(payload_start)
            .ok_or_else(|| ContainerError::invalid("OPPO tail", "packed offset underflows"))?;
        let length = usize::try_from(entry.length)
            .map_err(|_| ContainerError::invalid("OPPO tail", "entry length is negative"))?;

        let mut record = BTreeMap::<String, Value>::new();
        record.insert("length".to_owned(), Value::from(length));
        record.insert("name".to_owned(), Value::from(entry.name.clone()));
        record.insert("offset".to_owned(), Value::from(offset));
        record.insert(
            "version".to_owned(),
            Value::from(manifest_version(manifest, entry)),
        );
        records.push(record);
    }

    let manifest_json = serde_json::to_vec(&records).map_err(|error| {
        ContainerError::invalid("OPPO tail", format!("manifest encode failed: {error}"))
    })?;
    let footer_len = manifest_json
        .len()
        .checked_add(9)
        .ok_or_else(|| ContainerError::invalid("OPPO tail", "footer length overflows"))?;
    let footer_len = u32::try_from(footer_len)
        .map_err(|_| ContainerError::invalid("OPPO tail", "footer length exceeds u32"))?;

    let mut tail = Vec::with_capacity(payload.len() + usize::try_from(footer_len).unwrap_or(0));
    tail.extend_from_slice(&payload);
    tail.extend_from_slice(&manifest_json);
    tail.push(0);
    tail.extend_from_slice(&source_tail_tag(source));
    tail.extend_from_slice(&footer_len.to_le_bytes());
    Ok(tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_source() -> (Vec<u8>, ManifestInfo) {
        let mut source = b"AAAASECONDfirst".to_vec();
        let json_start = source.len();
        let manifest = br#"[{"length":5,"name":"local.hdr.meta.data","offset":5,"version":"2"},{"length":6,"name":"watermark.logo","offset":11,"version":3}]"#;
        source.extend_from_slice(manifest);
        let json_end = source.len();
        source.extend_from_slice(b"\0wtmk\x00\x00\x00\x00");

        let info = ManifestInfo {
            extension_start: 4,
            json_start,
            json_end,
            entries: vec![
                ManifestEntry {
                    name: "watermark.logo".to_owned(),
                    offset: 11,
                    length: 6,
                    json_order: 1,
                    start: 0,
                    end: 11,
                },
                ManifestEntry {
                    name: "local.hdr.meta.data".to_owned(),
                    offset: 5,
                    length: 5,
                    json_order: 0,
                    start: 6,
                    end: 5,
                },
            ],
        };
        (source, info)
    }

    #[test]
    fn private_hdr_name_policy_matches_product_contract() {
        for name in [
            "local.uhdr.gainmap.data",
            "local.uhdr.gainmap.info",
            "hdr.transform.data",
            "local.hdr.meta.data",
            "src.local.hdr.linear.mask",
        ] {
            assert!(is_oppo_private_hdr_tail_entry(name));
        }
        assert!(!is_oppo_private_hdr_tail_entry("watermark.logo"));
        assert!(is_oppo_portrait_editing_tail_entry("rear.depth"));
        assert!(is_oppo_watermark_tail_entry("watermark.logo"));
        assert!(is_oppo_watermark_tail_entry("color.space"));
        assert!(is_oppo_compact_tail_entry("rear.depth"));
        assert!(is_oppo_compact_tail_entry("hdr.transform.data"));
        assert!(!is_oppo_compact_tail_entry("local.uhdr.gainmap.data"));
    }

    #[test]
    fn preserve_returns_exact_post_extension_bytes() {
        let (source, info) = synthetic_source();
        assert_eq!(
            complete_oppo_camera_tail(&source, &info).unwrap(),
            source[4..]
        );
    }

    #[test]
    fn neutralization_changes_only_quoted_manifest_name_byte() {
        let (source, info) = synthetic_source();
        let original = complete_oppo_camera_tail(&source, &info).unwrap();
        let neutralized = neutralize_oppo_camera_tail_entries(&source, &info, |entry| {
            is_oppo_private_hdr_tail_entry(&entry.name)
        })
        .unwrap();
        assert_eq!(original.len(), neutralized.len());
        let differences = original
            .iter()
            .zip(&neutralized)
            .enumerate()
            .filter(|(_, (left, right))| left != right)
            .collect::<Vec<_>>();
        assert_eq!(differences.len(), 1);
        assert_eq!(*differences[0].1 .1, b'x');
        assert!(String::from_utf8_lossy(&neutralized).contains("\"xocal.hdr.meta.data\""));
        assert!(String::from_utf8_lossy(&neutralized).contains("\"watermark.logo\""));
    }

    #[test]
    fn filtered_repack_preserves_physical_payload_and_manifest_order() {
        let (source, info) = synthetic_source();
        let tail = pack_filtered_oppo_camera_tail(&source, &info, 4, |_| true).unwrap();
        assert!(tail.starts_with(b"SECONDfirst"));

        let json_start = 11;
        let marker = tail[json_start..]
            .windows(2)
            .position(|window| window == b"\0w")
            .map(|offset| json_start + offset)
            .unwrap();
        let manifest: Value = serde_json::from_slice(&tail[json_start..marker]).unwrap();
        let records = manifest.as_array().unwrap();
        assert_eq!(records[0]["name"], "local.hdr.meta.data");
        assert_eq!(records[0]["version"], 2);
        assert_eq!(records[1]["name"], "watermark.logo");
        assert_eq!(records[1]["version"], 3);
        assert!(String::from_utf8(tail[json_start..marker].to_vec())
            .unwrap()
            .starts_with("[{\"length\":"));
        assert_eq!(&tail[marker..marker + 5], b"\0wtmk");
        let footer = u32::from_le_bytes(tail[tail.len() - 4..].try_into().unwrap()) as usize;
        assert_eq!(footer, tail.len() - json_start);
    }

    #[test]
    fn default_private_hdr_filter_keeps_unrelated_vendor_payloads() {
        let (source, info) = synthetic_source();
        let tail = pack_filtered_oppo_camera_tail(&source, &info, 4, |entry| {
            !is_oppo_private_hdr_tail_entry(&entry.name)
        })
        .unwrap();
        assert!(tail.starts_with(b"SECOND"));
        assert!(!tail.windows(5).any(|window| window == b"first"));
        assert!(String::from_utf8_lossy(&tail).contains("watermark.logo"));
        assert!(!String::from_utf8_lossy(&tail).contains("local.hdr.meta.data"));
    }
}
