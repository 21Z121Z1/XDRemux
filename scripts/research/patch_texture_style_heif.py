#!/usr/bin/env python3
import hashlib
import json
import plistlib
import struct
import sys
from pathlib import Path


def u(data: bytes | bytearray, pos: int, width: int) -> int:
    return int.from_bytes(data[pos : pos + width], "big") if width else 0


def boxes(data: bytes, start: int, end: int):
    pos = start
    while pos + 8 <= end:
        size = u(data, pos, 4)
        typ = data[pos + 4 : pos + 8].decode("latin1")
        header = 8
        if size == 1:
            if pos + 16 > end:
                break
            size = u(data, pos + 8, 8)
            header = 16
        elif size == 0:
            size = end - pos
        if size < header or pos + size > end:
            break
        yield pos, size, header, typ
        pos += size


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: patch_texture_style_heif.py BASELINE PROBE_JSON OUTPUT_DIR", file=sys.stderr)
        return 64

    src = Path(sys.argv[1])
    probe_path = Path(sys.argv[2])
    out_dir = Path(sys.argv[3])
    out_dir.mkdir(parents=True, exist_ok=True)

    original = src.read_bytes()
    probe = json.loads(probe_path.read_text())

    top = list(boxes(original, 0, len(original)))
    meta = next((box for box in top if box[3] == "meta"), None)
    if not meta:
        raise SystemExit("meta box not found")

    meta_pos, meta_size, meta_header, _ = meta
    children_start = meta_pos + meta_header + 4  # FullBox version/flags.
    children_end = meta_pos + meta_size
    children = list(boxes(original, children_start, children_end))
    iloc = next((box for box in children if box[3] == "iloc"), None)
    idat = next((box for box in children if box[3] == "idat"), None)
    if not iloc:
        raise SystemExit("iloc box not found")
    idat_payload_start = None if not idat else idat[0] + idat[2]

    pos, _size, header, _ = iloc
    p = pos + header
    iloc_version = original[p]
    p += 4
    sizes0 = original[p]
    p += 1
    sizes1 = original[p]
    p += 1
    offset_size = sizes0 >> 4
    length_size = sizes0 & 0x0F
    base_offset_size = sizes1 >> 4
    index_size = (sizes1 & 0x0F) if iloc_version in (1, 2) else 0

    if iloc_version < 2:
        item_count = u(original, p, 2)
        p += 2
        item_id_size = 2
    else:
        item_count = u(original, p, 4)
        p += 4
        item_id_size = 4

    entries = []
    for _ in range(item_count):
        item_id = u(original, p, item_id_size)
        p += item_id_size
        method_pos = None
        method = 0
        if iloc_version in (1, 2):
            method_pos = p
            construction = u(original, p, 2)
            p += 2
            method = construction & 0x000F
        data_ref_pos = p
        data_ref = u(original, p, 2)
        p += 2
        base_pos = p
        base = u(original, p, base_offset_size)
        p += base_offset_size
        extent_count = u(original, p, 2)
        p += 2
        extents = []
        for _ in range(extent_count):
            if iloc_version in (1, 2) and index_size:
                p += index_size
            off_pos = p
            off = u(original, p, offset_size)
            p += offset_size
            len_pos = p
            length = u(original, p, length_size)
            p += length_size
            extents.append((off_pos, off, len_pos, length))
        entries.append(
            {
                "item_id": item_id,
                "method_pos": method_pos,
                "method": method,
                "data_ref_pos": data_ref_pos,
                "data_ref": data_ref,
                "base_pos": base_pos,
                "base": base,
                "extents": extents,
            }
        )

    def payload_for(entry):
        if entry["data_ref"] != 0 or len(entry["extents"]) != 1:
            return None
        _, off, _, length = entry["extents"][0]
        if not length:
            return None
        if entry["method"] == 0:
            absolute = entry["base"] + off
        elif entry["method"] == 1 and idat_payload_start is not None:
            absolute = idat_payload_start + entry["base"] + off
        else:
            return None
        if absolute < 0 or absolute + length > len(original):
            return None
        return original[absolute : absolute + length]

    style_entry = None
    style_object = None
    for entry in entries:
        payload = payload_for(entry)
        if not payload or not payload.startswith(b"bplist00"):
            continue
        try:
            obj = plistlib.loads(payload)
        except Exception:
            continue
        if (
            isinstance(obj, dict)
            and obj.get("0") == 15
            and isinstance(obj.get("1"), bytes)
            and len(obj["1"]) == 51_840
        ):
            style_entry = entry
            style_object = obj
            break

    if not style_entry or style_object is None:
        raise SystemExit("semantic style bplist item was not found through iloc")
    if len(style_entry["extents"]) != 1 or not offset_size or not length_size:
        raise SystemExit("style item layout cannot be safely redirected")

    symbols = probe.get("symbols", {})
    private_key = symbols.get("PITextureStyleAdjustmentKey") or "PITextureStyleAdjustmentKey"
    defaults = probe.get("defaultStyles") or []

    def pick_style(word: str, fallback: dict):
        word = word.lower()
        for item in defaults:
            if word in str(item.get("description", "")).lower():
                dictionary = item.get("dictionaryRepresentation")
                if isinstance(dictionary, dict):
                    return dictionary
        return fallback

    studio = pick_style("studio", {"Preset": "Studio", "Intensity": 1.0, "Grain": 0.0})
    soft = pick_style("soft", {"Preset": "Soft", "Intensity": 1.0, "Grain": 0.0})
    film = pick_style("film", {"Preset": "Filmic", "Intensity": 1.0, "Grain": 1.0})

    def enriched(style: dict, rendering_version: int):
        result = dict(style)
        result.setdefault("Preset", result.get("preset", "Studio"))
        result.setdefault("Intensity", result.get("intensity", 1.0))
        result.setdefault("Grain", result.get("grain", 0.0))
        result["RenderingVersion"] = rendering_version
        result["OriginalInsteadOfReversibility"] = False
        return result

    candidates = []
    for rendering_version in (1, 2, 3):
        candidates.append(
            (
                f"private-studio-v{rendering_version}",
                private_key,
                enriched(studio, rendering_version),
            )
        )
        candidates.append(
            (
                f"private-film-grain-v{rendering_version}",
                private_key,
                enriched(film, rendering_version),
            )
        )
    candidates += [
        ("private-soft-v2", private_key, enriched(soft, 2)),
        ("TextureStyle-studio-v2", "TextureStyle", enriched(studio, 2)),
        ("textureStyle-studio-v2", "textureStyle", enriched(studio, 2)),
        ("short-l-studio-v2", "l", enriched(studio, 2)),
    ]

    flat = dict(style_object)
    flat.update(
        {
            "TextureStylePreset": "Studio",
            "TextureStyleIntensity": 1.0,
            "TextureStyleGrain": 0.0,
            "TextureStyleRenderingVersion": 2,
            "TextureStyleOriginalInsteadOfReversibility": False,
        }
    )

    manifest = {
        "sourceSHA256": hashlib.sha256(original).hexdigest(),
        "ilocVersion": iloc_version,
        "styleItemID": style_entry["item_id"],
        "resolvedAdjustmentKey": private_key,
        "runtimeDefaultStyles": defaults,
        "candidates": [],
    }

    def write_candidate(name: str, obj: dict):
        payload = plistlib.dumps(obj, fmt=plistlib.FMT_BINARY, sort_keys=False)
        mutable = bytearray(original)
        payload_start = len(mutable) + 8
        if payload_start >= (1 << (8 * offset_size)):
            raise SystemExit("iloc offset field is too small for appended payload")
        if len(payload) >= (1 << (8 * length_size)):
            raise SystemExit("iloc length field is too small for candidate payload")

        mutable.extend(struct.pack(">I4s", 8 + len(payload), b"mdat"))
        mutable.extend(payload)

        if style_entry["method_pos"] is not None:
            construction = u(mutable, style_entry["method_pos"], 2)
            construction &= 0xFFF0  # construction_method = 0 (file offset).
            mutable[
                style_entry["method_pos"] : style_entry["method_pos"] + 2
            ] = construction.to_bytes(2, "big")
        mutable[
            style_entry["data_ref_pos"] : style_entry["data_ref_pos"] + 2
        ] = (0).to_bytes(2, "big")
        if base_offset_size:
            mutable[
                style_entry["base_pos"] : style_entry["base_pos"] + base_offset_size
            ] = b"\x00" * base_offset_size

        off_pos, _, len_pos, _ = style_entry["extents"][0]
        mutable[off_pos : off_pos + offset_size] = payload_start.to_bytes(offset_size, "big")
        mutable[len_pos : len_pos + length_size] = len(payload).to_bytes(length_size, "big")

        output = out_dir / f"{name}.heic"
        output.write_bytes(mutable)
        manifest["candidates"].append(
            {
                "name": name,
                "file": output.name,
                "stylePayloadBytes": len(payload),
                "rootKeys": [str(key) for key in obj.keys()],
            }
        )

    for name, key, texture in candidates:
        obj = dict(style_object)
        obj[key] = texture
        write_candidate(name, obj)
    write_candidate("flat-firmware-keys", flat)

    manifest_path = out_dir / "candidate-manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, default=str))
    print(json.dumps(manifest, indent=2, default=str))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
