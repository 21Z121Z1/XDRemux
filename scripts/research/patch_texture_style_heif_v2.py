#!/usr/bin/env python3
import hashlib
import json
import plistlib
import struct
import sys
from pathlib import Path


def u(data: bytes | bytearray, pos: int, width: int) -> int:
    return int.from_bytes(data[pos : pos + width], "big") if width else 0


def put(value: int, width: int) -> bytes:
    if width == 0:
        if value != 0:
            raise ValueError(f"cannot encode non-zero {value} in zero-width field")
        return b""
    if value < 0 or value >= 1 << (8 * width):
        raise ValueError(f"value {value} does not fit {width} bytes")
    return value.to_bytes(width, "big")


def make_box(typ: str, payload: bytes) -> bytes:
    raw_type = typ.encode("latin1")
    if len(raw_type) != 4:
        raise ValueError(f"box type must be 4 bytes: {typ!r}")
    size = 8 + len(payload)
    if size >= (1 << 32):
        return struct.pack(">I4sQ", 1, raw_type, size + 8) + payload
    return struct.pack(">I4s", size, raw_type) + payload


def boxes(data: bytes | bytearray, start: int, end: int):
    pos = start
    while pos + 8 <= end:
        size = u(data, pos, 4)
        typ = bytes(data[pos + 4 : pos + 8]).decode("latin1")
        header = 8
        if size == 1:
            if pos + 16 > end:
                raise ValueError(f"truncated large-size {typ} box at {pos}")
            size = u(data, pos + 8, 8)
            header = 16
        elif size == 0:
            size = end - pos
        if size < header or pos + size > end:
            raise ValueError(f"invalid {typ} box at {pos}: size={size} end={end}")
        yield {
            "pos": pos,
            "size": size,
            "header": header,
            "type": typ,
            "data_start": pos + header,
            "data_end": pos + size,
        }
        pos += size
    if pos != end:
        raise ValueError(f"box walk ended at {pos}, expected {end}")


def parse_iinf(data: bytes, box: dict):
    p = box["data_start"]
    version = data[p]
    p += 4
    if version >= 1:
        count = u(data, p, 4)
        p += 4
    else:
        count = u(data, p, 2)
        p += 2
    entries = []
    children = list(boxes(data, p, box["data_end"]))
    if len(children) != count:
        raise ValueError(f"iinf count={count}, actual infe count={len(children)}")
    for child in children:
        if child["type"] != "infe":
            raise ValueError(f"unexpected iinf child {child['type']}")
        raw = data[child["pos"] : child["pos"] + child["size"]]
        q = child["data_start"]
        infe_version = data[q]
        q += 4
        if infe_version < 2:
            raise ValueError("infe version <2 not supported by v2 probe")
        if infe_version >= 3:
            item_id = u(data, q, 4)
            q += 4
        else:
            item_id = u(data, q, 2)
            q += 2
        protection = u(data, q, 2)
        q += 2
        item_type = data[q : q + 4].decode("latin1")
        entries.append(
            {
                "id": item_id,
                "type": item_type,
                "raw": raw,
                "infe_version": infe_version,
                "protection": protection,
            }
        )
    return {"version": version, "entries": entries}


def make_iinf(version: int, raw_infes: list[bytes]) -> bytes:
    payload = bytes([version, 0, 0, 0])
    payload += put(len(raw_infes), 4 if version >= 1 else 2)
    payload += b"".join(raw_infes)
    return make_box("iinf", payload)


def make_uri_infe(item_id: int, name: str, uri: str, flags: int = 1) -> bytes:
    if not 0 <= item_id <= 0xFFFF:
        raise ValueError("v2 URI infe needs UInt16 item ID")
    payload = bytes(
        [2, (flags >> 16) & 0xFF, (flags >> 8) & 0xFF, flags & 0xFF]
    )
    payload += put(item_id, 2) + b"\x00\x00" + b"uri "
    payload += name.encode("utf-8") + b"\x00" + uri.encode("utf-8") + b"\x00"
    return make_box("infe", payload)


def parse_iref(data: bytes, box: dict):
    p = box["data_start"]
    version = data[p]
    p += 4
    id_width = 4 if version >= 1 else 2
    refs = []
    for child in boxes(data, p, box["data_end"]):
        q = child["data_start"]
        from_id = u(data, q, id_width)
        q += id_width
        count = u(data, q, 2)
        q += 2
        to_ids = []
        for _ in range(count):
            to_ids.append(u(data, q, id_width))
            q += id_width
        if q != child["data_end"]:
            raise ValueError(f"iref child {child['type']} has trailing bytes")
        refs.append({"type": child["type"], "from": from_id, "to": to_ids})
    return {"version": version, "refs": refs}


