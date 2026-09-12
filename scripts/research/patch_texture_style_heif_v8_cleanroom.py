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
import struct
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


def _bplist_count_header(kind, count):
    if count < 15:
        return bytes([kind | count])
    if count < 256:
        return bytes([kind | 0x0F, 0x10, count])
    if count < 65536:
        return bytes([kind | 0x0F, 0x11]) + int(count).to_bytes(2, "big")
    raise ValueError("binary plist collection is too large")


def _bplist_ascii(value):
    raw = value.encode("ascii")
    return _bplist_count_header(0x50, len(raw)) + raw


def _bplist_int(value):
    value = int(value)
    if 0 <= value <= 0xFF:
        return b"\x10" + value.to_bytes(1, "big")
    if 0 <= value <= 0xFFFF:
        return b"\x11" + value.to_bytes(2, "big")
    if -(1 << 31) <= value < (1 << 31):
        return b"\x12" + value.to_bytes(4, "big", signed=True)
    return b"\x13" + value.to_bytes(8, "big", signed=True)


def _bplist_float32(value):
    return b"\x22" + struct.pack(">f", float(value))


def _build_simple_bplist(objects, top_object=0):
    ref_size = 1 if len(objects) < 256 else 2
    out = bytearray(b"bplist00")
    offsets = []
    for obj in objects:
        offsets.append(len(out))
        out.extend(obj)
    offset_table = len(out)
    max_offset = max(offsets + [offset_table])
    offset_size = 1
    while max_offset >= (1 << (8 * offset_size)):
        offset_size += 1
    for offset in offsets:
        out.extend(int(offset).to_bytes(offset_size, "big"))
    trailer = bytearray(32)
    trailer[6] = offset_size
    trailer[7] = ref_size
    trailer[8:16] = len(objects).to_bytes(8, "big")
    trailer[16:24] = int(top_object).to_bytes(8, "big")
    trailer[24:32] = offset_table.to_bytes(8, "big")
    out.extend(trailer)
    return bytes(out)


def make_native_typed_tag84(tag84):
    # The PS3 SmartStyle numeric contract distinguishes Float32 slots from
    # integer / Boolean slots. Python plistlib writes floats as Float64, so use
    # a tiny deterministic binary-plist encoder here instead of widening them.
    order = ["10", "2", "3", "11", "4", "5", "12", "6", "7", "0", "8", "1", "9"]
    missing = [key for key in order if key not in tag84]
    if missing:
        raise ValueError(f"tag84 is missing required slots: {missing}")

    objects = [None]
    key_ids = []
    for key in order:
        key_ids.append(len(objects))
        objects.append(_bplist_ascii(key))

    value_ids = []
    value_cache = {}
    float_slots = {"10", "2", "3", "1", "9"}
    for key in order:
        value = tag84[key]
        if key in float_slots:
            encoded = _bplist_float32(value)
            cache_key = ("f32", encoded[1:])
        elif key == "11":
            encoded = b"\x09" if bool(value) else b"\x08"
            cache_key = ("bool", bool(value))
        else:
            encoded = _bplist_int(value)
            cache_key = ("int", int(value))
        if cache_key not in value_cache:
            value_cache[cache_key] = len(objects)
            objects.append(encoded)
        value_ids.append(value_cache[cache_key])

    ref_size = 1 if len(objects) < 256 else 2
    objects[0] = (
        _bplist_count_header(0xD0, len(order))
        + b"".join(i.to_bytes(ref_size, "big") for i in key_ids)
        + b"".join(i.to_bytes(ref_size, "big") for i in value_ids)
    )
    raw = _build_simple_bplist(objects)
    decoded = plistlib.loads(raw)
    if any(decoded[key] != tag84[key] for key in order):
        raise ValueError("native-typed tag84 binary plist failed roundtrip")
    return raw


def _bplist_layout(raw):
    if not raw.startswith(b"bplist00") or len(raw) < 40:
        raise ValueError("style metadata is not a binary plist")
    trailer = raw[-32:]
    offset_size = trailer[6]
    ref_size = trailer[7]
    count = int.from_bytes(trailer[8:16], "big")
    top = int.from_bytes(trailer[16:24], "big")
    offset_table = int.from_bytes(trailer[24:32], "big")
    offsets = [
        int.from_bytes(raw[offset_table + i * offset_size:offset_table + (i + 1) * offset_size], "big")
        for i in range(count)
    ]
    return offset_size, ref_size, count, top, offset_table, offsets


