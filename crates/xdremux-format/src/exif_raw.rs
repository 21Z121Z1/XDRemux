use crate::error::{FormatError, Result};
use crate::isobmff::{parse_meta_box, scan_top_level_boxes, EXIF, META};

const JPEG_CONTEXT: &str = "JPEG EXIF";
const TIFF_CONTEXT: &str = "TIFF EXIF";
const EXIF_APP1_SIGNATURE: &[u8] = b"Exif\0\0";
const EXIF_IFD_POINTER_TAG: u16 = 0x8769;
const MAKER_NOTE_TAG: u16 = 0x927c;
const TYPE_LONG: u16 = 4;
const TYPE_UNDEFINED: u16 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
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

    fn write_u16(self, value: u16, output: &mut Vec<u8>) {
        match self {
            Self::Little => output.extend_from_slice(&value.to_le_bytes()),
            Self::Big => output.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn write_u32(self, value: u32, output: &mut Vec<u8>) {
        match self {
            Self::Little => output.extend_from_slice(&value.to_le_bytes()),
            Self::Big => output.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn write_u32_at(self, output: &mut [u8], offset: usize, value: u32) -> Result<()> {
        let bytes = match self {
            Self::Little => value.to_le_bytes(),
            Self::Big => value.to_be_bytes(),
        };
        output
            .get_mut(offset..offset + 4)
            .ok_or_else(|| FormatError::invalid(TIFF_CONTEXT, "u32 patch is out of bounds"))?
            .copy_from_slice(&bytes);
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Ifd {
    entries: Vec<[u8; 12]>,
    next_ifd: u32,
}

fn invalid(context: &'static str, message: impl Into<String>) -> FormatError {
    FormatError::invalid(context, message)
}

fn checked_add(value: usize, amount: usize, context: &'static str) -> Result<usize> {
    value
        .checked_add(amount)
        .ok_or_else(|| FormatError::overflow(context))
}

fn read_slice<'a>(
    data: &'a [u8],
    start: usize,
    len: usize,
    context: &'static str,
) -> Result<&'a [u8]> {
    let end = checked_add(start, len, context)?;
    data.get(start..end).ok_or(FormatError::UnexpectedEof {
        context,
        offset: start,
        needed: len,
        end: data.len(),
    })
}

fn tiff_header(tiff: &[u8]) -> Result<(ByteOrder, usize)> {
    if tiff.len() < 8 {
        return Err(invalid(TIFF_CONTEXT, "header is shorter than 8 bytes"));
    }
    let order = match &tiff[..2] {
        b"II" => ByteOrder::Little,
        b"MM" => ByteOrder::Big,
        _ => return Err(invalid(TIFF_CONTEXT, "invalid byte-order marker")),
    };
    if order.read_u16(&tiff[2..4]) != 42 {
        return Err(invalid(TIFF_CONTEXT, "magic is not 42"));
    }
    let offset = usize::try_from(order.read_u32(&tiff[4..8]))
        .map_err(|_| FormatError::overflow("TIFF IFD0 offset"))?;
    if offset < 8 || offset >= tiff.len() {
        return Err(invalid(TIFF_CONTEXT, "IFD0 offset is outside TIFF data"));
    }
    Ok((order, offset))
}

fn parse_ifd(tiff: &[u8], offset: usize, order: ByteOrder, context: &'static str) -> Result<Ifd> {
    let count_bytes = read_slice(tiff, offset, 2, context)?;
    let count = usize::from(order.read_u16(count_bytes));
    let entries_start = checked_add(offset, 2, context)?;
    let entries_len = count
        .checked_mul(12)
        .ok_or_else(|| FormatError::overflow("TIFF IFD entries"))?;
    let entries_end = checked_add(entries_start, entries_len, context)?;
    let trailer = read_slice(tiff, entries_end, 4, context)?;
    let raw_entries = read_slice(tiff, entries_start, entries_len, context)?;
    let mut entries = Vec::with_capacity(count);
    for raw in raw_entries.as_chunks::<12>().0 {
        entries.push(*raw);
    }
    Ok(Ifd {
        entries,
        next_ifd: order.read_u32(trailer),
    })
}

fn entry_tag(entry: &[u8; 12], order: ByteOrder) -> u16 {
    order.read_u16(&entry[..2])
}

fn exif_ifd_pointer(ifd0: &Ifd, order: ByteOrder) -> Result<Option<(usize, u32)>> {
    let matches = ifd0
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry_tag(entry, order) == EXIF_IFD_POINTER_TAG)
        .collect::<Vec<_>>();
    let Some((index, entry)) = matches.first().copied() else {
        return Ok(None);
    };
    if matches.len() != 1 {
        return Err(invalid(TIFF_CONTEXT, "IFD0 has multiple ExifIFD pointers"));
    }
    let field_type = order.read_u16(&entry[2..4]);
    let count = order.read_u32(&entry[4..8]);
    if field_type != TYPE_LONG || count != 1 {
        return Err(invalid(
            TIFF_CONTEXT,
            "ExifIFD pointer must be LONG with count 1",
        ));
    }
    Ok(Some((index, order.read_u32(&entry[8..12]))))
}

fn append_even_padding(output: &mut Vec<u8>) {
    if !output.len().is_multiple_of(2) {
        output.push(0);
    }
}

fn u32_offset(value: usize, context: &'static str) -> Result<u32> {
    u32::try_from(value).map_err(|_| FormatError::overflow(context))
}

fn make_entry(
    order: ByteOrder,
    tag: u16,
    field_type: u16,
    count: u32,
    value_or_offset: u32,
) -> [u8; 12] {
    let mut output = Vec::with_capacity(12);
    order.write_u16(tag, &mut output);
    order.write_u16(field_type, &mut output);
    order.write_u32(count, &mut output);
    order.write_u32(value_or_offset, &mut output);
    output.try_into().expect("TIFF entry is exactly 12 bytes")
}

fn append_ifd(output: &mut Vec<u8>, order: ByteOrder, ifd: &Ifd) -> Result<u32> {
    let offset = u32_offset(output.len(), "TIFF appended IFD offset")?;
    let count = u16::try_from(ifd.entries.len())
        .map_err(|_| FormatError::overflow("TIFF appended IFD entry count"))?;
    order.write_u16(count, output);
    for entry in &ifd.entries {
        output.extend_from_slice(entry);
    }
    order.write_u32(ifd.next_ifd, output);
    Ok(offset)
}

fn minimal_tiff_with_makernote(maker_note: &[u8]) -> Result<Vec<u8>> {
    let order = ByteOrder::Big;
    let mut output = b"MM".to_vec();
    order.write_u16(42, &mut output);
    order.write_u32(8, &mut output);

    let ifd0_len = 2 + 12 + 4;
    let exif_ifd_offset = u32_offset(8 + ifd0_len, "minimal ExifIFD offset")?;
    let exif_ifd_len = 2 + 12 + 4;
    let note_offset = u32_offset(
        usize::try_from(exif_ifd_offset).unwrap_or(usize::MAX) + exif_ifd_len,
        "minimal MakerNote offset",
    )?;
    let note_count = u32::try_from(maker_note.len())
        .map_err(|_| FormatError::overflow("minimal MakerNote length"))?;

    let ifd0 = Ifd {
        entries: vec![make_entry(
            order,
            EXIF_IFD_POINTER_TAG,
            TYPE_LONG,
            1,
            exif_ifd_offset,
        )],
        next_ifd: 0,
    };
    let exif = Ifd {
        entries: vec![make_entry(
            order,
            MAKER_NOTE_TAG,
            TYPE_UNDEFINED,
            note_count,
            note_offset,
        )],
        next_ifd: 0,
    };
    append_ifd(&mut output, order, &ifd0)?;
    append_ifd(&mut output, order, &exif)?;
    output.extend_from_slice(maker_note);
    Ok(output)
}

/// Extract raw TIFF EXIF from a JPEG APP1 segment without decoding individual tags.
pub fn jpeg_exif_tiff(jpeg: &[u8]) -> Result<Option<Vec<u8>>> {
    if jpeg.get(..2) != Some(&[0xff, 0xd8]) {
        return Err(invalid(JPEG_CONTEXT, "missing JPEG SOI"));
    }
    let mut cursor = 2usize;
    let mut found: Option<Vec<u8>> = None;
    while cursor < jpeg.len() {
        if jpeg.get(cursor) != Some(&0xff) {
            return Err(invalid(JPEG_CONTEXT, "expected marker prefix before SOS"));
        }
        while jpeg.get(cursor) == Some(&0xff) {
            cursor = checked_add(cursor, 1, "JPEG EXIF marker offset")?;
        }
        let marker = *jpeg.get(cursor).ok_or(FormatError::UnexpectedEof {
            context: JPEG_CONTEXT,
            offset: cursor,
            needed: 1,
            end: jpeg.len(),
        })?;
        cursor = checked_add(cursor, 1, "JPEG EXIF marker offset")?;
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }
        let length_bytes = read_slice(jpeg, cursor, 2, JPEG_CONTEXT)?;
        let length = usize::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
        if length < 2 {
            return Err(invalid(JPEG_CONTEXT, "invalid APP/marker segment length"));
        }
        let payload_start = checked_add(cursor, 2, "JPEG EXIF payload offset")?;
        let segment_end = checked_add(cursor, length, "JPEG EXIF segment end")?;
        if segment_end > jpeg.len() {
            return Err(invalid(JPEG_CONTEXT, "segment exceeds JPEG bytes"));
        }
        let payload = &jpeg[payload_start..segment_end];
        if marker == 0xe1 && payload.starts_with(EXIF_APP1_SIGNATURE) {
            if found.is_some() {
                return Err(invalid(
                    JPEG_CONTEXT,
                    "multiple EXIF APP1 segments are ambiguous",
                ));
            }
            let tiff = payload[EXIF_APP1_SIGNATURE.len()..].to_vec();
            tiff_header(&tiff)?;
            found = Some(tiff);
        }
        cursor = segment_end;
    }
    Ok(found)
}

fn normalize_heif_exif_item(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.starts_with(b"II") || payload.starts_with(b"MM") {
        tiff_header(payload)?;
        return Ok(payload.to_vec());
    }
    if payload.len() < 4 {
        return Err(invalid("HEIF EXIF", "Exif item lacks TIFF offset prefix"));
    }
    let offset = usize::try_from(u32::from_be_bytes(payload[..4].try_into().unwrap()))
        .map_err(|_| FormatError::overflow("HEIF Exif TIFF offset"))?;
    for start in [offset, offset.saturating_add(4)] {
        if let Some(tiff) = payload.get(start..) {
            if tiff.starts_with(b"II") || tiff.starts_with(b"MM") {
                tiff_header(tiff)?;
                return Ok(tiff.to_vec());
            }
        }
    }
    Err(invalid(
        "HEIF EXIF",
        "Exif item TIFF offset does not locate an II/MM header",
    ))
}

/// Extract the sole raw Exif item from a HEIF file without interpreting vendor tags.
pub fn heif_exif_tiff(heif: &[u8]) -> Result<Option<Vec<u8>>> {
    let scan = scan_top_level_boxes(heif)?;
    let mut metas = scan.boxes.iter().filter(|header| header.kind == META);
    let Some(meta_header) = metas.next() else {
        return Ok(None);
    };
    if metas.next().is_some() {
        return Err(invalid(
            "HEIF EXIF",
            "multiple top-level meta boxes are ambiguous",
        ));
    }
    let meta = parse_meta_box(heif, meta_header)?;
    let exif_items = meta
        .iinf
        .entries
        .iter()
        .filter(|item| item.item_type == Some(EXIF))
        .collect::<Vec<_>>();
    let Some(item) = exif_items.first() else {
        return Ok(None);
    };
    if exif_items.len() != 1 {
        return Err(invalid("HEIF EXIF", "multiple Exif items are ambiguous"));
    }
    let entry = meta
        .iloc
        .entries
        .iter()
        .find(|entry| entry.item_id == item.item_id)
        .ok_or_else(|| invalid("HEIF EXIF", "Exif item has no iloc entry"))?;
    let payload = crate::exif::read_item_payload(heif, entry, meta.idat.as_ref())?;
    normalize_heif_exif_item(&payload).map(Some)
}

/// Replace only the TIFF ExifIFD MakerNote while preserving every other entry byte-for-byte.
///
/// Unknown vendor tags are never decoded. Existing TIFF-relative value offsets remain valid because
/// the original TIFF prefix is left untouched; a replacement ExifIFD (and, only when necessary, a
/// replacement IFD0) is appended and the single structural pointer is redirected to it.
pub fn replace_exif_makernote(tiff: Option<&[u8]>, maker_note: &[u8]) -> Result<Vec<u8>> {
    if maker_note.is_empty() {
        return Err(invalid(TIFF_CONTEXT, "replacement MakerNote is empty"));
    }
    let Some(tiff) = tiff else {
        return minimal_tiff_with_makernote(maker_note);
    };
    let (order, ifd0_offset) = tiff_header(tiff)?;
    let ifd0 = parse_ifd(tiff, ifd0_offset, order, "TIFF IFD0")?;
    let mut output = tiff.to_vec();
    append_even_padding(&mut output);
    let maker_note_offset = u32_offset(output.len(), "replacement MakerNote offset")?;
    output.extend_from_slice(maker_note);
    append_even_padding(&mut output);
    let maker_note_count = u32::try_from(maker_note.len())
        .map_err(|_| FormatError::overflow("replacement MakerNote length"))?;

    match exif_ifd_pointer(&ifd0, order)? {
        Some((pointer_index, old_exif_offset)) => {
            let old_exif_offset = usize::try_from(old_exif_offset)
                .map_err(|_| FormatError::overflow("ExifIFD offset"))?;
            let old_exif = parse_ifd(tiff, old_exif_offset, order, "TIFF ExifIFD")?;
            let mut entries = old_exif
                .entries
                .into_iter()
                .filter(|entry| entry_tag(entry, order) != MAKER_NOTE_TAG)
                .collect::<Vec<_>>();
            entries.push(make_entry(
                order,
                MAKER_NOTE_TAG,
                TYPE_UNDEFINED,
                maker_note_count,
                maker_note_offset,
            ));
            entries.sort_by_key(|entry| entry_tag(entry, order));
            let new_exif_offset = append_ifd(
                &mut output,
                order,
                &Ifd {
                    entries,
                    next_ifd: old_exif.next_ifd,
                },
            )?;
            let pointer_value_offset = ifd0_offset
                .checked_add(2)
                .and_then(|value| value.checked_add(pointer_index.checked_mul(12)?))
                .and_then(|value| value.checked_add(8))
                .ok_or_else(|| FormatError::overflow("ExifIFD pointer patch offset"))?;
            order.write_u32_at(&mut output, pointer_value_offset, new_exif_offset)?;
        }
        None => {
            let new_exif_offset = append_ifd(
                &mut output,
                order,
                &Ifd {
                    entries: vec![make_entry(
                        order,
                        MAKER_NOTE_TAG,
                        TYPE_UNDEFINED,
                        maker_note_count,
                        maker_note_offset,
                    )],
                    next_ifd: 0,
                },
            )?;
            append_even_padding(&mut output);
            let mut entries = ifd0.entries;
            entries.push(make_entry(
                order,
                EXIF_IFD_POINTER_TAG,
                TYPE_LONG,
                1,
                new_exif_offset,
            ));
            entries.sort_by_key(|entry| entry_tag(entry, order));
            let new_ifd0_offset = append_ifd(
                &mut output,
                order,
                &Ifd {
                    entries,
                    next_ifd: ifd0.next_ifd,
                },
            )?;
            order.write_u32_at(&mut output, 4, new_ifd0_offset)?;
        }
    }
    Ok(output)
}

/// Read the ExifIFD MakerNote without decoding unrelated TIFF entries.
pub fn exif_makernote(tiff: &[u8]) -> Result<Option<Vec<u8>>> {
    let (order, ifd0_offset) = tiff_header(tiff)?;
    let ifd0 = parse_ifd(tiff, ifd0_offset, order, "TIFF IFD0")?;
    let Some((_, exif_offset)) = exif_ifd_pointer(&ifd0, order)? else {
        return Ok(None);
    };
    let exif = parse_ifd(
        tiff,
        usize::try_from(exif_offset).map_err(|_| FormatError::overflow("ExifIFD offset"))?,
        order,
        "TIFF ExifIFD",
    )?;
    let notes = exif
        .entries
        .iter()
        .filter(|entry| entry_tag(entry, order) == MAKER_NOTE_TAG)
        .collect::<Vec<_>>();
    let Some(entry) = notes.first() else {
        return Ok(None);
    };
    if notes.len() != 1 {
        return Err(invalid(
            TIFF_CONTEXT,
            "ExifIFD has multiple MakerNote entries",
        ));
    }
    let field_type = order.read_u16(&entry[2..4]);
    if field_type != TYPE_UNDEFINED {
        return Err(invalid(TIFF_CONTEXT, "MakerNote is not UNDEFINED type"));
    }
    let count = usize::try_from(order.read_u32(&entry[4..8]))
        .map_err(|_| FormatError::overflow("MakerNote length"))?;
    let value = if count <= 4 {
        entry[8..8 + count].to_vec()
    } else {
        let offset = usize::try_from(order.read_u32(&entry[8..12]))
            .map_err(|_| FormatError::overflow("MakerNote offset"))?;
        read_slice(tiff, offset, count, "TIFF MakerNote")?.to_vec()
    };
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiff_with_opaque_vendor_entry() -> Vec<u8> {
        let order = ByteOrder::Little;
        let mut tiff = b"II".to_vec();
        order.write_u16(42, &mut tiff);
        order.write_u32(8, &mut tiff);
        let exif_offset = 26_u32;
        let ifd0 = Ifd {
            entries: vec![make_entry(
                order,
                EXIF_IFD_POINTER_TAG,
                TYPE_LONG,
                1,
                exif_offset,
            )],
            next_ifd: 0,
        };
        append_ifd(&mut tiff, order, &ifd0).unwrap();
        let vendor_entry = make_entry(order, 0x9286, 2, 4, u32::from_le_bytes(*b"bad!"));
        let exif = Ifd {
            entries: vec![vendor_entry],
            next_ifd: 0,
        };
        append_ifd(&mut tiff, order, &exif).unwrap();
        tiff
    }

    #[test]
    fn replaces_makernote_without_interpreting_vendor_tags() {
        let original = tiff_with_opaque_vendor_entry();
        let marker = original
            .windows(12)
            .find(|entry| entry[..2] == 0x9286_u16.to_le_bytes())
            .unwrap()
            .to_vec();
        let note = b"Apple iOS opaque note";
        let patched = replace_exif_makernote(Some(&original), note).unwrap();
        assert_eq!(
            exif_makernote(&patched).unwrap().as_deref(),
            Some(note.as_slice())
        );
        assert!(patched.windows(marker.len()).any(|window| window == marker));
    }

    #[test]
    fn creates_minimal_tiff_when_source_has_no_exif() {
        let note = b"Apple iOS minimal note";
        let tiff = replace_exif_makernote(None, note).unwrap();
        assert_eq!(
            exif_makernote(&tiff).unwrap().as_deref(),
            Some(note.as_slice())
        );
    }

    #[test]
    fn extracts_raw_jpeg_exif_without_decoding_entries() {
        let tiff = tiff_with_opaque_vendor_entry();
        let mut payload = EXIF_APP1_SIGNATURE.to_vec();
        payload.extend_from_slice(&tiff);
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
        jpeg.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
        jpeg.extend_from_slice(&payload);
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        assert_eq!(
            jpeg_exif_tiff(&jpeg).unwrap().as_deref(),
            Some(tiff.as_slice())
        );
    }
}
