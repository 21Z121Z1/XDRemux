#!/usr/bin/env python3
"""Clean-room PS3 TextureStyle contract completion post-processor.

Only generalized contracts are encoded. No private sample image bytes are used.
Semantic resources are aliases of an auxiliary matte already present in the input.
"""
from __future__ import annotations
import argparse, hashlib, importlib.util, json, plistlib
from pathlib import Path

TEXTURE_URI = b"tag:apple.com,2026:photo:metadata:texture_styles\0"
STYLE_URI = b"tag:apple.com,2023:photo:metadata:styles\0"
SEMANTIC_TYPES = [
    "tag:apple.com,2026:photo:aux:semanticnosematte",
    "tag:apple.com,2026:photo:aux:semanticskinmattev2",
    "tag:apple.com,2026:photo:aux:semanticnonfaceskinmatte",
    "tag:apple.com,2026:photo:aux:semanticlipsmatte",
    "tag:apple.com,2026:photo:aux:semanticteethmattev2",
    "tag:apple.com,2026:photo:aux:semanticpersonmatte",
    "tag:apple.com,2026:photo:aux:semanticglassesmattev2",
    "tag:apple.com,2026:photo:aux:semanticeyebrowsmatte",
    "tag:apple.com,2026:photo:aux:semantictattoomatte",
    "tag:apple.com,2026:photo:aux:semantichandsmatte",
    "tag:apple.com,2026:photo:aux:semanticearsmatte",
    "tag:apple.com,2026:photo:aux:semanticfaceskinmatte",
]
NATIVE_FTYP = bytes.fromhex("000000346674797068656963000000006d6966314d6948424d694841686569784d6948454d6950726d69616668656963746d6170")
NATIVE_META_ORDER = ["hdlr", "dinf", "pitm", "iinf", "iref", "iprp", "grpl", "idat", "iloc"]


