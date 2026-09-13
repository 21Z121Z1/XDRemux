#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, got {count}: {old[:96]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "crates/xdremux-heif/Cargo.toml",
    '[dependencies]\nxdremux-format = { path = "../xdremux-format" }\n',
    '[dependencies]\nsha2 = "=0.11.0"\nxdremux-format = { path = "../xdremux-format" }\n',
)

path = Path("crates/xdremux-heif/src/texture_styles.rs")
text = path.read_text()
text = text.replace(
    'use xdremux_format::{exif_makernote, heif_exif_tiff, replace_exif_makernote, FourCC};\n\n',
    'use sha2::{Digest, Sha256};\nuse xdremux_format::{exif_makernote, heif_exif_tiff, replace_exif_makernote, FourCC};\n\n',
    1,
)
if 'use sha2::{Digest, Sha256};' not in text:
    raise SystemExit("failed to add sha2 import")

constant_anchor = 'const SKIN_AUX_TYPE: &[u8] = b"urn:com:apple:photo:2019:aux:semanticskinmatte";\n\n'
constant_insert = '''const SKIN_AUX_TYPE: &[u8] = b"urn:com:apple:photo:2019:aux:semanticskinmatte";
const APPLE_MAKERNOTE_PREFIX: &[u8] = b"Apple iOS\\0\\0\\x01";
const APPLE_PHOTO_IDENTIFIER_TAG: u16 = 0x002b;
const APPLE_TEXTURE_STYLE_TAG: u16 = 0x0054;
const TIFF_TYPE_ASCII: u16 = 2;
const TIFF_TYPE_UNDEFINED: u16 = 7;

'''
if text.count(constant_anchor) != 1:
    raise SystemExit("MakerNote constant anchor changed")
text = text.replace(constant_anchor, constant_insert, 1)

