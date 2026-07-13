#!/usr/bin/env python3
"""Extract OPPO portrait depth/header/config metrics from original HEIC files."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import statistics
import struct
import subprocess
from pathlib import Path

def percentile(values: list[int], fraction: float) -> int:
    return values[int((len(values) - 1) * fraction)]


def original_candidates(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.glob("IMG*.heic")
        if "（" not in path.name and " (" not in path.name and "_" not in path.stem
    )


def exif_rows(paths: list[Path]) -> list[dict[str, object]]:
    command = [
        "exiftool",
        "-j",
        "-n",
        "-FileName",
        "-UserComment",
        "-FocalLength",
        "-FocalLengthIn35mmFormat",
        "-DigitalZoomRatio",
        "-FNumber",
        "-DateTimeOriginal",
        *(str(path) for path in paths),
    ]
    return json.loads(subprocess.check_output(command))


def extension_entries(data: bytes) -> list[dict[str, object]]:
    marker = max(data.rfind(b"jxrs"), data.rfind(b"wtmk"))
    if marker < 0:
        return []
    json_end = marker - 1 if marker > 0 and data[marker - 1] == 0 else marker
    json_start = data.rfind(b"[{", 0, json_end)
    if json_start < 0:
        return []
    manifest = json.loads(data[json_start:json_end].decode("utf-8"))
    entries: list[dict[str, object]] = []
    for item in manifest:
        if not isinstance(item, dict):
            continue
        try:
            length = int(item["length"])
            offset = int(item["offset"])
        except (KeyError, TypeError, ValueError):
            continue
        start = json_start - offset
        end = start + length
        if 0 <= start <= end <= len(data):
            entries.append({**item, "start": start, "end": end})
    return entries


def analyze(path: Path, exif: dict[str, object]) -> dict[str, object] | None:
    file_data = path.read_bytes()
    entries = extension_entries(file_data)
    depth_entry = next((entry for entry in entries if entry.get("name") == "rear.depth"), None)
    config_entry = next(
        (entry for entry in entries if entry.get("name") == "rear.depth.config"),
        None,
    )
    if depth_entry is None or config_entry is None:
        return None

    encoded_depth = file_data[depth_entry["start"] : depth_entry["end"]]
    decoded_depth = subprocess.check_output(
        ["zstd", "-d", "-q", "-c"],
        input=encoded_depth,
    )
    depth_width, depth_height = struct.unpack_from("<II", decoded_depth, 0)
    ranks = decoded_depth[768 : 768 + depth_width * depth_height]
    if len(ranks) != depth_width * depth_height:
        raise ValueError("truncated rank plane")

    config = file_data[config_entry["start"] : config_entry["end"]]
    config_width, config_height, focus_x, focus_y = struct.unpack_from("<4i", config, 4)
    blur_strength = struct.unpack_from("<i", config, 276)[0]
    config_fnumber = struct.unpack_from("<f", config, 292)[0]
    config_distance = struct.unpack_from("<i", config, 296)[0]
    sample_scale, focal_length_depth, stereo_baseline = struct.unpack_from(
        "<fff", decoded_depth, 0x18
    )
    plane_size = depth_width * depth_height
    plane_cursor = 768 + plane_size
    plane_stats: dict[str, object] = {}
    for name, flag_offset in (("hair", 0x24), ("portrait", 0x25), ("pet", 0x26)):
        present = decoded_depth[flag_offset] != 0
        plane_stats[f"{name}_flag"] = int(present)
        if present and plane_cursor + plane_size <= len(decoded_depth):
            plane = decoded_depth[plane_cursor : plane_cursor + plane_size]
            plane_stats[f"{name}_min"] = min(plane)
            plane_stats[f"{name}_max"] = max(plane)
            plane_stats[f"{name}_nonzero_fraction"] = sum(value != 0 for value in plane) / plane_size
            plane_cursor += plane_size
        else:
            plane_stats[f"{name}_min"] = ""
            plane_stats[f"{name}_max"] = ""
            plane_stats[f"{name}_nonzero_fraction"] = ""
    header_aux_1b8 = struct.unpack_from("<f", decoded_depth, 0x1B8)[0]
    header_zoom_index = struct.unpack_from("<f", decoded_depth, 0x1BC)[0]

    ordered = sorted(ranks)

    # rear.depth.config uses portrait/display coordinates (900x1200), while
    # the decoded rank plane is landscape-stored with EXIF Orientation 6.
    focus_rank_x = min(
        depth_width - 1,
        max(0, round(focus_y / config_height * depth_width)),
    )
    focus_rank_y = min(
        depth_height - 1,
        max(0, round((config_width - focus_x) / config_width * depth_height)),
    )
    local: list[int] = []
    for y in range(max(0, focus_rank_y - 10), min(depth_height, focus_rank_y + 11)):
        row = y * depth_width
        local.extend(
            ranks[
                row + max(0, focus_rank_x - 10) : row + min(depth_width, focus_rank_x + 11)
            ]
        )
    local.sort()

    return {
        "file": path.name,
        "date": exif.get("DateTimeOriginal", ""),
        "physical_focal_mm": exif.get("FocalLength", ""),
        "equivalent_focal_mm": exif.get("FocalLengthIn35mmFormat", ""),
        "digital_zoom": exif.get("DigitalZoomRatio", 1) or 1,
        "exif_fnumber": exif.get("FNumber", ""),
        "config_fnumber": config_fnumber,
        "config_distance": config_distance,
        "blur_strength": blur_strength,
        "config_width": config_width,
        "config_height": config_height,
        "focus_x": focus_x,
        "focus_y": focus_y,
        "focus_rank_x": focus_rank_x,
        "focus_rank_y": focus_rank_y,
        "depth_width": depth_width,
        "depth_height": depth_height,
        "header_sample_scale": sample_scale,
        "header_fx_depth": focal_length_depth,
        "header_baseline": stereo_baseline,
        "header_aux_1b8": header_aux_1b8,
        "header_zoom_index": header_zoom_index,
        **plane_stats,
        "depth_trailing_after_same_size_planes": len(decoded_depth) - plane_cursor,
        "manifest_names": ";".join(str(entry.get("name", "")) for entry in entries),
        "src_image_bytes": next(
            (int(entry["length"]) for entry in entries if entry.get("name") == "src.image"),
            0,
        ),
        "rear_depth_compressed_bytes": int(depth_entry["length"]),
        "rear_depth_decoded_bytes": len(decoded_depth),
        "effective_fx_src": focal_length_depth * 4096 / depth_width,
        "rank_min": ordered[0],
        "rank_p01": percentile(ordered, 0.01),
        "rank_p10": percentile(ordered, 0.10),
        "rank_p25": percentile(ordered, 0.25),
        "rank_p50": percentile(ordered, 0.50),
        "rank_p75": percentile(ordered, 0.75),
        "rank_p90": percentile(ordered, 0.90),
        "rank_p99": percentile(ordered, 0.99),
        "rank_max": ordered[-1],
        "rank_p99_p01": percentile(ordered, 0.99) - percentile(ordered, 0.01),
        "rank_p90_p10": percentile(ordered, 0.90) - percentile(ordered, 0.10),
        "rank_focus": ranks[focus_rank_y * depth_width + focus_rank_x],
        "rank_focus_local_median": statistics.median(local),
        "rank_focus_local_p10": percentile(local, 0.10),
        "rank_focus_local_p90": percentile(local, 0.90),
        "rank_unique": len(set(ranks)),
        "file_sha256": hashlib.sha256(file_data).hexdigest(),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("output_csv", type=Path)
    args = parser.parse_args()

    paths = original_candidates(args.input_dir)
    metadata = exif_rows(paths)
    rows = [
        row
        for path, exif in zip(paths, metadata)
        if (row := analyze(path, exif)) is not None
    ]
    if not rows:
        raise SystemExit("no OPPO rear portrait-depth samples found")

    args.output_csv.parent.mkdir(parents=True, exist_ok=True)
    with args.output_csv.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=rows[0].keys())
        writer.writeheader()
        writer.writerows(rows)
    print(f"wrote {len(rows)} rows to {args.output_csv}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
