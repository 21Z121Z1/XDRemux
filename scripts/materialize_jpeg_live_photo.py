from pathlib import Path


def ensure_replace(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text()
    if old in text:
        path.write_text(text.replace(old, new, 1))
        return
    if new in text:
        return
    raise SystemExit(f"expected old or new {label} fragment not found")


def ensure_replace_all(path: Path, replacements: dict[str, str], label: str) -> None:
    text = path.read_text()
    changed = False
    for old, new in replacements.items():
        if old in text:
            text = text.replace(old, new)
            changed = True
        elif new not in text:
            raise SystemExit(f"expected old or new {label} fragment not found: {old[:80]!r}")
    if changed:
        path.write_text(text)


metadata = Path("crates/xdremux-metadata/src/ultrahdr_jpeg.rs")
ensure_replace_all(
    metadata,
    {
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
    },
    "Ultra HDR quick-xml adaptation",
)

runtime = Path("crates/xdremux-runtime/src/live_photo.rs")
ensure_replace(
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
ensure_replace(
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

primary = Path("crates/xdremux-codec/src/portable/primary_heif.rs")
ensure_replace(
    primary,
    """        if let Some(exif) = request.exif_tiff.as_ref()
            && !(exif.starts_with(b"II") || exif.starts_with(b"MM"))
        {
            return Err(CodecError::invalid(
                "primary HEIF EXIF must begin at a TIFF II/MM header",
            ));
        }
""",
    """        if let Some(exif) = request.exif_tiff.as_ref() {
            if !(exif.starts_with(b"II") || exif.starts_with(b"MM")) {
                return Err(CodecError::invalid(
                    "primary HEIF EXIF must begin at a TIFF II/MM header",
                ));
            }
        }
""",
    "primary HEIF Rust-2021 EXIF guard",
)

raw_exif = Path("crates/xdremux-format/src/exif_raw.rs")
ensure_replace(
    raw_exif,
    "fn read_slice(data: &[u8], start: usize, len: usize, context: &'static str) -> Result<&[u8]> {",
    "fn read_slice<'a>(data: &'a [u8], start: usize, len: usize, context: &'static str) -> Result<&'a [u8]> {",
    "raw EXIF slice lifetime",
)
ensure_replace(
    raw_exif,
    """        if let Some(tiff) = payload.get(start..)
            && (tiff.starts_with(b"II") || tiff.starts_with(b"MM"))
        {
            tiff_header(tiff)?;
            return Ok(tiff.to_vec());
        }
""",
    """        if let Some(tiff) = payload.get(start..) {
            if tiff.starts_with(b"II") || tiff.starts_with(b"MM") {
                tiff_header(tiff)?;
                return Ok(tiff.to_vec());
            }
        }
""",
    "raw HEIF EXIF Rust-2021 guard",
)
ensure_replace(
    raw_exif,
    """    for raw in raw_entries.chunks_exact(12) {
        entries.push(raw.try_into().expect("chunks_exact yields 12-byte entries"));
    }
""",
    """    for raw in raw_entries.as_chunks::<12>().0 {
        entries.push(*raw);
    }
""",
    "raw EXIF fixed-size IFD chunks",
)
ensure_replace(
    raw_exif,
    "if output.len() % 2 != 0 {",
    "if !output.len().is_multiple_of(2) {",
    "raw EXIF even padding",
)
