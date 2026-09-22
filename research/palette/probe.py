#!/usr/bin/env python3
"""Construct explicitly unverified Palette experiments, never product output.

Only the observed backwards-offset JSON footer is supported. The Rust CLI
remains the product inspector. This module neither decodes nor converts images.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import struct
import tempfile
from typing import Any

CONTEXT_NAMES = frozenset({
    "basictone.info", "basictone.lmtlut.table", "basictone.vig.table",
    "hdr.transform.data",
})
MAX_MANIFEST_BYTES = 1024 * 1024
MAX_ENTRIES = 4096


@dataclass(frozen=True)
class Entry:
    fields: dict[str, Any]
    start: int
    end: int

    @property
    def name(self) -> str:
        return self.fields["name"]


@dataclass(frozen=True)
class Manifest:
    data: bytes
    json_start: int
    tag: bytes
    entries: tuple[Entry, ...]

    def payload(self, entry: Entry) -> bytes:
        return self.data[entry.start:entry.end]


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON value: {value}")


def parse(data: bytes) -> Manifest:
    """Reject ambiguous/truncated tails instead of searching arbitrary bytes."""
    if len(data) < 11 or data[-9] != 0:
        raise ValueError("missing supported manifest footer")
    span = struct.unpack_from("<I", data, len(data) - 4)[0]
    if not 11 <= span <= min(len(data), MAX_MANIFEST_BYTES + 9):
        raise ValueError("manifest span is out of bounds")
    tag = data[-8:-4]
    if any(byte < 32 or byte > 126 for byte in tag):
        raise ValueError("unsupported manifest tag")
    start = len(data) - span
    try:
        fields = json.loads(data[start:-9].decode("utf-8"),
                            object_pairs_hook=_unique_object,
                            parse_constant=_reject_constant)
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise ValueError("invalid manifest JSON") from error
    if not isinstance(fields, list) or not 1 <= len(fields) <= MAX_ENTRIES:
        raise ValueError("manifest must contain a bounded nonempty entry list")
    names: set[str] = set()
    entries = []
    for record in fields:
        if not isinstance(record, dict):
            raise ValueError("manifest entry is not an object")
        name, offset, length = (record.get(key) for key in ("name", "offset", "length"))
        if not isinstance(name, str) or not name or name in names:
            raise ValueError("missing or duplicate manifest entry name")
        if type(offset) is not int or type(length) is not int:
            raise ValueError("manifest offset and length must be integers, not booleans")
        if not 0 <= length <= offset <= start:
            raise ValueError(f"manifest payload is out of bounds: {name}")
        names.add(name)
        entries.append(Entry(record, start - offset, start - offset + length))
    end = 0
    for entry in sorted((e for e in entries if e.start != e.end), key=lambda e: e.start):
        if entry.start < end:
            raise ValueError("overlapping manifest payloads")
        end = entry.end
    return Manifest(data, start, tag, tuple(entries))


def _encode_manifest(records: list[dict[str, Any]], tag: bytes) -> bytes:
    payload = json.dumps(records, ensure_ascii=False, allow_nan=False,
                         separators=(",", ":")).encode("utf-8")
    if len(payload) > MAX_MANIFEST_BYTES:
        raise ValueError("resulting manifest is too large")
    return payload + b"\0" + tag + struct.pack("<I", len(payload) + 9)


def _c_string(value: str, size: int) -> bytes:
    encoded = value.encode("utf-8")
    if b"\0" in encoded or len(encoded) >= size:
        raise ValueError(f"string does not fit a {size}-byte C field")
    return encoded + bytes(size - len(encoded))


def filter_info(filter_type: str, capture_mode: str) -> bytes:
    """The historical 220-byte v5.1 hypothesis, NOT measured source facts."""
    if filter_type not in ("palette-default", "default"):
        raise ValueError("unsupported filter.info hypothesis")
    result = bytearray(220)
    struct.pack_into("<f6i", result, 0, 5.1, 100, 0, 0, 0, 0, 1)
    result[28:128] = _c_string(filter_type, 100)
    struct.pack_into("<3f4i", result, 128, 6500.0, 100.0, 0.0, 1, 1, 0, 0)
    result[156:206] = _c_string(capture_mode, 50)
    struct.pack_into("<3f", result, 208, 0.18, 0.75, 0.03)
    return bytes(result)


def replace_filter_info(source: bytes, filter_type: str, capture_mode: str) -> bytes:
    """Append the new payload; preserve ALL original pre-manifest bytes.

    Unknown records and gaps survive. An old filter payload is left unreferenced,
    not removed by a lossy repack. Only filter.info and relative offsets change.
    """
    manifest = parse(source)
    payload = filter_info(filter_type, capture_mode)
    records = []
    found = False
    for entry in manifest.entries:
        record = dict(entry.fields)
        if entry.name == "filter.info":
            record.update(offset=len(payload), length=len(payload), version=1)
            found = True
        else:
            record["offset"] += len(payload)
        records.append(record)
    if not found:
        records.append(dict(name="filter.info", offset=len(payload), length=len(payload), version=1))
    result = source[:manifest.json_start] + payload + _encode_manifest(records, manifest.tag)
    reparsed = parse(result)
    actual = {entry.name: reparsed.payload(entry) for entry in reparsed.entries}
    if any(actual[e.name] != manifest.payload(e) for e in manifest.entries if e.name != "filter.info"):
        raise ValueError("untouched extension payload changed")
    return result


def carry_context(source: bytes, base_heic: bytes) -> bytes:
    """Carry only existing render context onto a separately decoded HEIC base.

    The caller must derive that base from the same source. This function makes
    no claim about image conversion, Gallery admission, or reversible editing.
    """
    if len(base_heic) < 16 or base_heic[4:8] != b"ftyp":
        raise ValueError("base is not an ISO-BMFF image candidate")
    manifest = parse(source)
    selected = [e for e in manifest.entries if e.name in CONTEXT_NAMES]
    if not selected:
        raise ValueError("source has no Palette/BasicTone render context")
    physical = sorted(selected, key=lambda e: e.start)
    payload = b"".join(manifest.payload(e) for e in physical)
    offsets = {}
    cursor = 0
    for entry in physical:
        offsets[entry.name] = len(payload) - cursor
        cursor += entry.end - entry.start
    records = [dict(e.fields, offset=offsets[e.name]) for e in selected]
    result = base_heic + payload + _encode_manifest(records, manifest.tag)
    reparsed = parse(result)
    if any(reparsed.payload(e) != manifest.payload(old)
           for e, old in zip(reparsed.entries, selected, strict=True)):
        raise ValueError("selected render context changed")
    return result


def publish_new(path: Path, data: bytes) -> None:
    """Publish a complete new file, never replace a file or follow its symlink.

    A same-directory hard link is an atomic no-clobber operation. Unsupported
    filesystems fail explicitly; there is no replace()/rename() fallback.
    """
    fd, temporary = tempfile.mkstemp(prefix=".palette-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.link(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("filter-info", "context"))
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--base-heic", type=Path)
    parser.add_argument("--filter-type", choices=("palette-default", "default"), default="palette-default")
    parser.add_argument("--capture-mode", default="common",
                        help="explicit synthetic hypothesis; not inferred from missing source metadata")
    args = parser.parse_args()
    if (args.mode == "context") != (args.base_heic is not None):
        parser.error("--base-heic is required only for the context experiment")
    try:
        source = args.source.read_bytes()
        base = args.base_heic.read_bytes() if args.base_heic else None
        result = (carry_context(source, base) if base is not None else
                  replace_filter_info(source, args.filter_type, args.capture_mode))
        report = {
            "schema": "xdremux-palette-experiment-v1", "research_only": True,
            "consumer_verdict": "unverified", "mode": args.mode,
            "source_sha256": hashlib.sha256(source).hexdigest(),
            "base_sha256": hashlib.sha256(base).hexdigest() if base is not None else None,
            "output_sha256": hashlib.sha256(result).hexdigest(),
            "entries": [e.name for e in parse(result).entries],
            "hypothesis": None if base is not None else {
                "layout": "FilterPhotoInfoV51", "filter_type": args.filter_type,
                "capture_mode": args.capture_mode, "synthetic_bytes": 220,
            },
        }
        publish_new(args.output, result)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
