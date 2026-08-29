#!/usr/bin/env python3
"""Construct a complete candidate Photographic Style state from one image."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.apple_reverse_key1_training import _atomic_json, sha256_file
from xdremux_py.universal_photographic_style import (
    load_universal_image,
    load_universal_model,
    native_state_resources,
    predict_universal_state,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--checkpoint", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--device", choices=("auto", "mps", "cpu"), default="auto")
    parser.add_argument("--exiftool", default="exiftool")
    parser.add_argument(
        "--linear-rgb-sidecar",
        type=Path,
        help="decoded oriented 3x256x256 linear RGB .npy/.npz sidecar",
    )
    parser.add_argument(
        "--gain-map-sidecar",
        type=Path,
        help="decoded oriented 256x256 exposure-gain-ratio .npy/.npz sidecar",
    )
    args = parser.parse_args()

    image = load_universal_image(
        args.input,
        exiftool=args.exiftool,
        linear_rgb_sidecar=args.linear_rgb_sidecar,
        gain_map_sidecar=args.gain_map_sidecar,
    )
    model, checkpoint, device = load_universal_model(args.checkpoint, args.device)
    prediction, elapsed = predict_universal_state(image, model, device=device)
    resources = native_state_resources(image, prediction)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    paths = {
        "key1": output / "key1.bin",
        "gtc": output / "key3-gtc.bin",
        "c": output / "c.bin",
        "d": output / "d.bin",
    }
    for name, path in paths.items():
        path.write_bytes(resources[name])
    report = {
        "schema": "xdremux-universal-photographic-style-prediction-v1",
        "source": {
            "path": str(image.path),
            "sha256": image.source_sha256,
            "displayWidth": image.display_width,
            "displayHeight": image.display_height,
            "hasRAW": image.has_raw,
            "hasGainMap": image.has_gain_map,
            "usedLinearRGB": image.linear_rgb_features is not None,
            "usedGainMap": image.gain_map_features is not None,
            "make": image.source_make,
            "model": image.source_model,
        },
        "checkpoint": {
            "path": str(args.checkpoint.resolve()),
            "sha256": sha256_file(args.checkpoint.resolve()),
            "architecture": checkpoint.get("architecture"),
            "epoch": checkpoint.get("epoch"),
        },
        "runtime": {"device": device, "modelSeconds": elapsed},
        "resources": {
            name: {
                "path": str(path),
                "bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
            for name, path in paths.items()
        },
        "scalars": resources["scalars"],
        "uncertainty": resources["uncertainty"],
        "claimBoundary": (
            "Complete model state candidate only; native response and Photos consumer "
            "acceptance are separate gates."
        ),
    }
    _atomic_json(output / "report.json", report)
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
