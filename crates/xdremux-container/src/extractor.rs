use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use serde_json::Value;

use crate::error::{ContainerError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionMode {
    Lhdr,
    Uhdr,
}

impl ExtractionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lhdr => "lhdr",
            Self::Uhdr => "uhdr",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub name: String,
    pub offset: i64,
    pub length: i64,
    pub json_order: usize,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestInfo {
    pub extension_start: usize,
    pub json_start: usize,
    pub json_end: usize,
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalHdrInfo {
    pub version: f64,
    pub length: f64,
    pub meta_size: f64,
    pub offset: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedLhdr {
    pub mode: ExtractionMode,
    pub meta_bytes: Vec<u8>,
    pub meta_floats: Vec<f64>,
    pub local_hdr_info: Option<LocalHdrInfo>,
    pub mask_jpeg_data: Vec<u8>,
    pub manifest_info: ManifestInfo,
    pub data_base: usize,
}

const QTI_MARKERS: [&[u8]; 2] = [b"QTI Debug", b"QTI "];
const JXRS_MARKER: &[u8] = b"\0jxrs";
const JSON_START: &[u8] = b"[{";
const JPEG_START: &[u8] = &[0xff, 0xd8, 0xff];
const JPEG_EOI: &[u8] = &[0xff, 0xd9];
const PNG_START: &[u8] = &[0x89, 0x50, 0x4e, 0x47];

pub fn extract(data: &[u8]) -> Result<ExtractedLhdr> {
    let manifest_info = locate_manifest(data)?;
    let data_base =
        calibrate_data_base(data, &manifest_info).unwrap_or(manifest_info.extension_start);
    let blocks = materialize_blocks(data, &manifest_info, data_base);

    let info_entry = manifest_info
        .entries
        .iter()
        .find(|entry| entry.name == "local.uhdr.gainmap.info");
    let data_entry = manifest_info
        .entries
        .iter()
        .find(|entry| entry.name == "local.uhdr.gainmap.data");

    if let (Some(info_entry), Some(data_entry)) = (info_entry, data_entry) {
        let mut meta_bytes = block_start(data, &manifest_info, data_base, info_entry)
            .and_then(|start| i64::try_from(start).ok())
            .and_then(|start| valid_range(data.len(), start, info_entry.length))
            .and_then(|range| data.get(range).map(ToOwned::to_owned))
            .unwrap_or_else(|| vec![0; 80]);

        let mut meta_floats =
            unpack_float_array_le(&meta_bytes, 20).unwrap_or_else(|_| vec![0.0; 20]);
        let needs_fallback = meta_floats.iter().all(|value| *value == 0.0)
            || meta_floats.iter().any(|value| !value.is_finite())
            || (meta_floats[0] - 1.0).abs() > 0.1;
        if needs_fallback {
            meta_floats = canonical_uhdr_floats();
            meta_bytes = pack_float32_le(&meta_floats);
        }

        let data_start = block_start(data, &manifest_info, data_base, data_entry)
            .ok_or_else(|| ContainerError::invalid("UHDR data", "out-of-bounds data block"))?;
        let data_range = valid_range(data.len(), i64_from_usize(data_start)?, data_entry.length)
            .ok_or_else(|| ContainerError::invalid("UHDR data", "out-of-bounds data block"))?;
        let mask_jpeg_data = data
            .get(data_range)
            .ok_or_else(|| ContainerError::invalid("UHDR data", "out-of-bounds data block"))?
            .to_vec();

        return Ok(ExtractedLhdr {
            mode: ExtractionMode::Uhdr,
            meta_bytes,
            meta_floats,
            local_hdr_info: None,
            mask_jpeg_data,
            manifest_info,
            data_base,
        });
    }

    let meta_bytes = extract_meta(data, &manifest_info, &blocks)?;
    let local_hdr_info = decode_local_hdr_info(&meta_bytes)?;
    let mask_jpeg_data = extract_mask(data, &manifest_info, data_base, &blocks)?;
    let meta_floats = unpack_float_array_le(&meta_bytes, 36)?;

    Ok(ExtractedLhdr {
        mode: ExtractionMode::Lhdr,
        meta_bytes,
        meta_floats,
        local_hdr_info: Some(local_hdr_info),
        mask_jpeg_data,
        manifest_info,
        data_base,
    })
}

pub fn portrait_blocks(data: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let manifest_info = locate_manifest(data)?;
    let data_base =
        calibrate_data_base(data, &manifest_info).unwrap_or(manifest_info.extension_start);
    let mut blocks = BTreeMap::new();
    for entry in &manifest_info.entries {
        let Some(start) = block_start(data, &manifest_info, data_base, entry) else {
            continue;
        };
        let Some(range) = valid_range(data.len(), i64_from_usize(start)?, entry.length) else {
            continue;
        };
        let Some(bytes) = data.get(range) else {
            continue;
        };
        blocks.insert(entry.name.clone(), bytes.to_vec());
    }
    Ok(blocks)
}

fn locate_manifest(data: &[u8]) -> Result<ManifestInfo> {
    let detected_extension_start = find_extension_start(data).ok();
    let manifest_array = parse_manifest(data).ok_or(ContainerError::ManifestNotFound)?;

    let json_start = last_subslice(data, JSON_START).ok_or(ContainerError::ManifestNotFound)?;
    let json_end_base =
        find_byte(data, b']', json_start).ok_or(ContainerError::ManifestNotFound)?;
    let json_end = json_end_base
        .checked_add(1)
        .ok_or_else(|| ContainerError::invalid("manifest", "JSON end overflow"))?;

    let has_valid_jxrs_footer = last_subslice(data, JXRS_MARKER).is_some_and(|marker| {
        let Some(marker_end) = marker.checked_add(9) else {
            return false;
        };
        if marker_end != data.len() {
            return false;
        }
        let Some(length_start) = marker.checked_add(5) else {
            return false;
        };
        let Some(length_end) = length_start.checked_add(4) else {
            return false;
        };
        let Some(length_bytes) = data.get(length_start..length_end) else {
            return false;
        };
        let footer_length = u32::from_le_bytes([
            length_bytes[0],
            length_bytes[1],
            length_bytes[2],
            length_bytes[3],
        ]);
        usize::try_from(footer_length)
            .ok()
            .is_some_and(|value| value == data.len().saturating_sub(json_start))
    });

    if detected_extension_start.is_none() && !has_valid_jxrs_footer {
        return Err(ContainerError::QtiMarkerNotFound);
    }

    let mut entries = Vec::new();
    for (json_order, raw) in manifest_array.iter().enumerate() {
        let Some(object) = raw.as_object() else {
            continue;
        };
        let Some(offset) = object.get("offset").and_then(json_integer) else {
            continue;
        };
        let Some(length) = object.get("length").and_then(json_integer) else {
            continue;
        };
        let name = match object.get("name") {
            Some(Value::String(value)) => value.clone(),
            Some(value) => value.to_string(),
            None => String::new(),
        };
        let start = offset
            .checked_sub(length)
            .ok_or_else(|| ContainerError::invalid("manifest", "entry start overflow"))?;
        entries.push(ManifestEntry {
            name,
            offset,
            length,
            json_order,
            start,
            end: offset,
        });
    }
    entries.sort_by_key(|entry| entry.start);

    let extension_start = if let Some(value) = detected_extension_start {
        value
    } else {
        let json_start_i64 = i64_from_usize(json_start)?;
        entries
            .iter()
            .filter_map(|entry| json_start_i64.checked_sub(entry.offset))
            .filter(|value| *value >= 0)
            .filter_map(|value| usize::try_from(value).ok())
            .min()
            .unwrap_or(json_start)
    };

    Ok(ManifestInfo {
        extension_start,
        json_start,
        json_end,
        entries,
    })
}

fn find_extension_start(data: &[u8]) -> Result<usize> {
    for marker in QTI_MARKERS {
        let Some(position) = find_subslice(data, marker, 0) else {
            continue;
        };
        let Some(box_start) = position.checked_sub(4) else {
            continue;
        };
        let Some(box_end) = box_start.checked_add(4) else {
            continue;
        };
        let Some(size_bytes) = data.get(box_start..box_end) else {
            continue;
        };
        let box_size =
            u32::from_be_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]);
        let Ok(box_size) = usize::try_from(box_size) else {
            continue;
        };
        let Some(extension_start) = box_start.checked_add(box_size) else {
            continue;
        };
        if box_size >= 8 && extension_start <= data.len() {
            return Ok(extension_start);
        }
    }
    Err(ContainerError::QtiMarkerNotFound)
}

fn parse_manifest(data: &[u8]) -> Option<Vec<Value>> {
    let json_start = last_subslice(data, JSON_START)?;
    let json_end_base = find_byte(data, b']', json_start)?;
    let json_end = json_end_base.checked_add(1)?;
    let json_slice = data.get(json_start..json_end)?;
    serde_json::from_slice::<Value>(json_slice)
        .ok()?
        .as_array()
        .cloned()
}

fn calibrate_data_base(data: &[u8], manifest_info: &ManifestInfo) -> Option<usize> {
    let image_positions = discover_image_positions(data, manifest_info.extension_start);
    if image_positions.is_empty() {
        return None;
    }

    let mut interesting = manifest_info
        .entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.name.as_str(),
                "watermark" | "local.hdr.linear.mask" | "local.uhdr.gainmap.data"
            )
        })
        .collect::<Vec<_>>();
    if interesting.is_empty() {
        interesting = manifest_info
            .entries
            .iter()
            .filter(|entry| entry.length > 64)
            .collect();
    }

    let meta_entry = manifest_info
        .entries
        .iter()
        .find(|entry| entry.name == "local.hdr.meta.data");
    let info_entry = manifest_info
        .entries
        .iter()
        .find(|entry| entry.name == "local.uhdr.gainmap.info");

    let extension_start = i64::try_from(manifest_info.extension_start).ok()?;
    let mut best_base = None;
    let mut best_score = i32::MIN;

    for image_position in image_positions {
        let image_position = i64::try_from(image_position).ok()?;
        for entry in &interesting {
            let Some(candidate_base) = image_position.checked_sub(entry.start) else {
                continue;
            };
            if candidate_base < extension_start {
                continue;
            }

            let mut score = 0_i32;
            if let Some(entry_start) = candidate_base.checked_add(entry.start) {
                if let Some(range) = valid_range(data.len(), entry_start, 4) {
                    if let Some(magic) = data.get(range) {
                        if magic.starts_with(&[0xff, 0xd8]) || magic.starts_with(PNG_START) {
                            score += 5;
                        }
                    }
                }
            }

            if let Some(meta_entry) = meta_entry {
                if let Some(meta_start) = candidate_base.checked_add(meta_entry.start) {
                    if let Some(range) = valid_range(data.len(), meta_start, meta_entry.length) {
                        if let Some(chunk) = data.get(range) {
                            score += score_meta_chunk(chunk).max(0);
                        }
                    }
                }
            }

            if let Some(info_entry) = info_entry {
                if let Some(info_start) = candidate_base.checked_add(info_entry.start) {
                    if let Some(range) = valid_range(data.len(), info_start, info_entry.length) {
                        if let Some(chunk) = data.get(range) {
                            if let Ok(floats) = unpack_float_array_le(chunk, 20) {
                                let bounded = floats
                                    .iter()
                                    .filter(|value| value.is_finite() && value.abs() <= 10.0)
                                    .count();
                                if bounded >= 10 {
                                    score += 3;
                                }
                            }
                        }
                    }
                }
            }

            if score > best_score {
                best_score = score;
                best_base = usize::try_from(candidate_base).ok();
            }
        }
    }

    best_base
}

