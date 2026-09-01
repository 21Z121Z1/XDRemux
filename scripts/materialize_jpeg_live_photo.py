from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text()
    if old not in text:
        raise SystemExit(f"expected {label} fragment not found")
    path.write_text(text.replace(old, new, 1))


metadata = Path("crates/xdremux-metadata/src/ultrahdr_jpeg.rs")
text = metadata.read_text()
replacements = {
    "fn local_name(name: &[u8]) -> &[u8] {\n    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)\n}":
        "fn local_name(name: &str) -> &str {\n    name.rsplit(':').next().unwrap_or(name)\n}",
    "let key = std::str::from_utf8(local_name(attribute.key.as_ref()))\n            .map_err(|_| MetadataError::invalid(XMP_CONTEXT, \"non-UTF-8 XML attribute name\"))?;":
        "let key = local_name(attribute.key.as_ref());",
    "let name = std::str::from_utf8(local_name(element.name().as_ref()))\n                    .map_err(|_| MetadataError::invalid(XMP_CONTEXT, \"non-UTF-8 element name\"))?\n                    .to_owned();":
        "let name = local_name(element.name().as_ref()).to_owned();",
    "let value = std::str::from_utf8(text.as_ref())\n                    .map_err(|_| MetadataError::invalid(XMP_CONTEXT, \"non-UTF-8 text value\"))?\n                    .trim();":
        "let value = text.as_ref().trim();",
    "let name = std::str::from_utf8(local_name(element.name().as_ref()))\n                    .map_err(|_| MetadataError::invalid(XMP_CONTEXT, \"non-UTF-8 element name\"))?;":
        "let name = local_name(element.name().as_ref()).to_owned();",
    "if active_field.as_deref() == Some(name) {":
        "if active_field.as_deref() == Some(name.as_str()) {",
    "1000_i32 + channel as i32 * 100":
        "1000_i32 + channel * 100",
}
for old, new in replacements.items():
    if old not in text:
        raise SystemExit(f"expected metadata parser fragment not found: {old[:80]!r}")
    text = text.replace(old, new)
metadata.write_text(text)

runtime = Path("crates/xdremux-runtime/src/live_photo.rs")
replace_once(
    runtime,
    """    reconcile_live_photo_pair, resolve_live_photo_still_time, validate_live_photo_movie,
    write_live_photo_heif_still, write_live_photo_jpeg_metadata, write_live_photo_movie, ByteRange,
    MotionPhotoAsset, MotionPhotoSourceKind,
""",
    """    build_live_photo_jpeg_exif, reconcile_live_photo_pair, resolve_live_photo_still_time,
    validate_live_photo_movie, write_live_photo_heif_still, write_live_photo_movie, ByteRange,
    MotionPhotoAsset, MotionPhotoSourceKind,
""",
    "runtime Motion Photo import",
)
replace_once(
    runtime,
    """    let encoded_base = heif
        .encode_primary_heif(&PrimaryHeifEncodeRequest::live_photo(raster, icc_profile))
        .map_err(|error| RuntimeError::external("Motion Photo primary HEIC encode", error))?;
    let encoded_base = write_live_photo_jpeg_metadata(primary, encoded_base, content_identifier)
        .map_err(|error| RuntimeError::external("Live Photo JPEG EXIF transfer", error))?;
""",
    """    let exif_tiff = build_live_photo_jpeg_exif(primary, content_identifier)
        .map_err(|error| RuntimeError::external("Live Photo JPEG EXIF transfer", error))?;
    let encoded_base = heif
        .encode_primary_heif(
            &PrimaryHeifEncodeRequest::live_photo(raster, icc_profile)
                .with_exif_tiff(exif_tiff),
        )
        .map_err(|error| RuntimeError::external("Motion Photo primary HEIC encode", error))?;
""",
    "runtime JPEG primary encode",
)