old_block = '''fn texture_makernote() -> Result<Vec<u8>> {
    let tag84 = texture_tag84_bplist()?;
    let mut note = b"Apple iOS\\0\\0\\x01MM".to_vec();
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
    output.extend_from_slice(b"Exif\\0\\0");
    output.extend_from_slice(&patched);
    Ok(output)
}
'''
new_block = '''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppleMakerNoteByteOrder {
    Little,
    Big,
}

impl AppleMakerNoteByteOrder {
    fn read_u16(self, bytes: &[u8]) -> u16 {
        match self {
            Self::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
            Self::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
        }
    }

    fn read_u32(self, bytes: &[u8]) -> u32 {
        match self {
            Self::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Self::Big => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        }
    }

    fn write_u16_at(self, output: &mut [u8], offset: usize, value: u16) -> Result<()> {
        let bytes = match self {
            Self::Little => value.to_le_bytes(),
            Self::Big => value.to_be_bytes(),
        };
        output
            .get_mut(offset..offset + 2)
            .ok_or_else(|| invalid("Apple MakerNote u16 patch is outside payload"))?
            .copy_from_slice(&bytes);
        Ok(())
    }

    fn write_u32_at(self, output: &mut [u8], offset: usize, value: u32) -> Result<()> {
        let bytes = match self {
            Self::Little => value.to_le_bytes(),
            Self::Big => value.to_be_bytes(),
        };
        output
            .get_mut(offset..offset + 4)
            .ok_or_else(|| invalid("Apple MakerNote u32 patch is outside payload"))?
            .copy_from_slice(&bytes);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppleMakerNoteEntry {
    entry_offset: usize,
    field_type: u16,
    item_count: u32,
    value_offset: usize,
    value_length: usize,
}

fn apple_makernote_byte_order(note: &[u8]) -> Result<AppleMakerNoteByteOrder> {
    if !note.starts_with(APPLE_MAKERNOTE_PREFIX) || note.len() < 20 {
        return Err(invalid("Texture Style source does not contain an Apple MakerNote"));
    }
    match note.get(12..14) {
        Some(b"II") => Ok(AppleMakerNoteByteOrder::Little),
        Some(b"MM") => Ok(AppleMakerNoteByteOrder::Big),
        _ => Err(invalid("Apple MakerNote has an invalid byte-order marker")),
    }
}

fn tiff_type_size(field_type: u16) -> Option<usize> {
    match field_type {
        1 | 2 | 6 | 7 => Some(1),
        3 | 8 => Some(2),
        4 | 9 | 11 => Some(4),
        5 | 10 | 12 => Some(8),
        _ => None,
    }
}

fn apple_makernote_entry(note: &[u8], wanted_tag: u16) -> Result<Option<AppleMakerNoteEntry>> {
    let order = apple_makernote_byte_order(note)?;
    let count = usize::from(order.read_u16(
        note.get(14..16)
            .ok_or_else(|| invalid("Apple MakerNote entry count is truncated"))?,
    ));
    let table_end = 16usize
        .checked_add(
            count
                .checked_mul(12)
                .ok_or_else(|| invalid("Apple MakerNote entry table overflows"))?,
        )
        .and_then(|value| value.checked_add(4))
        .ok_or_else(|| invalid("Apple MakerNote entry table overflows"))?;
    if table_end > note.len() {
        return Err(invalid("Apple MakerNote entry table is truncated"));
    }

    let mut found = None;
    for index in 0..count {
        let entry_offset = 16 + index * 12;
        let entry = note
            .get(entry_offset..entry_offset + 12)
            .ok_or_else(|| invalid("Apple MakerNote entry is truncated"))?;
        if order.read_u16(&entry[..2]) != wanted_tag {
            continue;
        }
        if found.is_some() {
            return Err(invalid(format!(
                "Apple MakerNote contains duplicate tag {wanted_tag}"
            )));
        }
        let field_type = order.read_u16(&entry[2..4]);
        let item_count = order.read_u32(&entry[4..8]);
        let unit = tiff_type_size(field_type)
            .ok_or_else(|| invalid(format!("Apple MakerNote tag {wanted_tag} has unknown TIFF type {field_type}")))?;
        let value_length = usize::try_from(item_count)
            .ok()
            .and_then(|value| value.checked_mul(unit))
            .ok_or_else(|| invalid("Apple MakerNote value length overflows"))?;
        let value_offset = if value_length <= 4 {
            entry_offset + 8
        } else {
            usize::try_from(order.read_u32(&entry[8..12]))
                .map_err(|_| invalid("Apple MakerNote value offset exceeds usize"))?
        };
        let value_end = value_offset
            .checked_add(value_length)
            .ok_or_else(|| invalid("Apple MakerNote value range overflows"))?;
        if value_end > note.len() {
            return Err(invalid(format!(
                "Apple MakerNote tag {wanted_tag} value is outside payload"
            )));
        }
        found = Some(AppleMakerNoteEntry {
            entry_offset,
            field_type,
            item_count,
            value_offset,
            value_length,
        });
    }
    Ok(found)
}

fn source_photo_identifier(source: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"XDRemux Texture Style PhotoIdentifier\\0");
    hasher.update(source);
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // Keep the wire shape of Apple's UUID PhotoIdentifier while deriving it
    // deterministically from this photo instead of copying a donor identifier.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut identifier = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
    .into_bytes();
    identifier.push(0);
    identifier
}

fn build_minimal_texture_makernote(photo_identifier: &[u8], tag84: &[u8]) -> Result<Vec<u8>> {
    if photo_identifier.len() != 37
        || photo_identifier.last() != Some(&0)
        || tag84.is_empty()
    {
        return Err(invalid("Texture Style Apple MakerNote inputs are malformed"));
    }
    let value_offset = 14usize + 2 + 2 * 12 + 4;
    let tag84_offset = value_offset
        .checked_add(photo_identifier.len())
        .ok_or_else(|| invalid("Texture Style MakerNote value offset overflows"))?;
    let mut note = b"Apple iOS\\0\\0\\x01MM".to_vec();
    note.extend_from_slice(&2u16.to_be_bytes());

    note.extend_from_slice(&APPLE_PHOTO_IDENTIFIER_TAG.to_be_bytes());
    note.extend_from_slice(&TIFF_TYPE_ASCII.to_be_bytes());
    note.extend_from_slice(
        &u32::try_from(photo_identifier.len())
            .map_err(|_| invalid("PhotoIdentifier length exceeds u32"))?
            .to_be_bytes(),
    );
    note.extend_from_slice(
        &u32::try_from(value_offset)
            .map_err(|_| invalid("PhotoIdentifier offset exceeds u32"))?
            .to_be_bytes(),
    );

    note.extend_from_slice(&APPLE_TEXTURE_STYLE_TAG.to_be_bytes());
    note.extend_from_slice(&TIFF_TYPE_UNDEFINED.to_be_bytes());
    note.extend_from_slice(
        &u32::try_from(tag84.len())
            .map_err(|_| invalid("Texture Style tag84 length exceeds u32"))?
            .to_be_bytes(),
    );
    note.extend_from_slice(
        &u32::try_from(tag84_offset)
            .map_err(|_| invalid("Texture Style tag84 offset exceeds u32"))?
            .to_be_bytes(),
    );
    note.extend_from_slice(&0u32.to_be_bytes());
    note.extend_from_slice(photo_identifier);
    note.extend_from_slice(tag84);
    Ok(note)
}

fn surgical_replace_texture_tag84(note: &[u8], tag84: &[u8]) -> Result<Vec<u8>> {
    let order = apple_makernote_byte_order(note)?;
    let entry = apple_makernote_entry(note, APPLE_TEXTURE_STYLE_TAG)?
        .ok_or_else(|| invalid("Apple MakerNote is missing Texture Style tag84"))?;
    if entry.field_type != TIFF_TYPE_UNDEFINED {
        return Err(invalid("Apple MakerNote tag84 is not TIFF UNDEFINED"));
    }
    let old_end = entry
        .value_offset
        .checked_add(entry.value_length)
        .ok_or_else(|| invalid("Apple MakerNote tag84 range overflows"))?;
    let mut output = note.to_vec();
    let new_offset = if old_end == output.len() {
        output.truncate(entry.value_offset);
        entry.value_offset
    } else {
        output.len()
    };
    if new_offset == output.len() {
        output.extend_from_slice(tag84);
    } else {
        return Err(invalid("Apple MakerNote tag84 replacement offset is inconsistent"));
    }
    order.write_u16_at(&mut output, entry.entry_offset + 2, TIFF_TYPE_UNDEFINED)?;
    order.write_u32_at(
        &mut output,
        entry.entry_offset + 4,
        u32::try_from(tag84.len()).map_err(|_| invalid("Texture Style tag84 length exceeds u32"))?,
    )?;
    order.write_u32_at(
        &mut output,
        entry.entry_offset + 8,
        u32::try_from(new_offset).map_err(|_| invalid("Texture Style tag84 offset exceeds u32"))?,
    )?;
    Ok(output)
}

fn texture_makernote(source: &[u8], tiff: &[u8]) -> Result<Vec<u8>> {
    let tag84 = texture_tag84_bplist()?;
    if let Some(existing) = exif_makernote(tiff)? {
        if existing.starts_with(APPLE_MAKERNOTE_PREFIX) {
            let photo = apple_makernote_entry(&existing, APPLE_PHOTO_IDENTIFIER_TAG)?
                .ok_or_else(|| invalid("Apple MakerNote is missing PhotoIdentifier tag43"))?;
            if photo.field_type != TIFF_TYPE_ASCII
                || photo.item_count != 37
                || existing.get(photo.value_offset + 36) != Some(&0)
            {
                return Err(invalid("Apple MakerNote PhotoIdentifier is malformed"));
            }
            return surgical_replace_texture_tag84(&existing, &tag84);
        }
    }
    build_minimal_texture_makernote(&source_photo_identifier(source), &tag84)
}

fn texture_exif_payload(data: &[u8]) -> Result<Vec<u8>> {
    let tiff =
        heif_exif_tiff(data)?.ok_or_else(|| invalid("Texture Style source has no Exif item"))?;
    let note = texture_makernote(data, &tiff)?;
    let patched = replace_exif_makernote(Some(&tiff), &note)?;
    let mut output = Vec::with_capacity(10 + patched.len());
    output.extend_from_slice(&6u32.to_be_bytes());
    output.extend_from_slice(b"Exif\\0\\0");
    output.extend_from_slice(&patched);
    Ok(output)
}
'''
if text.count(old_block) != 1:
    raise SystemExit("old Texture Style MakerNote block changed")