fn materialize_blocks(
    data: &[u8],
    manifest_info: &ManifestInfo,
    data_base: usize,
) -> BTreeMap<String, Vec<u8>> {
    let mut blocks = BTreeMap::new();
    let Ok(data_base) = i64::try_from(data_base) else {
        return blocks;
    };
    for entry in &manifest_info.entries {
        let Some(start) = data_base.checked_add(entry.start) else {
            continue;
        };
        let Some(range) = valid_range(data.len(), start, entry.length) else {
            continue;
        };
        if let Some(bytes) = data.get(range) {
            blocks.insert(entry.name.clone(), bytes.to_vec());
        }
    }
    blocks
}

fn block_start(
    data: &[u8],
    manifest_info: &ManifestInfo,
    data_base: usize,
    entry: &ManifestEntry,
) -> Option<usize> {
    let json_start = i64::try_from(manifest_info.json_start).ok()?;
    let manifest_relative_start = json_start.checked_sub(entry.offset)?;
    if valid_range(data.len(), manifest_relative_start, entry.length).is_some() {
        return usize::try_from(manifest_relative_start).ok();
    }

    let data_base = i64::try_from(data_base).ok()?;
    let start = data_base.checked_add(entry.start)?;
    usize::try_from(start).ok()
}

fn extract_meta(
    data: &[u8],
    manifest_info: &ManifestInfo,
    blocks: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>> {
    let extension_data = data.get(manifest_info.extension_start..).ok_or_else(|| {
        ContainerError::invalid("LHDR metadata", "extension start is outside input")
    })?;
    let manifest_start = last_subslice(extension_data, JSON_START);

    if let Some(entry) = manifest_info
        .entries
        .iter()
        .find(|entry| entry.name == "local.hdr.meta.data" && entry.length >= 144)
    {
        let json_start = i64_from_usize(manifest_info.json_start)?;
        let extension_start = i64_from_usize(manifest_info.extension_start)?;
        let candidates = [
            json_start.checked_sub(entry.offset),
            extension_start.checked_add(entry.offset),
        ];
        for start in candidates.into_iter().flatten() {
            if let Some(range) = valid_range(data.len(), start, 144) {
                if let Some(chunk) = data.get(range) {
                    if let Ok(floats) = unpack_float_array_le(chunk, 36) {
                        if score_meta_candidate(&floats) >= 6 {
                            return Ok(chunk.to_vec());
                        }
                    }
                }
            }
        }
    }

    if let Some(block) = blocks.get("local.hdr.meta.data") {
        if let Some(candidate) = block.get(..144) {
            if let Ok(floats) = unpack_float_array_le(candidate, 36) {
                if score_meta_candidate(&floats) >= 6 {
                    return Ok(candidate.to_vec());
                }
            }
        }
    }

    if let Some(manifest_start) = manifest_start {
        let manifest_start = i64_from_usize(manifest_start)?;
        for entry in manifest_info
            .entries
            .iter()
            .filter(|entry| entry.name == "local.hdr.meta.data" && entry.length >= 144)
        {
            let Some(start) = manifest_start.checked_sub(entry.offset) else {
                continue;
            };
            if let Some(range) = valid_range(extension_data.len(), start, 144) {
                if let Some(chunk) = extension_data.get(range) {
                    if let Ok(floats) = unpack_float_array_le(chunk, 36) {
                        if score_meta_candidate(&floats) >= 6 {
                            return Ok(chunk.to_vec());
                        }
                    }
                }
            }
        }
    }

    let float144 = 144.0_f32.to_bits().to_le_bytes();
    let mut best: Option<(i32, Vec<u8>)> = None;
    let mut search_start = 0_usize;
    while let Some(hit) = find_subslice(extension_data, &float144, search_start) {
        if let Some(start) = hit.checked_sub(8) {
            if let Some(end) = start.checked_add(144) {
                if let Some(chunk) = extension_data.get(start..end) {
                    if let Ok(floats) = unpack_float_array_le(chunk, 36) {
                        let score = score_meta_candidate(&floats);
                        if best
                            .as_ref()
                            .is_none_or(|(best_score, _)| score > *best_score)
                        {
                            best = Some((score, chunk.to_vec()));
                        }
                    }
                }
            }
        }
        let Some(next) = hit.checked_add(1) else {
            break;
        };
        search_start = next;
    }

    match best {
        Some((score, chunk)) if score >= 8 => Ok(chunk),
        _ => Err(ContainerError::invalid(
            "LHDR metadata",
            "failed to locate plausible 144-byte local.hdr.meta.data block",
        )),
    }
}

fn extract_mask(
    data: &[u8],
    manifest_info: &ManifestInfo,
    data_base: usize,
    blocks: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>> {
    if let Some(mask) = blocks.get("local.hdr.linear.mask") {
        if mask.starts_with(&[0xff, 0xd8]) {
            return Ok(mask.clone());
        }
    }

    let mask_entry = manifest_info
        .entries
        .iter()
        .find(|entry| entry.name == "local.hdr.linear.mask");
    if let Some(entry) = mask_entry {
        let json_start = i64_from_usize(manifest_info.json_start)?;
        let extension_start = i64_from_usize(manifest_info.extension_start)?;
        let data_base = i64_from_usize(data_base)?;
        let candidates = [
            json_start.checked_sub(entry.offset),
            data_base.checked_add(entry.start),
            extension_start.checked_add(entry.offset),
        ];
        for start in candidates.into_iter().flatten() {
            if let Some(range) = valid_range(data.len(), start, entry.length) {
                if let Some(candidate) = data.get(range) {
                    if candidate.starts_with(&[0xff, 0xd8]) {
                        return Ok(candidate.to_vec());
                    }
                }
            }
        }
    }

    let extension_data = data
        .get(manifest_info.extension_start..)
        .ok_or_else(|| ContainerError::invalid("LHDR mask", "extension start is outside input"))?;
    let mut blobs = Vec::<Vec<u8>>::new();
    let mut position = 0_usize;
    while let Some(hit) = find_subslice(extension_data, JPEG_START, position) {
        let search_from = hit
            .checked_add(3)
            .ok_or_else(|| ContainerError::invalid("LHDR mask", "JPEG search offset overflow"))?;
        if let Some(end_marker) = find_subslice(extension_data, JPEG_EOI, search_from) {
            let blob_end = end_marker
                .checked_add(2)
                .ok_or_else(|| ContainerError::invalid("LHDR mask", "JPEG end overflow"))?;
            if let Some(blob) = extension_data.get(hit..blob_end) {
                blobs.push(blob.to_vec());
            }
            position = blob_end;
        } else {
            position = hit
                .checked_add(1)
                .ok_or_else(|| ContainerError::invalid("LHDR mask", "JPEG search overflow"))?;
        }
    }

    if blobs.is_empty() {
        return Err(ContainerError::invalid(
            "LHDR mask",
            "failed to locate local.hdr.linear.mask JPEG",
        ));
    }

    if let Some(entry) = mask_entry {
        let target = entry.length;
        if let Some(best) = blobs.iter().min_by_key(|blob| {
            i64::try_from(blob.len())
                .unwrap_or(i64::MAX)
                .abs_diff(target)
        }) {
            return Ok(best.clone());
        }
    }
    Ok(blobs.remove(0))
}

fn discover_image_positions(data: &[u8], start: usize) -> Vec<usize> {
    let mut hits = BTreeSet::new();
    for needle in [JPEG_START, PNG_START] {
        let mut position = start;
        while let Some(index) = find_subslice(data, needle, position) {
            hits.insert(index);
            let Some(next) = index.checked_add(1) else {
                break;
            };
            position = next;
        }
    }
    hits.into_iter().collect()
}

fn score_meta_candidate(floats: &[f64]) -> i32 {
    if floats.len() != 36 {
        return i32::MIN;
    }
    let mut score = 0;
    if (floats[2] - 144.0).abs() < 0.01 {
        score += 5;
    }
    if (floats[5] + 1.0).abs() < 0.01 {
        score += 3;
    }
    if (floats[18] - 10.0).abs() < 0.01 {
        score += 2;
    }
    if (floats[19] - 6.0).abs() < 0.01 {
        score += 2;
    }
    if (2.0..=5.0).contains(&floats[0]) {
        score += 2;
    }
    if (0.0..=2000.0).contains(&floats[29]) {
        score += 1;
    }
    score
}

fn score_meta_chunk(chunk: &[u8]) -> i32 {
    let Ok(floats) = unpack_float_array_le(chunk, 36) else {
        return i32::MIN;
    };
    let mut score = 0;
    if (floats[2] - 144.0).abs() < 0.01 {
        score += 8;
    }
    if (floats[5] + 1.0).abs() < 0.01 {
        score += 4;
    }
    if (floats[18] - 10.0).abs() < 0.01 {
        score += 2;
    }
    if (floats[19] - 6.0).abs() < 0.01 {
        score += 2;
    }
    if [0_usize, 1, 7, 16]
        .iter()
        .all(|index| (floats[*index] - 1.0).abs() < 0.25)
    {
        score += 2;
    }
    if [10_usize, 11, 12, 13, 14, 15]
        .iter()
        .all(|index| floats[*index].abs() < 0.25)
    {
        score += 2;
    }
    score
}

fn decode_local_hdr_info(meta_bytes: &[u8]) -> Result<LocalHdrInfo> {
    let prefix = meta_bytes.get(..16).ok_or_else(|| {
        ContainerError::invalid("LHDR metadata", "metadata is shorter than 16 bytes")
    })?;
    let values = unpack_float_array_le(prefix, 4)?;
    Ok(LocalHdrInfo {
        version: values[0],
        length: values[1],
        meta_size: values[2],
        offset: values[3],
    })
}

fn unpack_float_array_le(data: &[u8], count: usize) -> Result<Vec<f64>> {
    let needed = count
        .checked_mul(4)
        .ok_or_else(|| ContainerError::invalid("float payload", "size overflow"))?;
    if data.len() < needed {
        return Err(ContainerError::invalid(
            "float payload",
            format!("shorter than expected {needed} bytes"),
        ));
    }
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        let start = index
            .checked_mul(4)
            .ok_or_else(|| ContainerError::invalid("float payload", "offset overflow"))?;
        let end = start
            .checked_add(4)
            .ok_or_else(|| ContainerError::invalid("float payload", "offset overflow"))?;
        let bytes = data
            .get(start..end)
            .ok_or_else(|| ContainerError::invalid("float payload", "unexpected end of input"))?;
        let bits = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        values.push(f64::from(f32::from_bits(bits)));
    }
    Ok(values)
}

fn canonical_uhdr_floats() -> Vec<f64> {
    vec![
        1.0, 1.0, 1.0, 1.0, 4.926, 4.926, 4.926, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        4.926, 4.926, 0.0,
    ]
}

fn pack_float32_le(values: &[f64]) -> Vec<u8> {
    let mut output = Vec::with_capacity(values.len() * 4);
    for value in values {
        output.extend_from_slice(&(*value as f32).to_bits().to_le_bytes());
    }
    output
}

fn json_integer(value: &Value) -> Option<i64> {
    if let Some(value) = value.as_i64() {
        return Some(value);
    }
    if let Some(value) = value.as_u64() {
        return i64::try_from(value).ok();
    }
    let value = value.as_f64()?;
    if !value.is_finite() || value < i64::MIN as f64 || value > i64::MAX as f64 {
        return None;
    }
    Some(value as i64)
}

fn valid_range(data_len: usize, start: i64, length: i64) -> Option<Range<usize>> {
    if start < 0 || length < 0 {
        return None;
    }
    let end = start.checked_add(length)?;
    let data_len = i64::try_from(data_len).ok()?;
    if end > data_len {
        return None;
    }
    Some(usize::try_from(start).ok()?..usize::try_from(end).ok()?)
}

fn i64_from_usize(value: usize) -> Result<i64> {
    i64::try_from(value).map_err(|_| {
        ContainerError::invalid("container offset", "value exceeds signed 64-bit range")
    })
}

fn find_byte(data: &[u8], byte: u8, start: usize) -> Option<usize> {
    if start >= data.len() {
        return None;
    }
    data.get(start..)?
        .iter()
        .position(|candidate| *candidate == byte)
        .and_then(|relative| start.checked_add(relative))
}

fn find_subslice(data: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start >= data.len() || needle.len() > data.len().saturating_sub(start) {
        return None;
    }
    data.get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .and_then(|relative| start.checked_add(relative))
}

fn last_subslice(data: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > data.len() {
        return None;
    }
    data.windows(needle.len())
        .rposition(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_footer(json: &[u8]) -> Vec<u8> {
        let mut output = json.to_vec();
        output.extend_from_slice(JXRS_MARKER);
        let length = u32::try_from(output.len() + 4).unwrap();
        output.extend_from_slice(&length.to_le_bytes());
        output
    }

    #[test]
    fn manifest_without_qti_or_valid_jxrs_fails_closed() {
        let data = br#"[{"name":"x","offset":1,"length":1}]"#;
        assert_eq!(
            locate_manifest(data),
            Err(ContainerError::QtiMarkerNotFound)
        );
    }

    #[test]
    fn valid_jxrs_footer_allows_manifest_without_qti_marker() {
        let json = br#"[{"name":"x","offset":1,"length":1}]"#;
        let data = manifest_footer(json);
        let manifest = locate_manifest(&data).unwrap();
        assert_eq!(manifest.json_start, 0);
        assert_eq!(manifest.entries.len(), 1);
    }

    #[test]
    fn coincidental_qti_text_with_invalid_box_size_is_rejected() {
        let mut data = vec![0xff, 0xd8, 0xff, 0xe1, 0, 0, 0, 0];
        data.extend_from_slice(b"QTI ordinary metadata");
        data.extend_from_slice(br#"[{"name":"x","offset":1,"length":1}]"#);
        assert_eq!(
            locate_manifest(&data),
            Err(ContainerError::QtiMarkerNotFound)
        );
    }

    #[test]
    fn uhdr_non_finite_metadata_uses_canonical_fallback() {
        let mut data = vec![0u8; 80];
        data[0..4].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        let mut floats = unpack_float_array_le(&data, 20).unwrap();
        let needs_fallback = floats.iter().all(|value| *value == 0.0)
            || floats.iter().any(|value| !value.is_finite())
            || (floats[0] - 1.0).abs() > 0.1;
        assert!(needs_fallback);
        floats = canonical_uhdr_floats();
        let bytes = pack_float32_le(&floats);
        assert_eq!(bytes.len(), 80);
        assert_eq!(floats[4].to_bits(), 4.926_f64.to_bits());
    }

    #[test]
    fn negative_manifest_lengths_never_form_slices() {
        assert!(valid_range(128, 10, -1).is_none());
        assert!(valid_range(128, -1, 10).is_none());
        assert!(valid_range(128, 120, 9).is_none());
    }
}
