#!/usr/bin/env python3
"""Compare an OPPO HEIF before/after a targeted Gain Map mutation.

This is an offline, read-only companion to ``inspect_oppo_heif.py``. It does
not decide whether two files are visually equivalent; it reports the byte
ranges and logical structures that changed so a caller can enforce the
XDRemux mutation boundary.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Iterable

from inspect_oppo_heif import inspect


def item_map(report: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(item["id"]): item for item in report["items"]}


def item_types(report: dict[str, Any], item_ids: Iterable[int]) -> list[tuple[Any, ...]]:
    items = item_map(report)
    rows = []
    for item_id in item_ids:
        item = items.get(item_id)
        if not item:
            rows.append((item_id, None, None, None, None))
            continue
        rows.append(
            (
                item_id,
                item.get("type"),
                item.get("name"),
                item.get("payload_length"),
                item.get("payload_sha256"),
            )
        )
    return sorted(rows)


def payload_types(report: dict[str, Any], item_ids: Iterable[int]) -> list[tuple[Any, ...]]:
    items = item_map(report)
    rows = []
    for item_id in item_ids:
        item = items.get(item_id)
        if item:
            rows.append(
                (
                    item.get("type"),
                    item.get("name"),
                    item.get("payload_length"),
                    item.get("payload_sha256"),
                )
            )
    return sorted(rows)


def item_payload_multiset(report: dict[str, Any], excluded: set[int] | None = None) -> Counter[tuple[Any, ...]]:
    excluded = excluded or set()
    return Counter(
        (
            item.get("type"),
            item.get("name"),
            item.get("payload_length"),
            item.get("payload_sha256"),
        )
        for item in report["items"]
        if int(item["id"]) not in excluded
    )


def item_descriptors(report: dict[str, Any], item_ids: Iterable[int]) -> list[tuple[Any, ...]]:
    items = item_map(report)
    return sorted(
        (
            item_id,
            items[item_id].get("version"),
            items[item_id].get("flags"),
            items[item_id].get("protection"),
            items[item_id].get("type"),
            items[item_id].get("name"),
            items[item_id].get("content_type"),
            items[item_id].get("content_encoding"),
        )
        for item_id in item_ids
        if item_id in items
    )


def item_properties(report: dict[str, Any], item_ids: Iterable[int]) -> list[tuple[Any, ...]]:
    items = item_map(report)
    return sorted(
        (
            item_id,
            tuple(
                (prop.get("type"), prop.get("essential"), prop.get("sha256"))
                for prop in items[item_id].get("properties", [])
            ),
        )
        for item_id in item_ids
        if item_id in items
    )


def source_non_hdr_properties_preserved(
    source_report: dict[str, Any],
    output_report: dict[str, Any],
    item_ids: Iterable[int],
) -> bool:
    source_items = item_map(source_report)
    output_items = item_map(output_report)
    for item_id in item_ids:
        if item_id not in source_items or item_id not in output_items:
            return False
        source_props = Counter(
            (prop.get("type"), prop.get("essential"), prop.get("sha256"))
            for prop in source_items[item_id].get("properties", [])
        )
        output_props = Counter(
            (prop.get("type"), prop.get("essential"), prop.get("sha256"))
            for prop in output_items[item_id].get("properties", [])
        )
        if source_props - output_props:
            return False
    return True


def non_hdr_property_changes_whitelisted(
    source_report: dict[str, Any],
    output_report: dict[str, Any],
    item_ids: Iterable[int],
) -> bool:
    source_items = item_map(source_report)
    output_items = item_map(output_report)
    primary = source_report.get("primary_item")
    for item_id in item_ids:
        source_props = Counter(
            (prop.get("type"), prop.get("essential"), prop.get("sha256"))
            for prop in source_items.get(item_id, {}).get("properties", [])
        )
        output_props = Counter(
            (prop.get("type"), prop.get("essential"), prop.get("sha256"))
            for prop in output_items.get(item_id, {}).get("properties", [])
        )
        extras = output_props - source_props
        if item_id != primary and extras:
            return False
        if item_id == primary and any(prop_type not in {"colr", "irot"} for prop_type, _, _ in extras):
            return False
    return True


def references(report: dict[str, Any], ref_type: str | None = None) -> list[dict[str, Any]]:
    return [
        row for row in report["references"]
        if ref_type is None or row["type"] == ref_type
    ]


def transitive_dimg_ids(report: dict[str, Any], roots: set[int]) -> set[int]:
    result = set(roots)
    changed = True
    while changed:
        changed = False
        for ref in references(report, "dimg"):
            if ref["from"] not in result:
                continue
            for item_id in ref["to"]:
                if item_id not in result:
                    result.add(item_id)
                    changed = True
    return result


def graph_roles(report: dict[str, Any]) -> dict[str, Any]:
    items = item_map(report)
    primary = report.get("primary_item")
    primary_ids = transitive_dimg_ids(report, {primary} if primary is not None else set())

    gain_grids = {
        ref["from"]
        for ref in references(report, "auxl")
        if items.get(ref["from"], {}).get("type") == "grid"
        and ref["from"] not in primary_ids
    }
    if not gain_grids:
        # Fallback only classifies the existing role; it is not a rewrite rule.
        for summary in report.get("codec_summary", []):
            if summary["grid_item"] == primary:
                continue
            hvc = summary.get("hvcC") or {}
            if hvc.get("profile_idc") == 4 or hvc.get("chroma_format_idc") == 3:
                gain_grids.add(summary["grid_item"])

    gain_ids = transitive_dimg_ids(report, gain_grids)
    tmap_ids = {int(item["id"]) for item in report["items"] if item.get("type") == "tmap"}
    exif_ids = {int(item["id"]) for item in report["items"] if item.get("type") == "Exif"}
    xmp_ids = {
        int(item["id"])
        for item in report["items"]
        if item.get("type") == "mime"
        and item.get("payload_prefix", "").lstrip().startswith("<?xpacket")
    }
    if not xmp_ids:
        xmp_ids = {int(item["id"]) for item in report["items"] if item.get("type") == "mime"}
    private_gain_entries = [
        {
            "name": entry.get("name"),
            "length": entry.get("length"),
            "sha256": entry.get("sha256"),
        }
        for entry in report.get("extension_tail", {}).get("entries", [])
        if entry.get("name") in {"local.uhdr.gainmap.data", "local.uhdr.gainmap.info"}
    ]
    return {
        "primary_ids": sorted(primary_ids),
        "gain_grid_ids": sorted(gain_grids),
        "gain_codec_ids": sorted(gain_ids),
        "tmap_ids": sorted(tmap_ids),
        "exif_ids": sorted(exif_ids),
        "xmp_ids": sorted(xmp_ids),
        "private_gain_entries": private_gain_entries,
    }


def diff_ranges(source: bytes, output: bytes) -> list[dict[str, int]]:
    ranges: list[dict[str, int]] = []
    common = min(len(source), len(output))
    pos = 0
    while pos < common:
        if source[pos] == output[pos]:
            pos += 1
            continue
        start = pos
        pos += 1
        while pos < common and source[pos] != output[pos]:
            pos += 1
        ranges.append({"start": start, "end": pos, "length": pos - start})
    if len(source) != len(output):
        ranges.append({
            "start": common,
            "end": max(len(source), len(output)),
            "length": abs(len(source) - len(output)),
        })
    return ranges


def compact_item_rows(report: dict[str, Any], item_ids: Iterable[int]) -> list[dict[str, Any]]:
    items = item_map(report)
    rows = []
    for item_id in sorted(set(item_ids)):
        item = items.get(item_id)
        if not item:
            continue
        row: dict[str, Any] = {
            "id": item_id,
            "type": item.get("type"),
            "name": item.get("name"),
            "payload_length": item.get("payload_length"),
            "payload_sha256": item.get("payload_sha256"),
            "location": item.get("location"),
        }
        if item.get("properties"):
            row["properties"] = [
                {"type": prop.get("type"), "details": prop.get("details")}
                for prop in item["properties"]
                if prop.get("type") in {"hvcC", "ispe", "pixi", "colr", "auxC"}
            ]
        rows.append(row)
    return rows


def tail_summary(report: dict[str, Any]) -> dict[str, Any]:
    tail = report.get("extension_tail", {})
    return {
        "offset": tail.get("offset"),
        "length": tail.get("length"),
        "sha256": tail.get("sha256"),
        "entries": [
            {
                "name": entry.get("name"),
                "length": entry.get("length"),
                "sha256": entry.get("sha256"),
            }
            for entry in tail.get("entries", [])
        ],
    }


def normalized_tail_entry_name(name: str | None) -> str | None:
    if name is None:
        return None
    replacements = (
        ("xocal.uhdr.", "local.uhdr."),
        ("xocal.hdr.", "local.hdr."),
        ("xrc.local.hdr.", "src.local.hdr."),
        ("xdr.", "hdr."),
    )
    for disabled, original in replacements:
        if name.startswith(disabled):
            return original + name[len(disabled):]
    return name


def tail_entry_payloads(
    report: dict[str, Any],
    include: Any | None = None,
) -> list[tuple[Any, ...]]:
    rows = []
    for entry in report.get("extension_tail", {}).get("entries", []):
        name = normalized_tail_entry_name(entry.get("name"))
        if include is not None and not include(name):
            continue
        rows.append((name, entry.get("length"), entry.get("sha256")))
    return sorted(rows)


def is_hdr_tail_entry(name: str | None) -> bool:
    return bool(name) and (
        name.startswith("local.uhdr.")
        or name.startswith("hdr.")
        or name.startswith("local.hdr.")
        or name.startswith("src.local.hdr.")
    )


def normalized_references(report: dict[str, Any], roles: dict[str, Any]) -> list[tuple[Any, ...]]:
    gain_ids = set(roles["gain_codec_ids"])
    rows = []
    for ref in report["references"]:
        if ref["type"] == "dimg" and (
            ref["from"] in gain_ids or set(ref["to"]) & gain_ids
        ):
            continue
        rows.append((ref["type"], ref["from"], tuple(ref["to"])))
    return sorted(rows)


def hdr_related_item_ids(report: dict[str, Any], roles: dict[str, Any]) -> set[int]:
    """Return only items that belong to the rebuilt ISO 21496-1 HDR graph."""
    result = set(roles["gain_codec_ids"]) | set(roles["tmap_ids"])
    result.update(
        int(item["id"])
        for item in report["items"]
        if item.get("type") == "mime" and item.get("name") == "hdrgm-xmp"
    )
    return result


def normalized_non_hdr_references(
    report: dict[str, Any],
    hdr_item_ids: set[int],
) -> list[tuple[Any, ...]]:
    """Compare the surviving non-HDR item graph while ignoring rebuilt HDR edges."""
    rows = []
    for ref in report["references"]:
        if ref["from"] in hdr_item_ids:
            continue
        targets = tuple(item_id for item_id in ref["to"] if item_id not in hdr_item_ids)
        if targets:
            rows.append((ref["type"], ref["from"], targets))
    return sorted(rows)


def compare(source_path: Path, output_path: Path) -> dict[str, Any]:
    source = source_path.read_bytes()
    output = output_path.read_bytes()
    source_report = inspect(source_path)
    output_report = inspect(output_path)
    source_roles = graph_roles(source_report)
    output_roles = graph_roles(output_report)
    source_gain = set(source_roles["gain_codec_ids"])
    output_gain = set(output_roles["gain_codec_ids"])
    source_hdr = hdr_related_item_ids(source_report, source_roles)
    output_hdr = hdr_related_item_ids(output_report, output_roles)
    ranges = diff_ranges(source, output)

    def exact_payloads(key: str) -> bool:
        return payload_types(source_report, source_roles[key]) == payload_types(output_report, output_roles[key])

    source_non_gain = item_payload_multiset(source_report, source_gain)
    output_non_gain = item_payload_multiset(output_report, output_gain)
    source_non_gain_ids = {int(item["id"]) for item in source_report["items"]} - source_gain
    output_non_gain_ids = {int(item["id"]) for item in output_report["items"]} - output_gain
    source_non_hdr = item_payload_multiset(source_report, source_hdr)
    output_non_hdr = item_payload_multiset(output_report, output_hdr)
    source_non_hdr_ids = {int(item["id"]) for item in source_report["items"]} - source_hdr
    output_non_hdr_ids = {int(item["id"]) for item in output_report["items"]} - output_hdr
    source_non_hdr_non_exif_ids = source_non_hdr_ids - set(source_roles["exif_ids"])
    output_non_hdr_non_exif_ids = output_non_hdr_ids - set(output_roles["exif_ids"])
    invariant = {
        "primary_item_id_equal": source_report.get("primary_item") == output_report.get("primary_item"),
        "primary_payloads_equal": exact_payloads("primary_ids"),
        "gain_item_ids_preserved": source_roles["gain_codec_ids"] == output_roles["gain_codec_ids"],
        "private_gain_entries_equal": source_roles["private_gain_entries"] == output_roles["private_gain_entries"],
        "gain_semantic_tmap_payloads_equal": exact_payloads("tmap_ids"),
        "exif_payloads_equal": exact_payloads("exif_ids"),
        "xmp_payloads_equal": exact_payloads("xmp_ids"),
        "all_non_gain_item_payloads_equal": source_non_gain == output_non_gain,
        "non_gain_item_ids_and_payloads_equal": item_types(source_report, source_non_gain_ids)
        == item_types(output_report, output_non_gain_ids),
        "non_gain_references_equal": normalized_references(source_report, source_roles)
        == normalized_references(output_report, output_roles),
        "all_non_hdr_item_payloads_equal": source_non_hdr == output_non_hdr,
        "non_hdr_item_ids_and_payloads_equal": item_types(source_report, source_non_hdr_ids)
        == item_types(output_report, output_non_hdr_ids),
        "all_non_hdr_non_exif_item_payloads_equal": item_payload_multiset(
            source_report, source_hdr | set(source_roles["exif_ids"])
        ) == item_payload_multiset(output_report, output_hdr | set(output_roles["exif_ids"])),
        "non_hdr_non_exif_item_ids_and_payloads_equal": item_types(
            source_report, source_non_hdr_non_exif_ids
        ) == item_types(output_report, output_non_hdr_non_exif_ids),
        "non_hdr_item_descriptors_equal": item_descriptors(source_report, source_non_hdr_ids)
        == item_descriptors(output_report, output_non_hdr_ids),
        "non_hdr_item_properties_equal": item_properties(source_report, source_non_hdr_ids)
        == item_properties(output_report, output_non_hdr_ids),
        "source_non_hdr_properties_preserved": source_non_hdr_properties_preserved(
            source_report, output_report, source_non_hdr_ids
        ),
        "non_hdr_property_changes_whitelisted": non_hdr_property_changes_whitelisted(
            source_report, output_report, source_non_hdr_ids
        ),
        "non_hdr_references_equal": normalized_non_hdr_references(source_report, source_hdr)
        == normalized_non_hdr_references(output_report, output_hdr),
        "top_level_types_equal": [row["type"] for row in source_report["top_level"]]
        == [row["type"] for row in output_report["top_level"]],
        "unknown_top_level_equal": source_report.get("unknown_top_level") == output_report.get("unknown_top_level"),
        "extension_tail_equal": source_report["extension_tail"].get("sha256")
        == output_report["extension_tail"].get("sha256"),
        "extension_tail_length_equal": source_report["extension_tail"].get("length")
        == output_report["extension_tail"].get("length"),
        "tail_entry_payloads_equal_after_neutralization": tail_entry_payloads(source_report)
        == tail_entry_payloads(output_report),
        "non_hdr_tail_entries_equal": tail_entry_payloads(
            source_report, lambda name: not is_hdr_tail_entry(name)
        ) == tail_entry_payloads(output_report, lambda name: not is_hdr_tail_entry(name)),
        "master_mode_tail_entries_equal": tail_entry_payloads(
            source_report, lambda name: bool(name) and (name.startswith("master.") or name == "master.mode.preset.info")
        ) == tail_entry_payloads(
            output_report, lambda name: bool(name) and (name.startswith("master.") or name == "master.mode.preset.info")
        ),
        "watermark_tail_entries_equal": tail_entry_payloads(
            source_report, lambda name: bool(name) and name.startswith("watermark.")
        ) == tail_entry_payloads(
            output_report, lambda name: bool(name) and name.startswith("watermark.")
        ),
    }
    return {
        "source": {
            "path": str(source_path),
            "size": len(source),
            "sha256": hashlib.sha256(source).hexdigest(),
            "roles": source_roles,
            "gain_items": compact_item_rows(source_report, source_gain),
            "tail": tail_summary(source_report),
        },
        "output": {
            "path": str(output_path),
            "size": len(output),
            "sha256": hashlib.sha256(output).hexdigest(),
            "roles": output_roles,
            "gain_items": compact_item_rows(output_report, output_gain),
            "tail": tail_summary(output_report),
        },
        "invariants": invariant,
        "byte_diff": {
            "range_count": len(ranges),
            "changed_bytes_by_aligned_offset": sum(row["length"] for row in ranges),
            "ranges": ranges,
        },
        "codec_summary": {
            "source": [
                row for row in source_report.get("codec_summary", [])
                if row["grid_item"] in source_roles["gain_grid_ids"]
            ],
            "output": [
                row for row in output_report.get("codec_summary", [])
                if row["grid_item"] in output_roles["gain_grid_ids"]
            ],
        },
    }


def compact(report: dict[str, Any]) -> str:
    invariant = report["invariants"]
    diff = report["byte_diff"]
    lines = [
        f"source: {report['source']['path']} ({report['source']['size']} bytes)",
        f"output: {report['output']['path']} ({report['output']['size']} bytes)",
        f"gain source ids={report['source']['roles']['gain_codec_ids']} output ids={report['output']['roles']['gain_codec_ids']}",
        f"gain codec source={report['codec_summary']['source']}",
        f"gain codec output={report['codec_summary']['output']}",
        "invariants:",
    ]
    for key, value in invariant.items():
        lines.append(f"  {key}: {value}")
    lines.append(
        f"byte_diff: {diff['range_count']} ranges, {diff['changed_bytes_by_aligned_offset']} aligned bytes"
    )
    for row in diff["ranges"][:12]:
        lines.append(f"  {row['start']}..{row['end']} ({row['length']} bytes)")
    if len(diff["ranges"]) > 12:
        lines.append(f"  ... {len(diff['ranges']) - 12} more ranges")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--json", action="store_true", help="emit complete JSON")
    parser.add_argument(
        "--require-oppo-preservation",
        action="store_true",
        help="fail unless the primary, non-HDR item graph, Exif, and OPPO tail are byte-identical",
    )
    args = parser.parse_args(argv)
    try:
        report = compare(args.source, args.output)
    except (OSError, ValueError, struct.error) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    if args.json:
        json.dump(report, sys.stdout, ensure_ascii=False, indent=2)
        sys.stdout.write("\n")
    else:
        print(compact(report))
    if args.require_oppo_preservation:
        required = (
            "primary_item_id_equal",
            "primary_payloads_equal",
            "exif_payloads_equal",
            "all_non_hdr_item_payloads_equal",
            "non_hdr_item_ids_and_payloads_equal",
            "non_hdr_item_descriptors_equal",
            "source_non_hdr_properties_preserved",
            "non_hdr_property_changes_whitelisted",
            "non_hdr_references_equal",
            "unknown_top_level_equal",
            "extension_tail_equal",
        )
        failed = [key for key in required if not report["invariants"][key]]
        if failed:
            print(f"OPPO preservation gate failed: {', '.join(failed)}", file=sys.stderr)
            return 3
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