def make_iref(version: int, refs: list[dict]) -> bytes:
    id_width = 4 if version >= 1 else 2
    payload = bytes([version, 0, 0, 0])
    for ref in refs:
        child = put(ref["from"], id_width) + put(len(ref["to"]), 2)
        child += b"".join(put(item_id, id_width) for item_id in ref["to"])
        payload += make_box(ref["type"], child)
    return make_box("iref", payload)


def parse_iloc(data: bytes, box: dict):
    p = box["data_start"]
    version = data[p]
    flags = data[p + 1 : p + 4]
    p += 4
    sizes0 = data[p]
    p += 1
    sizes1 = data[p]
    p += 1
    offset_size = sizes0 >> 4
    length_size = sizes0 & 0x0F
    base_offset_size = sizes1 >> 4
    index_size = (sizes1 & 0x0F) if version in (1, 2) else 0
    if version < 2:
        count = u(data, p, 2)
        p += 2
        item_id_size = 2
    else:
        count = u(data, p, 4)
        p += 4
        item_id_size = 4
    entries = []
    for _ in range(count):
        item_id = u(data, p, item_id_size)
        p += item_id_size
        construction_field = 0
        construction_method = 0
        if version in (1, 2):
            construction_field = u(data, p, 2)
            p += 2
            construction_method = construction_field & 0x0F
        data_ref = u(data, p, 2)
        p += 2
        base_offset = u(data, p, base_offset_size)
        p += base_offset_size
        extent_count = u(data, p, 2)
        p += 2
        extents = []
        for _ in range(extent_count):
            extent_index = None
            if index_size:
                extent_index = u(data, p, index_size)
                p += index_size
            offset = u(data, p, offset_size)
            p += offset_size
            length = u(data, p, length_size)
            p += length_size
            extents.append(
                {"index": extent_index, "offset": offset, "length": length}
            )
        entries.append(
            {
                "id": item_id,
                "construction_field": construction_field,
                "method": construction_method,
                "data_ref": data_ref,
                "base_offset": base_offset,
                "extents": extents,
            }
        )
    if p != box["data_end"]:
        raise ValueError(f"iloc trailing bytes: {box['data_end'] - p}")
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


def make_iloc(info: dict, entries: list[dict]) -> bytes:
    version = info["version"]
    payload = bytes([version]) + bytes(info["flags"])
    payload += bytes(
        [
            (info["offset_size"] << 4) | info["length_size"],
            (info["base_offset_size"] << 4)
            | (info["index_size"] if version in (1, 2) else 0),
        ]
    )
    payload += put(len(entries), 2 if version < 2 else 4)
    for entry in entries:
        payload += put(entry["id"], info["item_id_size"])
        if version in (1, 2):
            payload += put(
                entry.get("construction_field", entry["method"] & 0x0F), 2
            )
        payload += put(entry.get("data_ref", 0), 2)
        payload += put(entry.get("base_offset", 0), info["base_offset_size"])
        payload += put(len(entry["extents"]), 2)
        for extent in entry["extents"]:
            if info["index_size"]:
                payload += put(extent.get("index") or 0, info["index_size"])
            payload += put(extent["offset"], info["offset_size"])
            payload += put(extent["length"], info["length_size"])
    return make_box("iloc", payload)


def item_payload(data: bytes, entry: dict, idat: dict | None) -> bytes:
    if entry["data_ref"] != 0:
        raise ValueError(f"item {entry['id']} external data_ref unsupported")
    output = bytearray()
    for extent in entry["extents"]:
        absolute = entry["base_offset"] + extent["offset"]
        if entry["method"] == 1:
            if idat is None:
                raise ValueError(f"item {entry['id']} uses idat but idat missing")
            absolute += idat["data_start"]
        elif entry["method"] != 0:
            raise ValueError(
                f"item {entry['id']}: unsupported method {entry['method']}"
            )
        end = absolute + extent["length"]
        if not 0 <= absolute <= end <= len(data):
            raise ValueError(f"item {entry['id']} extent out of range")
        output += data[absolute:end]
    return bytes(output)


def symbol_string(probe: dict, key: str):
    value = (probe.get("symbols") or {}).get(key)
    if isinstance(value, str) and value:
        return value
    if isinstance(value, dict):
        description = value.get("description")
        if isinstance(description, str) and (
            ":" in description or "." in description
        ):
            return description
    return None