text = text.replace(old_block, new_block, 1)

old_validation = '''    let tag84 = texture_tag84_bplist()?;
    if !note.starts_with(b"Apple iOS\\0\\0\\x01")
        || !note
            .windows(tag84.len())
            .any(|window| window == tag84.as_slice())
    {
        return Err(invalid(
            "Texture Style output MakerNote tag84 contract is missing",
        ));
    }
'''
new_validation = '''    let photo = apple_makernote_entry(&note, APPLE_PHOTO_IDENTIFIER_TAG)?
        .ok_or_else(|| invalid("Texture Style output MakerNote PhotoIdentifier is missing"))?;
    if photo.field_type != TIFF_TYPE_ASCII
        || photo.item_count != 37
        || note.get(photo.value_offset + 36) != Some(&0)
    {
        return Err(invalid(
            "Texture Style output MakerNote PhotoIdentifier contract is malformed",
        ));
    }
    let tag84_entry = apple_makernote_entry(&note, APPLE_TEXTURE_STYLE_TAG)?
        .ok_or_else(|| invalid("Texture Style output MakerNote tag84 is missing"))?;
    let tag84 = texture_tag84_bplist()?;
    let tag84_end = tag84_entry
        .value_offset
        .checked_add(tag84_entry.value_length)
        .ok_or_else(|| invalid("Texture Style output MakerNote tag84 range overflows"))?;
    if tag84_entry.field_type != TIFF_TYPE_UNDEFINED
        || tag84_entry.item_count != u32::try_from(tag84.len()).unwrap_or(u32::MAX)
        || note.get(tag84_entry.value_offset..tag84_end) != Some(tag84.as_slice())
    {
        return Err(invalid(
            "Texture Style output MakerNote tag84 contract is malformed",
        ));
    }
'''
if text.count(old_validation) != 1:
    raise SystemExit("MakerNote validation anchor changed")
