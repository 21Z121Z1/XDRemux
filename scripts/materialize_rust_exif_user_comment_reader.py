#!/usr/bin/env python3
from pathlib import Path


exif_path = Path("crates/xdremux-format/src/exif_raw.rs")
text = exif_path.read_text()

if "const USER_COMMENT_TAG: u16 = 0x9286;" not in text:
    anchor = "const MAKER_NOTE_TAG: u16 = 0x927c;\n"
    if anchor not in text:
        raise SystemExit("MakerNote tag anchor not found")
    text = text.replace(anchor, anchor + "const USER_COMMENT_TAG: u16 = 0x9286;\n", 1)

if "pub fn exif_user_comment(tiff: &[u8])" not in text:
    anchor = "\n#[cfg(test)]\nmod tests {\n"
    if anchor not in text:
        raise SystemExit("exif_raw test module anchor not found")
    function = r'''
/// Read the ExifIFD UserComment value through the active TIFF entry pointer.
///
/// This intentionally does not scan the TIFF byte stream for tag-looking text.
/// Rewriters may append a new value and repoint the IFD while leaving the old
/// bytes as unreachable padding; callers must observe the structurally referenced
/// value rather than stale payload bytes.
pub fn exif_user_comment(tiff: &[u8]) -> Result<Option<Vec<u8>>> {
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
    let comments = exif
        .entries
        .iter()
        .filter(|entry| entry_tag(entry, order) == USER_COMMENT_TAG)
        .collect::<Vec<_>>();
    let Some(entry) = comments.first() else {
        return Ok(None);
    };
    if comments.len() != 1 {
        return Err(invalid(
            TIFF_CONTEXT,
            "ExifIFD has multiple UserComment entries",
        ));
    }
    let field_type = order.read_u16(&entry[2..4]);
    if field_type != TYPE_UNDEFINED {
        return Err(invalid(TIFF_CONTEXT, "UserComment is not UNDEFINED type"));
    }
    let count = usize::try_from(order.read_u32(&entry[4..8]))
        .map_err(|_| FormatError::overflow("UserComment length"))?;
    let value = if count <= 4 {
        entry[8..8 + count].to_vec()
    } else {
        let offset = usize::try_from(order.read_u32(&entry[8..12]))
            .map_err(|_| FormatError::overflow("UserComment offset"))?;
        read_slice(tiff, offset, count, "TIFF UserComment")?.to_vec()
    };
    Ok(Some(value))
}
'''
    text = text.replace(anchor, function + anchor, 1)

if "fn reads_only_the_referenced_user_comment_value()" not in text:
    anchor = "    #[test]\n    fn replaces_makernote_without_interpreting_vendor_tags() {\n"
    if anchor not in text:
        raise SystemExit("exif_raw unit-test anchor not found")
    test = r'''    #[test]
    fn reads_only_the_referenced_user_comment_value() {
        let order = ByteOrder::Little;
        let stale = b"ASCII\0\0\0Oplus_00000001";
        let active = b"ASCII\0\0\0Oplus_536870913";
        let mut tiff = b"II".to_vec();
        order.write_u16(42, &mut tiff);
        order.write_u32(8, &mut tiff);
        let ifd0_len = 2 + 12 + 4;
        let exif_offset = u32::try_from(8 + ifd0_len).unwrap();
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
        let exif_ifd_len = 2 + 12 + 4;
        let stale_offset = usize::try_from(exif_offset).unwrap() + exif_ifd_len;
        let active_offset = stale_offset + stale.len();
        let exif = Ifd {
            entries: vec![make_entry(
                order,
                USER_COMMENT_TAG,
                TYPE_UNDEFINED,
                u32::try_from(active.len()).unwrap(),
                u32::try_from(active_offset).unwrap(),
            )],
            next_ifd: 0,
        };
        append_ifd(&mut tiff, order, &exif).unwrap();
        tiff.extend_from_slice(stale);
        tiff.extend_from_slice(active);

        assert!(tiff.windows(stale.len()).any(|window| window == stale));
        assert_eq!(
            exif_user_comment(&tiff).unwrap().as_deref(),
            Some(active.as_slice())
        );
    }

'''
    text = text.replace(anchor, test + anchor, 1)

for marker in (
    "const USER_COMMENT_TAG: u16 = 0x9286;",
    "pub fn exif_user_comment(tiff: &[u8])",
    "fn reads_only_the_referenced_user_comment_value()",
):
    if marker not in text:
        raise SystemExit(f"Exif UserComment reader marker missing: {marker}")

exif_path.write_text(text)

lib_path = Path("crates/xdremux-format/src/lib.rs")
lib = lib_path.read_text()
old = "pub use exif_raw::{exif_makernote, heif_exif_tiff, jpeg_exif_tiff, replace_exif_makernote};\n"
new = "pub use exif_raw::{\n    exif_makernote, exif_user_comment, heif_exif_tiff, jpeg_exif_tiff, replace_exif_makernote,\n};\n"
if new not in lib:
    if old not in lib:
        raise SystemExit("exif_raw export anchor not found")
    lib = lib.replace(old, new, 1)
if lib.count("exif_user_comment") != 1:
    raise SystemExit("Exif UserComment export was materialized more than once")
lib_path.write_text(lib)