def numeric_current_version(probe: dict):
    raw = (
        (probe.get("rawSymbols") or {}).get("_PITextureStyleCurrentMetadataVersion")
        or {}
    )
    hex_bytes = raw.get("first16BytesHex") if isinstance(raw, dict) else None
    if not isinstance(hex_bytes, str) or len(hex_bytes) < 16:
        return None
    try:
        first_eight = bytes.fromhex(hex_bytes[:16])
    except ValueError:
        return None
    options = [
        int.from_bytes(first_eight[:8], "little"),
        int.from_bytes(first_eight[:4], "little"),
    ]
    return next((value for value in options if 0 < value < 256), None)


def candidate_payloads(probe: dict):
    current = numeric_current_version(probe)
    versions = []
    if current is not None:
        versions.append(current)
    for version in (1, 2, 3, 4):
        if version not in versions:
            versions.append(version)

    contexts = []
    hardware_models = ["iPhone18,1", "iPhone17,1"]
    port_types = ["BackWide", "BackWideCamera"]
    capture_types = ["Photo", "StillImage"]
    for version in versions[:4]:
        for hardware in hardware_models:
            for port in port_types:
                for capture_type in capture_types:
                    contexts.append(
                        (
                            f"v{version}-{hardware.replace(',', '_')}-{port}-{capture_type}",
                            {
                                "Version": version,
                                "HardwareModel": hardware,
                                "PortType": port,
                                "CaptureType": capture_type,
                                "CaptureMode": "Photo",
                                "FilmGrainSeed": 12345,
                                "TextureStylePeopleDataVersion": 1,
                                "TextureStylePostProcessedPeopleData": [],
                            },
                        )
                    )

    preferred_versions = versions[:2]
    return [
        (name, payload)
        for name, payload in contexts
        if payload["Version"] in preferred_versions
    ]


