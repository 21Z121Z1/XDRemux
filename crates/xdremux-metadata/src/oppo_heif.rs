use xdremux_format::isobmff::{
    make_iloc_box, parse_boxes, parse_meta_box, scan_top_level_boxes, BoxHeader, EXIF, ILOC, MDAT,
    META,
};
use xdremux_format::{exif_user_comment, heif_exif_tiff};

use crate::oppo::{
    adjusted_extent_for_oppo_user_comment_patch, adjusted_oppo_user_comment,
    apply_oppo_user_comment_patch, find_oppo_tag_flag, target_oppo_tag_flags, OppoCompatibility,
};
use crate::{MetadataError, Result};

fn one_top_level<'a>(
    boxes: &'a [BoxHeader],
    kind: xdremux_format::FourCC,
    context: &'static str,
) -> Result<&'a BoxHeader> {
    let mut matches = boxes.iter().filter(|header| header.kind == kind);
    let first = matches
        .next()
        .ok_or_else(|| MetadataError::invalid(context, "required top-level box is missing"))?;
    if matches.next().is_some() {
        return Err(MetadataError::invalid(
            context,
            "more than one top-level box is present",
        ));
    }
    Ok(first)
}

fn source_size32(data: &[u8], header: &BoxHeader) -> Result<u32> {
    let end = header
        .box_start
        .checked_add(4)
        .ok_or_else(|| MetadataError::overflow("HEIF box size field"))?;
    let bytes: [u8; 4] = data
        .get(header.box_start..end)
        .ok_or_else(|| MetadataError::invalid("HEIF box", "size field is outside source"))?
        .try_into()
        .map_err(|_| MetadataError::invalid("HEIF box", "size field is truncated"))?;
    Ok(u32::from_be_bytes(bytes))
}

/// Rebuild a box without changing the source header representation.
///
/// File-backed `iloc` offsets are relative to absolute file positions. Preserving
/// an 8-byte, 16-byte largesize, or size-zero header keeps the payload start
/// stable while a metadata payload changes length, so the only relocation needed
/// is the explicit UserComment byte delta handled below.
fn rebuild_box_preserving_header(
    data: &[u8],
    header: &BoxHeader,
    payload: &[u8],
    context: &'static str,
) -> Result<Vec<u8>> {
    let size32 = source_size32(data, header)?;
    match size32 {
        0 => {
            let total = payload
                .len()
                .checked_add(8)
                .ok_or_else(|| MetadataError::overflow(context))?;
            let mut output = Vec::with_capacity(total);
            output.extend_from_slice(&0u32.to_be_bytes());
            output.extend_from_slice(header.kind.as_bytes());
            output.extend_from_slice(payload);
            Ok(output)
        }
        1 => {
            let total = payload
                .len()
                .checked_add(16)
                .ok_or_else(|| MetadataError::overflow(context))?;
            let total = u64::try_from(total).map_err(|_| MetadataError::overflow(context))?;
            let mut output = Vec::with_capacity(
                usize::try_from(total).map_err(|_| MetadataError::overflow(context))?,
            );
            output.extend_from_slice(&1u32.to_be_bytes());
            output.extend_from_slice(header.kind.as_bytes());
            output.extend_from_slice(&total.to_be_bytes());
            output.extend_from_slice(payload);
            Ok(output)
        }
        _ => {
            let total = payload
                .len()
                .checked_add(8)
                .ok_or_else(|| MetadataError::overflow(context))?;
            let total = u32::try_from(total).map_err(|_| {
                MetadataError::invalid(
                    context,
                    "rewritten payload no longer fits the source 32-bit box header",
                )
            })?;
            let mut output = Vec::with_capacity(total as usize);
            output.extend_from_slice(&total.to_be_bytes());
            output.extend_from_slice(header.kind.as_bytes());
            output.extend_from_slice(payload);
            Ok(output)
        }
    }
}

fn current_oppo_tag_flags(data: &[u8]) -> Result<Option<u32>> {
    let Some(tiff) = heif_exif_tiff(data)? else {
        return Ok(None);
    };
    let Some(comment) = exif_user_comment(&tiff)? else {
        return Ok(None);
    };
    Ok(find_oppo_tag_flag(&comment).map(|tag| tag.value))
}

/// Read the current OPPO routing flags from the source HEIF Exif item.
pub fn oppo_tag_flags_in_heif(data: &[u8]) -> Result<Option<u32>> {
    current_oppo_tag_flags(data)
}

