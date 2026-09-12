#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, got {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))


# Expose the new HEIF carrier operation.
replace_once(
    "crates/xdremux-heif/src/lib.rs",
    "mod styles;\nmod validation;",
    "mod styles;\nmod texture_styles;\nmod validation;",
)
replace_once(
    "crates/xdremux-heif/src/lib.rs",
    "pub use styles::{assemble_photographic_styles_heif, PhotographicStylesAssembly};",
    "pub use styles::{assemble_photographic_styles_heif, PhotographicStylesAssembly};\n"
    "pub use texture_styles::augment_texture_style_heif;",
)

# Derive a second serializer from the established v15 serializer. Both consume
# exactly the same source-derived request. The existing v15 object remains the
# preflight object for Apple's private Styles consumer; the final HEIF carries
# the physical-device-positive v16 contract with l=false.
engine_path = Path("crates/xdremux-engine/src/apple_photographic_styles.rs")
engine = engine_path.read_text()
start = engine.index("pub fn apple_style_property_list(")
end = engine.index("\nfn validate_style_statistics(", start)
original = engine[start:end]
texture = original.replace(
    "pub fn apple_style_property_list(",
    "pub fn apple_texture_style_property_list(",
    1,
).replace(
    '("0", PlistValue::Integer(15)),',
    '("0", PlistValue::Integer(16)),',
    1,
).replace(
    '("k", PlistValue::Bool(false)),\n    ]);',
    '("k", PlistValue::Bool(false)),\n        ("l", PlistValue::Bool(false)),\n    ]);',
    1,
)
if texture == original or '("l", PlistValue::Bool(false))' not in texture:
    raise SystemExit("failed to derive Texture Style v16 property-list serializer")
texture = "/// Synthesize the iOS 27 Texture Style v16 metadata object.\n" + texture
engine_path.write_text(engine[:end] + "\n\n" + texture + engine[end:])

replace_once(
    "crates/xdremux-engine/src/lib.rs",
    "apple_style_property_list, apple_style_solve_refinement_update, resolve_apple_style_scene_type,",
    "apple_style_property_list, apple_style_solve_refinement_update, apple_texture_style_property_list,\n"
    "    resolve_apple_style_scene_type,",
)

# Keep v15 private-consumer validation, but write v16 into the carrier and then
# add the complete 2026 Texture Style contract before publication.
runtime_path = Path("crates/xdremux-runtime/src/apple_styles.rs")
runtime = runtime_path.read_text()
old_import = "apple_style_property_list, resolve_apple_style_scene_type, AppleL8Mask,"
new_import = (
    "apple_style_property_list, apple_texture_style_property_list, "
    "resolve_apple_style_scene_type, AppleL8Mask,"
)
if runtime.count(old_import) != 1:
    raise SystemExit("runtime: Texture serializer import anchor changed")
runtime = runtime.replace(old_import, new_import, 1)

old_start = "    let style_properties = apple_style_property_list(&AppleStylePropertyListRequest {\n"
if runtime.count(old_start) != 1:
    raise SystemExit("runtime: style request start anchor changed")
runtime = runtime.replace(old_start, "    let style_request = AppleStylePropertyListRequest {\n", 1)

old_tail = (
    "        face_exposure_boost,\n"
    "    })\n"
    "    .map_err(|error| RuntimeError::external(\"Photographic Styles metadata\", error))?;\n\n"
    "    // Validate the exact Rust-owned key-1 resource"
)
new_tail = (
    "        face_exposure_boost,\n"
    "    };\n"
    "    let style_properties = apple_style_property_list(&style_request)\n"
    "        .map_err(|error| RuntimeError::external(\"Photographic Styles metadata\", error))?;\n"
    "    let texture_style_properties = apple_texture_style_property_list(&style_request)\n"
    "        .map_err(|error| RuntimeError::external(\"Texture Style v16 metadata\", error))?;\n\n"
    "    // Validate the exact Rust-owned key-1 resource"
)
if runtime.count(old_tail) != 1:
    raise SystemExit("runtime: style request tail anchor changed")
runtime = runtime.replace(old_tail, new_tail, 1)

if runtime.count("style_property_list: &style_properties,") != 1:
    raise SystemExit("runtime: assembly property-list anchor changed")