def _bplist_collection_count(raw, pos):
    marker = raw[pos]
    low = marker & 0x0F
    if low < 0x0F:
        return low, 1
    int_marker = raw[pos + 1]
    if int_marker >> 4 != 1:
        raise ValueError("extended binary-plist count is not encoded as an integer")
    width = 1 << (int_marker & 0x0F)
    return int.from_bytes(raw[pos + 2:pos + 2 + width], "big"), 2 + width


def _bplist_ascii_at(raw, pos):
    marker = raw[pos]
    if marker >> 4 != 5:
        raise ValueError("expected ASCII binary-plist key")
    count, header = _bplist_collection_count(raw, pos)
    return raw[pos + header:pos + header + count].decode("ascii")


def upgrade_style_bplist_v15_to_v16(raw):
    # Keep every existing style-data object byte-identical except the top-level
    # dictionary and its version object. This avoids reserializing the learned
    # latent / linear-thumbnail payloads or widening their numeric types.
    offset_size, ref_size, object_count, top, offset_table, offsets = _bplist_layout(raw)
    if top != 0:
        raise ValueError(f"unexpected style plist top object {top}")
    top_pos = offsets[top]
    if raw[top_pos] >> 4 != 0x0D:
        raise ValueError("style plist top object is not a dictionary")
    count, header = _bplist_collection_count(raw, top_pos)
    refs_pos = top_pos + header
    key_refs = [
        int.from_bytes(raw[refs_pos + i * ref_size:refs_pos + (i + 1) * ref_size], "big")
        for i in range(count)
    ]
    values_pos = refs_pos + count * ref_size
    value_refs = [
        int.from_bytes(raw[values_pos + i * ref_size:values_pos + (i + 1) * ref_size], "big")
        for i in range(count)
    ]
    keys = [_bplist_ascii_at(raw, offsets[obj]) for obj in key_refs]
    if "l" in keys:
        raise ValueError("style schema already contains key l")
    version_index = keys.index("0")
    version_object = value_refs[version_index]
    version_pos = offsets[version_object]
    if raw[version_pos:version_pos + 2] != b"\x10\x0f":
        raise ValueError("expected style schema version 15 as one-byte integer")

    ordered = sorted((offset, index) for index, offset in enumerate(offsets))
    ends = {}
    for i, (offset, index) in enumerate(ordered):
        ends[index] = ordered[i + 1][0] if i + 1 < len(ordered) else offset_table
    objects = [raw[offsets[i]:ends[i]] for i in range(object_count)]
    objects[version_object] = b"\x10\x10"

    l_key = len(objects)
    objects.append(b"\x51l")
    l_value = len(objects)
    objects.append(b"\x08")
    key_refs.append(l_key)
    value_refs.append(l_value)
    new_count = count + 1
    if max(key_refs + value_refs) >= (1 << (8 * ref_size)):
        raise ValueError("style plist would require a wider object reference size")
    objects[top] = (
        _bplist_count_header(0xD0, new_count)
        + b"".join(i.to_bytes(ref_size, "big") for i in key_refs)
        + b"".join(i.to_bytes(ref_size, "big") for i in value_refs)
    )

    out = bytearray(b"bplist00")
    new_offsets = []
    for obj in objects:
        new_offsets.append(len(out))
        out.extend(obj)
    new_offset_table = len(out)
    new_offset_size = offset_size
    max_offset = max(new_offsets + [new_offset_table])
    while max_offset >= (1 << (8 * new_offset_size)):
        new_offset_size += 1
    for offset in new_offsets:
        out.extend(offset.to_bytes(new_offset_size, "big"))
    trailer = bytearray(32)
    trailer[6] = new_offset_size
    trailer[7] = ref_size
    trailer[8:16] = len(objects).to_bytes(8, "big")
    trailer[16:24] = top.to_bytes(8, "big")
    trailer[24:32] = new_offset_table.to_bytes(8, "big")
    out.extend(trailer)
    upgraded = bytes(out)
    decoded = plistlib.loads(upgraded)
    if decoded.get("0") != 16 or decoded.get("l") is not False:
        raise ValueError("style v16 surgical rewrite failed roundtrip")
    return upgraded


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
    "exif-relocate-control": dict(patch_maker=False, relocate_exif=True, upgrade_style=False,
                                  relocate_style=False, style_name=None, add_texture=False),
    "maker-surgical": dict(patch_maker=True, relocate_exif=True, upgrade_style=False,
                            relocate_style=False, style_name=None, add_texture=False),
    "style-v16": dict(patch_maker=False, relocate_exif=False, upgrade_style=True,
                       relocate_style=True, style_name="metadata", add_texture=False),
    "style-v16-maker": dict(patch_maker=True, relocate_exif=True, upgrade_style=True,
                             relocate_style=True, style_name="metadata", add_texture=False),
    "full-v16": dict(patch_maker=True, relocate_exif=True, upgrade_style=True,
                      relocate_style=True, style_name="metadata", add_texture=True),
    "full-v16-legacy-name": dict(patch_maker=True, relocate_exif=True, upgrade_style=True,
                                  relocate_style=True, style_name="styleMetadata", add_texture=True),
    "full-carrier-v15": dict(patch_maker=True, relocate_exif=True, upgrade_style=False,
                              relocate_style=True, style_name="metadata", add_texture=True),
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
    if cfg["upgrade_style"]:
        if old_style_version != 15:
            raise ValueError(f"expected style schema 15, got {old_style_version!r}")
        new_style_payload = upgrade_style_bplist_v15_to_v16(old_style_payload)
    else:
        new_style_payload = old_style_payload

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
        new_tag84 = make_native_typed_tag84(tag84)
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

    relocate_ids = {exif_id} if cfg["relocate_exif"] else set()
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
    mdats = [b for b in top if b["type"] == "mdat"]
    if len(mdats) != 1 or mdats[0]["data_end"] != len(data):
        raise ValueError("clean-room baseline must have exactly one final mdat")
    carrier_mdat = mdats[0]
    trailing_data_start = len(data) + delta

    def append_relocated(item_id, payload):
        e = next(x for x in new_entries if x["id"] == item_id)
        e["extents"] = [{
            "index": None,
            "offset": trailing_data_start + len(trailing),
            "length": len(payload),
        }]
        trailing.extend(payload)

    if cfg["relocate_exif"]:
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
    modified = bytearray(data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:])
    if trailing:
        new_mdat_pos = carrier_mdat["pos"] + delta
        new_mdat_size = carrier_mdat["size"] + len(trailing)
        if carrier_mdat["header"] == 8:
            if new_mdat_size > 0xFFFFFFFF:
                raise ValueError("extended mdat size would require a 64-bit box header")
            modified[new_mdat_pos:new_mdat_pos + 4] = new_mdat_size.to_bytes(4, "big")
        elif carrier_mdat["header"] == 16:
            modified[new_mdat_pos + 8:new_mdat_pos + 16] = new_mdat_size.to_bytes(8, "big")
        else:
            raise ValueError("unsupported mdat header size")
        modified.extend(trailing)
    modified = bytes(modified)
    Path(output).write_bytes(modified)

    # Reparse and prove structural + payload invariants.
    ntop = list(V3.boxes(modified, 0, len(modified)))
    nmeta = next(b for b in ntop if b["type"] == "meta")
    nmdats = [b for b in ntop if b["type"] == "mdat"]
    if len(nmdats) != 1 or nmdats[0]["data_end"] != len(modified):
        raise ValueError("output did not preserve the single final mdat carrier")
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
    if cfg["upgrade_style"]:
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
        "exifRelocated": cfg["relocate_exif"],
        "maker84Encoding": "native-typed-float32-bplist" if cfg["patch_maker"] else None,
        "styleRewrite": "surgical-bplist-v15-to-v16" if cfg["upgrade_style"] else "byte-identical",
        "mdatCarrier": "single-final-mdat-extended",
        "mdatCountBefore": 1,
        "mdatCountAfter": 1,
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
