#!/usr/bin/env python3
"""Build PS3 TextureStyle candidates using the native CMPhoto custom-metadata layout.

Unlike the earlier V3 probe, this clones the structural contract of the existing
2023 style custom-metadata item: `uri ` infe v2/flags=1, the same cdsc targets,
and iloc construction method 1 with the payload stored inside meta/idat.
The existing 2023 style item and every pre-existing non-Exif item payload remain
byte-identical.
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
STYLE_URI_BYTES = b"tag:apple.com,2023:photo:metadata:styles\0"


def item_hashes(data, iloc, idat):
    return {
        e["id"]: hashlib.sha256(V3.item_payload(data, e, idat)).hexdigest()
        for e in iloc["entries"]
    }


def parse_uri_infe(raw):
    size = int.from_bytes(raw[:4], "big")
    if raw[4:8] != b"infe" or size != len(raw):
        raise ValueError("not an infe box")
    p = 8
    version = raw[p]
    flags = int.from_bytes(raw[p + 1:p + 4], "big")
    p += 4
    if version != 2:
        raise ValueError(f"expected infe v2, got {version}")
    item_id = int.from_bytes(raw[p:p + 2], "big")
    p += 2
    protection = int.from_bytes(raw[p:p + 2], "big")
    p += 2
    item_type = raw[p:p + 4]
    p += 4
    if item_type != b"uri ":
        raise ValueError(f"expected uri item, got {item_type!r}")
    end = raw.index(b"\0", p)
    name = raw[p:end].decode("utf-8")
    p = end + 1
    end = raw.index(b"\0", p)
    uri = raw[p:end].decode("utf-8")
    return {
        "version": version,
        "flags": flags,
        "itemID": item_id,
        "protection": protection,
        "itemType": "uri ",
        "name": name,
        "uri": uri,
    }


def make_cloned_uri_infe(template, item_id, name, uri):
    if template["version"] != 2 or item_id > 0xFFFF:
        raise ValueError("unsupported infe template")
    payload = bytes([2]) + int(template["flags"]).to_bytes(3, "big")
    payload += int(item_id).to_bytes(2, "big")
    payload += int(template["protection"]).to_bytes(2, "big")
    payload += b"uri "
    payload += name.encode() + b"\0" + uri.encode() + b"\0"
    return V3.make_box("infe", payload)


def build_one(source, output, maker84, texture_metadata, manifest_path=None):
    data = Path(source).read_bytes()
    top = list(V3.boxes(data, 0, len(data)))
    meta = next(b for b in top if b["type"] == "meta")
    children = list(V3.boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {b["type"]: b for b in children}
    iinf_version, infos = V3.parse_iinf(data, by_type["iinf"])
    iref_version, refs = V3.parse_iref(data, by_type["iref"])
    iloc = V3.parse_iloc(data, by_type["iloc"])
    idat = by_type["idat"]
    entry_map = {e["id"]: e for e in iloc["entries"]}
    item_types = {i["id"]: i["type"] for i in infos}
    exif_id = next(i for i, t in item_types.items() if t == "Exif")
    style_info = next(i for i in infos if i["type"] == "uri " and STYLE_URI_BYTES in i["raw"])
    style_id = style_info["id"]
    style_template = parse_uri_infe(style_info["raw"])
    style_entry = entry_map[style_id]
    if style_entry["method"] != 1 or (style_entry["construction_field"] & 0x0F) != 1:
        raise ValueError("baseline style custom metadata is not idat construction method 1")
    style_targets = next(r["to"] for r in refs if r["type"] == "cdsc" and r["from"] == style_id)

    old_hashes = item_hashes(data, iloc, idat)
    old_style_hash = old_hashes[style_id]
    exif_payload = V3.item_payload(data, entry_map[exif_id], idat)
    old_note, _ = V3.extract_makernote(exif_payload)
    _, old_note_entries = V3.parse_apple_makernote(old_note)
    old_tag84 = next(raw for tag, _, raw in old_note_entries if tag == 84)
    old_tag84_obj = plistlib.loads(old_tag84)

    merged84 = dict(old_tag84_obj)
    merged84.update(maker84)
    new_tag84 = plistlib.dumps(merged84, fmt=plistlib.FMT_BINARY, sort_keys=False)
    new_note = V3.rebuild_apple_makernote(old_note, new_tag84)
    new_exif = V3.replace_makernote(exif_payload, new_note)

    texture_payload = plistlib.dumps(texture_metadata, fmt=plistlib.FMT_BINARY, sort_keys=False)
    new_item_id = max(entry_map) + 1
    raw_infes = [i["raw"] for i in infos]
    raw_infes.append(make_cloned_uri_infe(style_template, new_item_id, "textureStyleMetadata", TEXTURE_URI))
    new_refs = [{"type": r["type"], "from": r["from"], "to": list(r["to"])} for r in refs]
    new_refs.append({"type": "cdsc", "from": new_item_id, "to": list(style_targets)})

    old_idat_payload = data[idat["data_start"]:idat["data_end"]]
    texture_idat_offset = len(old_idat_payload)
    new_idat = V3.make_box("idat", old_idat_payload + texture_payload)

    new_entries = [V3.clone_iloc_entry(e) for e in iloc["entries"]]
    new_entries.append({
        "id": new_item_id,
        "construction_field": 1,
        "method": 1,
        "data_ref": 0,
        "base_offset": 0,
        "extents": [{"index": None, "offset": texture_idat_offset, "length": len(texture_payload)}],
    })

    replacements = {
        "iinf": V3.make_iinf(iinf_version, raw_infes),
        "iref": V3.make_iref(iref_version, new_refs),
        "iloc": V3.make_iloc(iloc, new_entries),
        "idat": new_idat,
    }
    provisional_children = [
        replacements.get(c["type"], data[c["pos"]:c["pos"] + c["size"]]) for c in children
    ]
    provisional_meta = V3.make_box(
        "meta", data[meta["data_start"]:meta["data_start"] + 4] + b"".join(provisional_children)
    )
    delta = len(provisional_meta) - meta["size"]
    old_meta_end = meta["pos"] + meta["size"]

    # Relocate all pre-existing method-0 extents that live after the growing meta box.
    for e in new_entries:
        if e["id"] == new_item_id or e["method"] != 0:
            continue
        for ext in e["extents"]:
            absolute = e["base_offset"] + ext["offset"]
            if absolute >= old_meta_end:
                ext["offset"] += delta

    exif_entry = next(e for e in new_entries if e["id"] == exif_id)
    exif_entry["construction_field"] &= 0xFFF0
    exif_entry["method"] = 0
    exif_entry["data_ref"] = 0
    exif_entry["base_offset"] = 0
    exif_entry["extents"] = [{"index": None, "offset": len(data) + delta + 8, "length": len(new_exif)}]

    replacements["iloc"] = V3.make_iloc(iloc, new_entries)
    final_children = [
        replacements.get(c["type"], data[c["pos"]:c["pos"] + c["size"]]) for c in children
    ]
    final_meta = V3.make_box(
        "meta", data[meta["data_start"]:meta["data_start"] + 4] + b"".join(final_children)
    )
    if len(final_meta) != len(provisional_meta):
        raise ValueError("meta size changed after relocation")

    modified = data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:]
    modified += V3.make_box("mdat", new_exif)
    Path(output).write_bytes(modified)

    # Re-parse and prove the native carrier plus payload invariants.
    ntop = list(V3.boxes(modified, 0, len(modified)))
    nmeta = next(b for b in ntop if b["type"] == "meta")
    nch = list(V3.boxes(modified, nmeta["data_start"] + 4, nmeta["data_end"]))
    nbt = {b["type"]: b for b in nch}
    _, ninfos = V3.parse_iinf(modified, nbt["iinf"])
    nrefs_ver, nrefs = V3.parse_iref(modified, nbt["iref"])
    niloc = V3.parse_iloc(modified, nbt["iloc"])
    nidat = nbt["idat"]
    nmap = {e["id"]: e for e in niloc["entries"]}
    ninfo = next(i for i in ninfos if i["id"] == new_item_id)
    carrier = parse_uri_infe(ninfo["raw"])
    new_entry = nmap[new_item_id]
    roundtrip_payload = V3.item_payload(modified, new_entry, nidat)
    roundtrip_obj = plistlib.loads(roundtrip_payload)
    if carrier["uri"] != TEXTURE_URI or new_entry["method"] != 1:
        raise ValueError("TextureStyle custom metadata carrier mismatch")
    if roundtrip_obj != texture_metadata:
        raise ValueError("TextureStyle payload roundtrip mismatch")
    if next(r["to"] for r in nrefs if r["type"] == "cdsc" and r["from"] == new_item_id) != style_targets:
        raise ValueError("TextureStyle cdsc targets mismatch")

    new_hashes = item_hashes(modified, niloc, nidat)
    unexpected = [
        iid for iid, digest in old_hashes.items()
        if iid != exif_id and new_hashes.get(iid) != digest
    ]
    if unexpected:
        raise ValueError(f"pre-existing non-Exif item payloads changed: {unexpected}")
    if new_hashes.get(style_id) != old_style_hash:
        raise ValueError("existing 2023 style payload changed")

    manifest = {
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "output": str(output),
        "exifItemID": exif_id,
        "existingStyleItemID": style_id,
        "existingStylePayloadSHA256": old_style_hash,
        "newTextureItemID": new_item_id,
        "textureCarrier": carrier,
        "textureConstructionMethod": new_entry["method"],
        "textureIDATOffset": texture_idat_offset,
        "textureTargets": style_targets,
        "textureMetadata": roundtrip_obj,
        "maker84": merged84,
        "existing2023StylePayloadByteIdentical": True,
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
    p.add_argument("--preset", type=int, default=4)
    p.add_argument("--intensity", type=float, default=0.61)
    p.add_argument("--grain", type=float, default=0.73)
    p.add_argument("--people-version", type=int, default=1)
    p.add_argument("--film-grain-seed", type=int)
    args = p.parse_args()
    maker84 = {"8": args.preset, "9": args.intensity, "10": args.grain}
    texture = {"TextureStylePeopleDataVersion": args.people_version}
    if args.film_grain_seed is not None:
        texture["FilmGrainSeed"] = args.film_grain_seed
    build_one(args.source, args.output, maker84, texture, args.manifest)


if __name__ == "__main__":
    main()
