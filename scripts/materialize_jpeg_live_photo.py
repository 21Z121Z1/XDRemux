from pathlib import Path


REQUIRED_MARKERS = {
    Path("crates/xdremux-metadata/src/ultrahdr_jpeg.rs"): [
        "fn local_name(name: &str) -> &str",
        "let key = local_name(attribute.key.as_ref());",
        "Some(name.as_str())",
        "1000_i32 + channel * 100",
    ],
    Path("crates/xdremux-runtime/src/live_photo.rs"): [
        "build_live_photo_jpeg_exif",
        ".with_exif_tiff(exif_tiff)",
    ],
    Path("crates/xdremux-codec/src/portable/primary_heif.rs"): [
        "if let Some(exif) = request.exif_tiff.as_ref() {",
        ".set_color_profile_raw(&profile)",
    ],
    Path("crates/xdremux-format/src/exif_raw.rs"): [
        "fn read_slice<'a>(",
        "raw_entries.as_chunks::<12>().0",
        "!output.len().is_multiple_of(2)",
    ],
}


for path, markers in REQUIRED_MARKERS.items():
    text = path.read_text()
    for marker in markers:
        if marker not in text:
            raise SystemExit(f"missing completed JPEG Live Photo marker in {path}: {marker!r}")
