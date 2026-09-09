#!/usr/bin/env python3
import hashlib
import json
import plistlib
import struct
import sys
from pathlib import Path


def u(data, pos, width):
    return int.from_bytes(data[pos:pos + width], "big") if width else 0


def put(value, width):
    if width == 0:
        if value:
            raise ValueError("non-zero value in zero-width field")
        return b""
    return value.to_bytes(width, "big")


def make_box(box_type, payload):
    return struct.pack(">I4s", 8 + len(payload), box_type.encode("latin1")) + payload


def boxes(data, start, end):
    pos = start
    while pos + 8 <= end:
        size = u(data, pos, 4)
        box_type = data[pos + 4:pos + 8].decode("latin1")
        header = 8
        if size == 1:
            size = u(data, pos + 8, 8)
            header = 16
        elif size == 0:
            size = end - pos
        if size < header or pos + size > end:
            raise ValueError(f"invalid {box_type} box at {pos}")
        yield {
            "pos": pos,
            "size": size,
            "header": header,
            "type": box_type,
            "data_start": pos + header,
            "data_end": pos + size,
        }
        pos += size
    if pos != end:
        raise ValueError(f"box walk stopped {end - pos} bytes early")


def parse_iinf(data, box):
    pos = box["data_start"]
    version = data[pos]
    pos += 4
    count_width = 4 if version >= 1 else 2
    count = u(data, pos, count_width)
    pos += count_width
    entries = []
    for child in boxes(data, pos, box["data_end"]):
        p = child["data_start"]
        infe_version = data[p]
        flags = u(data, p + 1, 3)
        p += 4
        item_id_width = 4 if infe_version >= 3 else 2
        item_id = u(data, p, item_id_width)
        p += item_id_width
        p += 2
        item_type = data[p:p + 4].decode("latin1")
        entries.append({
            "id": item_id,
            "type": item_type,
            "flags": flags,
            "raw": data[child["pos"]:child["pos"] + child["size"]],
        })
    if len(entries) != count:
        raise ValueError(f"iinf count mismatch: {count} != {len(entries)}")
    return version, entries


def make_iinf(version, raw_infes):
    count_width = 4 if version >= 1 else 2
    payload = bytes([version, 0, 0, 0]) + put(len(raw_infes), count_width)
    payload += b"".join(raw_infes)
    return make_box("iinf", payload)


def make_uri_infe(item_id, name, uri):
    if item_id > 0xFFFF:
        raise ValueError("URI infe v2 item ID exceeds UInt16")
    payload = bytes([2, 0, 0, 1]) + put(item_id, 2) + b"\0\0" + b"uri "
    payload += name.encode("utf-8") + b"\0" + uri.encode("utf-8") + b"\0"
    return make_box("infe", payload)


def parse_iref(data, box):
    pos = box["data_start"]
    version = data[pos]
    pos += 4
    id_width = 4 if version >= 1 else 2
    refs = []
    for child in boxes(data, pos, box["data_end"]):
        p = child["data_start"]
        from_id = u(data, p, id_width)
        p += id_width
        count = u(data, p, 2)
        p += 2
        to_ids = []
        for _ in range(count):
            to_ids.append(u(data, p, id_width))
            p += id_width
        refs.append({"type": child["type"], "from": from_id, "to": to_ids})
    return version, refs


def make_iref(version, refs):
    id_width = 4 if version >= 1 else 2
    payload = bytes([version, 0, 0, 0])
    for ref in refs:
        body = put(ref["from"], id_width) + put(len(ref["to"]), 2)
        body += b"".join(put(item_id, id_width) for item_id in ref["to"])
        payload += make_box(ref["type"], body)
    return make_box("iref", payload)


def parse_iloc(data, box):
    pos = box["data_start"]
    version = data[pos]
    flags = data[pos + 1:pos + 4]
    pos += 4
    sizes0 = data[pos]
    sizes1 = data[pos + 1]
    pos += 2
    offset_size = sizes0 >> 4
    length_size = sizes0 & 0x0F
    base_offset_size = sizes1 >> 4
    index_size = (sizes1 & 0x0F) if version in (1, 2) else 0
    count_width = 4 if version >= 2 else 2
    item_count = u(data, pos, count_width)
    pos += count_width
    item_id_size = 4 if version >= 2 else 2
    entries = []
    for _ in range(item_count):
        item_id = u(data, pos, item_id_size)
        pos += item_id_size
        construction_field = 0
        method = 0
        if version in (1, 2):
            construction_field = u(data, pos, 2)
            pos += 2
            method = construction_field & 0x0F
        data_ref = u(data, pos, 2)
        pos += 2
        base_offset = u(data, pos, base_offset_size)
        pos += base_offset_size
        extent_count = u(data, pos, 2)
        pos += 2
        extents = []
        for _ in range(extent_count):
            extent_index = None
            if index_size:
                extent_index = u(data, pos, index_size)
                pos += index_size
            offset = u(data, pos, offset_size)
            pos += offset_size
            length = u(data, pos, length_size)
            pos += length_size
            extents.append({"index": extent_index, "offset": offset, "length": length})
        entries.append({
            "id": item_id,
            "construction_field": construction_field,
            "method": method,
            "data_ref": data_ref,
            "base_offset": base_offset,
            "extents": extents,
        })
    return {
        "version": version,
        "flags": flags,
        "offset_size": offset_size,
        "length_size": length_size,
        "base_offset_size": base_offset_size,
        "index_size": index_size,
        "item_id_size": item_id_size,
        "entries": entries,
    }


