#!/usr/bin/env python3
from pathlib import Path


path = Path("crates/xdremux-metadata/src/oppo_heif.rs")
text = path.read_text()

text = text.replace(
    "use xdremux_format::exif::read_item_payload;\n",
    "use xdremux_format::{exif_user_comment, heif_exif_tiff};\n",
    1,
)
text = text.replace(
    "    adjusted_extent_for_oppo_user_comment_patch, adjusted_oppo_user_comment_in_heif,\n",
    "    adjusted_extent_for_oppo_user_comment_patch, adjusted_oppo_user_comment,\n",
    1,
)

start = text.find("fn current_oppo_tag_flags(data: &[u8]) -> Result<Option<u32>> {")
end = text.find("/// Read the current OPPO routing flags", start)
if start == -1 or end == -1:
    if "let Some(tiff) = heif_exif_tiff(data)?" not in text:
        raise SystemExit("current OPPO routing reader anchor not found")
else:
    replacement = '''fn current_oppo_tag_flags(data: &[u8]) -> Result<Option<u32>> {
    let Some(tiff) = heif_exif_tiff(data)? else {
        return Ok(None);
    };
    let Some(comment) = exif_user_comment(&tiff)? else {
        return Ok(None);
    };
    Ok(find_oppo_tag_flag(&comment).map(|tag| tag.value))
}

'''
    text = text[:start] + replacement + text[end:]

old_patch = '''    let patched_comment = adjusted_oppo_user_comment_in_heif(data, compatibility)?
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "routing patch disappeared"))?;
'''
new_patch = '''    let tiff = heif_exif_tiff(data)?
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "Exif TIFF is missing"))?;
    let comment = exif_user_comment(&tiff)?
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "UserComment is missing"))?;
    let patched_comment = adjusted_oppo_user_comment(&comment, compatibility)?
        .ok_or_else(|| MetadataError::invalid("OPPO UserComment", "routing patch disappeared"))?;
'''
if new_patch not in text:
    if old_patch not in text:
        raise SystemExit("OPPO patched-comment anchor not found")
    text = text.replace(old_patch, new_patch, 1)

for marker in (
    "use xdremux_format::{exif_user_comment, heif_exif_tiff};",
    "let Some(tiff) = heif_exif_tiff(data)?",
    "let Some(comment) = exif_user_comment(&tiff)?",
    "let patched_comment = adjusted_oppo_user_comment(&comment, compatibility)?",
):
    if marker not in text:
        raise SystemExit(f"structural OPPO read marker missing: {marker}")
if "adjusted_oppo_user_comment_in_heif(data, compatibility)" in text:
    raise SystemExit("OPPO patcher still derives routing from a raw HEIF byte scan")
if "read_item_payload(data, exif_entry" in text:
    raise SystemExit("OPPO routing reader still scans the whole Exif item")

path.write_text(text)