def load_v3():
    path = Path(__file__).with_name("patch_texture_style_heif_v3.py")
    spec = importlib.util.spec_from_file_location("texture_v3", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


V3 = load_v3()


def full_box(box_type, version, flags, body):
    return V3.make_box(box_type, bytes([version]) + int(flags).to_bytes(3, "big") + body)


def clone_infe_id(raw, item_id):
    if raw[4:8] != b"infe" or raw[8] != 2:
        raise ValueError("expected infe v2")
    out = bytearray(raw)
    out[12:14] = int(item_id).to_bytes(2, "big")
    return bytes(out)


def make_mime_infe(item_id):
    payload = bytes([2, 0, 0, 1]) + int(item_id).to_bytes(2, "big") + b"\0\0mime\0application/rdf+xml\0"
    return V3.make_box("infe", payload)


def make_auxc(uri):
    return full_box("auxC", 0, 0, uri.encode() + b"\0")


def parse_ipma(data, box):
    pos = box["data_start"]
    version = data[pos]
    flags = int.from_bytes(data[pos + 1:pos + 4], "big")
    pos += 4
    count = int.from_bytes(data[pos:pos + 4], "big")
    pos += 4
    wide = bool(flags & 1)
    entries = []
    for _ in range(count):
        width = 4 if version >= 1 else 2
        item_id = int.from_bytes(data[pos:pos + width], "big")
        pos += width
        assoc_count = data[pos]
        pos += 1
        associations = []
        for _ in range(assoc_count):
            if wide:
                value = int.from_bytes(data[pos:pos + 2], "big")
                pos += 2
                associations.append({"index": value & 0x7FFF, "essential": bool(value & 0x8000)})
            else:
                value = data[pos]
                pos += 1
                associations.append({"index": value & 0x7F, "essential": bool(value & 0x80)})
        entries.append({"id": item_id, "associations": associations})
    return version, flags, entries


def make_ipma(version, flags, entries):
    wide = bool(flags & 1)
    body = len(entries).to_bytes(4, "big")
    for entry in entries:
        body += int(entry["id"]).to_bytes(4 if version >= 1 else 2, "big")
        associations = entry["associations"]
        body += bytes([len(associations)])
        for assoc in associations:
            if wide:
                value = int(assoc["index"]) | (0x8000 if assoc["essential"] else 0)
                body += value.to_bytes(2, "big")
            else:
                value = int(assoc["index"]) | (0x80 if assoc["essential"] else 0)
                body += value.to_bytes(1, "big")
    return full_box("ipma", version, flags, body)


def parse_iprp(data, box):
    children = list(V3.boxes(data, box["data_start"], box["data_end"]))
    ipco = next(child for child in children if child["type"] == "ipco")
    ipma = next(child for child in children if child["type"] == "ipma")
    properties = list(V3.boxes(data, ipco["data_start"], ipco["data_end"]))
    raw_properties = [data[prop["pos"]:prop["pos"] + prop["size"]] for prop in properties]
    version, flags, entries = parse_ipma(data, ipma)
    return raw_properties, version, flags, entries


def auxc_uri(raw):
    if raw[4:8] != b"auxC":
        return None
    end = raw.find(b"\0", 12)
    return raw[12:end].decode(errors="replace") if end >= 0 else None


def make_iprp(data, box, raw_properties, ipma_raw):
    children = list(V3.boxes(data, box["data_start"], box["data_end"]))
    output = []
    for child in children:
        if child["type"] == "ipco":
            output.append(V3.make_box("ipco", b"".join(raw_properties)))
        elif child["type"] == "ipma":
            output.append(ipma_raw)
        else:
            output.append(data[child["pos"]:child["pos"] + child["size"]])
    return V3.make_box("iprp", b"".join(output))


def item_hashes(data, iloc, idat):
    return {
        entry["id"]: hashlib.sha256(V3.item_payload(data, entry, idat)).hexdigest()
        for entry in iloc["entries"]
    }


def patch_ascii_ifd0(exif_payload, replacements):
    if len(exif_payload) < 14 or exif_payload[4:10] != b"Exif\0\0":
        raise ValueError("unexpected Exif payload")
    out = bytearray(exif_payload)
    base = 10
    endian = "big" if out[base:base + 2] == b"MM" else "little"

    def read(pos, width):
        return int.from_bytes(out[base + pos:base + pos + width], endian)

    ifd0 = read(4, 4)
    count = read(ifd0, 2)
    names = {0x010F: "Make", 0x0110: "Model"}
    changed = {}
    for index in range(count):
        entry_pos = ifd0 + 2 + index * 12
        tag = read(entry_pos, 2)
        name = names.get(tag)
        if name not in replacements or read(entry_pos + 2, 2) != 2:
            continue
        value = replacements[name].encode("ascii") + b"\0"
        if (len(out) - base) & 1:
            out.append(0)
        offset = len(out) - base
        out.extend(value)
        out[base + entry_pos + 4:base + entry_pos + 8] = len(value).to_bytes(4, endian)
        out[base + entry_pos + 8:base + entry_pos + 12] = offset.to_bytes(4, endian)
        changed[name] = {"value": replacements[name], "offset": offset, "count": len(value)}
    missing = set(replacements) - set(changed)
    if missing:
        raise ValueError(f"IFD0 tags not patched: {sorted(missing)}")
    return bytes(out), changed


def parse_pitm(data, box):
    pos = box["data_start"]
    version = data[pos]
    pos += 4
    return int.from_bytes(data[pos:pos + (4 if version else 2)], "big")


def build(source, output, *, semantic_aliases=False, provenance=False, pack_early=False, canonical=False, manifest=None):
    data = Path(source).read_bytes()
    top = list(V3.boxes(data, 0, len(data)))
    meta = next(box for box in top if box["type"] == "meta")
    children = list(V3.boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {box["type"]: box for box in children}
    iinf_version, infos = V3.parse_iinf(data, by_type["iinf"])
    iref_version, refs = V3.parse_iref(data, by_type["iref"])
    iloc = V3.parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    entry_map = {entry["id"]: entry for entry in iloc["entries"]}
    item_types = {info["id"]: info["type"] for info in infos}
    old_hashes = item_hashes(data, iloc, idat)
    exif_id = next(item_id for item_id, item_type in item_types.items() if item_type == "Exif")
    style = next(info for info in infos if info["type"] == "uri " and STYLE_URI in info["raw"])
    texture = next(info for info in infos if info["type"] == "uri " and TEXTURE_URI in info["raw"])
    style_id = style["id"]
    texture_id = texture["id"]
    targets = next(ref["to"] for ref in refs if ref["type"] == "cdsc" and ref["from"] == style_id)

    raw_infes = [info["raw"] for info in infos]
    new_refs = [{"type": ref["type"], "from": ref["from"], "to": list(ref["to"])} for ref in refs]
    new_entries = [V3.clone_iloc_entry(entry) for entry in iloc["entries"]]
    replacements = {}
    deliberately_changed = set()
    provenance_info = None
    exif_new = None

    if provenance:
        if not pack_early:
            raise ValueError("provenance patch requires --pack-early because Exif grows")
        exif_old = V3.item_payload(data, entry_map[exif_id], idat)
        exif_new, provenance_info = patch_ascii_ifd0(
            exif_old, {"Make": "Apple", "Model": "iPhone 18 Pro"}
        )
        deliberately_changed.add(exif_id)

    semantic_ids = []
    if semantic_aliases:
        raw_properties, ipma_version, ipma_flags, ipma_entries = parse_iprp(data, by_type["iprp"])
        aux_uris = {index + 1: auxc_uri(raw) for index, raw in enumerate(raw_properties)}
        source_aux_index = next(
            index for index, uri in aux_uris.items()
            if uri == "urn:com:apple:photo:2019:aux:semanticskinmatte"
        )
        ipma_map = {entry["id"]: entry for entry in ipma_entries}
        source_image = next(
            entry["id"] for entry in ipma_entries
            if any(assoc["index"] == source_aux_index for assoc in entry["associations"])
        )
        source_info = next(info for info in infos if info["id"] == source_image)
        source_location = entry_map[source_image]
        source_descriptor = next(
            (
                ref["from"] for ref in refs
                if ref["type"] == "cdsc" and ref["to"] == [source_image]
                and item_types.get(ref["from"]) == "mime"
            ),
            None,
        )
        if source_descriptor is None:
            raise ValueError("source semantic matte descriptor missing")
        source_descriptor_location = entry_map[source_descriptor]
        next_id = max(max(entry_map), texture_id) + 1
        for uri in SEMANTIC_TYPES:
            image_id = next_id
            descriptor_id = next_id + 1
            next_id += 2
            raw_properties.append(make_auxc(uri))
            new_aux_index = len(raw_properties)
            raw_infes.append(clone_infe_id(source_info["raw"], image_id))
            raw_infes.append(make_mime_infe(descriptor_id))
            image_entry = V3.clone_iloc_entry(source_location)
            image_entry["id"] = image_id
            descriptor_entry = V3.clone_iloc_entry(source_descriptor_location)
            descriptor_entry["id"] = descriptor_id
            new_entries.extend([image_entry, descriptor_entry])
            associations = [
                {
                    "index": new_aux_index if assoc["index"] == source_aux_index else assoc["index"],
                    "essential": assoc["essential"],
                }
                for assoc in ipma_map[source_image]["associations"]
            ]
            ipma_entries.append({"id": image_id, "associations": associations})
            new_refs.append({"type": "auxl", "from": image_id, "to": list(targets)})
            new_refs.append({"type": "cdsc", "from": descriptor_id, "to": [image_id]})
            semantic_ids.append({"uri": uri, "image": image_id, "descriptor": descriptor_id})
        replacements["iprp"] = make_iprp(
            data,
            by_type["iprp"],
            raw_properties,
            make_ipma(ipma_version, ipma_flags, ipma_entries),
        )

    replacements["iinf"] = V3.make_iinf(iinf_version, raw_infes)
    replacements["iref"] = V3.make_iref(iref_version, new_refs)
    replacements["iloc"] = V3.make_iloc(iloc, new_entries)
    order = NATIVE_META_ORDER if canonical else [child["type"] for child in children]

    def make_meta(current):
        child_map = {child["type"]: child for child in children}
        output_children = []
        seen = set()
        for box_type in order:
            if box_type in child_map:
                child = child_map[box_type]
                output_children.append(current.get(box_type, data[child["pos"]:child["pos"] + child["size"]]))
                seen.add(box_type)
        for child in children:
            if child["type"] not in seen:
                output_children.append(current.get(child["type"], data[child["pos"]:child["pos"] + child["size"]]))
        return V3.make_box(
            "meta",
            data[meta["data_start"]:meta["data_start"] + 4] + b"".join(output_children),
        )

    provisional_meta = make_meta(replacements)
    old_mdat = next(box for box in top if box["type"] == "mdat")
    old_mdat_data_start = old_mdat["data_start"]
    if canonical:
        top_delta = len(NATIVE_FTYP) + len(provisional_meta) - old_mdat["pos"]
    else:
        top_delta = len(provisional_meta) - meta["size"]

    for entry in new_entries:
        if entry["method"] != 0:
            continue
        for extent in entry["extents"]:
            absolute = entry["base_offset"] + extent["offset"]
            if absolute >= old_mdat_data_start:
                extent["offset"] += top_delta

    selected = []
    if pack_early:
        selected = [style_id, texture_id, exif_id]
        payloads = {
            style_id: V3.item_payload(data, entry_map[style_id], idat),
            texture_id: V3.item_payload(data, entry_map[texture_id], idat),
            exif_id: exif_new if exif_new is not None else V3.item_payload(data, entry_map[exif_id], idat),
        }
        prefix = b"".join(payloads[item_id] for item_id in selected)
        for entry in new_entries:
            if entry["method"] == 0:
                for extent in entry["extents"]:
                    extent["offset"] += len(prefix)
        cursor = old_mdat_data_start + top_delta
        for item_id in selected:
            entry = next(candidate for candidate in new_entries if candidate["id"] == item_id)
            payload = payloads[item_id]
            entry["construction_field"] &= 0xFFF0
            entry["method"] = 0
            entry["data_ref"] = 0
            entry["base_offset"] = 0
            entry["extents"] = [{"index": None, "offset": cursor, "length": len(payload)}]
            cursor += len(payload)
    else:
        prefix = b""

    replacements["iloc"] = V3.make_iloc(iloc, new_entries)
    final_meta = make_meta(replacements)
    if len(final_meta) != len(provisional_meta):
        raise ValueError("meta size instability")

    if canonical:
        mdat_raw = bytearray(data[old_mdat["pos"]:old_mdat["pos"] + old_mdat["size"]])
        if prefix:
            mdat_raw[0:4] = (old_mdat["size"] + len(prefix)).to_bytes(4, "big")
            mdat_raw = mdat_raw[:old_mdat["header"]] + prefix + mdat_raw[old_mdat["header"]:]
        modified = bytearray(NATIVE_FTYP + final_meta + bytes(mdat_raw))
    else:
        modified = bytearray(data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:])
        if prefix:
            mdat_pos = old_mdat["pos"] + top_delta
            modified[mdat_pos:mdat_pos + 4] = (old_mdat["size"] + len(prefix)).to_bytes(4, "big")
            modified[mdat_pos + old_mdat["header"]:mdat_pos + old_mdat["header"]] = prefix

    modified = bytes(modified)
    Path(output).write_bytes(modified)

    new_top = list(V3.boxes(modified, 0, len(modified)))
    new_meta = next(box for box in new_top if box["type"] == "meta")
    new_children = list(V3.boxes(modified, new_meta["data_start"] + 4, new_meta["data_end"]))
    new_by_type = {box["type"]: box for box in new_children}
    _, new_infos = V3.parse_iinf(modified, new_by_type["iinf"])
    _, new_refs_check = V3.parse_iref(modified, new_by_type["iref"])
    new_iloc = V3.parse_iloc(modified, new_by_type["iloc"])
    new_idat = new_by_type.get("idat")
    new_entry_map = {entry["id"]: entry for entry in new_iloc["entries"]}
    new_hashes = item_hashes(modified, new_iloc, new_idat)
    unexpected = [
        item_id for item_id, digest in old_hashes.items()
        if item_id not in deliberately_changed and new_hashes.get(item_id) != digest
    ]
    if unexpected:
        raise ValueError(f"pre-existing item payloads changed: {unexpected}")
    if provenance and new_hashes[exif_id] == old_hashes[exif_id]:
        raise ValueError("provenance patch did not change Exif")
    plistlib.loads(V3.item_payload(modified, new_entry_map[style_id], new_idat))
    plistlib.loads(V3.item_payload(modified, new_entry_map[texture_id], new_idat))

    if semantic_aliases:
        source_payload = V3.item_payload(data, entry_map[source_image], idat)
        for semantic in semantic_ids:
            if semantic["uri"].encode() + b"\0" not in modified[new_meta["pos"]:new_meta["pos"] + new_meta["size"]]:
                raise ValueError(f"missing semantic auxC {semantic['uri']}")
            if V3.item_payload(modified, new_entry_map[semantic["image"]], new_idat) != source_payload:
                raise ValueError("semantic alias payload changed")

    if canonical:
        if [box["type"] for box in new_top] != ["ftyp", "meta", "mdat"]:
            raise ValueError("canonical top-level order mismatch")
        if [box["type"] for box in new_children][:9] != NATIVE_META_ORDER:
            raise ValueError("canonical meta order mismatch")

    report = {
        "schema": "xdremux-texture-style-cleanroom-v10",
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "styleItemID": style_id,
        "textureItemID": texture_id,
        "exifItemID": exif_id,
        "targets": targets,
        "semanticAliases": semantic_ids,
        "semanticAliasCount": len(semantic_ids),
        "provenancePatched": provenance_info,
        "earlyPackedItemIDs": selected,
        "canonicalContainer": canonical,
        "topLevelBoxes": [box["type"] for box in new_top],
        "metaChildOrder": [box["type"] for box in new_children],
        "preExistingUnrelatedPayloadsByteIdentical": True,
    }
    if manifest:
        Path(manifest).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source")
    parser.add_argument("output")
    parser.add_argument("--semantic-aliases", action="store_true")
    parser.add_argument("--provenance", action="store_true")
    parser.add_argument("--pack-early", action="store_true")
    parser.add_argument("--canonical", action="store_true")
    parser.add_argument("--manifest")
    args = parser.parse_args()
    build(
        args.source,
        args.output,
        semantic_aliases=args.semantic_aliases,
        provenance=args.provenance,
        pack_early=args.pack_early,
        canonical=args.canonical,
        manifest=args.manifest,
    )


if __name__ == "__main__":
    main()