def make_iloc(info, entries):
    version = info["version"]
    payload = bytes([version]) + info["flags"]
    payload += bytes([
        (info["offset_size"] << 4) | info["length_size"],
        (info["base_offset_size"] << 4)
        | (info["index_size"] if version in (1, 2) else 0),
    ])
    payload += put(len(entries), 4 if version >= 2 else 2)
    for entry in entries:
        payload += put(entry["id"], info["item_id_size"])
        if version in (1, 2):
            payload += put(entry.get("construction_field", entry["method"] & 0x0F), 2)
        payload += put(entry.get("data_ref", 0), 2)
        payload += put(entry.get("base_offset", 0), info["base_offset_size"])
        payload += put(len(entry["extents"]), 2)
        for extent in entry["extents"]:
            if info["index_size"]:
                payload += put(extent.get("index") or 0, info["index_size"])
            payload += put(extent["offset"], info["offset_size"])
            payload += put(extent["length"], info["length_size"])
    return make_box("iloc", payload)


def item_payload(data, entry, idat):
    output = bytearray()
    for extent in entry["extents"]:
        absolute = entry["base_offset"] + extent["offset"]
        if entry["method"] == 1:
            if idat is None:
                raise ValueError(f"item {entry['id']} uses idat but idat is missing")
            absolute += idat["data_start"]
        elif entry["method"] != 0:
            raise ValueError(f"unsupported construction method {entry['method']}")
        output += data[absolute:absolute + extent["length"]]
    return bytes(output)


def extract_makernote(exif_payload):
    if len(exif_payload) < 14 or exif_payload[4:10] != b"Exif\0\0":
        raise ValueError("unexpected HEIF Exif item prefix")
    tiff = exif_payload[10:]
    endian = "big" if tiff[:2] == b"MM" else "little"

    def read(pos, width):
        return int.from_bytes(tiff[pos:pos + width], endian)

    ifd0 = read(4, 4)
    count = read(ifd0, 2)
    exif_ifd = None
    for index in range(count):
        pos = ifd0 + 2 + index * 12
        if read(pos, 2) == 0x8769:
            exif_ifd = read(pos + 8, 4)
            break
    if exif_ifd is None:
        raise ValueError("ExifIFD pointer is missing")
    count = read(exif_ifd, 2)
    maker_entry = None
    for index in range(count):
        pos = exif_ifd + 2 + index * 12
        if read(pos, 2) == 0x927C:
            maker_entry = pos
            break
    if maker_entry is None:
        raise ValueError("MakerNote tag is missing")
    length = read(maker_entry + 4, 4)
    offset = read(maker_entry + 8, 4)
    return tiff[offset:offset + length], (10, maker_entry, endian)


def replace_makernote(exif_payload, new_note):
    _, (prefix_size, maker_entry, endian) = extract_makernote(exif_payload)
    tiff = bytearray(exif_payload[prefix_size:])
    if len(tiff) & 1:
        tiff.append(0)
    offset = len(tiff)
    tiff.extend(new_note)
    tiff[maker_entry + 4:maker_entry + 8] = len(new_note).to_bytes(4, endian)
    tiff[maker_entry + 8:maker_entry + 12] = offset.to_bytes(4, endian)
    return exif_payload[:prefix_size] + bytes(tiff)


def parse_apple_makernote(note):
    if not note.startswith(b"Apple iOS\0\0\x01"):
        raise ValueError("unexpected Apple MakerNote header")
    endian = "big" if note[12:14] == b"MM" else "little"
    count = int.from_bytes(note[14:16], endian)
    type_sizes = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8, 12: 8}
    entries = []
    for index in range(count):
        pos = 16 + index * 12
        tag = int.from_bytes(note[pos:pos + 2], endian)
        field_type = int.from_bytes(note[pos + 2:pos + 4], endian)
        item_count = int.from_bytes(note[pos + 4:pos + 8], endian)
        size = type_sizes.get(field_type, 1) * item_count
        offset = pos + 8 if size <= 4 else int.from_bytes(note[pos + 8:pos + 12], endian)
        entries.append((tag, field_type, note[offset:offset + size]))
    return endian, entries