/// Patch OPPO Exif routing flags while preserving a structurally consistent HEIF.
///
/// A UserComment may grow when a compatibility bit is introduced. The lower-level
/// TIFF patcher therefore reports the byte delta. This function applies that delta
/// to every affected file-backed `iloc` extent before publishing the new `mdat`.
/// It deliberately owns only vendor metadata mutation; Gain Map graph construction
/// remains in `xdremux-heif`.
pub fn patch_oppo_user_comment_in_heif(
    data: &[u8],
    compatibility: OppoCompatibility,
) -> Result<Vec<u8>> {
    let Some(source_flags) = current_oppo_tag_flags(data)? else {
        return Ok(data.to_vec());
    };
    let expected_flags = target_oppo_tag_flags(source_flags, compatibility);
    if expected_flags == source_flags {
        return Ok(data.to_vec());
    }
    let tiff = heif_exif_tiff(data)?
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "Exif TIFF is missing"))?;
    let comment = exif_user_comment(&tiff)?
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "UserComment is missing"))?;
    let patched_comment = adjusted_oppo_user_comment(&comment, compatibility)?
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "routing patch disappeared"))?;

    let top = scan_top_level_boxes(data)?;
    let meta_header = one_top_level(&top.boxes, META, "OPPO HEIF meta")?;
    let mdat_header = one_top_level(&top.boxes, MDAT, "OPPO HEIF mdat")?;
    let meta = parse_meta_box(data, meta_header)?;
    let exif_item = meta
        .iinf
        .entries
        .iter()
        .find(|item| item.item_type == Some(EXIF))
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "Exif item is missing"))?;
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

    let mut mdat_payload = data
        .get(mdat_header.payload_range())
        .ok_or_else(|| MetadataError::invalid("OPPO HEIF mdat", "payload is outside source"))?
        .to_vec();
    let mdat_data_start = u64::try_from(mdat_header.data_start)
        .map_err(|_| MetadataError::overflow("OPPO mdat data start"))?;
    let patch = apply_oppo_user_comment_patch(
        &mut mdat_payload,
        mdat_data_start,
        exif_entry,
        &patched_comment,
    )?
    .ok_or_else(|| {
        MetadataError::invalid(
            "OPPO UserComment",
            "Exif layout cannot be patched without guessing",
        )
    })?;

    let mut relocated_entries = meta.iloc.entries.clone();
    for entry in &mut relocated_entries {
        if entry.construction_method != 0 {
            continue;
        }
        let base_offset = entry.base_offset;
        for extent in &mut entry.extents {
            let absolute = base_offset
                .checked_add(extent.offset)
                .ok_or_else(|| MetadataError::overflow("OPPO iloc extent offset"))?;
            let Some((adjusted_absolute, adjusted_length)) =
                adjusted_extent_for_oppo_user_comment_patch(absolute, extent.length, Some(patch))?
            else {
                return Err(MetadataError::invalid(
                    "OPPO UserComment",
                    format!("patch crosses item {} extent boundary", entry.item_id),
                ));
            };
            extent.offset = adjusted_absolute.checked_sub(base_offset).ok_or_else(|| {
                MetadataError::invalid(
                    "OPPO iloc relocation",
                    format!("item {} moved before its base offset", entry.item_id),
                )
            })?;
            extent.length = adjusted_length;
        }
    }

    let rebuilt_iloc = make_iloc_box(
        meta.iloc.version,
        meta.iloc.offset_size,
        meta.iloc.length_size,
        meta.iloc.base_offset_size,
        meta.iloc.index_size,
        &relocated_entries,
    )?;
    let children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| MetadataError::overflow("OPPO meta child start"))?;
    if children_start > meta_header.data_end {
        return Err(MetadataError::invalid(
            "OPPO HEIF meta",
            "full-box header is truncated",
        ));
    }
    let children = parse_boxes(data, children_start..meta_header.data_end)?;
    let full_header = data
        .get(meta_header.data_start..children_start)
        .ok_or_else(|| MetadataError::invalid("OPPO HEIF meta", "full-box header is missing"))?;
    let mut meta_payload = full_header.to_vec();
    let mut replaced_iloc = false;
    for child in &children {
        if child.kind == ILOC {
            if replaced_iloc {
                return Err(MetadataError::invalid(
                    "OPPO HEIF meta",
                    "more than one iloc box is present",
                ));
            }
            replaced_iloc = true;
            meta_payload.extend_from_slice(&rebuilt_iloc);
        } else {
            meta_payload.extend_from_slice(data.get(child.box_range()).ok_or_else(|| {
                MetadataError::invalid("OPPO HEIF meta", "child box is outside source")
            })?);
        }
    }
    if !replaced_iloc {
        return Err(MetadataError::invalid(
            "OPPO HEIF meta",
            "iloc box is missing",
        ));
    }
    let rebuilt_meta =
        rebuild_box_preserving_header(data, meta_header, &meta_payload, "OPPO HEIF meta")?;
    if rebuilt_meta.len() != meta_header.size {
        return Err(MetadataError::invalid(
            "OPPO HEIF meta",
            "iloc rewrite changed meta size; refusing to relocate unrelated top-level data",
        ));
    }

    let rebuilt_mdat =
        rebuild_box_preserving_header(data, mdat_header, &mdat_payload, "OPPO HEIF mdat")?;
    let rebuilt_mdat_header_size = rebuilt_mdat
        .len()
        .checked_sub(mdat_payload.len())
        .ok_or_else(|| MetadataError::overflow("OPPO rebuilt mdat header size"))?;
    let source_mdat_header_size = mdat_header
        .data_start
        .checked_sub(mdat_header.box_start)
        .ok_or_else(|| MetadataError::overflow("OPPO source mdat header size"))?;
    if rebuilt_mdat_header_size != source_mdat_header_size {
        return Err(MetadataError::invalid(
            "OPPO HEIF mdat",
            "rewriter did not preserve source mdat header width",
        ));
    }

    let trailing = data
        .get(top.trailing_range.clone())
        .ok_or_else(|| MetadataError::invalid("OPPO HEIF", "trailing bytes are outside source"))?;
    let mut output = Vec::with_capacity(
        data.len()
            .checked_add_signed(patch.delta as isize)
            .ok_or_else(|| MetadataError::overflow("OPPO patched HEIF size"))?,
    );
    for header in &top.boxes {
        if header.kind == META {
            output.extend_from_slice(&rebuilt_meta);
        } else if header.kind == MDAT {
            output.extend_from_slice(&rebuilt_mdat);
        } else {
            output.extend_from_slice(data.get(header.box_range()).ok_or_else(|| {
                MetadataError::invalid("OPPO HEIF", "top-level box is outside source")
            })?);
        }
    }
    output.extend_from_slice(trailing);

    let actual_flags = current_oppo_tag_flags(&output)?;
    if actual_flags != Some(expected_flags) {
        return Err(MetadataError::invalid(
            "OPPO UserComment",
            format!("post-write routing flags are {actual_flags:?}, expected {expected_flags}"),
        ));
    }
    Ok(output)
}
