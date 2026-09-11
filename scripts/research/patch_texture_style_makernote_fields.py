#!/usr/bin/env python3
"""Patch explicit Apple MakerNote TIFF fields inside a HEIC Exif item.

This is intentionally lower level than the earlier TextureStyle probes.  It does
not assume Texture Style lives in SmartStyle tag 84 or in another bplist
container.  A contract supplies the exact MakerNote tag ID, TIFF type and value.
All pre-existing MakerNote payload bytes are preserved as values, and the HEIF
writer redirects only the Exif item to a newly appended mdat payload.
"""

import argparse
import hashlib
import importlib.util
import json
import math
import struct
import sys
from pathlib import Path


def load_v3():
    path = Path(__file__).with_name("patch_texture_style_heif_v3.py")
    spec = importlib.util.spec_from_file_location("texture_v3", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


V3 = load_v3()
TYPE_SIZES = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8, 11: 4, 12: 8}


def endian_prefix(endian):
    return ">" if endian == "big" else "<"


def encode_value(field_type, value, endian):
    p = endian_prefix(endian)
    if field_type == 1:  # BYTE
        values = value if isinstance(value, list) else [value]
        return bytes(int(v) & 0xFF for v in values)
    if field_type == 2:  # ASCII
        raw = str(value).encode("ascii")
        return raw if raw.endswith(b"\0") else raw + b"\0"
    if field_type == 3:  # SHORT
        values = value if isinstance(value, list) else [value]
        return b"".join(struct.pack(p + "H", int(v)) for v in values)
    if field_type == 4:  # LONG
        values = value if isinstance(value, list) else [value]
        return b"".join(struct.pack(p + "I", int(v)) for v in values)
    if field_type == 5:  # RATIONAL
        values = value if isinstance(value, list) and value and isinstance(value[0], list) else [value]
        out = bytearray()
        for pair in values:
            if not isinstance(pair, (list, tuple)) or len(pair) != 2:
                raise ValueError("RATIONAL value must be [numerator, denominator] or a list of pairs")
            out += struct.pack(p + "II", int(pair[0]), int(pair[1]))
        return bytes(out)
    if field_type == 7:  # UNDEFINED
        if isinstance(value, str):
            if value.startswith("hex:"):
                return bytes.fromhex(value[4:])
            return value.encode("utf-8")
        if isinstance(value, list):
            return bytes(int(v) & 0xFF for v in value)
        raise ValueError("UNDEFINED value must be hex:string, string, or byte list")
    if field_type == 9:  # SLONG
        values = value if isinstance(value, list) else [value]
        return b"".join(struct.pack(p + "i", int(v)) for v in values)
    if field_type == 10:  # SRATIONAL
        values = value if isinstance(value, list) and value and isinstance(value[0], list) else [value]
        out = bytearray()
        for pair in values:
            if not isinstance(pair, (list, tuple)) or len(pair) != 2:
                raise ValueError("SRATIONAL value must be [numerator, denominator] or a list of pairs")
            out += struct.pack(p + "ii", int(pair[0]), int(pair[1]))
        return bytes(out)
    if field_type == 11:  # FLOAT
        values = value if isinstance(value, list) else [value]
        return b"".join(struct.pack(p + "f", float(v)) for v in values)
    if field_type == 12:  # DOUBLE
        values = value if isinstance(value, list) else [value]
        return b"".join(struct.pack(p + "d", float(v)) for v in values)
    raise ValueError(f"unsupported TIFF field type {field_type}")


def rebuild_makernote(note, changes):
    endian, parsed = V3.parse_apple_makernote(note)
    entries = {tag: (field_type, raw) for tag, field_type, raw in parsed}

    for change in changes:
        tag = int(change["tag"])
        if change.get("remove"):
            entries.pop(tag, None)
            continue
        field_type = int(change["type"])
        if field_type not in TYPE_SIZES:
            raise ValueError(f"tag {tag}: unsupported TIFF type {field_type}")
        if "rawHex" in change:
            raw = bytes.fromhex(change["rawHex"])
        else:
            raw = encode_value(field_type, change["value"], endian)
        unit = TYPE_SIZES[field_type]
        if len(raw) == 0 or len(raw) % unit:
            raise ValueError(f"tag {tag}: payload length {len(raw)} is invalid for TIFF type {field_type}")
        entries[tag] = (field_type, raw)

    ordered = [(tag, *entries[tag]) for tag in sorted(entries)]
    data_start = 14 + 2 + 12 * len(ordered) + 4
    table = bytearray()
    values = bytearray()

    for tag, field_type, raw in ordered:
        unit = TYPE_SIZES[field_type]
        count = len(raw) // unit
        table += int(tag).to_bytes(2, endian)
        table += int(field_type).to_bytes(2, endian)
        table += int(count).to_bytes(4, endian)
        if len(raw) <= 4:
            table += raw + b"\0" * (4 - len(raw))
        else:
            if (data_start + len(values)) & 1:
                values += b"\0"
            table += (data_start + len(values)).to_bytes(4, endian)
            values += raw

    return note[:14] + len(ordered).to_bytes(2, endian) + bytes(table) + b"\0\0\0\0" + bytes(values)


