#!/usr/bin/env python3
"""Measure a final HEIC's Photos-editor style response through the private renderer.

Renders the axis settings from
docs/plans/active/apple-styles-editor-response-optimization-20260726.md
section 5 with the same PLPhotoEditRenderer helper the constrained solver
uses, then reports ROI OKLab hue / R-G / luma / chroma deltas per axis.

The OKLab math and ROI rules intentionally match
ConstrainedPolynomialStyleDataProducer.responseMetricSample so the offline
numbers are comparable with solver-result.json responseObjective entries.
"""

from __future__ import annotations

import argparse
import glob
import hashlib
import json
import math
import os
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image

P3_TO_XYZ = np.asarray(
    [
        [0.48657095, 0.26566769, 0.19821729],
        [0.22897456, 0.69173852, 0.07928691],
        [0.00000000, 0.04511338, 1.04394437],
    ],
    dtype=np.float64,
)
XYZ_TO_LMS = np.asarray(
    [
        [0.81902244, 0.36190626, -0.12887378],
        [0.03298367, 0.92928685, 0.03614467],
        [0.04817720, 0.26423952, 0.63354783],
    ],
    dtype=np.float64,
)
LMS_TO_OKLAB = np.asarray(
    [
        [0.21045426, 0.79361779, -0.00407205],
        [1.97799850, -2.42859221, 0.45059371],
        [0.02590404, 0.78277177, -0.80867577],
    ],
    dtype=np.float64,
)

MINIMUM_ROI_PIXELS = 500

RENDERS = [
    ("disabled", 0.0, 0.0, False),
    ("neutral", 0.0, 0.0, True),
    ("tone_-1", -1.0, 0.0, True),
    ("tone_+1", 1.0, 0.0, True),
    ("color_-1", 0.0, -1.0, True),
    ("color_+1", 0.0, 1.0, True),
    ("tc100_mid", 0.0, 1.0, True),
    ("tc100_plus", 1.0, 1.0, True),
]

NATIVE_TC100_HUE_ENVELOPE = [-1.702725, 19.16482]
NATIVE_TC100_RG_ENVELOPE = [-0.47665203, 0.09553468]


