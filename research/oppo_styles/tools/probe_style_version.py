#!/usr/bin/env python3
"""Research-only, byte-exact Styles v15/v16 single-variable carrier probe.

This does not create a complete v16 Texture Styles asset and must never be
used as a product conversion or as proof that Photos adjustments are active.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import struct


def root_version_offset(plist: bytes, expected_version: int = 15) -> int:
    if not plist.startswith(b"bplist00") or len(plist) < 40:
        raise ValueError("Styles payload is not a binary property list")
    offset_bytes, ref_bytes, count, root_index, table = struct.unpack(
        ">6xBBQQQ", plist[-32:]
    )
    if not 1 <= offset_bytes <= 8 or not 1 <= ref_bytes <= 8 or count == 0:
        raise ValueError("Invalid property-list trailer")
    if root_index >= count or table + count * offset_bytes > len(plist) - 32:
        raise ValueError("Invalid object offset table")
    offsets = [
        int.from_bytes(plist[table + i * offset_bytes:table + (i + 1) * offset_bytes], "big")
        for i in range(count)
    ]
    if any(o < 8 or o >= table for o in offsets):
        raise ValueError("Invalid object offset")
    root = offsets[root_index]
    marker = plist[root]
    if marker >> 4 != 0xD:
        raise ValueError("Styles top-level object is not a dictionary")
    size = marker & 0x0F
    cursor = root + 1
    if size == 0x0F:
        length_marker = plist[cursor]
        if length_marker >> 4 != 1:
            raise ValueError("Invalid extended dictionary length")
        n = 1 << (length_marker & 0x0F)
        size = int.from_bytes(plist[cursor + 1:cursor + 1 + n], "big")
        cursor += 1 + n
    if cursor + size * ref_bytes * 2 > table:
        raise ValueError("Dictionary references overflow")
    key_refs = [
        int.from_bytes(plist[cursor + i * ref_bytes:cursor + (i + 1) * ref_bytes], "big")
        for i in range(size)
    ]
    value_cursor = cursor + size * ref_bytes
    value_refs = [
        int.from_bytes(plist[value_cursor + i * ref_bytes:value_cursor + (i + 1) * ref_bytes], "big")
        for i in range(size)
    ]
    for key_index, value_index in zip(key_refs, value_refs):
        if key_index >= count or value_index >= count:
            raise ValueError("Object reference out of range")
        pos = offsets[key_index]
        if plist[pos:pos + 2] != bytes((0x51, 0x30)):  # ASCII string "0"
            continue
        pos = offsets[value_index]
        if plist[pos] != 0x10 or plist[pos + 1] != expected_version:
            raise ValueError("Styles version is not the expected one-byte integer")
        return pos + 1
    raise ValueError("Styles root version key '0' was not found")


def change_only_version(payload: bytes, target_version: int = 16) -> bytes:
    if target_version not in (15, 16):
        raise ValueError("Only v15/v16 version-byte probes are supported")
    source_version = 15 if target_version == 16 else 16
    original = plistlib.loads(payload)
    if not isinstance(original, dict) or original.get("0") != source_version:
        raise ValueError("Source Styles version differs from expected version")
    changed = bytearray(payload)
    changed[root_version_offset(payload, source_version)] = target_version
    result = plistlib.loads(changed)
    if result != {**original, "0": target_version}:
        raise ValueError("Version patch changed an unrelated property-list field")
    if sum(a != b for a, b in zip(payload, changed)) != 1:
        raise ValueError("Version patch did not change exactly one byte")
    return bytes(changed)


def patch_heic(
    source: bytes, inspection: dict, target_version: int = 16
) -> tuple[bytes, dict]:
    items = [
        item for item in inspection["items"]
        if item.get("type") == "uri " and item.get("name") == "styleMetadata"
    ]
    idats = [box for box in inspection["meta_children"] if box.get("type") == "idat"]
    if len(items) != 1 or len(idats) != 1:
        raise ValueError("Expected one Styles metadata item and one idat box")
    extents = items[0]["location"]["extents"]
    if len(extents) != 1:
        raise ValueError("Styles metadata is not one contiguous idat extent")
    extent = extents[0]
    start = idats[0]["start"] + 8 + extent["offset"]
    end = start + extent["length"]
    if start < 0 or end > len(source) or start >= end:
        raise ValueError("Styles extent is outside the source container")
    original = source[start:end]
    changed = change_only_version(original, target_version)
    patched = source[:start] + changed + source[end:]
    if len(patched) != len(source):
        raise ValueError("A single-variable experiment cannot resize the HEIF")
    differences = [i for i, (a, b) in enumerate(zip(source, patched)) if a != b]
    if len(differences) != 1:
        raise ValueError("Other HEIF resources changed")
    receipt = {
        "hypothesis": "version field only; no Texture Style carrier",
        "source_sha256": hashlib.sha256(source).hexdigest(),
        "candidate_sha256": hashlib.sha256(patched).hexdigest(),
        "changed_byte_offset": differences[0],
        "before": 15 if target_version == 16 else 16,
        "after": target_version,
        "other_bytes_identical": True,
        "scope": "structural/native-parser diagnostic, not active Photos editing",
    }
    return patched, receipt


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--inspect", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--target-version", type=int, choices=(15, 16), default=16)
    args = parser.parse_args()
    source = args.input.read_bytes()
    inspection = json.loads(args.inspect.read_text(encoding="utf-8"))
    changed, receipt = patch_heic(source, inspection, args.target_version)
    if args.output.exists() or args.report.exists():
        parser.error("Refusing to overwrite an existing output or report")
    args.output.write_bytes(changed)
    args.report.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(receipt, sort_keys=True))


if __name__ == "__main__":
    main()
