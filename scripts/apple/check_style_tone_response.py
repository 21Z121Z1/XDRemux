#!/usr/bin/env python3
"""Score native PISemanticStyleFilter tone renders.

The published harness compares its reconstructed Metal model with Apple's
output. This script treats the native half RGBA texture as the render under
test. Tone 0 on an Apple sample is the host gate: a non-zero differing-component
count means the later XDRemux numbers are not comparable to the 26A434 evidence.

The harness sets inputToneBias and does not expose a color control.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

# Display P3 luminance weights already used by the Styles runtime.
Y_WEIGHTS = np.array([0.22897456, 0.69173852, 0.07928691], dtype=np.float64)
TONES = (-1.0, -0.35, 0.0, 0.35, 1.0)


def luminance(rgb: np.ndarray) -> np.ndarray:
    return rgb.astype(np.float64) @ Y_WEIGHTS


def load_native_rgb(renderer: Path) -> np.ndarray:
    trace = json.loads((renderer / "trace.json").read_text())
    texture = trace["textures"]["_outputImageTexture"]
    width = int(texture["width"])
    height = int(texture["height"])
    raw = np.fromfile(renderer / "_outputImageTexture.bin", dtype="<f2")
    expected = height * width * 4
    if raw.size != expected:
        raise ValueError(f"{renderer}: expected {expected} half values, got {raw.size}")
    return raw.reshape(height, width, 4)[:, :, :3].astype(np.float32)


def _edge_ratio(base: np.ndarray, other: np.ndarray) -> float:
    y = luminance(base)
    magnitude = np.abs(np.diff(y, axis=0)[:, :-1]) + np.abs(np.diff(y, axis=1)[:-1, :])
    delta = np.abs(luminance(other) - y)[:-1, :-1]
    if magnitude.size == 0:
        return 1.0
    cutoff = np.quantile(magnitude, 0.95)
    edge = delta[magnitude >= cutoff]
    flat = delta[magnitude <= np.quantile(magnitude, 0.50)]
    flat_mean = float(flat.mean()) if flat.size else 0.0
    if flat_mean <= 1e-8:
        return float(edge.mean()) if edge.size else 1.0
    return float(edge.mean() / flat_mean)


def score_photo(images: dict[float, np.ndarray]) -> dict:
    base = images[0.0]
    rows = []
    for tone in sorted(images):
        image = images[tone]
        delta = image.astype(np.float64) - base.astype(np.float64)
        y = luminance(image)
        y0 = luminance(base)
        shown = np.clip(y, 0.0, 1.0)
        codes = np.rint(shown * 255.0).astype(np.uint8)
        rows.append(
            {
                "tone": tone,
                "width": int(image.shape[1]),
                "height": int(image.shape[0]),
                "mean_luminance": float(y.mean()),
                "mean_rgb": [float(value) for value in image.reshape(-1, 3).mean(axis=0)],
                "max_abs_vs_tone0": float(np.max(np.abs(delta))),
                "mean_abs_vs_tone0": float(np.mean(np.abs(delta))),
                "mean_delta_rgb": [float(value) for value in delta.reshape(-1, 3).mean(axis=0)],
                "mean_luminance_delta": float((y - y0).mean()),
                "unique_8bit_luma": int(np.unique(codes).size),
                "fraction_above_one": float(np.mean(image > 1.0)),
                "fraction_below_zero": float(np.mean(image < 0.0)),
            }
        )
    means = [row["mean_luminance"] for row in rows]
    steps = [
        {
            "from_tone": rows[index]["tone"],
            "to_tone": rows[index + 1]["tone"],
            "mean_luminance_step": means[index + 1] - means[index],
        }
        for index in range(len(rows) - 1)
    ]
    step_values = [step["mean_luminance_step"] for step in steps]
    positive = [value for value in step_values if value > 0.002]
    negative = [value for value in step_values if value < -0.002]
    monotonic = not (positive and negative)
    inactive = all(row["max_abs_vs_tone0"] < 1e-3 for row in rows)
    concerns = []
    if inactive:
        concerns.append("tone bias leaves the native render unchanged")
    if not monotonic:
        concerns.append("mean luminance is not monotonic across the tone sweep")
    base_levels = next(row["unique_8bit_luma"] for row in rows if row["tone"] == 0.0)
    for row in rows:
        if row["tone"] == 0.0:
            continue
        if base_levels > 128 and row["unique_8bit_luma"] < 32:
            concerns.append(f"tone {row['tone']:g} collapses 8-bit luminance levels")
        deltas = np.array(row["mean_delta_rgb"])
        order = np.argsort(np.abs(deltas))
        if abs(deltas[order[-1]]) > 0.05 and abs(deltas[order[-1]]) > 3.0 * abs(deltas[order[-2]]):
            concerns.append(f"tone {row['tone']:g} shifts one channel much more than the others")
        ratio = _edge_ratio(base, images[row["tone"]])
        row["edge_to_flat_delta_ratio"] = ratio
        if ratio > 8.0 and row["mean_abs_vs_tone0"] > 0.02:
            concerns.append(f"tone {row['tone']:g} concentrates its change on edges")
        if row["fraction_above_one"] > 0.25 or row["fraction_below_zero"] > 0.02:
            concerns.append(f"tone {row['tone']:g} clips a large share of samples")
    return {
        "tones": rows,
        "luminance_steps": steps,
        "monotonic_mean_luminance": monotonic,
        "tone_changes_render": not inactive,
        "concerns": concerns,
    }


def tone_dirs(photo_dir: Path) -> dict[float, Path]:
    found = {}
    for child in photo_dir.iterdir():
        if not child.is_dir() or not child.name.startswith("tone-"):
            continue
        try:
            tone = float(child.name.removeprefix("tone-"))
        except ValueError:
            continue
        renderer = child / "renderer-0"
        if (renderer / "_outputImageTexture.bin").is_file():
            found[tone] = renderer
    return found


def read_sweep(path: Path) -> list[dict]:
    if not path.is_file():
        return []
    return json.loads(path.read_text())


def sanity_gate(rows: list[dict]) -> dict:
    tone0 = [row for row in rows if row.get("tone") == 0 or row.get("tone") == 0.0]
    if len(tone0) != 1:
        return {"trusted": False, "reason": "tone 0 row missing", "rows": tone0}
    row = tone0[0]
    differing = row.get("differing_components")
    trusted = differing == 0 and not row.get("error")
    return {
        "trusted": trusted,
        "photo": row.get("photo"),
        "differing_components": differing,
        "max_abs": row.get("max_abs"),
        "psnr_peak1_db": row.get("psnr_peak1_db"),
        "returncode": row.get("returncode"),
        "error": row.get("error"),
        "reason": None if trusted else "tone 0 model and native output differ, or the render failed",
    }


def _srgb(linear: np.ndarray) -> np.ndarray:
    clipped = np.clip(linear, 0.0, 1.0)
    encoded = np.where(
        clipped <= 0.0031308,
        clipped * 12.92,
        1.055 * np.power(clipped, 1.0 / 2.4) - 0.055,
    )
    return np.clip(np.rint(encoded * 255.0), 0, 255).astype(np.uint8)


def _box(image: np.ndarray, long_edge: int) -> np.ndarray:
    height, width = image.shape[:2]
    scale = long_edge / max(height, width)
    if scale >= 1:
        return image
    out_h = max(1, int(round(height * scale)))
    out_w = max(1, int(round(width * scale)))
    ys = (np.arange(out_h) * height / out_h).astype(int)
    xs = (np.arange(out_w) * width / out_w).astype(int)
    y2 = np.clip(ys + np.maximum(1, height // out_h), 0, height)
    x2 = np.clip(xs + np.maximum(1, width // out_w), 0, width)
    output = np.empty((out_h, out_w, image.shape[2]), dtype=np.float32)
    for row, (y0, y1) in enumerate(zip(ys, y2)):
        for column, (x0, x1) in enumerate(zip(xs, x2)):
            output[row, column] = image[y0:y1, x0:x1].mean(axis=(0, 1))
    return output


def write_comparison(images: dict[float, np.ndarray], path: Path) -> None:
    try:
        from PIL import Image
    except ImportError:
        return
    tones = [tone for tone in TONES if tone in images]
    tiles = [_box(images[tone], 160) for tone in tones]
    base = _box(images[0.0], 160) if 0.0 in images else tiles[0]
    height, width = tiles[0].shape[:2]
    canvas = np.zeros((height * 2, width * len(tiles), 3), dtype=np.uint8)
    for index, (tone, tile) in enumerate(zip(tones, tiles)):
        canvas[:height, index * width : (index + 1) * width] = _srgb(tile)
        delta = np.clip(0.5 + 4.0 * (tile.astype(np.float64) - base.astype(np.float64)), 0.0, 1.0)
        canvas[height:, index * width : (index + 1) * width] = _srgb(delta)
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(canvas, mode="RGB").save(path)


def input_keys(photo_dir: Path) -> list[str]:
    for child in sorted(photo_dir.glob("tone-*/checkpoint.json")):
        payload = json.loads(child.read_text())
        defaults = payload.get("defaults")
        if isinstance(defaults, dict):
            return sorted(defaults)
    return []


def analyze(sanity_path: Path, render_root: Path, out_dir: Path) -> dict:
    sanity_rows = read_sweep(sanity_path)
    gate = sanity_gate(sanity_rows)
    photos = []
    if render_root.is_dir():
        for photo_dir in sorted(path for path in render_root.iterdir() if path.is_dir()):
            renders = tone_dirs(photo_dir)
            entry = {
                "photo": photo_dir.name,
                "rendered_tones": sorted(renders),
                "input_keys": input_keys(photo_dir),
            }
            if 0.0 not in renders or len(renders) < 2:
                entry["concerns"] = ["native render missing for tone 0 or for a comparison tone"]
                photos.append(entry)
                continue
            images = {tone: load_native_rgb(path) for tone, path in renders.items()}
            entry.update(score_photo(images))
            write_comparison(images, out_dir / "comparisons" / f"{photo_dir.name}.png")
            photos.append(entry)
    agreement = read_sweep(render_root / "tone-sweep.json") if render_root else []
    report = {
        "sanity": gate,
        "sanity_rows": sanity_rows,
        "model_agreement": agreement,
        "photos": photos,
        "color_control": "The harness sets inputToneBias only. It has no color-bias argument.",
        "trusted": bool(gate.get("trusted")),
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "tone-response.json").write_text(json.dumps(report, indent=2) + "\n")
    (out_dir / "tone-response.md").write_text(render_markdown(report))
    return report


def render_markdown(report: dict) -> str:
    lines = ["# Photographic Styles tone response", ""]
    gate = report["sanity"]
    lines.append(
        f"Host gate trusted: {gate.get('trusted')}. "
        f"Tone 0 differing components: {gate.get('differing_components')}."
    )
    if gate.get("reason"):
        lines.append(gate["reason"])
    lines.append("")
    lines.append(report["color_control"])
    lines.append("")
    for photo in report["photos"]:
        lines.append(f"## {photo['photo']}")
        lines.append("")
        lines.append("| tone | mean Y | dY vs 0 | mean dR | mean dG | mean dB | max abs | 8-bit levels |")
        lines.append("|---:|---:|---:|---:|---:|---:|---:|---:|")
        for row in photo.get("tones", []):
            dr, dg, db = row["mean_delta_rgb"]
            lines.append(
                f"| {row['tone']:g} | {row['mean_luminance']:.5f} | {row['mean_luminance_delta']:.5f} | "
                f"{dr:.5f} | {dg:.5f} | {db:.5f} | {row['max_abs_vs_tone0']:.5f} | {row['unique_8bit_luma']} |"
            )
        if photo.get("concerns"):
            lines.append("")
            for concern in photo["concerns"]:
                lines.append(f"- {concern}")
        lines.append("")
    lines.append("## Model versus native")
    lines.append("")
    lines.append("| photo | tone | differing | max abs | PSNR |")
    lines.append("|---|---:|---:|---:|---:|")
    for row in report.get("model_agreement", []):
        lines.append(
            f"| {row.get('photo')} | {row.get('tone')} | {row.get('differing_components', row.get('error'))} | "
            f"{row.get('max_abs', '')} | {row.get('psnr_peak1_db', '')} |"
        )
    lines.append("")
    return "\n".join(lines)


def self_test() -> None:
    height, width = 24, 32
    base = np.full((height, width, 3), 0.4, dtype=np.float32)
    images = {}
    for tone in TONES:
        shift = 0.05 * tone
        images[tone] = np.clip(base + shift, 0.0, 1.0)
    scored = score_photo(images)
    if not scored["monotonic_mean_luminance"] or not scored["tone_changes_render"]:
        raise SystemExit(f"monotonic sweep was not recognized: {scored}")
    flat = {tone: base.copy() for tone in TONES}
    inactive = score_photo(flat)
    if inactive["tone_changes_render"] or "tone bias leaves the native render unchanged" not in inactive["concerns"]:
        raise SystemExit(f"inactive sweep was not recognized: {inactive}")
    cast = {tone: base.copy() for tone in TONES}
    cast[1.0] = base + np.array([0.2, 0.0, 0.0], dtype=np.float32)
    cast[-1.0] = base + np.array([-0.08, 0.0, 0.0], dtype=np.float32)
    colored = score_photo(cast)
    if not any("one channel" in concern for concern in colored["concerns"]):
        raise SystemExit(f"channel imbalance was not recognized: {colored['concerns']}")
    gate = sanity_gate([{"photo": "IMG_0032", "tone": 0, "differing_components": 0}])
    if not gate["trusted"]:
        raise SystemExit(gate)
    bad = sanity_gate([{"photo": "IMG_0032", "tone": 0, "differing_components": 12}])
    if bad["trusted"]:
        raise SystemExit(bad)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--sanity", type=Path)
    parser.add_argument("--renders", type=Path)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
        print("style tone response self-test passed")
        return 0
    if not args.sanity or not args.renders or not args.out:
        parser.error("--sanity, --renders, and --out are required")
    report = analyze(args.sanity, args.renders, args.out)
    print((args.out / "tone-response.md").read_text())
    return 0 if report["trusted"] else 3


if __name__ == "__main__":
    sys.exit(main())
