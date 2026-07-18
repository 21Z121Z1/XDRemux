#!/usr/bin/env python3
"""Read-only OPPO HEIF/HEIC structure and extension-container inspector.

The script intentionally uses only the Python standard library.  It does not
decode, rewrite, normalize, or otherwise mutate the input file.  Default output
is compact; ``--json`` is intended for a file under /tmp or an evidence folder.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from pathlib import Path
from typing import Any, Iterable


def u16(data: bytes, pos: int) -> int:
    return struct.unpack_from(">H", data, pos)[0]


def u32(data: bytes, pos: int) -> int:
    return struct.unpack_from(">I", data, pos)[0]


def u64(data: bytes, pos: int) -> int:
    return struct.unpack_from(">Q", data, pos)[0]


def box_header(data: bytes, start: int, limit: int) -> tuple[str, int, int, int]:
    if start + 8 > limit:
        raise ValueError(f"truncated box header at {start}")
    size = u32(data, start)
    typ = data[start + 4 : start + 8].decode("latin1")
    header = 8
    if size == 1:
        if start + 16 > limit:
            raise ValueError(f"truncated extended box header at {start}")
        size = u64(data, start + 8)
        header = 16
    elif size == 0:
        size = limit - start
    if size < header or start + size > limit:
        raise ValueError(f"invalid {typ} box size={size} at {start}")
    return typ, start + header, start + size, size


def boxes(data: bytes, start: int, end: int) -> Iterable[dict[str, Any]]:
    pos = start
    while pos < end:
        typ, payload_start, box_end, size = box_header(data, pos, end)
        yield {
            "type": typ,
            "start": pos,
            "payload_start": payload_start,
            "end": box_end,
            "size": size,
        }
        pos = box_end


def fullbox(data: bytes, box: dict[str, Any]) -> tuple[int, int, int]:
    pos = box["payload_start"]
    if pos + 4 > box["end"]:
        raise ValueError(f"short FullBox {box['type']}")
    value = u32(data, pos)
    return value >> 24, value & 0xFFFFFF, pos + 4


def ascii0(data: bytes, pos: int, end: int) -> tuple[str, int]:
    stop = data.find(b"\0", pos, end)
    if stop < 0:
        stop = end
    return data[pos:stop].decode("utf-8", "replace"), min(stop + 1, end)


def parse_ftyp(data: bytes, box: dict[str, Any]) -> dict[str, Any]:
    pos = box["payload_start"]
    major = data[pos : pos + 4].decode("latin1")
    minor = u32(data, pos + 4)
    brands = [data[p : p + 4].decode("latin1") for p in range(pos + 8, box["end"], 4)]
    return {"major": major, "minor": minor, "compatible": brands}


def parse_infe(data: bytes, box: dict[str, Any]) -> dict[str, Any]:
    version, flags, pos = fullbox(data, box)
    if version == 2:
        item_id = u16(data, pos)
        pos += 2
        protection = u16(data, pos)
        pos += 2
        item_type = data[pos : pos + 4].decode("latin1")
        pos += 4
        item_name, pos = ascii0(data, pos, box["end"])
        content_type = content_encoding = None
        if item_type == "mime":
            content_type, pos = ascii0(data, pos, box["end"])
            content_encoding, pos = ascii0(data, pos, box["end"])
        return {
            "id": item_id,
            "version": version,
            "flags": flags,
            "protection": protection,
            "type": item_type,
            "name": item_name,
            "content_type": content_type,
            "content_encoding": content_encoding,
        }
    if version == 3:
        item_id = u32(data, pos)
        pos += 4
        protection = u16(data, pos)
        pos += 2
        item_type = data[pos : pos + 4].decode("latin1")
        pos += 4
        item_name, _ = ascii0(data, pos, box["end"])
        return {
            "id": item_id,
            "version": version,
            "flags": flags,
            "protection": protection,
            "type": item_type,
            "name": item_name,
            "content_type": None,
            "content_encoding": None,
        }
    return {"version": version, "flags": flags, "raw": True}


def parse_iinf(data: bytes, box: dict[str, Any]) -> dict[int, dict[str, Any]]:
    version, _, pos = fullbox(data, box)
    count = u32(data, pos) if version >= 1 else u16(data, pos)
    pos += 4 if version >= 1 else 2
    result: dict[int, dict[str, Any]] = {}
    for child in boxes(data, pos, box["end"]):
        if child["type"] != "infe":
            continue
        entry = parse_infe(data, child)
        if "id" in entry:
            result[entry["id"]] = entry
    if len(result) != count:
        # Keep the mismatch visible; some vendor files contain malformed counts.
        result[-1] = {"declared_count": count, "parsed_count": len(result)}
    return result


def read_uint(data: bytes, pos: int, size: int) -> tuple[int, int]:
    if size == 0:
        return 0, pos
    return int.from_bytes(data[pos : pos + size], "big"), pos + size


def parse_iloc(data: bytes, box: dict[str, Any]) -> dict[int, dict[str, Any]]:
    version, _, pos = fullbox(data, box)
    sizes0 = data[pos]
    sizes1 = data[pos + 1]
    offset_size = sizes0 >> 4
    length_size = sizes0 & 0x0F
    base_size = sizes1 >> 4
    index_size = (sizes1 & 0x0F) if version in (1, 2) else 0
    pos += 2
    item_count, pos = read_uint(data, pos, 4 if version >= 2 else 2)
    result: dict[int, dict[str, Any]] = {}
    for _ in range(item_count):
        item_id, pos = read_uint(data, pos, 4 if version >= 2 else 2)
        construction_method = 0
        if version in (1, 2):
            construction_method = u16(data, pos) & 0x0F
            pos += 2
        data_ref, pos = read_uint(data, pos, 2)
        base_offset, pos = read_uint(data, pos, base_size)
        extent_count, pos = read_uint(data, pos, 2)
        extents = []
        for _ in range(extent_count):
            extent_index, pos = read_uint(data, pos, index_size)
            offset, pos = read_uint(data, pos, offset_size)
            length, pos = read_uint(data, pos, length_size)
            extents.append({
                "index": extent_index,
                "offset": base_offset + offset,
                "length": length,
            })
        result[item_id] = {
            "construction_method": construction_method,
            "data_reference": data_ref,
            "base_offset": base_offset,
            "extents": extents,
        }
    return result


def parse_iref(data: bytes, box: dict[str, Any]) -> list[dict[str, Any]]:
    version, _, pos = fullbox(data, box)
    id_size = 4 if version >= 1 else 2
    result = []
    while pos + 8 <= box["end"]:
        child_type, child_pos, child_end, _ = box_header(data, pos, box["end"])
        if child_end <= child_pos:
            break
        from_id = int.from_bytes(data[child_pos : child_pos + id_size], "big")
        count_pos = child_pos + id_size
        count = u16(data, count_pos)
        cursor = count_pos + 2
        to = [int.from_bytes(data[cursor + i * id_size : cursor + (i + 1) * id_size], "big")
              for i in range(count)]
        result.append({"type": child_type, "from": from_id, "to": to})
        pos = child_end
    return result


def parse_iprp(data: bytes, box: dict[str, Any]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    ipco = next((b for b in boxes(data, box["payload_start"], box["end"]) if b["type"] == "ipco"), None)
    ipma = next((b for b in boxes(data, box["payload_start"], box["end"]) if b["type"] == "ipma"), None)
    props = []
    if ipco:
        for index, child in enumerate(boxes(data, ipco["payload_start"], ipco["end"]), 1):
            details: dict[str, Any] = {}
            p = child["payload_start"]
            if child["type"] == "hvcC" and child["end"] - p >= 19:
                details = {
                    "profile_idc": data[p + 1] & 0x1F,
                    "level_idc": data[p + 12],
                    "chroma_format_idc": data[p + 16] & 0x03,
                    "bit_depth_luma": (data[p + 17] & 0x07) + 8,
                    "bit_depth_chroma": (data[p + 18] & 0x07) + 8,
                }
            elif child["type"] == "ispe" and child["end"] - p >= 12:
                details = {"width": u32(data, p + 4), "height": u32(data, p + 8)}
            elif child["type"] == "pixi" and child["end"] - p >= 5:
                count = data[p + 4]
                details = {"channels": count, "bits_per_channel": list(data[p + 5 : p + 5 + count])}
            elif child["type"] == "colr" and child["end"] - p >= 4:
                kind = data[p : p + 4].decode("latin1")
                details = {"kind": kind}
                if kind == "nclx" and child["end"] - p >= 11:
                    details.update({
                        "primaries": u16(data, p + 4),
                        "transfer": u16(data, p + 6),
                        "matrix": u16(data, p + 8),
                        "full_range": bool(data[p + 10] & 0x80),
                    })
            elif child["type"] == "auxC" and child["end"] - p >= 4:
                aux_type, _ = ascii0(data, p + 4, child["end"])
                details = {"aux_type": aux_type}
            props.append({
                "index": index,
                "type": child["type"],
                "offset": child["start"],
                "size": child["size"],
                "sha256": hashlib.sha256(data[child["start"] : child["end"]]).hexdigest(),
                "raw_hex": data[child["start"] : min(child["end"], child["start"] + 32)].hex(),
                "details": details,
            })
    associations = []
    if ipma:
        version, flags, pos = fullbox(data, ipma)
        count = u32(data, pos)
        pos += 4
        item_id_size = 4 if flags & 1 else 2
        association_size = 2 if flags & 1 else 1
        for _ in range(count):
            item_id, pos = read_uint(data, pos, item_id_size)
            assoc_count = data[pos]
            pos += 1
            values = []
            for _ in range(assoc_count):
                value, pos = read_uint(data, pos, association_size)
                mask = 0x7FFF if flags & 1 else 0x7F
                values.append({"property": value & mask, "essential": bool(value & (0x8000 if flags & 1 else 0x80))})
            associations.append({"item": item_id, "properties": values})
    return props, associations


def parse_exif_markers(payload: bytes) -> dict[str, Any]:
    # HEIF Exif item convention: 4-byte offset to TIFF header, then TIFF/IFD.
    result: dict[str, Any] = {"present": bool(payload), "user_comment": False, "maker_note": False}
    if len(payload) < 8:
        return result
    offset = u32(payload, 0)
    tiff_start = 4 + offset
    if tiff_start + 8 > len(payload):
        return result
    tiff = payload[tiff_start:]
    if tiff.startswith(b"Exif\x00\x00"):
        tiff = tiff[6:]
    endian = tiff[:2]
    if endian not in (b"II", b"MM"):
        return result
    little = endian == b"II"
    order = "<" if little else ">"
    if struct.unpack_from(order + "H", tiff, 2)[0] != 42:
        return result

    def read16(p: int) -> int:
        return struct.unpack_from(order + "H", tiff, p)[0]

    def read32(p: int) -> int:
        return struct.unpack_from(order + "I", tiff, p)[0]

    type_sizes = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8}

    def value_bytes(entry: int, kind: int) -> bytes:
        count = read32(entry + 4)
        size = type_sizes.get(kind, 1) * count
        start = entry + 8 if size <= 4 else read32(entry + 8)
        if start < 0 or start + size > len(tiff):
            return b""
        return tiff[start : start + size]

    def scan_ifd(offset_ifd: int, depth: int = 0) -> None:
        if depth > 4 or offset_ifd + 2 > len(tiff):
            return
        count = read16(offset_ifd)
        for i in range(count):
            entry = offset_ifd + 2 + i * 12
            if entry + 12 > len(tiff):
                return
            tag = read16(entry)
            kind = read16(entry + 2)
            if tag == 0x9286:
                result["user_comment"] = True
                raw = value_bytes(entry, kind)
                for prefix in (b"ASCIIOplus_", b"ASCIIoppo_", b"Oplus_", b"oplus_", b"oppo_"):
                    marker = raw.find(prefix)
                    if marker >= 0:
                        end = marker + len(prefix)
                        while end < len(raw) and 48 <= raw[end] <= 57:
                            end += 1
                        result["user_comment_value"] = raw[marker:end].decode("ascii")
                        break
            elif tag == 0x927C:
                result["maker_note"] = True
                raw = value_bytes(entry, kind)
                result["maker_note_length"] = len(raw)
                result["maker_note_sha256"] = hashlib.sha256(raw).hexdigest()
            if tag == 0x8769 or tag == 0x8825:
                value = read32(entry + 8)
                scan_ifd(value, depth + 1)
            # A MakerNote may itself contain a nested TIFF, but presence is enough.

    result["tiff_offset"] = tiff_start
    result["byte_order"] = endian.decode()
    scan_ifd(read32(4))
    return result


def payload_for_item(data: bytes, entry: dict[str, Any], idat: dict[str, Any] | None) -> bytes:
    chunks = []
    for extent in entry.get("extents", []):
        start = extent["offset"]
        if entry.get("construction_method") == 1:
            if idat is None:
                return b""
            start = idat["payload_start"] + extent["offset"]
        end = start + extent["length"]
        if start < 0 or end > len(data):
            return b""
        chunks.append(data[start:end])
    return b"".join(chunks)


def parse_extension_tail(data: bytes, mdat_end: int) -> dict[str, Any]:
    tail = data[mdat_end:]
    result: dict[str, Any] = {"offset": mdat_end, "length": len(tail), "sha256": hashlib.sha256(tail).hexdigest()}
    if not tail:
        result["entries"] = []
        return result
    marker, footer_tag = max(
        ((tail.rfind(tag), tag) for tag in (b"jxrs", b"wtmk")),
        key=lambda value: value[0],
    )
    result["jxrs_offset_in_tail"] = marker if marker >= 0 else None
    result["footer_tag"] = footer_tag.decode("ascii") if marker >= 0 else None
    if marker >= 4:
        result["jxrs_size_le"] = int.from_bytes(tail[marker - 4 : marker], "little")
        if marker + 8 <= len(tail):
            result["footer_size_le"] = int.from_bytes(tail[marker + 4 : marker + 8], "little")
    for marker_name in (b"QTI Debug", b"QTI "):
        qti = tail.find(marker_name)
        if qti >= 4:
            box_start = qti - 4
            box_size = u32(tail, box_start)
            result["qti_box_offset_in_tail"] = box_start
            result["qti_box_size"] = box_size
            result["extension_data_offset_in_tail"] = box_start + box_size
            break

    # FileExtendedContainer manifests are JSON immediately before a NUL + jxrs footer.
    json_end = marker - 1 if marker > 0 and tail[marker - 1] == 0 else marker
    start = tail.rfind(b"[{", 0, json_end)
    if start >= 0:
        try:
            manifest = json.loads(tail[start:json_end].decode("utf-8"))
            result["manifest"] = manifest
            result["manifest_offset_in_tail"] = start
            json_abs = mdat_end + start
            for entry in manifest:
                if not isinstance(entry, dict):
                    continue
                try:
                    length = int(entry["length"])
                    offset = int(entry["offset"])
                except (KeyError, TypeError, ValueError):
                    continue
                begin = json_abs - offset
                end = begin + length
                record = {
                    "name": entry.get("name"),
                    "length": length,
                    "offset_from_manifest": offset,
                    "start": begin,
                    "end": end,
                    "version": entry.get("version"),
                }
                if 0 <= begin <= end <= len(data):
                    blob = data[begin:end]
                    record.update({
                        "sha256": hashlib.sha256(blob).hexdigest(),
                        "magic": blob[:12].hex(),
                    })
                    if entry.get("name") == "local.uhdr.gainmap.info" and len(blob) >= 80:
                        record["float32_le"] = list(struct.unpack("<20f", blob[:80]))
                result.setdefault("entries", []).append(record)
        except (UnicodeDecodeError, json.JSONDecodeError):
            result["manifest_parse"] = "failed"
    result["prefix_sha256"] = hashlib.sha256(tail[: max(0, marker)]).hexdigest()
    return result


def inspect(path: Path) -> dict[str, Any]:
    data = path.read_bytes()
    top = []
    for top_box in boxes(data, 0, len(data)):
        top.append(top_box)
        # OPPO's FileExtendedContainer starts after the ISO mdat box and is
        # not an ISOBMFF top-level box stream.  Do not parse it as boxes.
        if top_box["type"] == "mdat":
            break
    meta = next((b for b in top if b["type"] == "meta"), None)
    mdat = next((b for b in top if b["type"] == "mdat"), None)
    if meta is None:
        raise ValueError("no top-level meta box")
    meta_children = list(boxes(data, meta["payload_start"] + 4, meta["end"]))
    child_by_type = {b["type"]: b for b in meta_children}
    idat = child_by_type.get("idat")
    items = parse_iinf(data, child_by_type["iinf"]) if "iinf" in child_by_type else {}
    items = {k: v for k, v in items.items() if k >= 0}
    locations = parse_iloc(data, child_by_type["iloc"]) if "iloc" in child_by_type else {}
    refs = parse_iref(data, child_by_type["iref"]) if "iref" in child_by_type else []
    props, associations = parse_iprp(data, child_by_type["iprp"]) if "iprp" in child_by_type else ([], [])
    prop_types = {p["index"]: p["type"] for p in props}
    prop_by_index = {p["index"]: p for p in props}
    item_rows = []
    for item_id, info in sorted(items.items()):
        loc = locations.get(item_id, {})
        payload = payload_for_item(data, loc, idat)
        row = dict(info)
        row.update({
            "id": item_id,
            "location": loc,
            "payload_length": len(payload),
            "payload_sha256": hashlib.sha256(payload).hexdigest() if payload else None,
        })
        item_assoc = next((a for a in associations if a["item"] == item_id), None)
        if item_assoc:
            row["properties"] = [
                prop_by_index.get(value["property"], {"index": value["property"], "type": None})
                | {"essential": value["essential"]}
                for value in item_assoc["properties"]
            ]
        if info.get("type") == "Exif":
            row["exif_markers"] = parse_exif_markers(payload)
        if info.get("type") == "mime" and payload:
            row["payload_prefix"] = payload[:96].decode("utf-8", "replace")
            row["xmp_namespaces"] = [
                value.decode("ascii", "ignore")
                for value in (b"http://ns.adobe.com", b"http://ns.oplus.com", b"http://ns.google.com")
                if value in payload
            ]
        item_rows.append(row)
    item_by_id = {item["id"]: item for item in item_rows}
    codec_summary = []
    for ref in refs:
        if ref["type"] != "dimg" or not ref["to"]:
            continue
        parent = item_by_id.get(ref["from"])
        child = item_by_id.get(ref["to"][0])
        if not parent or not child or parent.get("type") != "grid":
            continue
        hvc = next((p for p in child.get("properties", []) if p.get("type") == "hvcC"), None)
        codec_summary.append({
            "grid_item": ref["from"],
            "first_tile": ref["to"][0],
            "tile_count": len(ref["to"]),
            "hvcC": hvc.get("details") if hvc else None,
        })
    assoc_rows = []
    for assoc in associations:
        assoc_rows.append({
            "item": assoc["item"],
            "properties": [
                {"index": value["property"], "type": prop_types.get(value["property"]), "essential": value["essential"]}
                for value in assoc["properties"]
            ],
        })
    logical = {
        "file": {"path": str(path), "size": len(data), "sha256": hashlib.sha256(data).hexdigest()},
        "top_level": [
            {k: b[k] for k in ("type", "start", "size", "end")}
            | {"sha256": hashlib.sha256(data[b["start"] : b["end"]]).hexdigest()}
            for b in top
        ],
        "ftyp": parse_ftyp(data, top[0]) if top and top[0]["type"] == "ftyp" else None,
        "meta_children": [
            {k: b[k] for k in ("type", "start", "size", "end")}
            | {"sha256": hashlib.sha256(data[b["start"] : b["end"]]).hexdigest()}
            for b in meta_children
        ],
        "primary_item": None,
        "items": item_rows,
        "references": refs,
        "properties": props,
        "associations": assoc_rows,
        "codec_summary": codec_summary,
        "unknown_top_level": [b["type"] for b in top if b["type"] not in {"ftyp", "meta", "mdat", "idat"}],
        "extension_tail": parse_extension_tail(data, mdat["end"] if mdat else len(data)),
    }
    pitm = child_by_type.get("pitm")
    if pitm:
        version, _, pos = fullbox(data, pitm)
        logical["primary_item"] = int.from_bytes(data[pos : pos + (4 if version else 2)], "big")
    return logical


def compact_report(report: dict[str, Any]) -> str:
    item_counts: dict[str, int] = {}
    for item in report["items"]:
        item_counts[item.get("type", "?")] = item_counts.get(item.get("type", "?"), 0) + 1
    lines = [
        f"file: {report['file']['path']}",
        f"size: {report['file']['size']} sha256: {report['file']['sha256']}",
        f"top: {' '.join(b['type'] for b in report['top_level'])}",
        f"ftyp: {report.get('ftyp')}",
        f"primary: {report.get('primary_item')}",
        f"item_counts: {item_counts}",
        f"codec_summary: {report['codec_summary']}",
        "items:",
    ]
    for item in report["items"]:
        if item.get("type") in {"grid", "tmap", "Exif", "mime", "jpeg"}:
            loc = item.get("location", {})
            extents = ",".join(f"{e['offset']}+{e['length']}" for e in loc.get("extents", []))
            lines.append(
                f"  {item['id']:>5} {item.get('type','?'):>4} len={item.get('payload_length')} "
                f"extent={extents} sha={str(item.get('payload_sha256'))[:16]}"
            )
            if item.get("properties"):
                lines.append("    props: " + str([(p.get("index"), p.get("type"), p.get("details")) for p in item["properties"]]))
            if item.get("exif_markers"):
                lines.append(f"    exif: {item['exif_markers']}")
    lines.append("references:")
    for ref in report["references"]:
        targets = ref["to"]
        if len(targets) > 8:
            target_text = f"{targets[:3]} ... {targets[-3:]} (count={len(targets)})"
        else:
            target_text = str(targets)
        lines.append(f"  {ref['type']} {ref['from']} -> {target_text}")
    lines.append(f"extension_tail: {report['extension_tail']}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("--json", action="store_true", help="emit complete JSON")
    args = parser.parse_args(argv)
    try:
        report = inspect(args.input)
    except (OSError, ValueError, struct.error) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    if args.json:
        json.dump(report, sys.stdout, ensure_ascii=False, indent=2)
        sys.stdout.write("\n")
    else:
        print(compact_report(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
