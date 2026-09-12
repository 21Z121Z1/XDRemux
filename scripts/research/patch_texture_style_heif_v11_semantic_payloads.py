#!/usr/bin/env python3
"""Replace clean-room PS3 TextureStyle semantic-role aliases with real blank HEVC mattes.

The blank 768x576 monochrome HEVC frame is generated independently with x265
from an all-zero image; no private photo bytes are embedded. The RDF payload is
a schema-only FSINC MatteVersion=0 record.
"""
from __future__ import annotations
import argparse, hashlib, importlib.util, json
from pathlib import Path

SEMANTIC_PREFIX = "tag:apple.com,2026:photo:aux:semantic"
FSINC_XMP = (b'<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="XMP Core 6.0.0">\n'
 b'   <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">\n'
 b'      <rdf:Description rdf:about=""\n'
 b'            xmlns:fsincMattes="http://ns.apple.com/fsinc/1.0/">\n'
 b'         <fsincMattes:FSINCMatteVersion>0</fsincMattes:FSINCMatteVersion>\n'
 b'      </rdf:Description>\n   </rdf:RDF>\n</x:xmpmeta>\n')
# Clean-room x265 encoding of a black 768x576 monochrome still. hvcC contains
# only independently generated VPS/SPS/PPS; item data contains one IDR NAL.
CLEANROOM_HVCC = bytes.fromhex(
    "00000075687663430104080000009fe8000000005af000fcfcf8f800000f03"
    "a00001001740010c01ffff0408000003009fe800000300005aba0240"
    "a1000100294201010408000003009fe800000300005ac01808024165ba924caf016c08000003000800000300c840"
    "a2000100074401c172b06240"
)
CLEANROOM_HEVC_ITEM = bytes.fromhex(
    "0000008b2801aee24b87358c62d7cb0fbfffca7b8730db26d106f41de25c"
    "000003000003000c38b7b25607e8f94e2c8000000300014b9d9245f2e8"
    "00000300000300040c8dc880000003000003000010f000000300000300000300000828"
    "0000030000030000030000d98000000300000300000300049c000003000003000003000f68"
    "000003000003000003002160"
)