def item_hashes(data, iloc, idat):
    return {
        entry["id"]: hashlib.sha256(V3.item_payload(data, entry, idat)).hexdigest()
        for entry in iloc["entries"]
    }


def patch_heic(source, output, contract):
    data = Path(source).read_bytes()
    top = list(V3.boxes(data, 0, len(data)))
    meta = next(box for box in top if box["type"] == "meta")
    children = list(V3.boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {box["type"]: box for box in children}
    _, infos = V3.parse_iinf(data, by_type["iinf"])
    iloc = V3.parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    entry_map = {entry["id"]: entry for entry in iloc["entries"]}
    exif_id = next(info["id"] for info in infos if info["type"] == "Exif")
    exif_payload = V3.item_payload(data, entry_map[exif_id], idat)
    old_note, _ = V3.extract_makernote(exif_payload)
    old_endian, old_entries = V3.parse_apple_makernote(old_note)
    before = {tag: {"type": field_type, "sha256": hashlib.sha256(raw).hexdigest(), "length": len(raw)} for tag, field_type, raw in old_entries}
    old_hashes = item_hashes(data, iloc, idat)

    changes = contract["fields"]
    new_note = rebuild_makernote(old_note, changes)
    new_exif = V3.replace_makernote(exif_payload, new_note)

    new_entries = [V3.clone_iloc_entry(entry) for entry in iloc["entries"]]
    exif_entry = next(entry for entry in new_entries if entry["id"] == exif_id)
    exif_entry["construction_field"] &= 0xFFF0
    exif_entry["method"] = 0
    exif_entry["data_ref"] = 0
    exif_entry["base_offset"] = 0
    # meta size is unchanged: only an existing iloc entry's numeric fields change.
    new_exif_offset = len(data) + 8
    exif_entry["extents"] = [{"index": None, "offset": new_exif_offset, "length": len(new_exif)}]

    replacement_iloc = V3.make_iloc(iloc, new_entries)
    old_iloc = by_type["iloc"]
    if len(replacement_iloc) != old_iloc["size"]:
        raise ValueError("iloc size changed; field-level patcher requires fixed-size iloc encoding")
    new_children = [
        replacement_iloc if child["type"] == "iloc" else data[child["pos"]:child["pos"] + child["size"]]
        for child in children
    ]
    new_meta = V3.make_box("meta", data[meta["data_start"]:meta["data_start"] + 4] + b"".join(new_children))
    if len(new_meta) != meta["size"]:
        raise ValueError("meta size changed unexpectedly")

    modified = data[:meta["pos"]] + new_meta + data[meta["pos"] + meta["size"]:] + V3.make_box("mdat", new_exif)
    Path(output).write_bytes(modified)

    # Re-parse final file and prove only Exif payload changed among pre-existing items.
    new_top = list(V3.boxes(modified, 0, len(modified)))
    new_meta_box = next(box for box in new_top if box["type"] == "meta")
    new_children2 = list(V3.boxes(modified, new_meta_box["data_start"] + 4, new_meta_box["data_end"]))
    new_by_type = {box["type"]: box for box in new_children2}
    new_iloc = V3.parse_iloc(modified, new_by_type["iloc"])
    new_hashes = item_hashes(modified, new_iloc, new_by_type.get("idat"))
    unexpected = [item_id for item_id, digest in old_hashes.items() if item_id != exif_id and new_hashes.get(item_id) != digest]
    if unexpected:
        raise ValueError(f"non-Exif item payloads changed: {unexpected}")

    final_entry = next(entry for entry in new_iloc["entries"] if entry["id"] == exif_id)
    final_exif = V3.item_payload(modified, final_entry, new_by_type.get("idat"))
    final_note, _ = V3.extract_makernote(final_exif)
    final_endian, final_entries = V3.parse_apple_makernote(final_note)
    after = {tag: {"type": field_type, "sha256": hashlib.sha256(raw).hexdigest(), "length": len(raw), "rawHex": raw.hex()} for tag, field_type, raw in final_entries}

    unchanged_originals = {}
    changed_tags = {int(x["tag"]) for x in changes}
    for tag, info in before.items():
        if tag not in changed_tags:
            unchanged_originals[str(tag)] = after.get(tag, {}).get("sha256") == info["sha256"] and after.get(tag, {}).get("type") == info["type"]
    if not all(unchanged_originals.values()):
        raise ValueError(f"pre-existing MakerNote value changed unexpectedly: {unchanged_originals}")

    manifest = {
        "source": str(source),
        "output": str(output),
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "exifItemID": exif_id,
        "makerNoteEndianBefore": old_endian,
        "makerNoteEndianAfter": final_endian,
        "fields": changes,
        "before": {str(k): v for k, v in before.items()},
        "after": {str(k): v for k, v in after.items()},
        "unchangedOriginalMakerNoteValues": unchanged_originals,
        "nonExifPayloadInvariant": True,
    }
    return manifest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source")
    parser.add_argument("contract")
    parser.add_argument("output")
    parser.add_argument("--manifest")
    args = parser.parse_args()
    contract = json.loads(Path(args.contract).read_text())
    manifest = patch_heic(args.source, args.output, contract)
    text = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    if args.manifest:
        Path(args.manifest).write_text(text)
    sys.stdout.write(text)


if __name__ == "__main__":
    main()
