#!/usr/bin/env python3
"""Clean-room PS3 TextureStyle HEIF candidate builder.

This script intentionally uses only generalized container/metadata contracts:
- Apple MakerNote tag 84 numeric TextureStyle slots 8...12.
- `tag:apple.com,2026:photo:metadata:texture_styles` as a URI item.
- native-style URI carrier (`infe` v2, flags=1, name=`metadata`).
- method-0 payload storage with a `cdsc` association to the primary image,
  optionally also the existing tone-map item.

No private sample bytes, per-person records, masks, or image-derived payloads are
embedded or required.
"""

import argparse
import hashlib
import importlib.util
import json
import plistlib
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
TEXTURE_URI = "tag:apple.com,2026:photo:metadata:texture_styles"


def make_native_uri_infe(item_id, name, uri):
    if item_id > 0xFFFF:
        raise ValueError("native URI infe v2 item ID exceeds UInt16")
    payload = bytes([2, 0, 0, 1])
    payload += int(item_id).to_bytes(2, "big")
    payload += b"\0\0"
    payload += b"uri "
    payload += name.encode("utf-8") + b"\0"
    payload += uri.encode("utf-8") + b"\0"
    return V3.make_box("infe", payload)


def parse_pitm(data, box):
    pos = box["data_start"]
    version = data[pos]
    pos += 4
    return int.from_bytes(data[pos:pos + (4 if version else 2)], "big")


def item_hashes(data, iloc, idat):
    return {
        entry["id"]: hashlib.sha256(V3.item_payload(data, entry, idat)).hexdigest()
        for entry in iloc["entries"]
    }