runtime = runtime.replace(
    "style_property_list: &style_properties,",
    "style_property_list: &texture_style_properties,",
    1,
)

old_assembly = '''    let assembled = xdremux_heif::assemble_photographic_styles_heif(&semantic_base, &assembly)
        .map_err(|error| RuntimeError::external("Rust Photographic Styles graph", error))?;
    xdremux_heif::validate_gain_map_structure(&assembled).map_err(|error| {
        RuntimeError::external("Photographic Styles HDR structural validation", error)
    })?;

    let mut publisher = AtomicFilePublisher::new(output.to_path_buf());
    let published = publisher.publish_bytes(assembled)?;
'''
new_assembly = '''    let assembled = xdremux_heif::assemble_photographic_styles_heif(&semantic_base, &assembly)
        .map_err(|error| RuntimeError::external("Rust Photographic Styles graph", error))?;
    let textured = xdremux_heif::augment_texture_style_heif(&assembled)
        .map_err(|error| RuntimeError::external("Rust Texture Style carrier", error))?;
    xdremux_heif::validate_gain_map_structure(&textured).map_err(|error| {
        RuntimeError::external("Texture Style HDR structural validation", error)
    })?;

    let mut publisher = AtomicFilePublisher::new(output.to_path_buf());
    let published = publisher.publish_bytes(textured)?;
'''
if runtime.count(old_assembly) != 1:
    raise SystemExit("runtime: final assembly anchor changed")
runtime_path.write_text(runtime.replace(old_assembly, new_assembly, 1))

# The old integration probe assumed the Styles item was named styleMetadata and
# lived in idat. A Texture-positive carrier has two URI items named `metadata`;
# both are method-0 and the large one is the v16 Photographic Styles plist.
consumer_path = Path("scripts/check_rust_style_consumer.sh")
consumer = consumer_path.read_text()
old_reader = '''def read_style_plist(path, inspect_path):
    report = json.load(open(inspect_path, encoding="utf-8"))
    metadata = next(
        item
        for item in report["items"]
        if item.get("type") == "uri " and item.get("name") == "styleMetadata"
    )
    idat = next(child for child in report["meta_children"] if child["type"] == "idat")
    extent = metadata["location"]["extents"][0]
    with open(path, "rb") as stream:
        data = stream.read()
    start = idat["start"] + 8 + extent["offset"]
    payload = data[start : start + extent["length"]]
    return plistlib.loads(payload)
'''
new_reader = '''def read_style_plist(path, inspect_path):
    report = json.load(open(inspect_path, encoding="utf-8"))
    candidates = [
        item for item in report["items"]
        if item.get("type") == "uri " and item.get("name") == "metadata"
    ]
    if len(candidates) != 2:
        raise SystemExit(f"{path}: expected style+Texture metadata items, got {len(candidates)}")
    metadata = max(candidates, key=lambda item: item.get("payload_length") or 0)
    location = metadata["location"]
    if location.get("construction_method") != 0:
        raise SystemExit(f"{path}: Styles v16 metadata is not method-0: {location!r}")
    extent = location["extents"][0]
    with open(path, "rb") as stream:
        data = stream.read()
    start = extent["offset"]
    payload = data[start : start + extent["length"]]
    plist = plistlib.loads(payload)
    if plist.get("0") != 16 or plist.get("l") is not False:
        raise SystemExit(f"{path}: final Styles carrier is not v16+l=false")
    if b"tag:apple.com,2026:photo:metadata:texture_styles\\0" not in data:
        raise SystemExit(f"{path}: missing Texture Style metadata URI")
    roles = [
        b"semanticnosematte", b"semanticskinmattev2", b"semanticnonfaceskinmatte",
        b"semanticlipsmatte", b"semanticteethmattev2", b"semanticpersonmatte",
        b"semanticglassesmattev2", b"semanticeyebrowsmatte", b"semantictattoomatte",
        b"semantichandsmatte", b"semanticearsmatte", b"semanticfaceskinmatte",
    ]
    missing = [role.decode() for role in roles if role not in data]
    if missing:
        raise SystemExit(f"{path}: missing Texture Style semantic roles {missing}")
    return plist
'''
if consumer.count(old_reader) != 1:
    raise SystemExit("consumer gate: style plist reader anchor changed")
consumer_path.write_text(consumer.replace(old_reader, new_reader, 1))
