#!/usr/bin/env python3
"""Build a label-free ReverseKey1Net runtime safety envelope."""

from __future__ import annotations

import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from scripts.evaluate_reverse_key1_ensemble import _load_model, _predict_split
from xdremux_py.apple_reverse_key1_training import (
    _atomic_json,
    _require_torch,
    load_manifest,
    runtime_gate_passes,
    runtime_gate_scores,
    sha256_file,
)


def _conformal_upper(values: list[float], alpha: float) -> float:
    if not values:
        raise ValueError("runtime envelope requires calibration samples")
    ordered = np.sort(np.asarray(values, dtype=np.float64))
    rank = min(len(ordered) - 1, math.ceil((len(ordered) + 1) * (1 - alpha)) - 1)
    return float(ordered[rank])


def _score_rows(
    values: list[dict[str, Any]], weight: float, scales: np.ndarray
) -> list[dict[str, Any]]:
    rows = []
    for value in values:
        prediction = (1.0 - weight) * value["baseline"] + weight * value["candidate"]
        rows.append(
            {
                "model": value["model"],
                "session": value["session"],
                "scores": runtime_gate_scores(
                    prediction,
                    value["baseline"],
                    value["candidate"],
                    scales,
                    value["mask"],
                ),
            }
        )
    return rows


def _coverage(
    rows: list[dict[str, Any]], thresholds: dict[str, float]
) -> dict[str, Any]:
    decisions = [runtime_gate_passes(row["scores"], thresholds) for row in rows]
    per_model: dict[str, list[bool]] = defaultdict(list)
    for row, accepted in zip(rows, decisions):
        per_model[row["model"]].append(accepted)
    return {
        "sampleCount": len(rows),
        "acceptedCount": int(sum(decisions)),
        "acceptedFraction": float(np.mean(decisions)),
        "perModelAcceptedFraction": {
            name: float(np.mean(values)) for name, values in sorted(per_model.items())
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--candidate-weight", required=True, type=float)
    parser.add_argument("--alpha", type=float, default=0.01)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if not 0.0 <= args.candidate_weight <= 1.0:
        raise ValueError("candidate weight must be between zero and one")
    if not 0.0 < args.alpha < 1.0:
        raise ValueError("alpha must be between zero and one")

    torch, _ = _require_torch()
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    manifest = args.manifest.resolve()
    header, samples = load_manifest(manifest)
    baseline, baseline_checkpoint = _load_model(torch, args.baseline.resolve(), device)
    candidate, candidate_checkpoint = _load_model(torch, args.candidate.resolve(), device)
    manifest_hash = sha256_file(manifest)
    for checkpoint in (baseline_checkpoint, candidate_checkpoint):
        if checkpoint.get("manifestSHA256") != manifest_hash:
            raise ValueError("checkpoint manifest provenance mismatch")
    scales = np.asarray(baseline_checkpoint["coefficientScales"], dtype=np.float32)
    if not np.array_equal(
        scales,
        np.asarray(candidate_checkpoint["coefficientScales"], dtype=np.float32),
    ):
        raise ValueError("checkpoint coefficient scales differ")
    vocabulary = tuple(baseline_checkpoint.get("profileVocabulary", ()))
    calibration = _predict_split(
        torch, manifest.parent, samples, "calibration", vocabulary,
        baseline, candidate, device,
    )
    heldout = _predict_split(
        torch, manifest.parent, samples, "heldout", vocabulary,
        baseline, candidate, device,
    )
    calibration_rows = _score_rows(calibration, args.candidate_weight, scales)
    heldout_rows = _score_rows(heldout, args.candidate_weight, scales)
    names = tuple(calibration_rows[0]["scores"])
    thresholds = {
        name: _conformal_upper(
            [row["scores"][name] for row in calibration_rows], args.alpha
        )
        for name in names
    }
    report = {
        "schema": "xdremux-reverse-key1-runtime-envelope-v1",
        "purpose": "label-free native-key1 distribution proxy",
        "device": device,
        "dataset": {
            "manifestSHA256": manifest_hash,
            "corpusSHA256": header["corpusSHA256"],
        },
        "ensemble": {
            "baselineSHA256": sha256_file(args.baseline.resolve()),
            "candidateSHA256": sha256_file(args.candidate.resolve()),
            "candidateWeight": args.candidate_weight,
        },
        "selection": {
            "source": "calibration-only",
            "alpha": args.alpha,
            "calibrationSampleCount": len(calibration_rows),
            "thresholds": thresholds,
        },
        "coverage": {
            "calibration": _coverage(calibration_rows, thresholds),
            "heldout": _coverage(heldout_rows, thresholds),
        },
        "claimBoundary": {
            "nativeKey1DistributionProxy": True,
            "consumerResponseEquivalent": False,
            "replacesNeutrinoResponseValidation": False,
        },
    }
    _atomic_json(args.output.resolve(), report)
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
