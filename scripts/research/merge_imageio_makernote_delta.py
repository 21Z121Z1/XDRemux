#!/usr/bin/env python3
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from patch_texture_style_heif_v3 import (  # noqa: E402
    boxes,
    parse_iinf,
    parse_iloc,
    item_payload,
    extract_makernote,
    replace_makernote,
    parse_apple_makernote,
    make_iloc,
    make_box,
    clone_iloc_entry,
)

TYPE_SIZES = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 7: 1, 9: 4, 10: 8, 12: 8}


def jpeg_exif_payload(data: bytes) -> bytes:
    if not data.startswith(b"\xff\xd8"):
        raise ValueError("not JPEG")
    p = 2
    while p + 4 <= len(data):
        if data[p] != 0xFF:
            p += 1
            continue
        marker = data[p + 1]
        p += 2
        if marker in (0xD8, 0xD9):
            continue
        if marker == 0xDA:
            break
        n = int.from_bytes(data[p:p+2], "big")
        if n < 2 or p + n > len(data):
            raise ValueError("bad JPEG segment")
        payload = data[p+2:p+n]
        if marker == 0xE1 and payload.startswith(b"Exif\0\0"):
            return b"\0\0\0\0" + payload
        p += n
    raise ValueError("JPEG Exif APP1 missing")


def maker_from_jpeg(path: str) -> bytes:
    exif = jpeg_exif_payload(Path(path).read_bytes())
    note, _ = extract_makernote(exif)
    return note


def entries_map(note: bytes):
    endian, entries = parse_apple_makernote(note)
    return endian, {tag: (field_type, raw) for tag, field_type, raw in entries}


def rebuild_note(original_note: bytes, replacements: dict[int, tuple[int, bytes]]) -> bytes:
    endian, entries = parse_apple_makernote(original_note)
    merged = []
    seen = set()
    for tag, field_type, raw in entries:
        if tag in replacements:
            field_type, raw = replacements[tag]
        merged.append((tag, field_type, raw))
        seen.add(tag)
    for tag in sorted(set(replacements) - seen):
        field_type, raw = replacements[tag]
        merged.append((tag, field_type, raw))

    base_offset = 14 + 2 + 12 * len(merged) + 4
    table = bytearray()
    values = bytearray()
    for tag, field_type, raw in merged:
        unit = TYPE_SIZES.get(field_type, 1)
        if len(raw) % unit:
            raise ValueError(f"tag {tag}: payload length incompatible with type {field_type}")
        count = len(raw) // unit
        table += tag.to_bytes(2, endian)
        table += field_type.to_bytes(2, endian)
        table += count.to_bytes(4, endian)
        if len(raw) <= 4:
            table += raw + b"\0" * (4 - len(raw))
        else:
            table += (base_offset + len(values)).to_bytes(4, endian)
            values += raw
    return original_note[:14] + len(merged).to_bytes(2, endian) + bytes(table) + b"\0\0\0\0" + bytes(values)


