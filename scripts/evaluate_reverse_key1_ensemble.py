#!/usr/bin/env python3
"""Select and verify a ReverseKey1Net ensemble without held-out leakage."""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.apple_reverse_key1_training import (
    _CachedDataset,
    _atomic_json,
    _require_torch,
    build_model,
    load_manifest,
    select_linear_blend_weight,
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


def _predict_split(
    torch: Any,
    root: Path,
    samples: list[dict[str, Any]],
    split: str,
    vocabulary: tuple[str, ...],
    baseline: Any,
    candidate: Any,
    device: str,
) -> list[dict[str, Any]]:
    values = [sample for sample in samples if sample["split"] == split]
    loader = torch.utils.data.DataLoader(
        _CachedDataset(root, values, vocabulary), batch_size=8, shuffle=False
    )
    result: list[dict[str, Any]] = []
    with torch.no_grad():
        for features, target, mask, models, sessions, profile_ids in loader:
            features = features.to(device)
            baseline_prediction = baseline(
                features, profile_ids.to(device)
            ).cpu().numpy()
            candidate_prediction = candidate(features).cpu().numpy()
            for index, (model_name, session) in enumerate(zip(models, sessions)):
                result.append(
                    {
                        "model": str(model_name),
                        "session": str(session),
                        "baseline": baseline_prediction[index],
                        "candidate": candidate_prediction[index],
                        "target": target[index].numpy(),
                        "mask": mask[index].numpy(),
                    }
                )
    return result


def _metrics(
    values: list[dict[str, Any]], weight: float, scales: np.ndarray
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    per_model: dict[str, list[float]] = defaultdict(list)
    all_errors = []
    paired = []
    for value in values:
        prediction = (
            (1.0 - weight) * value["baseline"]
            + weight * value["candidate"]
        )
        baseline_error = np.abs(value["baseline"] - value["target"]) / scales
        blend_error = np.abs(prediction - value["target"]) / scales
        baseline_mae = float(baseline_error[value["mask"]].mean())
        blend_selected = blend_error[value["mask"]]
        blend_mae = float(blend_selected.mean())
        per_model[value["model"]].append(blend_mae)
        all_errors.append(blend_selected.reshape(-1))
        paired.append(
            {
                "model": value["model"],
                "session": value["session"],
                "baseline": baseline_mae,
                "blend": blend_mae,
                "delta": blend_mae - baseline_mae,
            }
        )
    per_model_mean = {
        name: float(np.mean(errors)) for name, errors in sorted(per_model.items())
    }
    return (
        {
            "normalizedMAE": float(np.concatenate(all_errors).mean()),
            "macroModelNormalizedMAE": float(
                np.mean(list(per_model_mean.values()))
            ),
            "perModelNormalizedMAE": per_model_mean,
        },
        paired,
    )


def _bootstrap(
    paired: list[dict[str, Any]], iterations: int, seed: int
) -> dict[str, Any]:
    by_session: dict[str, list[float]] = defaultdict(list)
    for value in paired:
        by_session[value["session"]].append(value["delta"])
    sessions = sorted(by_session)
    rng = np.random.default_rng(seed)
    means = np.empty(iterations, dtype=np.float64)
    for index in range(iterations):
        selected = rng.choice(sessions, size=len(sessions), replace=True)
        sampled = [delta for session in selected for delta in by_session[session]]
        means[index] = np.mean(sampled)
    observed = float(np.mean([value["delta"] for value in paired]))
    return {
        "sampleCount": len(paired),
        "sessionCount": len(sessions),
        "meanDelta": observed,
        "relativePercent": float(
            observed / np.mean([value["baseline"] for value in paired]) * 100
        ),
        "clusterBootstrap95PercentCI": [
            float(np.quantile(means, 0.025)),
            float(np.quantile(means, 0.975)),
        ],
        "bootstrapProbabilityImproved": float(np.mean(means < 0)),
        "improvedSampleFraction": float(
            np.mean([value["delta"] < 0 for value in paired])
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--grid-size", type=int, default=41)
    parser.add_argument("--bootstrap-iterations", type=int, default=20_000)
    parser.add_argument("--seed", type=int, default=260819)
    args = parser.parse_args()
    if args.bootstrap_iterations < 100:
        raise ValueError("bootstrap iterations must be at least 100")

    torch, _ = _require_torch()
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    manifest = args.manifest.resolve()
    header, samples = load_manifest(manifest)
    baseline, baseline_checkpoint = _load_model(
        torch, args.baseline.resolve(), device
    )
    candidate, candidate_checkpoint = _load_model(
        torch, args.candidate.resolve(), device
    )
    expected_manifest_hash = sha256_file(manifest)
    for checkpoint in (baseline_checkpoint, candidate_checkpoint):
        if checkpoint.get("manifestSHA256") != expected_manifest_hash:
            raise ValueError("checkpoint manifest provenance mismatch")
    baseline_scales = np.asarray(
        baseline_checkpoint["coefficientScales"], dtype=np.float32
    )
    candidate_scales = np.asarray(
        candidate_checkpoint["coefficientScales"], dtype=np.float32
    )
    if not np.array_equal(baseline_scales, candidate_scales):
        raise ValueError("checkpoint coefficient scales differ")
    vocabulary = tuple(baseline_checkpoint.get("profileVocabulary", ()))
    calibration = _predict_split(
        torch,
        manifest.parent,
        samples,
        "calibration",
        vocabulary,
        baseline,
        candidate,
        device,
    )
    heldout = _predict_split(
        torch,
        manifest.parent,
        samples,
        "heldout",
        vocabulary,
        baseline,
        candidate,
        device,
    )
    weight, calibration_mae = select_linear_blend_weight(
        np.stack([value["baseline"] for value in calibration]),
        np.stack([value["candidate"] for value in calibration]),
        np.stack([value["target"] for value in calibration]),
        baseline_scales,
        np.stack([value["mask"] for value in calibration]),
        grid_size=args.grid_size,
    )
    calibration_metrics, _ = _metrics(calibration, weight, baseline_scales)
    heldout_metrics, paired = _metrics(heldout, weight, baseline_scales)
    baseline_calibration_metrics, _ = _metrics(
        calibration, 0.0, baseline_scales
    )
    baseline_heldout_metrics, _ = _metrics(heldout, 0.0, baseline_scales)
    candidate_calibration_metrics, _ = _metrics(
        calibration, 1.0, baseline_scales
    )
    candidate_heldout_metrics, _ = _metrics(heldout, 1.0, baseline_scales)
    models = sorted({value["model"] for value in paired})
    bootstrap = {
        "overall": _bootstrap(paired, args.bootstrap_iterations, args.seed),
        "perModel": {
            model_name: _bootstrap(
                [value for value in paired if value["model"] == model_name],
                args.bootstrap_iterations,
                args.seed + index + 1,
            )
            for index, model_name in enumerate(models)
        },
    }
    accepted = (
        bootstrap["overall"]["clusterBootstrap95PercentCI"][1] < 0
        and all(
            value["clusterBootstrap95PercentCI"][1] < 0
            for value in bootstrap["perModel"].values()
        )
    )
    report = {
        "schema": "xdremux-reverse-key1-ensemble-report-v1",
        "device": device,
        "dataset": {
            "manifestSHA256": expected_manifest_hash,
            "corpusSHA256": header["corpusSHA256"],
        },
        "baseline": {
            "path": str(args.baseline.resolve()),
            "sha256": sha256_file(args.baseline.resolve()),
            "architecture": baseline_checkpoint.get("architecture"),
        },
        "candidate": {
            "path": str(args.candidate.resolve()),
            "sha256": sha256_file(args.candidate.resolve()),
            "architecture": candidate_checkpoint.get("architecture"),
        },
        "selection": {
            "source": "calibration-only",
            "gridSize": args.grid_size,
            "candidateWeight": weight,
            "calibrationNormalizedMAE": calibration_mae,
        },
        "calibration": calibration_metrics,
        "heldout": heldout_metrics,
        "comparators": {
            "baseline": {
                "calibration": baseline_calibration_metrics,
                "heldout": baseline_heldout_metrics,
            },
            "candidate": {
                "calibration": candidate_calibration_metrics,
                "heldout": candidate_heldout_metrics,
            },
        },
        "bootstrap": bootstrap,
        "acceptance": {
            "allSessionClusterIntervalsBelowZero": accepted,
            "offlineCandidateAccepted": accepted,
            "nativeConsumerAccepted": False,
        },
    }
    _atomic_json(args.output.resolve(), report)
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