def patch_one(
    original: bytes,
    probe: dict,
    payload_obj: dict,
    out_path: Path,
    item_name="textureStyleInfo",
):
    top = list(boxes(original, 0, len(original)))
    meta = next((box for box in top if box["type"] == "meta"), None)
    if meta is None:
        raise ValueError("meta box missing")
    meta_fullbox = original[meta["data_start"] : meta["data_start"] + 4]
    children = list(
        boxes(original, meta["data_start"] + 4, meta["data_end"])
    )
    by_type = {box["type"]: box for box in children}
    for required in ("iinf", "iref", "iloc"):
        if required not in by_type:
            raise ValueError(f"{required} missing")

    iinf_info = parse_iinf(original, by_type["iinf"])
    iref_info = parse_iref(original, by_type["iref"])
    iloc_info = parse_iloc(original, by_type["iloc"])
    idat = by_type.get("idat")

    old_hashes = {
        entry["id"]: hashlib.sha256(item_payload(original, entry, idat)).hexdigest()
        for entry in iloc_info["entries"]
    }
    new_id = max(entry["id"] for entry in iloc_info["entries"]) + 1
    if new_id > 0xFFFF:
        raise ValueError("new item ID exceeds infe v2/iloc v1 range")

    uri = symbol_string(probe, "_kMetadataIdentifier_TextureStyleInfo")
    if not uri:
        raise ValueError(
            "iOS 27 runtime did not resolve _kMetadataIdentifier_TextureStyleInfo; refusing to guess URI"
        )

    style_uri_item = None
    for info in iinf_info["entries"]:
        if info["type"] == "uri " and b"tag:apple.com,2023:photo:metadata:styles\x00" in info["raw"]:
            style_uri_item = info["id"]
            break
    if style_uri_item is None:
        raise ValueError("known SemanticStyle URI item not found")
    style_cdsc = next(
        (
            ref
            for ref in iref_info["refs"]
            if ref["type"] == "cdsc" and ref["from"] == style_uri_item
        ),
        None,
    )
    if style_cdsc is None or not style_cdsc["to"]:
        raise ValueError("SemanticStyle cdsc relationship not found")

    texture_payload = plistlib.dumps(
        payload_obj, fmt=plistlib.FMT_BINARY, sort_keys=False
    )
    raw_infes = [entry["raw"] for entry in iinf_info["entries"]]
    raw_infes.append(make_uri_infe(new_id, item_name, uri, flags=1))
    new_iinf = make_iinf(iinf_info["version"], raw_infes)

    new_refs = list(iref_info["refs"]) + [
        {"type": "cdsc", "from": new_id, "to": list(style_cdsc["to"])}
    ]
    new_iref = make_iref(iref_info["version"], new_refs)

    new_entry = {
        "id": new_id,
        "construction_field": 0,
        "method": 0,
        "data_ref": 0,
        "base_offset": 0,
        "extents": [
            {
                "index": 0 if iloc_info["index_size"] else None,
                "offset": 0,
                "length": len(texture_payload),
            }
        ],
    }
    placeholder_iloc = make_iloc(
        iloc_info, iloc_info["entries"] + [new_entry]
    )

    replacements = {"iinf": new_iinf, "iref": new_iref, "iloc": placeholder_iloc}

    def rebuild_meta(replacement_map):
        body = bytearray(meta_fullbox)
        for child in children:
            body += replacement_map.get(
                child["type"],
                original[child["pos"] : child["pos"] + child["size"]],
            )
        return make_box("meta", bytes(body))

    placeholder_meta = rebuild_meta(replacements)
    meta_delta = len(placeholder_meta) - meta["size"]
    old_meta_end = meta["pos"] + meta["size"]

    adjusted_entries = []
    for entry in iloc_info["entries"]:
        copied = {key: value for key, value in entry.items() if key != "extents"}
        copied["extents"] = []
        for extent in entry["extents"]:
            adjusted = dict(extent)
            if entry["method"] == 0:
                absolute = entry["base_offset"] + adjusted["offset"]
                if absolute >= old_meta_end:
                    adjusted["offset"] += meta_delta
            copied["extents"].append(adjusted)
        adjusted_entries.append(copied)

    new_file_without_extra_mdat_len = len(original) + meta_delta
    new_entry["extents"][0]["offset"] = new_file_without_extra_mdat_len + 8
    replacements["iloc"] = make_iloc(
        iloc_info, adjusted_entries + [new_entry]
    )
    new_meta = rebuild_meta(replacements)
    if len(new_meta) - meta["size"] != meta_delta:
        raise AssertionError("iloc finalization changed meta size unexpectedly")

    result = (
        original[: meta["pos"]]
        + new_meta
        + original[meta["data_end"] :]
        + make_box("mdat", texture_payload)
    )

    top2 = list(boxes(result, 0, len(result)))
    meta2 = next(box for box in top2 if box["type"] == "meta")
    children2 = list(boxes(result, meta2["data_start"] + 4, meta2["data_end"]))
    by_type2 = {box["type"]: box for box in children2}
    iloc2 = parse_iloc(result, by_type2["iloc"])
    idat2 = by_type2.get("idat")
    hashes2 = {
        entry["id"]: hashlib.sha256(item_payload(result, entry, idat2)).hexdigest()
        for entry in iloc2["entries"]
    }
    changed = [
        item_id
        for item_id, digest in old_hashes.items()
        if hashes2.get(item_id) != digest
    ]
    if changed:
        raise AssertionError(f"old item payloads changed: {changed}")
    new_parsed = next(entry for entry in iloc2["entries"] if entry["id"] == new_id)
    if item_payload(result, new_parsed, idat2) != texture_payload:
        raise AssertionError("new TextureStyleInfo payload did not round-trip")

    out_path.write_bytes(result)
    return {
        "file": out_path.name,
        "itemID": new_id,
        "metadataIdentifier": uri,
        "cdscTargets": style_cdsc["to"],
        "payloadBytes": len(texture_payload),
        "payloadSHA256": hashlib.sha256(texture_payload).hexdigest(),
        "fileSHA256": hashlib.sha256(result).hexdigest(),
        "metaDelta": meta_delta,
        "oldPayloadsVerifiedUnchanged": len(old_hashes),
        "payload": payload_obj,
    }


def main() -> int:
    if len(sys.argv) != 4:
        print(
            "usage: patch_texture_style_heif_v2.py BASELINE RUNTIME_PROBE_JSON OUTPUT_DIR",
            file=sys.stderr,
        )
        return 64
    baseline = Path(sys.argv[1])
    probe_path = Path(sys.argv[2])
    output_dir = Path(sys.argv[3])
    output_dir.mkdir(parents=True, exist_ok=True)
    original = baseline.read_bytes()
    probe = json.loads(probe_path.read_text())

    reports = []
    for suffix, payload in candidate_payloads(probe):
        reports.append(
            patch_one(
                original,
                probe,
                payload,
                output_dir / f"texture-info-{suffix}.heic",
            )
        )
    manifest = {
        "schema": "xdremux-texture-style-v2-probe-v1",
        "source": str(baseline),
        "sourceSHA256": hashlib.sha256(original).hexdigest(),
        "runtimeProbe": str(probe_path),
        "candidateCount": len(reports),
        "candidates": reports,
    }
    (output_dir / "candidate-manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False)
    )
    print(json.dumps(manifest, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