def patch_heif(source: str, new_note: bytes, output: str):
    data = Path(source).read_bytes()
    top = list(boxes(data, 0, len(data)))
    meta = next(b for b in top if b["type"] == "meta")
    children = list(boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {b["type"]: b for b in children}
    _, infos = parse_iinf(data, by_type["iinf"])
    iloc = parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    item_types = {i["id"]: i["type"] for i in infos}
    exif_id = next(i for i, t in item_types.items() if t == "Exif")
    old_entries = {e["id"]: e for e in iloc["entries"]}
    old_exif = item_payload(data, old_entries[exif_id], idat)
    new_exif = replace_makernote(old_exif, new_note)

    new_entries = [clone_iloc_entry(e) for e in iloc["entries"]]
    old_meta_end = meta["pos"] + meta["size"]
    # iloc size does not change: only the Exif extent location/length changes.
    # Relocate any method-0 extents after meta only if meta size somehow changes.
    exif_entry = next(e for e in new_entries if e["id"] == exif_id)
    exif_entry["construction_field"] &= 0xFFF0
    exif_entry["method"] = 0
    exif_entry["data_ref"] = 0
    exif_entry["base_offset"] = 0

    replacements = {"iloc": make_iloc(iloc, new_entries)}
    provisional_children = [
        replacements.get(c["type"], data[c["pos"]:c["pos"] + c["size"]])
        for c in children
    ]
    provisional_meta = make_box(
        "meta",
        data[meta["data_start"]:meta["data_start"] + 4] + b"".join(provisional_children),
    )
    delta = len(provisional_meta) - meta["size"]
    if delta:
        for e in new_entries:
            if e["method"] == 0:
                for x in e["extents"]:
                    absolute = e["base_offset"] + x["offset"]
                    if absolute >= old_meta_end:
                        x["offset"] += delta

    new_exif_offset = len(data) + delta + 8
    exif_entry["extents"] = [{"index": None, "offset": new_exif_offset, "length": len(new_exif)}]
    replacements["iloc"] = make_iloc(iloc, new_entries)
    final_children = [
        replacements.get(c["type"], data[c["pos"]:c["pos"] + c["size"]])
        for c in children
    ]
    final_meta = make_box(
        "meta",
        data[meta["data_start"]:meta["data_start"] + 4] + b"".join(final_children),
    )
    modified = data[:meta["pos"]] + final_meta + data[meta["pos"] + meta["size"]:]
    modified += make_box("mdat", new_exif)
    Path(output).write_bytes(modified)
    return {
        "sourceSHA256": hashlib.sha256(data).hexdigest(),
        "outputSHA256": hashlib.sha256(modified).hexdigest(),
        "exifItemID": exif_id,
        "oldMakerNoteBytes": len(extract_makernote(old_exif)[0]),
        "newMakerNoteBytes": len(new_note),
    }


def main():
    if len(sys.argv) != 6:
        print("usage: merge_imageio_makernote_delta.py SOURCE_HEIC BASELINE_JPEG CANDIDATE_JPEG OUTPUT_HEIC REPORT_JSON", file=sys.stderr)
        return 64
    source, baseline_jpeg, candidate_jpeg, output, report_path = sys.argv[1:]
    base_note = maker_from_jpeg(baseline_jpeg)
    candidate_note = maker_from_jpeg(candidate_jpeg)
    _, base = entries_map(base_note)
    _, cand = entries_map(candidate_note)
    delta = {
        tag: value for tag, value in cand.items()
        if tag not in base or base[tag] != value
    }
    removed = sorted(set(base) - set(cand))
    if removed:
        raise ValueError(f"candidate encoder removed baseline tags: {removed}")
    if not delta:
        raise ValueError("ImageIO produced no MakerNote delta")
    if len(delta) > 8:
        raise ValueError(f"unexpectedly broad MakerNote delta: {sorted(delta)}")

    source_data = Path(source).read_bytes()
    top = list(boxes(source_data, 0, len(source_data)))
    meta = next(b for b in top if b["type"] == "meta")
    children = list(boxes(source_data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {b["type"]: b for b in children}
    _, infos = parse_iinf(source_data, by_type["iinf"])
    iloc = parse_iloc(source_data, by_type["iloc"])
    idat = by_type.get("idat")
    item_types = {i["id"]: i["type"] for i in infos}
    exif_id = next(i for i, t in item_types.items() if t == "Exif")
    entry_map = {e["id"]: e for e in iloc["entries"]}
    original_note, _ = extract_makernote(item_payload(source_data, entry_map[exif_id], idat))
    new_note = rebuild_note(original_note, delta)
    patch = patch_heif(source, new_note, output)

    report = {
        **patch,
        "deltaTags": [
            {
                "tag": tag,
                "type": delta[tag][0],
                "bytes": len(delta[tag][1]),
                "sha256": hashlib.sha256(delta[tag][1]).hexdigest(),
                "previewHex": delta[tag][1][:64].hex(),
            }
            for tag in sorted(delta)
        ],
        "baselineEncodedTags": sorted(base),
        "candidateEncodedTags": sorted(cand),
    }
    Path(report_path).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
