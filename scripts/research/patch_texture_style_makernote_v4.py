#!/usr/bin/env python3
import argparse
import hashlib
import json
import plistlib
from pathlib import Path

import patch_texture_style_heif_v3 as heif


def rebuild_note_with_tag(note, tag_id, payload, field_type=7, replace=False):
    endian, entries = heif.parse_apple_makernote(note)
    if any(tag == tag_id for tag, _, _ in entries) and not replace:
        raise ValueError(f"MakerNote tag {tag_id} already exists")
    out = []
    seen = False
    for tag, typ, raw in entries:
        if tag == tag_id:
            out.append((tag_id, field_type, payload))
            seen = True
        else:
            out.append((tag, typ, raw))
    if not seen:
        out.append((tag_id, field_type, payload))
    out.sort(key=lambda item: item[0])

    type_sizes = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8, 12: 8}
    base_offset = 14 + 2 + 12 * len(out) + 4
    table = bytearray()
    values = bytearray()
    for tag, typ, raw in out:
        unit = type_sizes.get(typ, 1)
        if len(raw) % unit:
            raise ValueError(f"tag {tag}: payload is not aligned to TIFF type {typ}")
        count = len(raw) // unit
        table += tag.to_bytes(2, endian)
        table += typ.to_bytes(2, endian)
        table += count.to_bytes(4, endian)
        if len(raw) <= 4:
            table += raw + b"\0" * (4 - len(raw))
        else:
            table += (base_offset + len(values)).to_bytes(4, endian)
            values += raw
    return note[:14] + len(out).to_bytes(2, endian) + bytes(table) + b"\0\0\0\0" + bytes(values)


def patch_one(source, output, tag_id, preset, intensity, grain, replace=False):
    data = Path(source).read_bytes()
    top = list(heif.boxes(data, 0, len(data)))
    meta = next(box for box in top if box["type"] == "meta")
    children = list(heif.boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {box["type"]: box for box in children}
    iloc = heif.parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    _, infos = heif.parse_iinf(data, by_type["iinf"])
    item_types = {info["id"]: info["type"] for info in infos}
    exif_id = next(item_id for item_id, typ in item_types.items() if typ == "Exif")
    entry_map = {entry["id"]: entry for entry in iloc["entries"]}
    exif_payload = heif.item_payload(data, entry_map[exif_id], idat)
    old_note, _ = heif.extract_makernote(exif_payload)
    _, old_entries = heif.parse_apple_makernote(old_note)
    old_tag84 = next((raw for tag, _, raw in old_entries if tag == 84), None)

    texture = {
        "TextureStylePreset": int(preset),
        "TextureStyleIntensity": float(intensity),
        "TextureStyleGrain": float(grain),
    }
    texture_payload = plistlib.dumps(texture, fmt=plistlib.FMT_BINARY, sort_keys=False)
    new_note = rebuild_note_with_tag(old_note, tag_id, texture_payload, 7, replace=replace)
    new_exif = heif.replace_makernote(exif_payload, new_note)

    old_hashes = {
        entry["id"]: hashlib.sha256(heif.item_payload(data, entry, idat)).hexdigest()
        for entry in iloc["entries"]
    }
    new_entries = [heif.clone_iloc_entry(entry) for entry in iloc["entries"]]
    exif_entry = next(entry for entry in new_entries if entry["id"] == exif_id)
    exif_entry["construction_field"] &= 0xFFF0
    exif_entry["method"] = 0
    exif_entry["data_ref"] = 0
    exif_entry["base_offset"] = 0
    exif_entry["extents"] = [{"index": None, "offset": 0, "length": len(new_exif)}]

    provisional_iloc = heif.make_iloc(iloc, new_entries)
    provisional_children = [
        provisional_iloc if child["type"] == "iloc" else data[child["pos"]:child["pos"] + child["size"]]
        for child in children
    ]
    provisional_meta = heif.make_box(
        "meta",
        data[meta["data_start"]:meta["data_start"] + 4] + b"".join(provisional_children),
    )
    delta = len(provisional_meta) - meta["size"]
    old_meta_end = meta["pos"] + meta["size"]

    for entry in new_entries:
        if entry["id"] == exif_id or entry["method"] != 0:
            continue
        for extent in entry["extents"]:
            absolute = entry["base_offset"] + extent["offset"]
            if absolute >= old_meta_end:
                extent["offset"] += delta

    exif_entry["extents"][0]["offset"] = len(data) + delta + 8
    final_iloc = heif.make_iloc(iloc, new_entries)
    final_children = [
        final_iloc if child["type"] == "iloc" else data[child["pos"]:child["pos"] + child["size"]]
        for child in children
    ]
    final_meta = heif.make_box(
        "meta",
        data[meta["data_start"]:meta["data_start"] + 4] + b"".join(final_children),
    )
    if len(final_meta) != len(provisional_meta):
        raise ValueError("meta size changed after relocation")

    modified = data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:]
    modified += heif.make_box("mdat", new_exif)
    Path(output).write_bytes(modified)

    # Structural invariants: all pre-existing non-Exif HEIF item payloads remain byte-identical.
    new_top = list(heif.boxes(modified, 0, len(modified)))
    new_meta = next(box for box in new_top if box["type"] == "meta")
    new_children = list(heif.boxes(modified, new_meta["data_start"] + 4, new_meta["data_end"]))
    new_by_type = {box["type"]: box for box in new_children}
    new_iloc = heif.parse_iloc(modified, new_by_type["iloc"])
    new_idat = new_by_type.get("idat")
    new_hashes = {
        entry["id"]: hashlib.sha256(heif.item_payload(modified, entry, new_idat)).hexdigest()
        for entry in new_iloc["entries"]
    }
    changed = [
        item_id for item_id, digest in old_hashes.items()
        if item_id != exif_id and new_hashes.get(item_id) != digest
    ]
    if changed:
        raise ValueError(f"non-Exif payloads changed: {changed}")

    new_entry_map = {entry["id"]: entry for entry in new_iloc["entries"]}
    new_exif_payload = heif.item_payload(modified, new_entry_map[exif_id], new_idat)
    verify_note, _ = heif.extract_makernote(new_exif_payload)
    _, verify_entries = heif.parse_apple_makernote(verify_note)
    verify_payload = next(raw for tag, _, raw in verify_entries if tag == tag_id)
    verify_object = plistlib.loads(verify_payload)
    verify_tag84 = next((raw for tag, _, raw in verify_entries if tag == 84), None)
    if old_tag84 != verify_tag84:
        raise ValueError("tag84 changed byte-for-byte")

    result = {
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "output": str(output),
        "exifItemID": exif_id,
        "makerNoteTag": tag_id,
        "makerNoteType": 7,
        "textureObject": verify_object,
        "tag84ByteIdentical": old_tag84 == verify_tag84,
        "nonExifPayloadsByteIdentical": True,
        "addedStandaloneTextureURIItem": False,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return result


def main():
    p = argparse.ArgumentParser()
    p.add_argument("source")
    p.add_argument("output")
    p.add_argument("--tag-id", type=int, required=True)
    p.add_argument("--preset", type=int, default=2)
    p.add_argument("--intensity", type=float, default=0.37)
    p.add_argument("--grain", type=float, default=0.63)
    p.add_argument("--replace", action="store_true")
    args = p.parse_args()
    patch_one(args.source, args.output, args.tag_id, args.preset, args.intensity, args.grain, args.replace)


if __name__ == "__main__":
    main()