def rebuild_apple_makernote(note, new_tag84_payload):
    endian, entries = parse_apple_makernote(note)
    replaced = []
    for tag, field_type, raw in entries:
        replaced.append((tag, 7, new_tag84_payload) if tag == 84 else (tag, field_type, raw))
    type_sizes = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8, 12: 8}
    base_offset = 14 + 2 + 12 * len(replaced) + 4
    table = bytearray()
    values = bytearray()
    for tag, field_type, raw in replaced:
        unit = type_sizes.get(field_type, 1)
        count = len(raw) // unit
        table += tag.to_bytes(2, endian)
        table += field_type.to_bytes(2, endian)
        table += count.to_bytes(4, endian)
        if len(raw) <= 4:
            table += raw + b"\0" * (4 - len(raw))
        else:
            table += (base_offset + len(values)).to_bytes(4, endian)
            values += raw
    return note[:14] + len(replaced).to_bytes(2, endian) + bytes(table) + b"\0\0\0\0" + bytes(values)


def clone_iloc_entry(entry):
    return {
        "id": entry["id"],
        "construction_field": entry["construction_field"],
        "method": entry["method"],
        "data_ref": entry["data_ref"],
        "base_offset": entry["base_offset"],
        "extents": [dict(extent) for extent in entry["extents"]],
    }


