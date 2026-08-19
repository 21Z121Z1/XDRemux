#!/usr/bin/env python3
"""Run the exported ReverseKey1Net Core ML ensemble on one image pair."""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from scripts.predict_reverse_key1 import _read_fitted_rgb
from xdremux_py.apple_reverse_key1_training import (
    GRID_LONG,
    GRID_SHORT,
    encode_key1,
    input_features,
    sha256_file,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--styled", required=True, type=Path)
    parser.add_argument("--unstyled", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--reference-key1", type=Path)
    args = parser.parse_args()
    try:
        import coremltools as ct
    except ImportError as error:
        raise RuntimeError("coremltools is required for the Core ML probe") from error

    total_started = time.perf_counter()
    preprocess_started = time.perf_counter()
    styled, display_width, display_height = _read_fitted_rgb(args.styled.resolve())
    unstyled, _, _ = _read_fitted_rgb(args.unstyled.resolve())
    features = input_features(np.stack((styled, unstyled), axis=0))[None]
    preprocess_seconds = time.perf_counter() - preprocess_started
    load_started = time.perf_counter()
    runtime = ct.models.MLModel(str(args.model.resolve()), compute_units=ct.ComputeUnit.ALL)
    load_seconds = time.perf_counter() - load_started
    inference_started = time.perf_counter()
    prediction = np.asarray(runtime.predict({"features": features})["key1"], dtype=np.float32)
    inference_seconds = time.perf_counter() - inference_started
    prediction = prediction.reshape(1, GRID_LONG, GRID_LONG, 8, 10, 3)[0]
    landscape = display_width >= display_height
    width_slots = GRID_LONG if landscape else GRID_SHORT
    height_slots = GRID_SHORT if landscape else GRID_LONG
    payload = encode_key1(
        prediction, width_slots=width_slots, height_slots=height_slots
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(payload)
    parity = None
    if args.reference_key1 is not None:
        reference = np.fromfile(args.reference_key1.resolve(), dtype="<f2").astype(np.float32)
        actual = np.frombuffer(payload, dtype="<f2").astype(np.float32)
        absolute = np.abs(actual - reference)
        parity = {
            "reference": str(args.reference_key1.resolve()),
            "referenceSHA256": sha256_file(args.reference_key1.resolve()),
            "maximumAbsoluteError": float(absolute.max()),
            "meanAbsoluteError": float(absolute.mean()),
        }
    report = {
        "schema": "xdremux-reverse-key1-coreml-prediction-v1",
        "model": str(args.model.resolve()),
        "input": {
            "styled": str(args.styled.resolve()),
            "unstyled": str(args.unstyled.resolve()),
            "displayWidth": display_width,
            "displayHeight": display_height,
        },
        "output": {
            "path": str(args.output.resolve()),
            "sha256": sha256_file(args.output.resolve()),
            "finite": bool(np.isfinite(prediction).all()),
        },
        "timing": {
            "preprocessSeconds": preprocess_seconds,
            "modelLoadSeconds": load_seconds,
            "inferenceSeconds": inference_seconds,
            "totalSeconds": time.perf_counter() - total_started,
        },
        "parity": parity,
        "claimBoundary": "Core ML computeUnits=ALL runtime; Neural Engine placement is not proven.",
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