def sha256_file(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def locate_helper(explicit: str | None) -> str:
    if explicit:
        if not os.access(explicit, os.X_OK):
            raise SystemExit(f"helper is not executable: {explicit}")
        return explicit
    cache = os.path.expanduser(
        "~/Library/Caches/com.proxdr.XDRemux/AppleNativeTools/*/learnnode-coefficient-probe"
    )
    candidates = [path for path in glob.glob(cache) if os.access(path, os.X_OK)]
    if not candidates:
        raise SystemExit(
            "no cached learnnode-coefficient-probe helper; run one xdremux "
            "--apple-photographic-styles conversion first or pass --helper"
        )
    return max(candidates, key=os.path.getmtime)


def oklab(rgb8: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """rgb8 float [H,W,3] 0..255 -> (oklab [H,W,3], linear rgb [H,W,3])."""
    value = np.clip(rgb8, 0, 255) / 255.0
    linear = np.where(
        value <= 0.04045, value / 12.92, ((value + 0.055) / 1.055) ** 2.4
    )
    xyz = linear @ P3_TO_XYZ.T
    lms = np.cbrt(np.maximum(xyz @ XYZ_TO_LMS.T, 0))
    return lms @ LMS_TO_OKLAB.T, linear


def roi_metrics(png_path: str, mask: np.ndarray | None) -> dict:
    with Image.open(png_path) as image:
        rgb8 = np.asarray(image.convert("RGB"), dtype=np.float64)
    lab, linear = oklab(rgb8)
    lightness = lab[..., 0]
    a_channel = lab[..., 1]
    b_channel = lab[..., 2]
    chroma = np.hypot(a_channel, b_channel)
    eligible = (lightness >= 0.15) & (lightness <= 0.97) & (chroma > 0.02)
    roi_kind = "none"
    roi = None
    if mask is not None:
        height, width = lightness.shape
        y_index = (np.arange(height) * mask.shape[0] // height)[:, None]
        x_index = (np.arange(width) * mask.shape[1] // width)[None, :]
        sampled = mask[y_index, x_index] >= 128
        candidate = eligible & sampled
        if int(candidate.sum()) >= MINIMUM_ROI_PIXELS:
            roi_kind, roi = "skin-mask", candidate
    if roi is None:
        hue = np.degrees(np.arctan2(b_channel, a_channel))
        candidate = eligible & (hue >= 5) & (hue <= 65)
        if int(candidate.sum()) >= MINIMUM_ROI_PIXELS:
            roi_kind, roi = "warm-fallback", candidate
    if roi is None:
        return {
            "roiKind": "none",
            "roiPixelCount": 0,
            "hueDegrees": 0.0,
            "rgRatio": 0.0,
            "meanL": 0.0,
            "meanChroma": 0.0,
        }
    mean_a = float(a_channel[roi].mean())
    mean_b = float(b_channel[roi].mean())
    return {
        "roiKind": roi_kind,
        "roiPixelCount": int(roi.sum()),
        "hueDegrees": math.degrees(math.atan2(mean_b, mean_a)),
        "rgRatio": float(linear[..., 0][roi].mean())
        / max(float(linear[..., 1][roi].mean()), 1e-6),
        "meanL": float(lightness[roi].mean()),
        "meanChroma": float(chroma[roi].mean()),
    }


def wrap_degrees(value: float) -> float:
    wrapped = math.fmod(value, 360.0)
    if wrapped <= -180.0:
        wrapped += 360.0
    if wrapped > 180.0:
        wrapped -= 360.0
    return wrapped


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, help="final styles HEIC")
    parser.add_argument("--output", required=True, help="result JSON path")
    parser.add_argument("--helper", help="learnnode-coefficient-probe path")
    parser.add_argument("--skin-mask", help="grayscale PNG, 255 = skin")
    parser.add_argument("--max-dim", type=int, default=1024)
    parser.add_argument(
        "--keep-renders",
        action="store_true",
        help="keep PNG renders next to the output JSON",
    )
    args = parser.parse_args()

    input_path = os.path.abspath(args.input)
    if not os.path.isfile(input_path):
        raise SystemExit(f"missing input HEIC: {input_path}")
    helper = locate_helper(args.helper)
    output_path = os.path.abspath(args.output)
    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
    render_directory = (
        os.path.join(os.path.dirname(output_path), "editor-response-renders")
        if args.keep_renders
        else tempfile.mkdtemp(prefix="xdremux-editor-response-")
    )
    os.makedirs(render_directory, exist_ok=True)

    mask = None
    if args.skin_mask:
        with Image.open(args.skin_mask) as mask_image:
            mask = np.asarray(mask_image.convert("L"), dtype=np.uint8)

    requests = []
    for label, tone, color, enabled in RENDERS:
        requests.append(
            {
                "photo": input_path,
                "output": os.path.join(render_directory, f"{label}.png"),
                "manifest": os.path.join(render_directory, f"{label}.json"),
                "tone": tone,
                "color": color,
                "intensity": 1.0,
                "enabled": enabled,
                "maximumDimension": args.max_dim,
                "cast": "Standard",
            }
        )
    plan_path = os.path.join(render_directory, "plan.json")
    with open(plan_path, "w", encoding="utf-8") as handle:
        json.dump({"requests": requests}, handle, indent=2, sort_keys=True)
    completed = subprocess.run(
        [helper, "--render-style-batch", plan_path],
        capture_output=True,
        text=True,
        timeout=1200,
        check=False,
    )
    try:
        batch = json.loads(completed.stdout or "{}")
    except json.JSONDecodeError:
        batch = {}
    if completed.returncode != 0 or not batch.get("passed"):
        raise SystemExit(
            "render batch failed: "
            f"exit={completed.returncode} stderr={completed.stderr[-2000:]} "
            f"result={json.dumps(batch)[:2000]}"
        )

    per_render = {}
    for request, (label, _, _, _) in zip(requests, RENDERS):
        metrics = roi_metrics(request["output"], mask)
        per_render[label] = {
            "settings": {
                "tone": request["tone"],
                "color": request["color"],
                "enabled": request["enabled"],
            },
            "sha256": sha256_file(request["output"]),
            "metrics": metrics,
        }

    def axis(plus: str, minus: str) -> dict:
        plus_metrics = per_render[plus]["metrics"]
        minus_metrics = per_render[minus]["metrics"]
        if plus_metrics["roiKind"] == "none" or minus_metrics["roiKind"] == "none":
            return {"available": False}
        return {
            "available": True,
            "lumaDelta": plus_metrics["meanL"] - minus_metrics["meanL"],
            "chromaDelta": plus_metrics["meanChroma"] - minus_metrics["meanChroma"],
            "hueDeltaDegrees": wrap_degrees(
                plus_metrics["hueDegrees"] - minus_metrics["hueDegrees"]
            ),
            "rgDelta": plus_metrics["rgRatio"] - minus_metrics["rgRatio"],
        }

    tc100 = axis("tc100_plus", "tc100_mid")
    hue_delta = tc100.get("hueDeltaDegrees")
    inside = (
        hue_delta is not None
        and NATIVE_TC100_HUE_ENVELOPE[0] <= hue_delta <= NATIVE_TC100_HUE_ENVELOPE[1]
    )
    result = {
        "schema": "xdremux-editor-response-measurement-v1",
        "input": input_path,
        "inputSHA256": sha256_file(input_path),
        "helper": helper,
        "maximumDimension": args.max_dim,
        "roi": {
            "kind": per_render["neutral"]["metrics"]["roiKind"],
            "pixelCount": per_render["neutral"]["metrics"]["roiPixelCount"],
            "skinMaskProvided": mask is not None,
        },
        "axes": {
            "tone": axis("tone_+1", "tone_-1"),
            "color": axis("color_+1", "color_-1"),
            "tone_at_color100": tc100,
        },
        "nativeEnvelope": {
            "tone_at_color100.hueDeltaDegrees": NATIVE_TC100_HUE_ENVELOPE,
            "tone_at_color100.rgDelta": NATIVE_TC100_RG_ENVELOPE,
        },
        "toneAtColor100HueInsideNativeEnvelope": inside,
        "renders": per_render,
    }
    with open(output_path, "w", encoding="utf-8") as handle:
        json.dump(result, handle, indent=2, sort_keys=True)
    print(json.dumps({
        "output": output_path,
        "roiKind": result["roi"]["kind"],
        "toneAtColor100": tc100,
        "insideNativeHueEnvelope": inside,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
