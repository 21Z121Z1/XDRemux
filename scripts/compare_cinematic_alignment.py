#!/usr/bin/env python3
"""Compare two real native Cinematic verification runs and check an improvement.

This tool reads reports only. It never provides reference metadata to a generator.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import statistics
import subprocess


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def comparisons(document: dict) -> dict[str, dict]:
    assets = {}
    for asset in document["assets"]:
        if "headless_comparison" not in asset:
            continue
        name = Path(asset["source"]).stem
        require(name not in assets, f"duplicate sample name: {name}")
        require(asset["source_unchanged"] and asset["ordinary_unchanged"], f"source changed: {name}")
        assets[name] = asset
    require(bool(assets), "no native render comparisons")
    return assets


def compare_runs(baseline: dict, current: dict) -> dict:
    require(baseline["runtime"] == current["runtime"], "OS versions differ")
    old_assets, new_assets = comparisons(baseline), comparisons(current)
    require(old_assets.keys() == new_assets.keys(), "native sample sets differ")
    rows = []
    old_total = new_total = 0.0
    total_components = 0
    for name, new in new_assets.items():
        old = old_assets[name]
        require(old["source_sha256"] == new["source_sha256"], f"reference input differs: {name}")
        # A fresh MOV can have a different container layout or creation time.
        # Both runs independently prove that stripping copied every media
        # packet from the same hash-identified native source unchanged.
        require(old["stripped_compressed_media"] == new["stripped_compressed_media"], f"ordinary media differs: {name}")
        for asset in [old, new]:
            require(asset["strip"]["capability"] == "none" and asset["strip"]["cinematic_data_removed"],
                    f"ordinary input retains Cinematic data: {name}")
            for records in [asset["stripped_compressed_media"], asset["compressed_media"]]:
                require(bool(records) and all(x["packet_bytes_timestamps_and_trim_identical"] for x in records),
                        f"media packet identity is not proven: {name}")
        a, b = old["headless_comparison"], new["headless_comparison"]
        for key in ["renderer", "quality", "texture_format", "input_pixel_format", "comparison_space", "source_transform", "native_rendering_version"]:
            require(a[key] == b[key], f"render configuration differs: {name}/{key}")
        require(a["quality"] == "export", "comparison is not export quality")
        keyed = lambda document: {(x["pts"], x["f_number"]): x for x in document["frames"]}
        before, after = keyed(a), keyed(b)
        require(before.keys() == after.keys() and len(before) == 6, f"sample times or apertures differ: {name}")
        for x in b["frames"]:
            require(x.get("decoded_color_identical") is True, f"decoded color identity is not proven: {name}")
        first = next(x for x in b["frames"] if "native_repeat_difference" in x)
        require(first["native_repeat_difference"]["identical"], f"native renderer is not deterministic: {name}")
        old_rmse, new_rmse, old_relative, new_relative = [], [], [], []
        old_strength, new_strength, cosine, regional = [], [], [], []
        pairs = []
        for key, candidate in after.items():
            reference = before[key]
            if key[1] == 2:
                old_rmse.append(reference["render_difference"]["rmse"])
                new_rmse.append(candidate["render_difference"]["rmse"])
                pairs.append({"pts": key[0], "baseline_f2_rmse": old_rmse[-1], "current_f2_rmse": new_rmse[-1],
                              "current_f2_psnr_db": candidate["render_difference"]["psnr_db"],
                              "native_focus_detection": candidate["native_focus_detection"],
                              "current_focus_detection": candidate["generated_focus_detection"]})
            if "effect_comparison" in candidate:
                original_effect, effect = reference["effect_comparison"], candidate["effect_comparison"]
                require(math.isclose(original_effect["native_effect_rmse"], effect["native_effect_rmse"], rel_tol=1e-8, abs_tol=1e-12),
                        f"native reference effect changed: {name}/{key[0]}")
                old_relative.append(original_effect["relative_effect_error"])
                new_relative.append(effect["relative_effect_error"])
                old_strength.append(original_effect["generated_to_native_effect_strength"])
                new_strength.append(effect["generated_to_native_effect_strength"])
                cosine.append(effect["effect_cosine_similarity"])
                regional.append(candidate["native_focus_region_effect_comparison"]["effect_error_rmse"])
                count = candidate["render_difference"]["components"]
                old_total += original_effect["effect_error_rmse"] ** 2 * count
                new_total += effect["effect_error_rmse"] ** 2 * count
                total_components += count
        mean_old, mean_new = statistics.mean(old_rmse), statistics.mean(new_rmse)
        relative_old, relative_new = statistics.mean(old_relative), statistics.mean(new_relative)
        rows.append({"sample": name, "reference_sha256": old["source_sha256"],
                     "baseline_ordinary_sha256": old["ordinary_sha256"], "current_ordinary_sha256": new["ordinary_sha256"],
                     "ordinary_media_packets_identical_to_reference": True,
                     "mean_baseline_f2_rmse": mean_old, "mean_current_f2_rmse": mean_new,
                     "f2_rmse_reduction": 1 - mean_new / mean_old,
                     "mean_baseline_relative_effect_error": relative_old, "mean_current_relative_effect_error": relative_new,
                     "relative_effect_error_reduction": 1 - relative_new / relative_old,
                     "mean_baseline_effect_strength_ratio": statistics.mean(old_strength),
                     "mean_current_effect_strength_ratio": statistics.mean(new_strength),
                     "mean_current_effect_cosine_similarity": statistics.mean(cosine),
                     "mean_current_focus_region_effect_rmse": statistics.mean(regional),
                     "pairs": pairs})
    return {"schema": "xdremux-cinematic-alignment-v1", "baseline_head": baseline["head"], "current_head": current["head"],
            "runtime": current["runtime"], "scope": "three sampled frames per asset; independent RGB generation; public native rendering",
            "assets": rows, "components": total_components,
            "pooled_baseline_effect_error_rmse": math.sqrt(old_total / total_components),
            "pooled_current_effect_error_rmse": math.sqrt(new_total / total_components),
            "pooled_effect_error_reduction": 1 - math.sqrt(new_total / old_total)}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--current", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--require-improvement", action="append", default=[])
    parser.add_argument("--minimum-reduction", type=float, default=0.25)
    args = parser.parse_args()
    require(0 < args.minimum_reduction < 1, "minimum reduction must be between zero and one")
    require(not args.output.exists(), "alignment output already exists")
    current = json.loads(args.current.read_text())
    repo = Path(__file__).resolve().parents[1]
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    require(current["head"] == head, "current verification report does not match HEAD")
    report = compare_runs(json.loads(args.baseline.read_text()), current)
    by_name = {row["sample"]: row for row in report["assets"]}
    for name in args.require_improvement:
        require(name in by_name, f"required sample is missing: {name}")
        row = by_name[name]
        require(row["f2_rmse_reduction"] >= args.minimum_reduction and
                row["relative_effect_error_reduction"] >= args.minimum_reduction,
                f"required alignment improvement did not pass: {name}")
    report["required_improvement_samples"] = args.require_improvement
    report["minimum_required_reduction"] = args.minimum_reduction
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for row in report["assets"]:
        print(f"{row['sample']}: f2 RMSE reduction={row['f2_rmse_reduction']:.2%}; relative effect error="
              f"{row['mean_baseline_relative_effect_error']:.4f}->{row['mean_current_relative_effect_error']:.4f}")
    print(f"pooled effect error reduction={report['pooled_effect_error_reduction']:.2%}")


if __name__ == "__main__":
    main()
