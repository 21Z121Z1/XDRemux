#!/usr/bin/env python3
"""Score a three-tone PISemanticStyleFilter sweep of one system-gain-map HEIF.

The shape being checked is the one already measured on the v16 Apple captures
and the earlier XDRemux outputs: tone +1 brightens, and that change is larger
than the tone −1 change. This is not the 11-tone product scorer.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

Y_WEIGHTS = np.array([0.22897456, 0.69173852, 0.07928691], dtype=np.float64)


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


def renderer_for(root: Path, tone: float) -> Path:
    matches = sorted(root.glob(f"**/tone-{tone:g}/renderer-0/_outputImageTexture.bin"))
    if len(matches) != 1:
        raise ValueError(f"{root}: expected one tone {tone:g} render, found {len(matches)}")
    return matches[0].parent


def main() -> None:
    root = Path(sys.argv[1])
    destination = Path(sys.argv[2])
    images = {tone: load_native_rgb(renderer_for(root, tone)) for tone in (-1.0, 0.0, 1.0)}
    luminance = {
        tone: float((image.astype(np.float64) @ Y_WEIGHTS).mean()) for tone, image in images.items()
    }
    signed_minus = luminance[-1.0] - luminance[0.0]
    signed_plus = luminance[1.0] - luminance[0.0]
    activated = abs(signed_plus) > 1.0e-4
    shape_matches = signed_plus > 0.0 and abs(signed_minus) < abs(signed_plus)
    report = {
        "mean_luminance": {f"{tone:g}": value for tone, value in luminance.items()},
        "signed_luminance_at_minus_1": signed_minus,
        "signed_luminance_at_plus_1": signed_plus,
        "tone_changes_render": activated,
        "matches_earlier_asymmetric_shape": shape_matches,
    }
    destination.write_text(json.dumps(report, indent=2) + "\n")
    print(destination.read_text(), end="")
    if not activated or not shape_matches:
        raise SystemExit("system gain-map Styles tone sweep did not keep the earlier shape")


if __name__ == "__main__":
    main()