def load_v3():
    path = Path(__file__).with_name("patch_texture_style_heif_v3.py")
    spec = importlib.util.spec_from_file_location("texture_v3", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

V3 = load_v3()


def full_box(box_type, version, flags, body):
    return V3.make_box(box_type, bytes([version]) + int(flags).to_bytes(3, "big") + body)


def make_ispe(width, height):
    return full_box("ispe", 0, 0, int(width).to_bytes(4, "big") + int(height).to_bytes(4, "big"))


def make_pixi(bits=8):
    return full_box("pixi", 0, 0, bytes([1, bits]))


def make_irot_zero():
    return V3.make_box("irot", b"\0")


def auxc_uri(raw):
    if raw[4:8] != b"auxC" or len(raw) < 13:
        return None
    end = raw.find(b"\0", 12)
    return raw[12:end].decode("utf-8", "replace") if end >= 0 else None


def parse_ipma(data, box):
    p = box["data_start"]
    version = data[p]
    flags = int.from_bytes(data[p + 1:p + 4], "big")
    p += 4
    count = int.from_bytes(data[p:p + 4], "big")
    p += 4
    wide = bool(flags & 1)
    entries = []
    for _ in range(count):
        width = 4 if version >= 1 else 2
        item_id = int.from_bytes(data[p:p + width], "big")
        p += width
        n = data[p]
        p += 1
        associations = []
        for _ in range(n):
            if wide:
                v = int.from_bytes(data[p:p + 2], "big")
                p += 2
                associations.append({"index": v & 0x7FFF, "essential": bool(v & 0x8000)})
            else:
                v = data[p]
                p += 1
                associations.append({"index": v & 0x7F, "essential": bool(v & 0x80)})
        entries.append({"id": item_id, "associations": associations})
    return version, flags, entries


def make_ipma(version, flags, entries):
    wide = bool(flags & 1)
    body = len(entries).to_bytes(4, "big")
    for entry in entries:
        body += int(entry["id"]).to_bytes(4 if version >= 1 else 2, "big")
        body += bytes([len(entry["associations"])])
        for assoc in entry["associations"]:
            if wide:
                v = int(assoc["index"]) | (0x8000 if assoc["essential"] else 0)
                body += v.to_bytes(2, "big")
            else:
                v = int(assoc["index"]) | (0x80 if assoc["essential"] else 0)
                body += v.to_bytes(1, "big")
    return full_box("ipma", version, flags, body)


def parse_iprp(data, box):
    children = list(V3.boxes(data, box["data_start"], box["data_end"]))
    ipco = next(c for c in children if c["type"] == "ipco")
    ipma = next(c for c in children if c["type"] == "ipma")
    props = list(V3.boxes(data, ipco["data_start"], ipco["data_end"]))
    raw_props = [data[p["pos"]:p["pos"] + p["size"]] for p in props]
    version, flags, entries = parse_ipma(data, ipma)
    return children, raw_props, version, flags, entries


def rebuild_iprp(data, box, raw_props, ipma_raw):
    children = list(V3.boxes(data, box["data_start"], box["data_end"]))
    out = []
    for child in children:
        if child["type"] == "ipco":
            out.append(V3.make_box("ipco", b"".join(raw_props)))
        elif child["type"] == "ipma":
            out.append(ipma_raw)
        else:
            out.append(data[child["pos"]:child["pos"] + child["size"]])
    return V3.make_box("iprp", b"".join(out))


def item_hashes(data, iloc, idat):
    return {
        e["id"]: hashlib.sha256(V3.item_payload(data, e, idat)).hexdigest()
        for e in iloc["entries"]
    }


def build(source, output, manifest=None):
    data = Path(source).read_bytes()
    top = list(V3.boxes(data, 0, len(data)))
    meta = next(b for b in top if b["type"] == "meta")
    mdat = next(b for b in top if b["type"] == "mdat")
    children = list(V3.boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {b["type"]: b for b in children}
    _, infos = V3.parse_iinf(data, by_type["iinf"])
    _, refs = V3.parse_iref(data, by_type["iref"])
    iloc = V3.parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    entries = [V3.clone_iloc_entry(e) for e in iloc["entries"]]
    entry_map = {e["id"]: e for e in entries}
    old_hashes = item_hashes(data, iloc, idat)

    _, raw_props, ipma_version, ipma_flags, ipma_entries = parse_iprp(data, by_type["iprp"])
    prop_uris = {idx + 1: auxc_uri(raw) for idx, raw in enumerate(raw_props)}
    semantic_prop_indices = {
        idx: uri for idx, uri in prop_uris.items()
        if uri and uri.startswith(SEMANTIC_PREFIX)
    }
    if len(semantic_prop_indices) != 12:
        raise ValueError(f"expected 12 2026 semantic auxC roles, got {len(semantic_prop_indices)}")

    ipma_map = {e["id"]: e for e in ipma_entries}
    role_items = []
    for prop_index, uri in sorted(semantic_prop_indices.items()):
        matches = [
            e["id"] for e in ipma_entries
            if any(a["index"] == prop_index for a in e["associations"])
        ]
        if len(matches) != 1:
            raise ValueError(f"role {uri} maps to {matches}")
        role_items.append((uri, prop_index, matches[0]))

    ref_map = {}
    for ref in refs:
        if ref["type"] == "cdsc" and len(ref["to"]) == 1:
            ref_map.setdefault(ref["to"][0], []).append(ref["from"])
    descriptor_ids = {}
    item_types = {i["id"]: i["type"] for i in infos}
    for uri, _, image_id in role_items:
        candidates = [x for x in ref_map.get(image_id, []) if item_types.get(x) == "mime"]
        if len(candidates) != 1:
            raise ValueError(f"semantic image {image_id} descriptor candidates {candidates}")
        descriptor_ids[image_id] = candidates[0]

    # Add clean-room common properties once, then give every semantic role the
    # native-shaped association sequence: ispe, pixi, auxC, hvcC, irot.
    raw_props.append(make_ispe(768, 576)); ispe_idx = len(raw_props)
    raw_props.append(make_pixi(8)); pixi_idx = len(raw_props)
    raw_props.append(CLEANROOM_HVCC); hvcc_idx = len(raw_props)
    raw_props.append(make_irot_zero()); irot_idx = len(raw_props)
    for uri, aux_index, image_id in role_items:
        ipma_map[image_id]["associations"] = [
            {"index": ispe_idx, "essential": False},
            {"index": pixi_idx, "essential": False},
            {"index": aux_index, "essential": True},
            {"index": hvcc_idx, "essential": True},
            {"index": irot_idx, "essential": True},
        ]
    new_ipma = make_ipma(ipma_version, ipma_flags, ipma_entries)
    new_iprp = rebuild_iprp(data, by_type["iprp"], raw_props, new_ipma)

    replacements = {"iprp": new_iprp, "iloc": V3.make_iloc(iloc, entries)}
    def make_meta(repl):
        parts = [repl.get(c["type"], data[c["pos"]:c["pos"] + c["size"]]) for c in children]
        return V3.make_box("meta", data[meta["data_start"]:meta["data_start"] + 4] + b"".join(parts))
    provisional = make_meta(replacements)
    delta = len(provisional) - meta["size"]
    old_mdat_data_start = mdat["data_start"]

    role_image_ids = {image_id for _, _, image_id in role_items}
    role_descriptor_ids = set(descriptor_ids.values())
    replaced_ids = role_image_ids | role_descriptor_ids
    for entry in entries:
        if entry["id"] in replaced_ids or entry["method"] != 0:
            continue
        for extent in entry["extents"]:
            absolute = entry["base_offset"] + extent["offset"]
            if absolute >= old_mdat_data_start:
                extent["offset"] += delta

    old_payload = data[mdat["data_start"]:mdat["data_end"]]
    new_mdat_data_start = old_mdat_data_start + delta
    cursor = new_mdat_data_start + len(old_payload)
    tail = bytearray()
    payload_manifest = []
    for uri, _, image_id in role_items:
        descriptor_id = descriptor_ids[image_id]
        for item_id, payload, kind in (
            (image_id, CLEANROOM_HEVC_ITEM, "blank-hevc"),
            (descriptor_id, FSINC_XMP, "fsinc-xmp"),
        ):
            entry = entry_map[item_id]
            entry["construction_field"] &= 0xFFF0
            entry["method"] = 0
            entry["data_ref"] = 0
            entry["base_offset"] = 0
            entry["extents"] = [{"index": None, "offset": cursor, "length": len(payload)}]
            tail += payload
            payload_manifest.append({
                "itemID": item_id, "role": uri, "kind": kind,
                "offset": cursor, "length": len(payload),
                "sha256": hashlib.sha256(payload).hexdigest(),
            })
            cursor += len(payload)

    replacements["iloc"] = V3.make_iloc(iloc, entries)
    final_meta = make_meta(replacements)
    if len(final_meta) != len(provisional):
        raise ValueError("meta size changed after iloc relocation")

    new_mdat = V3.make_box("mdat", old_payload + bytes(tail))
    prefix = data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:mdat["pos"]]
    suffix = data[mdat["data_end"]:]
    modified = prefix + new_mdat + suffix
    Path(output).write_bytes(modified)

    ntop = list(V3.boxes(modified, 0, len(modified)))
    nmeta = next(b for b in ntop if b["type"] == "meta")
    nch = list(V3.boxes(modified, nmeta["data_start"] + 4, nmeta["data_end"]))
    nbt = {b["type"]: b for b in nch}
    niloc = V3.parse_iloc(modified, nbt["iloc"])
    nidat = nbt.get("idat")
    nentry = {e["id"]: e for e in niloc["entries"]}
    new_hashes = item_hashes(modified, niloc, nidat)
    bad = [iid for iid, digest in old_hashes.items() if iid not in replaced_ids and new_hashes.get(iid) != digest]
    if bad:
        raise ValueError(f"unrelated item payloads changed: {bad}")
    for uri, _, image_id in role_items:
        if V3.item_payload(modified, nentry[image_id], nidat) != CLEANROOM_HEVC_ITEM:
            raise ValueError(f"semantic image payload mismatch: {uri}")
        did = descriptor_ids[image_id]
        if V3.item_payload(modified, nentry[did], nidat) != FSINC_XMP:
            raise ValueError(f"descriptor payload mismatch: {uri}")

    report = {
        "schema": "xdremux-texture-style-cleanroom-v11",
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "semanticRoleCount": len(role_items),
        "semanticRoles": [uri for uri, _, _ in role_items],
        "blankMatte": {
            "width": 768, "height": 576, "bitDepth": 8, "monochrome": True,
            "itemPayloadLength": len(CLEANROOM_HEVC_ITEM),
            "itemPayloadSHA256": hashlib.sha256(CLEANROOM_HEVC_ITEM).hexdigest(),
            "hvcC_SHA256": hashlib.sha256(CLEANROOM_HVCC).hexdigest(),
            "origin": "clean-room all-zero frame encoded independently with x265",
        },
        "descriptor": {
            "length": len(FSINC_XMP),
            "sha256": hashlib.sha256(FSINC_XMP).hexdigest(),
            "FSINCMatteVersion": 0,
        },
        "payloads": payload_manifest,
        "preExistingUnrelatedPayloadsByteIdentical": True,
        "topLevelBoxes": [b["type"] for b in ntop],
        "metaChildOrder": [b["type"] for b in nch],
    }
    if manifest:
        Path(manifest).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))


def main():
    p = argparse.ArgumentParser()
    p.add_argument("source")
    p.add_argument("output")
    p.add_argument("--manifest")
    a = p.parse_args()
    build(a.source, a.output, a.manifest)

if __name__ == "__main__":
    main()
