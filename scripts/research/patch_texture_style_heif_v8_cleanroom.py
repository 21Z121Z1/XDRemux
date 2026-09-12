#!/usr/bin/env python3
"""Clean-room PS3 TextureStyle coherent-contract candidate builder.

This builder encodes only generalized structural contracts. It does not embed
private sample bytes or image-derived people data.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import plistlib
from pathlib import Path

STYLE_URI = "tag:apple.com,2023:photo:metadata:styles"
TEXTURE_URI = "tag:apple.com,2026:photo:metadata:texture_styles"


def load_v3():
    path = Path(__file__).with_name("patch_texture_style_heif_v3.py")
    spec = importlib.util.spec_from_file_location("texture_v3", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


V3 = load_v3()


def make_uri_infe(item_id, name, uri):
    if item_id > 0xFFFF:
        raise ValueError("URI infe v2 item ID exceeds UInt16")
    payload = bytes([2, 0, 0, 1])
    payload += int(item_id).to_bytes(2, "big") + b"\0\0" + b"uri "
    payload += name.encode("utf-8") + b"\0" + uri.encode("utf-8") + b"\0"
    return V3.make_box("infe", payload)


def uri_infe_fields(raw):
    # infe box header + FullBox(v2/flags) + itemID + protection + itemType
    if len(raw) < 20 or raw[4:8] != b"infe" or raw[8] != 2 or raw[16:20] != b"uri ":
        raise ValueError("unexpected URI infe layout")
    pos = 20
    end = raw.find(b"\0", pos)
    if end < 0:
        raise ValueError("URI infe item name is not terminated")
    name = raw[pos:end].decode("utf-8", "replace")
    pos = end + 1
    end = raw.find(b"\0", pos)
    if end < 0:
        raise ValueError("URI infe URI is not terminated")
    return name, raw[pos:end].decode("utf-8", "replace")


def parse_pitm(data, box):
    pos = box["data_start"]
    version = data[pos]
    pos += 4
    return int.from_bytes(data[pos:pos + (4 if version else 2)], "big")


def item_hashes(data, iloc, idat):
    return {
        e["id"]: hashlib.sha256(V3.item_payload(data, e, idat)).hexdigest()
        for e in iloc["entries"]
    }


def parse_apple_tag84(note):
    if not note.startswith(b"Apple iOS\0\0\x01"):
        raise ValueError("unexpected Apple MakerNote header")
    endian = "big" if note[12:14] == b"MM" else "little"
    count = int.from_bytes(note[14:16], endian)
    sizes = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8, 12: 8}
    for index in range(count):
        pos = 16 + index * 12
        tag = int.from_bytes(note[pos:pos + 2], endian)
        field_type = int.from_bytes(note[pos + 2:pos + 4], endian)
        item_count = int.from_bytes(note[pos + 4:pos + 8], endian)
        size = sizes.get(field_type, 1) * item_count
        offset = pos + 8 if size <= 4 else int.from_bytes(note[pos + 8:pos + 12], endian)
        if tag == 84:
            return {
                "endian": endian,
                "entryPos": pos,
                "fieldType": field_type,
                "offset": offset,
                "size": size,
                "raw": note[offset:offset + size],
            }
    raise ValueError("Apple MakerNote tag 84 is missing")


def surgical_replace_tag84(note, payload):
    info = parse_apple_tag84(note)
    if info["fieldType"] != 7:
        raise ValueError(f"unexpected tag84 TIFF type {info['fieldType']}")
    endian = info["endian"]
    entry = info["entryPos"]
    old_start = info["offset"]
    old_end = old_start + info["size"]
    out = bytearray(note)
    if old_end == len(note):
        out = out[:old_start] + payload
        new_offset = old_start
        mode = "replace-tail"
    else:
        new_offset = len(out)
        out.extend(payload)
        mode = "append-retarget"
    out[entry + 2:entry + 4] = (7).to_bytes(2, endian)
    out[entry + 4:entry + 8] = len(payload).to_bytes(4, endian)
    out[entry + 8:entry + 12] = new_offset.to_bytes(4, endian)
    return bytes(out), {
        "mode": mode,
        "oldOffset": old_start,
        "oldLength": info["size"],
        "newOffset": new_offset,
        "newLength": len(payload),
        "unrelatedPrefixByteIdentical": bytes(out[:old_start]) == note[:old_start],
    }


MODES = {
    "maker-surgical": dict(patch_maker=True, upgrade_style=False, relocate_style=False,
                            style_name=None, add_texture=False),
    "style-v16": dict(patch_maker=False, upgrade_style=True, relocate_style=True,
                       style_name="metadata", add_texture=False),
    "style-v16-maker": dict(patch_maker=True, upgrade_style=True, relocate_style=True,
                             style_name="metadata", add_texture=False),
    "full-v16": dict(patch_maker=True, upgrade_style=True, relocate_style=True,
                      style_name="metadata", add_texture=True),
    "full-v16-legacy-name": dict(patch_maker=True, upgrade_style=True, relocate_style=True,
                                  style_name="styleMetadata", add_texture=True),
    "full-carrier-v15": dict(patch_maker=True, upgrade_style=False, relocate_style=True,
                              style_name="metadata", add_texture=True),
}


def build_candidate(source, output, *, mode, manifest_path=None,
                    hardware_model="iPhone19,2", capture_type="LF",
                    capture_mode="Still", port_type="PortTypeBack",
                    film_grain_seed=0):
    cfg = MODES[mode]
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
    style_info = next(
        i for i in infos
        if i["type"] == "uri " and (STYLE_URI.encode() + b"\0") in i["raw"]
    )
    style_id = style_info["id"]
    style_name_before, style_uri = uri_infe_fields(style_info["raw"])
    if style_uri != STYLE_URI:
        raise ValueError("unexpected style URI")
    style_targets = next(
        r["to"] for r in refs if r["type"] == "cdsc" and r["from"] == style_id
    )

    old_hashes = item_hashes(data, iloc, idat)
    old_style_payload = V3.item_payload(data, entry_map[style_id], idat)
    old_style = plistlib.loads(old_style_payload)
    old_style_version = old_style.get("0")
    style = dict(old_style)
    if cfg["upgrade_style"]:
        if old_style_version != 15:
            raise ValueError(f"expected style schema 15, got {old_style_version!r}")
        style["0"] = 16
        style["l"] = False
    new_style_payload = plistlib.dumps(style, fmt=plistlib.FMT_BINARY, sort_keys=False)

    exif_payload = V3.item_payload(data, entry_map[exif_id], idat)
    old_note, _ = V3.extract_makernote(exif_payload)
    tag84_info = parse_apple_tag84(old_note)
    tag84 = dict(plistlib.loads(tag84_info["raw"]))
    old_tag84_types = {k: type(tag84.get(k)).__name__ for k in sorted(tag84)}
    maker_surgery = None
    new_exif = exif_payload
    if cfg["patch_maker"]:
        for key in ("1", "2", "3"):
            if key in tag84:
                tag84[key] = float(tag84[key])
        tag84.update({"8": 1, "9": 1.0, "10": 0.0, "11": False, "12": 1})
        new_tag84 = plistlib.dumps(tag84, fmt=plistlib.FMT_BINARY, sort_keys=False)
        new_note, maker_surgery = surgical_replace_tag84(old_note, new_tag84)
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

    raw_infes = []
    for info in infos:
        if info["id"] == style_id and cfg["style_name"] is not None:
            raw_infes.append(make_uri_infe(style_id, cfg["style_name"], STYLE_URI))
        else:
            raw_infes.append(info["raw"])

    new_refs = [{"type": r["type"], "from": r["from"], "to": list(r["to"])} for r in refs]
    new_entries = [V3.clone_iloc_entry(e) for e in iloc["entries"]]
    texture_id = None
    if cfg["add_texture"]:
        texture_id = max(entry_map) + 1
        raw_infes.append(make_uri_infe(texture_id, "metadata", TEXTURE_URI))
        new_refs.append({"type": "cdsc", "from": texture_id, "to": list(style_targets)})
        new_entries.append({
            "id": texture_id,
            "construction_field": 0,
            "method": 0,
            "data_ref": 0,
            "base_offset": 0,
            "extents": [{"index": None, "offset": 0, "length": len(texture_payload)}],
        })

    relocate_ids = {exif_id} if cfg["patch_maker"] else set()
    if cfg["relocate_style"]:
        relocate_ids.add(style_id)
    if texture_id is not None:
        relocate_ids.add(texture_id)
    for e in new_entries:
        if e["id"] in relocate_ids:
            e["construction_field"] &= 0xFFF0
            e["method"] = 0
            e["data_ref"] = 0
            e["base_offset"] = 0

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
        "meta", data[meta["data_start"]:meta["data_start"] + 4] + b"".join(provisional_children)
    )
    delta = len(provisional_meta) - meta["size"]
    old_meta_end = meta["pos"] + meta["size"]
    for e in new_entries:
        if e["id"] in relocate_ids or e["method"] != 0:
            continue
        for ext in e["extents"]:
            absolute = e["base_offset"] + ext["offset"]
            if absolute >= old_meta_end:
                ext["offset"] += delta

    trailing = bytearray()
    trailing_data_start = len(data) + delta + 8

    def append_relocated(item_id, payload):
        e = next(x for x in new_entries if x["id"] == item_id)
        e["extents"] = [{
            "index": None,
            "offset": trailing_data_start + len(trailing),
            "length": len(payload),
        }]
        trailing.extend(payload)

    if cfg["patch_maker"]:
        append_relocated(exif_id, new_exif)
    if cfg["relocate_style"]:
        append_relocated(style_id, new_style_payload)
    if texture_id is not None:
        append_relocated(texture_id, texture_payload)

    replacements["iloc"] = V3.make_iloc(iloc, new_entries)
    final_children = [
        replacements.get(c["type"], data[c["pos"]:c["pos"] + c["size"]])
        for c in children
    ]
    final_meta = V3.make_box(
        "meta", data[meta["data_start"]:meta["data_start"] + 4] + b"".join(final_children)
    )
    if len(final_meta) != len(provisional_meta):
        raise ValueError("meta size changed after final iloc relocation")
    modified = data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:]
    if trailing:
        modified += V3.make_box("mdat", bytes(trailing))
    Path(output).write_bytes(modified)

    # Reparse and prove structural + payload invariants.
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
    deliberately_changed = set()
    if cfg["patch_maker"]:
        deliberately_changed.add(exif_id)
    if cfg["relocate_style"]:
        deliberately_changed.add(style_id)
    unexpected = [
        iid for iid, digest in old_hashes.items()
        if iid not in deliberately_changed and new_hashes.get(iid) != digest
    ]
    if unexpected:
        raise ValueError(f"unrelated pre-existing item payloads changed: {unexpected}")

    nstyle_info = next(i for i in ninfos if i["id"] == style_id)
    nstyle_name, nstyle_uri = uri_infe_fields(nstyle_info["raw"])
    if nstyle_uri != STYLE_URI:
        raise ValueError("style URI changed")
    nstyle = plistlib.loads(V3.item_payload(modified, nmap[style_id], nidat))
    if cfg["upgrade_style"] and (nstyle.get("0") != 16 or nstyle.get("l") is not False):
        raise ValueError("style v16 roundtrip failed")
    if cfg["style_name"] is not None and nstyle_name != cfg["style_name"]:
        raise ValueError("style item name roundtrip failed")
    if cfg["relocate_style"] and nmap[style_id]["method"] != 0:
        raise ValueError("style item was not relocated to method-0")

    roundtrip_tag84 = None
    if cfg["patch_maker"]:
        nexif = V3.item_payload(modified, nmap[exif_id], nidat)
        nnote, _ = V3.extract_makernote(nexif)
        roundtrip_tag84 = plistlib.loads(parse_apple_tag84(nnote)["raw"])
        expected = {"8": 1, "9": 1.0, "10": 0.0, "11": False, "12": 1}
        if any(roundtrip_tag84.get(k) != v for k, v in expected.items()):
            raise ValueError("MakerNote TextureStyle slot roundtrip failed")
        for key in ("1", "2", "3"):
            if key in roundtrip_tag84 and not isinstance(roundtrip_tag84[key], float):
                raise ValueError(f"MakerNote slot {key} did not normalize to float")

    roundtrip_texture = None
    if texture_id is not None:
        ntex_info = next(i for i in ninfos if i["id"] == texture_id)
        tex_name, tex_uri = uri_infe_fields(ntex_info["raw"])
        if tex_name != "metadata" or tex_uri != TEXTURE_URI:
            raise ValueError("TextureStyle infe roundtrip failed")
        if nmap[texture_id]["method"] != 0:
            raise ValueError("TextureStyle item is not method-0")
        roundtrip_texture = plistlib.loads(V3.item_payload(modified, nmap[texture_id], nidat))
        if roundtrip_texture != texture:
            raise ValueError("TextureStyle payload roundtrip mismatch")
        tex_targets = next(r["to"] for r in nrefs if r["type"] == "cdsc" and r["from"] == texture_id)
        if tex_targets != style_targets:
            raise ValueError("TextureStyle targets do not mirror the 2023 style item")

    manifest = {
        "schema": "xdremux-texture-style-cleanroom-v2",
        "mode": mode,
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "output": str(output),
        "primaryItemID": primary_id,
        "exifItemID": exif_id,
        "styleItemID": style_id,
        "styleTargets": style_targets,
        "styleBefore": {
            "schemaVersion": old_style_version,
            "itemName": style_name_before,
            "constructionMethod": entry_map[style_id]["method"],
            "hasL": "l" in old_style,
        },
        "styleAfter": {
            "schemaVersion": nstyle.get("0"),
            "itemName": nstyle_name,
            "constructionMethod": nmap[style_id]["method"],
            "l": nstyle.get("l", "<absent>"),
        },
        "makerPatched": cfg["patch_maker"],
        "makerSurgery": maker_surgery,
        "maker84TypesBefore": old_tag84_types,
        "maker84TypesAfter": ({k: type(roundtrip_tag84.get(k)).__name__ for k in sorted(roundtrip_tag84)}
                               if roundtrip_tag84 is not None else None),
        "maker84TextureSlots": ({k: roundtrip_tag84[k] for k in ("8", "9", "10", "11", "12")}
                                if roundtrip_tag84 is not None else None),
        "textureItemAdded": texture_id is not None,
        "textureItemID": texture_id,
        "textureMetadata": roundtrip_texture,
        "textureTargetsMirrorStyle": texture_id is None or True,
        "preExistingUnrelatedPayloadsByteIdentical": True,
    }
    if manifest_path:
        Path(manifest_path).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return manifest


def main():
    p = argparse.ArgumentParser()
    p.add_argument("source")
    p.add_argument("output")
    p.add_argument("--mode", choices=tuple(MODES), required=True)
    p.add_argument("--manifest")
    p.add_argument("--hardware-model", default="iPhone19,2")
    p.add_argument("--capture-type", default="LF")
    p.add_argument("--capture-mode", default="Still")
    p.add_argument("--port-type", default="PortTypeBack")
    p.add_argument("--film-grain-seed", type=int, default=0)
    args = p.parse_args()
    build_candidate(
        args.source, args.output, mode=args.mode, manifest_path=args.manifest,
        hardware_model=args.hardware_model, capture_type=args.capture_type,
        capture_mode=args.capture_mode, port_type=args.port_type,
        film_grain_seed=args.film_grain_seed,
    )


if __name__ == "__main__":
    main()
