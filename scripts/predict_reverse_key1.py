#!/usr/bin/env python3
"""Predict an Apple reverse key1 for one styled/unstyled image pair."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image, ImageOps

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.apple_reverse_key1_training import (
    GRID_LONG,
    GRID_SHORT,
    _fit_rgb,
    _require_torch,
    build_model,
    encode_key1,
    input_features,
    runtime_gate_passes,
    runtime_gate_scores,
    sha256_file,
)


def _architecture(checkpoint: dict[str, Any]) -> str:
    name = str(checkpoint.get("architecture") or "")
    return "multiscale_large" if "multiscale-large" in name else "small"


def _load_model(torch: Any, path: Path, device: str) -> tuple[Any, dict[str, Any]]:
    checkpoint = torch.load(path, map_location="cpu", weights_only=False)
    scales = np.asarray(checkpoint["coefficientScales"], dtype=np.float32)
    vocabulary = tuple(checkpoint.get("profileVocabulary", ()))
    model = build_model(
        scales,
        profile_count=len(vocabulary),
        architecture=_architecture(checkpoint),
    )
    model.load_state_dict(checkpoint["model"])
    model.to(device).eval()
    return model, checkpoint


def _read_fitted_rgb(path: Path) -> tuple[np.ndarray, int, int]:
    try:
        import pillow_heif
    except ImportError:
        pillow_heif = None
    if pillow_heif is not None:
        pillow_heif.register_heif_opener()
    with Image.open(path) as image:
        display = ImageOps.exif_transpose(image)
        width, height = display.size
        return _fit_rgb(display), width, height


def _profile_id(torch: Any, checkpoint: dict[str, Any], requested: str) -> Any | None:
    vocabulary = tuple(checkpoint.get("profileVocabulary", ()))
    if not vocabulary:
        return None
    fallback = vocabulary.index("__unknown__") if "__unknown__" in vocabulary else 0
    selected = vocabulary.index(requested) if requested in vocabulary else fallback
    return torch.tensor([selected], dtype=torch.long)


def _predict(
    torch: Any,
    model: Any,
    checkpoint: dict[str, Any],
    features: Any,
    requested_profile: str,
    device: str,
) -> np.ndarray:
    profile_id = _profile_id(torch, checkpoint, requested_profile)
    with torch.no_grad():
        if profile_id is None:
            value = model(features.to(device))
        else:
            value = model(features.to(device), profile_id.to(device))
    return value.cpu().numpy()[0]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--styled", required=True, type=Path)
    parser.add_argument("--unstyled", required=True, type=Path)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--candidate-weight", type=float, default=0.0)
    parser.add_argument("--profile", default="__unknown__")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--envelope", type=Path)
    args = parser.parse_args()
    if not 0.0 <= args.candidate_weight <= 1.0:
        raise ValueError("candidate weight must be between zero and one")
    if args.candidate_weight > 0 and args.candidate is None:
        raise ValueError("candidate checkpoint is required for a nonzero weight")

    torch, _ = _require_torch()
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    styled, display_width, display_height = _read_fitted_rgb(args.styled.resolve())
    unstyled, _, _ = _read_fitted_rgb(args.unstyled.resolve())
    features = torch.from_numpy(
        input_features(np.stack((styled, unstyled), axis=0))
    ).unsqueeze(0)

    baseline, baseline_checkpoint = _load_model(
        torch, args.baseline.resolve(), device
    )
    baseline_prediction = _predict(
        torch,
        baseline,
        baseline_checkpoint,
        features,
        args.profile,
        device,
    )
    prediction = baseline_prediction
    candidate_prediction = baseline_prediction
    checkpoints = [
        {
            "role": "baseline",
            "path": str(args.baseline.resolve()),
            "sha256": sha256_file(args.baseline.resolve()),
            "architecture": baseline_checkpoint.get("architecture"),
        }
    ]
    if args.candidate is not None and args.candidate_weight > 0:
        candidate, candidate_checkpoint = _load_model(
            torch, args.candidate.resolve(), device
        )
        candidate_prediction = _predict(
            torch,
            candidate,
            candidate_checkpoint,
            features,
            args.profile,
            device,
        )
        prediction = (
            (1.0 - args.candidate_weight) * prediction
            + args.candidate_weight * candidate_prediction
        )
        checkpoints.append(
            {
                "role": "candidate",
                "path": str(args.candidate.resolve()),
                "sha256": sha256_file(args.candidate.resolve()),
                "architecture": candidate_checkpoint.get("architecture"),
            }
        )

    landscape = display_width >= display_height
    width_slots = GRID_LONG if landscape else GRID_SHORT
    height_slots = GRID_SHORT if landscape else GRID_LONG
    mask = np.zeros((GRID_LONG, GRID_LONG), dtype=np.bool_)
    mask[:height_slots, :width_slots] = True
    native_distribution_gate = None
    if args.envelope is not None:
        envelope_path = args.envelope.resolve()
        envelope = json.loads(envelope_path.read_text(encoding="utf-8"))
        if envelope.get("schema") != "xdremux-reverse-key1-runtime-envelope-v1":
            raise ValueError("unsupported runtime envelope schema")
        ensemble = envelope["ensemble"]
        if ensemble["baselineSHA256"] != checkpoints[0]["sha256"]:
            raise ValueError("runtime envelope baseline checkpoint differs")
        if len(checkpoints) != 2 or ensemble["candidateSHA256"] != checkpoints[1]["sha256"]:
            raise ValueError("runtime envelope candidate checkpoint differs")
        if not np.isclose(float(ensemble["candidateWeight"]), args.candidate_weight):
            raise ValueError("runtime envelope candidate weight differs")
        scores = runtime_gate_scores(
            prediction,
            baseline_prediction,
            candidate_prediction,
            np.asarray(baseline_checkpoint["coefficientScales"], dtype=np.float32),
            mask,
        )
        thresholds = {
            name: float(value)
            for name, value in envelope["selection"]["thresholds"].items()
        }
        native_distribution_gate = {
            "passed": runtime_gate_passes(scores, thresholds),
            "scores": scores,
            "thresholds": thresholds,
            "envelope": str(envelope_path),
            "envelopeSHA256": sha256_file(envelope_path),
            "claimBoundary": "Native key1 distribution proxy; not a Neutrino response-equivalence result.",
        }
    payload = encode_key1(
        prediction, width_slots=width_slots, height_slots=height_slots
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(payload)
    report_path = args.report or args.output.with_suffix(args.output.suffix + ".json")
    report = {
        "schema": "xdremux-reverse-key1-single-prediction-v1",
        "device": device,
        "input": {
            "styled": str(args.styled.resolve()),
            "styledSHA256": sha256_file(args.styled.resolve()),
            "unstyled": str(args.unstyled.resolve()),
            "unstyledSHA256": sha256_file(args.unstyled.resolve()),
            "displayWidth": display_width,
            "displayHeight": display_height,
        },
        "model": {
            "requestedProfile": args.profile,
            "candidateWeight": args.candidate_weight,
            "checkpoints": checkpoints,
        },
        "output": {
            "path": str(args.output.resolve()),
            "sha256": sha256_file(args.output.resolve()),
            "bytes": len(payload),
            "finite": bool(np.isfinite(prediction).all()),
            "maximumAbsoluteValue": float(np.abs(prediction).max()),
            "gridWidth": width_slots,
            "gridHeight": height_slots,
        },
        "nativeDistributionGate": native_distribution_gate,
        "claimBoundary": "Offline OOD key1 prediction; native consumer behavior is not implied.",
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
