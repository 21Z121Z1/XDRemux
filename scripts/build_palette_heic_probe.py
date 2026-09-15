#!/usr/bin/env python3
"""Build a HEIC compatibility probe from an existing OPPO/OnePlus image.

The probe is deliberately narrow: it carries only metadata families that are
needed by the Palette/BasicTone render path under test. It does not mutate the
source fixture and it does not claim device-side acceptance; that remains a
manual validation step.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path
from typing import Any

from xdremux_py import container


BASIC_TONE_ENTRY_NAMES = {
    "basictone.info",
    "basictone.lmtlut.table",
    "basictone.vig.table",
}

# Palette processing is HDR-aware. Keep the existing per-image HDR transform
# alongside BasicTone payloads when the source fixture has it, but do not copy
# unrelated preset, watermark, portrait, or filter metadata into this probe.
PALETTE_CONTEXT_ENTRY_NAMES = BASIC_TONE_ENTRY_NAMES | {
    "hdr.transform.data",
}


def _footer_tag(data: bytes) -> bytes:
    if len(data) >= 9 and data[-9] == 0:
        tag = data[-8:-4]
        if len(tag) == 4 and all(32 <= byte <= 126 for byte in tag):
            return tag
    return b"jxrs"


def extract_manifested_payloads(data: bytes) -> tuple[list[tuple[dict[str, Any], bytes]], bytes]:
    parsed = container.parse_manifest(data)
    if parsed is None:
        raise ValueError("source does not contain a parseable OPPO extension manifest")

    entries, json_start, _ = parsed
    result: list[tuple[dict[str, Any], bytes]] = []
    for entry in entries:
        name = str(entry.get("name", ""))
        try:
            length = int(entry["length"])
            start = json_start - int(entry["offset"])
        except (KeyError, TypeError, ValueError) as exc:
            raise ValueError(f"invalid manifest entry: {name or '<unnamed>'}") from exc
        end = start + length
        if length < 0 or start < 0 or end > json_start:
            raise ValueError(f"manifest entry is out of bounds: {name or '<unnamed>'}")
        result.append((dict(entry), data[start:end]))
    return result, _footer_tag(data)


def build_tail(entries: list[tuple[dict[str, Any], bytes]], tag: bytes) -> bytes:
    payload = bytearray()
    starts: list[tuple[dict[str, Any], int, int]] = []
    for entry, block in entries:
        starts.append((entry, len(payload), len(block)))
        payload.extend(block)

    payload_length = len(payload)
    records: list[dict[str, Any]] = []
    for entry, start, length in starts:
        record = dict(entry)
        record["length"] = length
        record["offset"] = payload_length - start
        records.append(record)

    manifest = json.dumps(records, separators=(",", ":"), sort_keys=True).encode("utf-8")
    return bytes(payload) + manifest + b"\x00" + tag + struct.pack("<I", len(manifest) + 9)


def select_palette_entries(
    entries: list[tuple[dict[str, Any], bytes]],
) -> list[tuple[dict[str, Any], bytes]]:
    return [
        (entry, payload)
        for entry, payload in entries
        if str(entry.get("name", "")) in PALETTE_CONTEXT_ENTRY_NAMES
    ]


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def make_report(
    source: Path,
    entries: list[tuple[dict[str, Any], bytes]],
    selected: list[tuple[dict[str, Any], bytes]],
) -> dict[str, Any]:
    names = [str(entry.get("name", "")) for entry, _ in entries]
    selected_names = [str(entry.get("name", "")) for entry, _ in selected]
    return {
        "source": source.as_posix(),
        "source_entries": [
            {
                "name": str(entry.get("name", "")),
                "length": len(payload),
                "sha256": sha256(payload),
            }
            for entry, payload in entries
        ],
        "selected_entries": [
            {
                "name": str(entry.get("name", "")),
                "length": len(payload),
                "sha256": sha256(payload),
            }
            for entry, payload in selected
        ],
        "basic_tone_entries_present": sorted(BASIC_TONE_ENTRY_NAMES.intersection(names)),
        "basic_tone_entries_missing": sorted(BASIC_TONE_ENTRY_NAMES.difference(names)),
        "has_hdr_transform": "hdr.transform.data" in names,
        "candidate_contains": selected_names,
        "device_acceptance": "unverified",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--base-heic", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()

    source_data = args.source.read_bytes()
    entries, tag = extract_manifested_payloads(source_data)
    selected = select_palette_entries(entries)
    if not selected:
        raise SystemExit("source fixture has no Palette/BasicTone context entries to carry")

    tail = build_tail(selected, tag)
    base_data = args.base_heic.read_bytes()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(base_data + tail)

    report = make_report(args.source, entries, selected)
    report.update(
        {
            "footer_tag": tag.decode("ascii", errors="replace"),
            "base_heic_sha256": sha256(base_data),
            "tail_sha256": sha256(tail),
            "output_sha256": sha256(base_data + tail),
            "output_size": len(base_data) + len(tail),
        }
    )
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