def build_candidate(
    source,
    output,
    *,
    add_texture_item,
    targets_mode="primary",
    hardware_model="iPhone 18 Pro",
    capture_type="LF",
    capture_mode="Still",
    port_type="PortTypeBack",
    film_grain_seed=0,
    manifest_path=None,
):
    data = Path(source).read_bytes()
    top = list(V3.boxes(data, 0, len(data)))
    meta = next(b for b in top if b["type"] == "meta")
    children = list(V3.boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {b["type"]: b for b in children}

    iinf_version, infos = V3.parse_iinf(data, by_type["iinf"])
    iref_version, refs = V3.parse_iref(data, by_type["iref"])
    iloc = V3.parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    entry_map = {e["id"]: e for e in iloc["entries"]}
    item_types = {i["id"]: i["type"] for i in infos}
    exif_id = next(i for i, t in item_types.items() if t == "Exif")
    primary_id = parse_pitm(data, by_type["pitm"])
    tmap_id = next((i["id"] for i in infos if i["type"] == "tmap"), None)

    old_hashes = item_hashes(data, iloc, idat)
    exif_payload = V3.item_payload(data, entry_map[exif_id], idat)
    old_note, _ = V3.extract_makernote(exif_payload)
    _, old_note_entries = V3.parse_apple_makernote(old_note)
    old_tag84 = next(raw for tag, _, raw in old_note_entries if tag == 84)
    tag84 = dict(plistlib.loads(old_tag84))
    tag84.update({
        "8": 1,
        "9": 1.0,
        "10": 0.0,
        "11": False,
        "12": 1,
    })
    new_tag84 = plistlib.dumps(tag84, fmt=plistlib.FMT_BINARY, sort_keys=False)
    new_note = V3.rebuild_apple_makernote(old_note, new_tag84)
    new_exif = V3.replace_makernote(exif_payload, new_note)

    texture = {
        "Preset": "Standard",
        "CaptureType": capture_type,
        "CaptureMode": capture_mode,
        "PortType": port_type,
        "HardwareModel": hardware_model,
        "TextureStylePeopleDataVersion": 3,
        "FilmGrainSeed": int(film_grain_seed),
    }
    texture_payload = plistlib.dumps(texture, fmt=plistlib.FMT_BINARY, sort_keys=False)

    raw_infes = [i["raw"] for i in infos]
    new_refs = [{"type": r["type"], "from": r["from"], "to": list(r["to"])} for r in refs]
    new_entries = [V3.clone_iloc_entry(e) for e in iloc["entries"]]

    texture_id = None
    targets = []
    if add_texture_item:
        texture_id = max(entry_map) + 1
        raw_infes.append(make_native_uri_infe(texture_id, "metadata", TEXTURE_URI))
        targets = [primary_id]
        if targets_mode == "primary+tmap":
            if tmap_id is None:
                raise ValueError("primary+tmap requested but source has no tmap item")
            targets.append(tmap_id)
        elif targets_mode != "primary":
            raise ValueError(f"unsupported targets mode: {targets_mode}")
        new_refs.append({"type": "cdsc", "from": texture_id, "to": targets})
        new_entries.append({
            "id": texture_id,
            "construction_field": 0,
            "method": 0,
            "data_ref": 0,
            "base_offset": 0,
            "extents": [{"index": None, "offset": 0, "length": len(texture_payload)}],
        })

    replacements = {
        "iinf": V3.make_iinf(iinf_version, raw_infes),
        "iref": V3.make_iref(iref_version, new_refs),
        "iloc": V3.make_iloc(iloc, new_entries),
    }
    provisional_children = [
        replacements.get(c["type"], data[c["pos"]:c["pos"] + c["size"]])
        for c in children
    ]
    provisional_meta = V3.make_box(
        "meta",
        data[meta["data_start"]:meta["data_start"] + 4] + b"".join(provisional_children),
    )
    delta = len(provisional_meta) - meta["size"]
    old_meta_end = meta["pos"] + meta["size"]

    # Growing meta shifts every original method-0 extent located after it.
    for e in new_entries:
        if e["id"] == texture_id or e["method"] != 0:
            continue
        for ext in e["extents"]:
            absolute = e["base_offset"] + ext["offset"]
            if absolute >= old_meta_end:
                ext["offset"] += delta

    # Relocate Exif to a fresh trailing mdat rather than mutating image payloads.
    exif_entry = next(e for e in new_entries if e["id"] == exif_id)
    exif_entry["construction_field"] &= 0xFFF0
    exif_entry["method"] = 0
    exif_entry["data_ref"] = 0
    exif_entry["base_offset"] = 0

    trailing_payload = bytearray(new_exif)
    trailing_data_start = len(data) + delta + 8
    exif_entry["extents"] = [{
        "index": None,
        "offset": trailing_data_start,
        "length": len(new_exif),
    }]

    if texture_id is not None:
        texture_entry = next(e for e in new_entries if e["id"] == texture_id)
        texture_entry["extents"] = [{
            "index": None,
            "offset": trailing_data_start + len(trailing_payload),
            "length": len(texture_payload),
        }]
        trailing_payload += texture_payload

    replacements["iloc"] = V3.make_iloc(iloc, new_entries)
    final_children = [
        replacements.get(c["type"], data[c["pos"]:c["pos"] + c["size"]])
        for c in children
    ]
    final_meta = V3.make_box(
        "meta",
        data[meta["data_start"]:meta["data_start"] + 4] + b"".join(final_children),
    )
    if len(final_meta) != len(provisional_meta):
        raise ValueError("meta size changed after final iloc relocation")

    modified = data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:]
    modified += V3.make_box("mdat", bytes(trailing_payload))
    Path(output).write_bytes(modified)

    # Reparse and prove clean-room invariants.
    ntop = list(V3.boxes(modified, 0, len(modified)))
    nmeta = next(b for b in ntop if b["type"] == "meta")
    nch = list(V3.boxes(modified, nmeta["data_start"] + 4, nmeta["data_end"]))
    nbt = {b["type"]: b for b in nch}
    _, ninfos = V3.parse_iinf(modified, nbt["iinf"])
    _, nrefs = V3.parse_iref(modified, nbt["iref"])
    niloc = V3.parse_iloc(modified, nbt["iloc"])
    nidat = nbt.get("idat")
    nmap = {e["id"]: e for e in niloc["entries"]}

    new_hashes = item_hashes(modified, niloc, nidat)
    unexpected = [
        iid for iid, digest in old_hashes.items()
        if iid != exif_id and new_hashes.get(iid) != digest
    ]
    if unexpected:
        raise ValueError(f"pre-existing non-Exif item payloads changed: {unexpected}")

    roundtrip_texture = None
    if texture_id is not None:
        ninfo = next(i for i in ninfos if i["id"] == texture_id)
        if b"tag:apple.com,2026:photo:metadata:texture_styles\0" not in ninfo["raw"]:
            raise ValueError("TextureStyle URI missing after rewrite")
        if nmap[texture_id]["method"] != 0:
            raise ValueError("TextureStyle item is not method-0")
        roundtrip_texture = plistlib.loads(V3.item_payload(modified, nmap[texture_id], nidat))
        if roundtrip_texture != texture:
            raise ValueError("TextureStyle payload roundtrip mismatch")
        got_targets = next(
            r["to"] for r in nrefs
            if r["type"] == "cdsc" and r["from"] == texture_id
        )
        if got_targets != targets:
            raise ValueError(f"TextureStyle targets mismatch: {got_targets} != {targets}")

    manifest = {
        "schema": "xdremux-texture-style-cleanroom-v1",
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "output": str(output),
        "exifItemID": exif_id,
        "primaryItemID": primary_id,
        "toneMapItemID": tmap_id,
        "textureItemAdded": bool(add_texture_item),
        "textureItemID": texture_id,
        "textureTargetsMode": targets_mode if add_texture_item else None,
        "textureTargets": targets,
        "textureMetadata": roundtrip_texture,
        "maker84TextureSlots": {k: tag84[k] for k in ("8", "9", "10", "11", "12")},
        "preExistingNonExifPayloadsByteIdentical": True,
    }
    if manifest_path:
        Path(manifest_path).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return manifest


def main():
    p = argparse.ArgumentParser()
    p.add_argument("source")
    p.add_argument("output")
    p.add_argument("--manifest")
    p.add_argument("--mode", choices=("tag84-only", "primary", "primary+tmap"), default="primary")
    p.add_argument("--hardware-model", default="iPhone 18 Pro")
    p.add_argument("--capture-type", default="LF")
    p.add_argument("--capture-mode", default="Still")
    p.add_argument("--port-type", default="PortTypeBack")
    p.add_argument("--film-grain-seed", type=int, default=0)
    args = p.parse_args()
    build_candidate(
        args.source,
        args.output,
        add_texture_item=args.mode != "tag84-only",
        targets_mode="primary+tmap" if args.mode == "primary+tmap" else "primary",
        hardware_model=args.hardware_model,
        capture_type=args.capture_type,
        capture_mode=args.capture_mode,
        port_type=args.port_type,
        film_grain_seed=args.film_grain_seed,
        manifest_path=args.manifest,
    )


if __name__ == "__main__":
    main()