def patch_candidates(source, contract, output_dir):
    data = Path(source).read_bytes()
    top = list(boxes(data, 0, len(data)))
    meta = next(box for box in top if box["type"] == "meta")
    children = list(boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {box["type"]: box for box in children}
    iinf_version, infos = parse_iinf(data, by_type["iinf"])
    iref_version, refs = parse_iref(data, by_type["iref"])
    iloc = parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    entry_map = {entry["id"]: entry for entry in iloc["entries"]}
    item_types = {info["id"]: info["type"] for info in infos}
    exif_id = next(item_id for item_id, item_type in item_types.items() if item_type == "Exif")
    style_info = next(
        info for info in infos
        if info["type"] == "uri "
        and b"tag:apple.com,2023:photo:metadata:styles\0" in info["raw"]
    )
    style_id = style_info["id"]
    style_targets = next(
        ref["to"] for ref in refs
        if ref["type"] == "cdsc" and ref["from"] == style_id
    )
    exif_payload = item_payload(data, entry_map[exif_id], idat)
    old_note, _ = extract_makernote(exif_payload)
    _, old_note_entries = parse_apple_makernote(old_note)
    old_tag84 = next(raw for tag, _, raw in old_note_entries if tag == 84)
    old_tag84_object = plistlib.loads(old_tag84)
    old_hashes = {
        entry["id"]: hashlib.sha256(item_payload(data, entry, idat)).hexdigest()
        for entry in iloc["entries"]
    }

    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest = {
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "exifItemID": exif_id,
        "styleItemID": style_id,
        "styleTargets": style_targets,
        "baselineMaker84": old_tag84_object,
        "candidates": [],
    }
    maker_variants = {"baseline": old_tag84_object}
    maker_variants.update(contract.get("maker84Variants", {}))
    texture_variants = contract.get("textureInfoVariants", {})

    for candidate in contract.get("combinations", []):
        name = candidate["name"]
        maker_name = candidate.get("maker84", "baseline")
        texture_name = candidate.get("textureInfo")
        maker_object = maker_variants[maker_name]
        new_tag84 = plistlib.dumps(maker_object, fmt=plistlib.FMT_BINARY, sort_keys=False)
        new_note = rebuild_apple_makernote(old_note, new_tag84)
        new_exif = replace_makernote(exif_payload, new_note) if maker_name != "baseline" else None

        texture_object = texture_variants.get(texture_name) if texture_name else None
        texture_payload = (
            plistlib.dumps(texture_object, fmt=plistlib.FMT_BINARY, sort_keys=False)
            if texture_object is not None else None
        )
        new_item_id = max(entry_map) + 1 if texture_payload else None

        raw_infes = [info["raw"] for info in infos]
        if new_item_id:
            raw_infes.append(make_uri_infe(
                new_item_id,
                "textureStyleMetadata",
                contract["textureURI"],
            ))
        new_refs = [
            {"type": ref["type"], "from": ref["from"], "to": list(ref["to"])}
            for ref in refs
        ]
        if new_item_id:
            new_refs.append({"type": "cdsc", "from": new_item_id, "to": list(style_targets)})

        new_entries = [clone_iloc_entry(entry) for entry in iloc["entries"]]
        if new_item_id:
            new_entries.append({
                "id": new_item_id,
                "construction_field": 0,
                "method": 0,
                "data_ref": 0,
                "base_offset": 0,
                "extents": [{"index": None, "offset": 0, "length": len(texture_payload)}],
            })

        replacements = {
            "iinf": make_iinf(iinf_version, raw_infes),
            "iref": make_iref(iref_version, new_refs),
            "iloc": make_iloc(iloc, new_entries),
        }
        new_children = [
            replacements.get(child["type"], data[child["pos"]:child["pos"] + child["size"]])
            for child in children
        ]
        new_meta = make_box(
            "meta",
            data[meta["data_start"]:meta["data_start"] + 4] + b"".join(new_children),
        )
        delta = len(new_meta) - meta["size"]
        old_meta_end = meta["pos"] + meta["size"]

        for entry in new_entries:
            if entry["id"] not in entry_map:
                continue
            if entry["method"] == 0:
                for extent in entry["extents"]:
                    absolute = entry["base_offset"] + extent["offset"]
                    if absolute >= old_meta_end:
                        extent["offset"] += delta

        appended_payloads = []
        cursor = len(data) + delta + 8
        if new_exif is not None:
            exif_entry = next(entry for entry in new_entries if entry["id"] == exif_id)
            exif_entry["construction_field"] &= 0xFFF0
            exif_entry["method"] = 0
            exif_entry["data_ref"] = 0
            exif_entry["base_offset"] = 0
            exif_entry["extents"] = [{"index": None, "offset": cursor, "length": len(new_exif)}]
            appended_payloads.append(new_exif)
            cursor += len(new_exif)
        if new_item_id:
            texture_entry = next(entry for entry in new_entries if entry["id"] == new_item_id)
            texture_entry["extents"][0]["offset"] = cursor
            appended_payloads.append(texture_payload)
            cursor += len(texture_payload)

        replacements["iloc"] = make_iloc(iloc, new_entries)
        new_children = [
            replacements.get(child["type"], data[child["pos"]:child["pos"] + child["size"]])
            for child in children
        ]
        final_meta = make_box(
            "meta",
            data[meta["data_start"]:meta["data_start"] + 4] + b"".join(new_children),
        )
        if len(final_meta) != len(new_meta):
            raise ValueError("meta size changed after offset relocation")
        modified = data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:]
        if appended_payloads:
            modified += make_box("mdat", b"".join(appended_payloads))

        output = output_dir / f"{name}.heic"
        output.write_bytes(modified)

        new_top = list(boxes(modified, 0, len(modified)))
        new_meta_box = next(box for box in new_top if box["type"] == "meta")
        new_meta_children = list(boxes(
            modified,
            new_meta_box["data_start"] + 4,
            new_meta_box["data_end"],
        ))
        new_by_type = {box["type"]: box for box in new_meta_children}
        new_iloc = parse_iloc(modified, new_by_type["iloc"])
        new_idat = new_by_type.get("idat")
        new_hashes = {
            entry["id"]: hashlib.sha256(item_payload(modified, entry, new_idat)).hexdigest()
            for entry in new_iloc["entries"]
        }
        allowed_changed = {exif_id} if new_exif is not None else set()
        unexpectedly_changed = [
            item_id for item_id, digest in old_hashes.items()
            if item_id not in allowed_changed and new_hashes.get(item_id) != digest
        ]
        if unexpectedly_changed:
            raise ValueError(f"{name}: old payloads changed unexpectedly: {unexpectedly_changed}")

        manifest["candidates"].append({
            "name": name,
            "file": output.name,
            "maker84": maker_name,
            "textureInfo": texture_name,
            "sha256": hashlib.sha256(modified).hexdigest(),
            "payloadInvariant": True,
            "exifChanged": new_exif is not None,
            "newTextureItemID": new_item_id,
        })

    manifest_path = output_dir / "candidate-manifest-v3.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, default=str))
    print(json.dumps(manifest, indent=2, default=str))


def main():
    if len(sys.argv) != 4:
        raise SystemExit("usage: patch_texture_style_heif_v3.py BASELINE CONTRACT_JSON OUTPUT_DIR")
    patch_candidates(
        sys.argv[1],
        json.loads(Path(sys.argv[2]).read_text()),
        sys.argv[3],
    )


if __name__ == "__main__":
    main()