text = text.replace(old_validation, new_validation, 1)

old_test = '''    #[test]
    fn verified_texture_contract_constants_are_stable() {
        assert_eq!(TEXTURE_ROLE_AUX_TYPES.len(), 12);
        assert_eq!(texture_metadata_bplist().unwrap().len(), 216);
        assert_eq!(texture_tag84_bplist().unwrap().len(), 133);
        let note = texture_makernote().unwrap();
        assert!(note.starts_with(b"Apple iOS\\0\\0\\x01MM"));
        assert!(note.ends_with(&texture_tag84_bplist().unwrap()));
    }
'''
new_test = '''    #[test]
    fn verified_texture_contract_constants_are_stable() {
        assert_eq!(TEXTURE_ROLE_AUX_TYPES.len(), 12);
        assert_eq!(texture_metadata_bplist().unwrap().len(), 216);
        assert_eq!(texture_tag84_bplist().unwrap().len(), 133);
        let photo = source_photo_identifier(b"fixture");
        let note = build_minimal_texture_makernote(&photo, &texture_tag84_bplist().unwrap()).unwrap();
        assert_eq!(note.len(), 214);
        assert!(note.starts_with(b"Apple iOS\\0\\0\\x01MM"));
        assert!(note.ends_with(&texture_tag84_bplist().unwrap()));
        let photo_entry = apple_makernote_entry(&note, APPLE_PHOTO_IDENTIFIER_TAG)
            .unwrap()
            .unwrap();
        assert_eq!(photo_entry.field_type, TIFF_TYPE_ASCII);
        assert_eq!(photo_entry.item_count, 37);
        assert_eq!(
            &note[photo_entry.value_offset..photo_entry.value_offset + photo_entry.value_length],
            photo.as_slice()
        );
    }

    #[test]
    fn photo_identifier_is_source_derived_stable_and_uuid_shaped() {
        let first = source_photo_identifier(b"photo-a");
        let second = source_photo_identifier(b"photo-a");
        let other = source_photo_identifier(b"photo-b");
        assert_eq!(first, second);
        assert_ne!(first, other);
        assert_eq!(first.len(), 37);
        assert_eq!(first.last(), Some(&0));
        for index in [8, 13, 18, 23] {
            assert_eq!(first[index], b'-');
        }
    }

    #[test]
    fn apple_makernote_patch_preserves_photo_identifier_and_retargets_tag84() {
        let photo = source_photo_identifier(b"native-photo");
        let old_tag84 = b"old-tag84-payload";
        let note = build_minimal_texture_makernote(&photo, old_tag84).unwrap();
        let old_photo = apple_makernote_entry(&note, APPLE_PHOTO_IDENTIFIER_TAG)
            .unwrap()
            .unwrap();
        let old_photo_bytes = note
            [old_photo.value_offset..old_photo.value_offset + old_photo.value_length]
            .to_vec();
        let new_tag84 = texture_tag84_bplist().unwrap();
        let patched = surgical_replace_texture_tag84(&note, &new_tag84).unwrap();
        let new_photo = apple_makernote_entry(&patched, APPLE_PHOTO_IDENTIFIER_TAG)
            .unwrap()
            .unwrap();
        assert_eq!(
            &patched[new_photo.value_offset..new_photo.value_offset + new_photo.value_length],
            old_photo_bytes.as_slice()
        );
        let tag84 = apple_makernote_entry(&patched, APPLE_TEXTURE_STYLE_TAG)
            .unwrap()
            .unwrap();
        assert_eq!(tag84.field_type, TIFF_TYPE_UNDEFINED);
        assert_eq!(tag84.item_count, 133);
        assert_eq!(
            &patched[tag84.value_offset..tag84.value_offset + tag84.value_length],
            new_tag84.as_slice()
        );
    }
'''
if text.count(old_test) != 1:
    raise SystemExit("Texture Style unit-test anchor changed")
text = text.replace(old_test, new_test, 1)
path.write_text(text)
