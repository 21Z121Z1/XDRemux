use std::collections::HashSet;

use xdremux_format::exif::read_item_payload;
use xdremux_format::isobmff::{parse_meta_box, scan_top_level_boxes, IlocEntry, EXIF, META};
use xdremux_format::Endian;

use crate::{MetadataError, Result};

pub const OPPO_ULTRA_HDR_FLAG: u32 = 0x2000_0000;
pub const ISO_ULTRA_HDR_FLAG: u32 = 0x0020_0000;
pub const LOCAL_HDR_FLAG: u32 = 0x0004_0000;

const TAG_FLAG_PREFIXES: [&str; 5] = ["ASCIIOplus_", "ASCIIoppo_", "Oplus_", "oplus_", "oppo_"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OppoCompatibility {
    Off,
    Auto,
    On,
    Tail,
    Iso,
    IsoNoLocal,
    IsoGraph,
}

impl OppoCompatibility {
    pub const ALL: [Self; 7] = [
        Self::Off,
        Self::Auto,
        Self::On,
        Self::Tail,
        Self::Iso,
        Self::IsoNoLocal,
        Self::IsoGraph,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::On => "on",
            Self::Tail => "tail",
            Self::Iso => "iso",
            Self::IsoNoLocal => "iso-no-local",
            Self::IsoGraph => "iso-graph",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OppoTagFlag {
    pub prefix: &'static str,
    pub offset: usize,
    pub digits_start: usize,
    pub digits_end: usize,
    pub value: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OppoUserCommentPatch {
    pub source_start: u64,
    pub source_end: u64,
    pub delta: i64,
}

pub const fn target_oppo_tag_flags(source: u32, compatibility: OppoCompatibility) -> u32 {
    match compatibility {
        OppoCompatibility::Off | OppoCompatibility::Auto => source,
        OppoCompatibility::On | OppoCompatibility::Tail => source | OPPO_ULTRA_HDR_FLAG,
        OppoCompatibility::Iso => (source & !OPPO_ULTRA_HDR_FLAG) | ISO_ULTRA_HDR_FLAG,
        OppoCompatibility::IsoNoLocal => {
            (source & !OPPO_ULTRA_HDR_FLAG & !LOCAL_HDR_FLAG) | ISO_ULTRA_HDR_FLAG
        }
        OppoCompatibility::IsoGraph => source & !OPPO_ULTRA_HDR_FLAG & !ISO_ULTRA_HDR_FLAG,
    }
}

pub fn find_oppo_tag_flag(data: &[u8]) -> Option<OppoTagFlag> {
    for offset in 0..data.len() {
        for prefix in TAG_FLAG_PREFIXES {
            let prefix_bytes = prefix.as_bytes();
            let prefix_end = offset.checked_add(prefix_bytes.len())?;
            if data.get(offset..prefix_end) != Some(prefix_bytes) {
                continue;
            }
            let digits_start = prefix_end;
            let mut digits_end = digits_start;
            let mut value = 0u32;
            while let Some(byte) = data.get(digits_end).copied() {
                if !byte.is_ascii_digit() {
                    break;
                }
                value = match value
                    .checked_mul(10)
                    .and_then(|current| current.checked_add(u32::from(byte - b'0')))
                {
                    Some(value) => value,
                    None => {
                        digits_end = digits_start;
                        break;
                    }
                };
                digits_end += 1;
            }
            if digits_end > digits_start {
                return Some(OppoTagFlag {
                    prefix,
                    offset,
                    digits_start,
                    digits_end,
                    value,
                });
            }
        }
    }
    None
}

pub fn adjusted_oppo_user_comment(
    data: &[u8],
    compatibility: OppoCompatibility,
) -> Result<Option<String>> {
    let Some(tag) = find_oppo_tag_flag(data) else {
        return Ok(None);
    };
    let target = target_oppo_tag_flags(tag.value, compatibility);
    if target == tag.value {
        return Ok(None);
    }
    let target_digits = target.to_string();
    let original_width = tag.digits_end - tag.digits_start;
    let zero_count = original_width.saturating_sub(target_digits.len());
    let mut output = String::with_capacity(tag.prefix.len() + zero_count + target_digits.len());
    output.push_str(tag.prefix);
    output.extend(std::iter::repeat_n('0', zero_count));
    output.push_str(&target_digits);
    Ok(Some(output))
}

pub fn adjusted_oppo_user_comment_in_heif(
    data: &[u8],
    compatibility: OppoCompatibility,
) -> Result<Option<String>> {
    let top = scan_top_level_boxes(data)?;
    let Some(meta_header) = top.boxes.iter().find(|header| header.kind == META) else {
        return Ok(None);
    };
    let meta = parse_meta_box(data, meta_header)?;
    let Some(exif_item) = meta.iinf.entries.iter().find(|item| item.item_type == Some(EXIF)) else {
        return Ok(None);
    };
    let exif_entry = meta
        .iloc
        .entries
        .iter()
        .find(|entry| entry.item_id == exif_item.item_id)
        .ok_or_else(|| {
            MetadataError::invalid(
                "OPPO UserComment",
                format!("Exif item {} has no iloc entry", exif_item.item_id),
            )
        })?;
    let payload = read_item_payload(data, exif_entry, meta.idat.as_ref())?;
    adjusted_oppo_user_comment(&payload, compatibility)
}

fn read_u16(data: &[u8], offset: usize, endian: Endian, context: &'static str) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| MetadataError::overflow(context))?;
    let bytes: [u8; 2] = data
        .get(offset..end)
        .ok_or_else(|| MetadataError::invalid(context, format!("u16 at {offset} is truncated")))?
        .try_into()
        .map_err(|_| MetadataError::invalid(context, "u16 conversion failed"))?;
    Ok(match endian {
        Endian::Little => u16::from_le_bytes(bytes),
        Endian::Big => u16::from_be_bytes(bytes),
    })
}

fn read_u32(data: &[u8], offset: usize, endian: Endian, context: &'static str) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| MetadataError::overflow(context))?;
    let bytes: [u8; 4] = data
        .get(offset..end)
        .ok_or_else(|| MetadataError::invalid(context, format!("u32 at {offset} is truncated")))?
        .try_into()
        .map_err(|_| MetadataError::invalid(context, "u32 conversion failed"))?;
    Ok(match endian {
        Endian::Little => u32::from_le_bytes(bytes),
        Endian::Big => u32::from_be_bytes(bytes),
    })
}

fn write_u32(
    data: &mut [u8],
    offset: usize,
    endian: Endian,
    value: u32,
    context: &'static str,
) -> Result<()> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| MetadataError::overflow(context))?;
    let destination = data
        .get_mut(offset..end)
        .ok_or_else(|| MetadataError::invalid(context, format!("u32 at {offset} is truncated")))?;
    let bytes = match endian {
        Endian::Little => value.to_le_bytes(),
        Endian::Big => value.to_be_bytes(),
    };
    destination.copy_from_slice(&bytes);
    Ok(())
}

pub fn apply_oppo_user_comment_patch(
    mdat_payload: &mut Vec<u8>,
    mdat_data_start: u64,
    exif_entry: &IlocEntry,
    patched_user_comment: &str,
) -> Result<Option<OppoUserCommentPatch>> {
    if exif_entry.construction_method != 0 || exif_entry.extents.len() != 1 {
        return Ok(None);
    }
    let extent = &exif_entry.extents[0];
    let extent_start = exif_entry.resolved_extent_offset(extent)?;
    let extent_end = extent_start
        .checked_add(extent.length)
        .ok_or_else(|| MetadataError::overflow("Exif extent end"))?;
    let local_start_u64 = extent_start
        .checked_sub(mdat_data_start)
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment patch", "Exif extent starts before mdat"))?;
    let local_start = usize::try_from(local_start_u64)
        .map_err(|_| MetadataError::overflow("Exif local start"))?;
    let extent_length = usize::try_from(extent.length)
        .map_err(|_| MetadataError::overflow("Exif extent length"))?;
    let local_end = local_start
        .checked_add(extent_length)
        .ok_or_else(|| MetadataError::overflow("Exif local end"))?;
    let mut exif_payload = mdat_payload
        .get(local_start..local_end)
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment patch", "Exif extent is outside mdat"))?
        .to_vec();

    if exif_payload.len() < 12 {
        return Ok(None);
    }
    let tiff_offset = u32::from_be_bytes(
        exif_payload[0..4]
            .try_into()
            .map_err(|_| MetadataError::invalid("Exif item", "missing TIFF offset"))?,
    );
    let tiff_start = 4usize
        .checked_add(usize::try_from(tiff_offset).map_err(|_| MetadataError::overflow("TIFF offset"))?)
        .ok_or_else(|| MetadataError::overflow("TIFF start"))?;
    let tiff_header_end = tiff_start
        .checked_add(8)
        .ok_or_else(|| MetadataError::overflow("TIFF header"))?;
    if tiff_start < 4 || tiff_header_end > exif_payload.len() {
        return Ok(None);
    }
    let endian = match exif_payload.get(tiff_start..tiff_start + 2) {
        Some(bytes) if bytes == b"II" => Endian::Little,
        Some(bytes) if bytes == b"MM" => Endian::Big,
        _ => return Ok(None),
    };
    if read_u16(&exif_payload, tiff_start + 2, endian, "TIFF header")? != 42 {
        return Ok(None);
    }
    let first_ifd = read_u32(&exif_payload, tiff_start + 4, endian, "TIFF header")?;
    let mut pending = vec![first_ifd];
    let mut visited = HashSet::new();
    let mut user_comment_entry = None;

    while let Some(relative_ifd) = pending.pop() {
        if user_comment_entry.is_some() || !visited.insert(relative_ifd) {
            continue;
        }
        let ifd = tiff_start
            .checked_add(usize::try_from(relative_ifd).map_err(|_| MetadataError::overflow("TIFF IFD offset"))?)
            .ok_or_else(|| MetadataError::overflow("TIFF IFD start"))?;
        let count = usize::from(read_u16(&exif_payload, ifd, endian, "TIFF IFD")?);
        if count > 4096 {
            return Ok(None);
        }
        for index in 0..count {
            let entry_offset = ifd
                .checked_add(2)
                .and_then(|value| index.checked_mul(12).and_then(|delta| value.checked_add(delta)))
                .ok_or_else(|| MetadataError::overflow("TIFF IFD entry"))?;
            let entry_end = entry_offset
                .checked_add(12)
                .ok_or_else(|| MetadataError::overflow("TIFF IFD entry"))?;
            if entry_end > exif_payload.len() {
                return Ok(None);
            }
            let tag = read_u16(&exif_payload, entry_offset, endian, "TIFF IFD entry")?;
            if tag == 0x9286 {
                user_comment_entry = Some(entry_offset);
                break;
            }
            if tag == 0x8769 || tag == 0x8825 {
                pending.push(read_u32(
                    &exif_payload,
                    entry_offset + 8,
                    endian,
                    "TIFF child IFD",
                )?);
            }
        }
    }

    let Some(entry) = user_comment_entry else {
        return Ok(None);
    };
    if read_u16(&exif_payload, entry + 2, endian, "UserComment entry")? != 7 {
        return Ok(None);
    }
    let old_count = read_u32(&exif_payload, entry + 4, endian, "UserComment entry")?;
    if old_count == 0 {
        return Ok(None);
    }
    let old_value_offset = read_u32(&exif_payload, entry + 8, endian, "UserComment entry")?;
    let old_value_start = if old_count <= 4 {
        entry + 8
    } else {
        tiff_start
            .checked_add(usize::try_from(old_value_offset).map_err(|_| MetadataError::overflow("UserComment offset"))?)
            .ok_or_else(|| MetadataError::overflow("UserComment start"))?
    };
    let old_count_usize = usize::try_from(old_count)
        .map_err(|_| MetadataError::overflow("UserComment count"))?;
    let old_value_end = old_value_start
        .checked_add(old_count_usize)
        .ok_or_else(|| MetadataError::overflow("UserComment end"))?;
    let mut new_value = match exif_payload.get(old_value_start..old_value_end) {
        Some(value) => value.to_vec(),
        None => return Ok(None),
    };
    let Some(tag) = find_oppo_tag_flag(&new_value) else {
        return Ok(None);
    };
    new_value.splice(tag.offset..tag.digits_end, patched_user_comment.as_bytes().iter().copied());

    while exif_payload.len() % 4 != 0 {
        exif_payload.push(0);
    }
    let new_value_offset = exif_payload
        .len()
        .checked_sub(tiff_start)
        .ok_or_else(|| MetadataError::overflow("new UserComment offset"))?;
    let new_count = u32::try_from(new_value.len())
        .map_err(|_| MetadataError::overflow("new UserComment count"))?;
    let new_value_offset = u32::try_from(new_value_offset)
        .map_err(|_| MetadataError::overflow("new UserComment offset"))?;
    exif_payload.extend_from_slice(&new_value);
    write_u32(&mut exif_payload, entry + 4, endian, new_count, "UserComment entry")?;
    write_u32(
        &mut exif_payload,
        entry + 8,
        endian,
        new_value_offset,
        "UserComment entry",
    )?;

    let new_extent_length = exif_payload.len();
    mdat_payload.splice(local_start..local_end, exif_payload);
    let old_len_i64 = i64::try_from(extent_length)
        .map_err(|_| MetadataError::overflow("Exif patch delta"))?;
    let new_len_i64 = i64::try_from(new_extent_length)
        .map_err(|_| MetadataError::overflow("Exif patch delta"))?;
    let delta = new_len_i64
        .checked_sub(old_len_i64)
        .ok_or_else(|| MetadataError::overflow("Exif patch delta"))?;

    Ok(Some(OppoUserCommentPatch {
        source_start: extent_start,
        source_end: extent_end,
        delta,
    }))
}

fn add_signed(value: u64, delta: i64) -> Option<u64> {
    if delta >= 0 {
        value.checked_add(delta as u64)
    } else {
        value.checked_sub(delta.unsigned_abs())
    }
}

pub fn adjusted_extent_for_oppo_user_comment_patch(
    offset: u64,
    length: u64,
    patch: Option<OppoUserCommentPatch>,
) -> Result<Option<(u64, u64)>> {
    let Some(patch) = patch else {
        return Ok(Some((offset, length)));
    };
    if patch.delta == 0 {
        return Ok(Some((offset, length)));
    }
    let end = offset
        .checked_add(length)
        .ok_or_else(|| MetadataError::overflow("extent end"))?;
    if end <= patch.source_start {
        return Ok(Some((offset, length)));
    }
    if offset >= patch.source_end {
        let adjusted = add_signed(offset, patch.delta)
            .ok_or_else(|| MetadataError::overflow("adjusted extent offset"))?;
        return Ok(Some((adjusted, length)));
    }
    if offset <= patch.source_start && end >= patch.source_end {
        let adjusted_length = add_signed(length, patch.delta)
            .ok_or_else(|| MetadataError::overflow("adjusted extent length"))?;
        return Ok(Some((offset, adjusted_length)));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::isobmff::IlocExtent;

    fn push_u16_le(value: u16, output: &mut Vec<u8>) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32_le(value: u32, output: &mut Vec<u8>) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn synthetic_exif(comment: &str) -> Vec<u8> {
        let user_comment = [b"ASCII\0\0\0".as_slice(), comment.as_bytes()].concat();
        let ifd0_offset = 8u32;
        let exif_ifd_offset = 26u32;
        let user_comment_offset = 44u32;
        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"II");
        push_u16_le(42, &mut tiff);
        push_u32_le(ifd0_offset, &mut tiff);
        push_u16_le(1, &mut tiff);
        push_u16_le(0x8769, &mut tiff);
        push_u16_le(4, &mut tiff);
        push_u32_le(1, &mut tiff);
        push_u32_le(exif_ifd_offset, &mut tiff);
        push_u32_le(0, &mut tiff);
        push_u16_le(1, &mut tiff);
        push_u16_le(0x9286, &mut tiff);
        push_u16_le(7, &mut tiff);
        push_u32_le(user_comment.len() as u32, &mut tiff);
        push_u32_le(user_comment_offset, &mut tiff);
        push_u32_le(0, &mut tiff);
        tiff.extend_from_slice(&user_comment);
        let mut exif = 0u32.to_be_bytes().to_vec();
        exif.extend_from_slice(&tiff);
        exif
    }

    #[test]
    fn routing_matches_current_swift_contract() {
        let source = OPPO_ULTRA_HDR_FLAG | ISO_ULTRA_HDR_FLAG | LOCAL_HDR_FLAG | 0x1234;
        assert_eq!(target_oppo_tag_flags(source, OppoCompatibility::Off), source);
        assert_eq!(target_oppo_tag_flags(source, OppoCompatibility::Auto), source);
        assert_eq!(target_oppo_tag_flags(source, OppoCompatibility::On), source);
        assert_eq!(target_oppo_tag_flags(source, OppoCompatibility::Tail), source);
        assert_eq!(
            target_oppo_tag_flags(source, OppoCompatibility::Iso),
            (source & !OPPO_ULTRA_HDR_FLAG) | ISO_ULTRA_HDR_FLAG
        );
        assert_eq!(
            target_oppo_tag_flags(source, OppoCompatibility::IsoNoLocal),
            (source & !OPPO_ULTRA_HDR_FLAG & !LOCAL_HDR_FLAG) | ISO_ULTRA_HDR_FLAG
        );
        assert_eq!(
            target_oppo_tag_flags(source, OppoCompatibility::IsoGraph),
            source & !OPPO_ULTRA_HDR_FLAG & !ISO_ULTRA_HDR_FLAG
        );
    }

    #[test]
    fn adjustment_preserves_digit_width_and_expands_when_required() {
        assert_eq!(
            adjusted_oppo_user_comment(b"Oplus_00000001", OppoCompatibility::Iso)
                .unwrap()
                .as_deref(),
            Some("Oplus_02097153")
        );
        assert_eq!(
            adjusted_oppo_user_comment(b"Oplus_00000001", OppoCompatibility::On)
                .unwrap()
                .as_deref(),
            Some("Oplus_536870913")
        );
    }

    #[test]
    fn malformed_decimal_overflow_is_not_misparsed() {
        assert!(find_oppo_tag_flag(b"Oplus_999999999999999999999999").is_none());
    }

    #[test]
    fn usercomment_patch_repoints_tiff_value_and_tracks_extent_delta() {
        let payload = synthetic_exif("Oplus_00000001");
        let prefix = vec![0x55; 13];
        let suffix = vec![0x77; 11];
        let mut mdat = [prefix.clone(), payload.clone(), suffix.clone()].concat();
        let entry = IlocEntry {
            item_id: 7,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 1013,
                length: payload.len() as u64,
            }],
        };
        let patched = adjusted_oppo_user_comment(&payload, OppoCompatibility::On)
            .unwrap()
            .unwrap();
        let patch = apply_oppo_user_comment_patch(&mut mdat, 1000, &entry, &patched)
            .unwrap()
            .unwrap();
        assert_eq!(patch.source_start, 1013);
        assert_eq!(patch.source_end, 1013 + payload.len() as u64);
        assert!(patch.delta > 0);
        assert_eq!(&mdat[..13], prefix.as_slice());
        assert_eq!(&mdat[mdat.len() - suffix.len()..], suffix.as_slice());
        assert!(find_oppo_tag_flag(&mdat).is_some());
    }

    #[test]
    fn extent_adjustment_matches_swift_overlap_policy() {
        let patch = OppoUserCommentPatch {
            source_start: 100,
            source_end: 120,
            delta: 8,
        };
        assert_eq!(
            adjusted_extent_for_oppo_user_comment_patch(10, 20, Some(patch)).unwrap(),
            Some((10, 20))
        );
        assert_eq!(
            adjusted_extent_for_oppo_user_comment_patch(130, 20, Some(patch)).unwrap(),
            Some((138, 20))
        );
        assert_eq!(
            adjusted_extent_for_oppo_user_comment_patch(90, 40, Some(patch)).unwrap(),
            Some((90, 48))
        );
        assert_eq!(
            adjusted_extent_for_oppo_user_comment_patch(110, 20, Some(patch)).unwrap(),
            None
        );
    }
}
