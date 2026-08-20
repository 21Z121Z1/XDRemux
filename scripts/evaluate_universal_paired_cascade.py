#!/usr/bin/env python3
"""Evaluate Universal -> frozen paired ReverseKey1 cascade on held-out iPhone.

All weights and alpha are fixed before held-out evaluation.  The paired oracle
uses the native disabled thumbnail; the cascade uses Universal's predicted
unstyled thumbnail and never exposes the paired target to runtime inference.
"""
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
    _require_torch,
    build_model,
    device_profile_vocabulary,
    input_features,
    load_manifest as load_paired_manifest,
)
from xdremux_py.universal_photographic_style_training import (
    _UniversalDataset,
    build_universal_model,
    load_universal_manifest,
    primary_image_features,
    universal_state_statistics,
)


def _load_universal(torch: Any, path: Path, stats: dict[str, np.ndarray], device: str) -> Any:
    checkpoint = torch.load(path.resolve(), map_location="cpu", weights_only=False)
    model = build_universal_model(stats, architecture=checkpoint.get("architectureConfig", "multiscale_large"))
    model.load_state_dict(checkpoint["model"])
    return model.to(device).eval(), checkpoint


def _load_paired(torch: Any, path: Path, device: str) -> tuple[Any, dict[str, Any]]:
    checkpoint = torch.load(path.resolve(), map_location="cpu", weights_only=False)
    vocabulary = tuple(checkpoint.get("profileVocabulary", ()))
    architecture = "multiscale_large" if "multiscale-large" in str(checkpoint.get("architecture", "")) else "small"
    model = build_model(np.asarray(checkpoint["coefficientScales"], dtype=np.float32), profile_count=len(vocabulary), architecture=architecture)
    model.load_state_dict(checkpoint["model"])
    return model.to(device).eval(), checkpoint


def _metrics(values: list[dict[str, Any]], key: str, scales: np.ndarray) -> dict[str, Any]:
    errors: list[float] = []
    by_model: dict[str, list[float]] = defaultdict(list)
    by_session: dict[str, list[float]] = defaultdict(list)
    for value in values:
        error = np.abs(value[key] - value["target"]) / scales
        selected = error[value["mask"]]
        score = float(selected.mean())
        errors.append(score)
        by_model[value["model"]].append(score)
        by_session[value["session"]].append(score)
    return {
        "normalizedMAE": float(np.mean(errors)),
        "perDeviceNormalizedMAE": {name: float(np.mean(v)) for name, v in sorted(by_model.items())},
        "perSessionNormalizedMAE": {name: float(np.mean(v)) for name, v in sorted(by_session.items())},
        "sessionCount": len(by_session),
        "sampleCount": len(errors),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--universal-manifest", required=True, type=Path)
    parser.add_argument("--universal-checkpoint", required=True, type=Path)
    parser.add_argument("--paired-baseline", required=True, type=Path)
    parser.add_argument("--paired-candidate", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--alpha", type=float, default=0.625)
    parser.add_argument("--unstyled-upsample", choices=("nearest", "bilinear", "bicubic"), default="bilinear")
    args = parser.parse_args()
    if not 0 <= args.alpha <= 1:
        raise ValueError("alpha must be in [0, 1]")
    torch, _ = _require_torch()
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    uh, records = load_universal_manifest(args.universal_manifest.resolve())
    heldout = [r for r in records if r["split"] == "heldout"]
    stats = universal_state_statistics(args.universal_manifest.resolve(), records)
    universal, universal_checkpoint = _load_universal(torch, args.universal_checkpoint, stats, device)
    baseline, baseline_checkpoint = _load_paired(torch, args.paired_baseline, device)
    candidate, candidate_checkpoint = _load_paired(torch, args.paired_candidate, device)
    scales = np.asarray(baseline_checkpoint["coefficientScales"], dtype=np.float32)
    if not np.array_equal(scales, np.asarray(candidate_checkpoint["coefficientScales"], dtype=np.float32)):
        raise ValueError("paired checkpoint coefficient scales differ")
    vocabulary = tuple(baseline_checkpoint.get("profileVocabulary", ()))
    profile_ids = {name: i for i, name in enumerate(vocabulary)}
    unknown = profile_ids.get("__unknown__", 0)
    paths = [Path(str(r["samplePath"])) for r in heldout]
    rows: list[dict[str, Any]] = []
    for start in range(0, len(heldout), 6):
        batch_records = heldout[start : start + 6]
        images: list[np.ndarray] = []
        primary: list[np.ndarray] = []
        metadata: list[np.ndarray] = []
        metadata_mask: list[np.ndarray] = []
        targets: list[np.ndarray] = []
        masks: list[np.ndarray] = []
        actual_unstyled: list[np.ndarray] = []
        for record in batch_records:
            with np.load(str(record["samplePath"]), allow_pickle=False) as sample:
                image_pair = np.asarray(sample["images"], dtype=np.uint8)
                targets.append(np.asarray(sample["key1"], dtype=np.float32))
                masks.append(np.asarray(sample["mask"], dtype=np.bool_))
            images.append(image_pair)
            primary.append(primary_image_features(image_pair[0]))
            actual_unstyled.append(image_pair[1].astype(np.float32) / 255.0)
            index = int(record["index"])
            labels = np.load(args.universal_manifest.resolve().parent / "labels.npz", allow_pickle=False)
            metadata.append(labels["metadata"][index].astype(np.float32))
            metadata_mask.append(labels["metadata_mask"][index].astype(np.float32))
        p = torch.from_numpy(np.stack(primary)).to(device)
        m = torch.from_numpy(np.stack(metadata)).to(device)
        mm = torch.from_numpy(np.stack(metadata_mask)).to(device)
        with torch.no_grad():
            universal_output = universal(p, m, mm)
            predicted_unstyled = universal_output["unstyled"]
            styled = p[:, :3]
            interpolation = {"nearest": {}, "bilinear": {"align_corners": False}, "bicubic": {"align_corners": False}}
            predicted_up = torch.nn.functional.interpolate(
                predicted_unstyled, size=(256, 256), mode=args.unstyled_upsample,
                **interpolation[args.unstyled_upsample]
            )
            actual = torch.from_numpy(np.stack(actual_unstyled)).to(device)
            shuffled = torch.roll(actual, shifts=1, dims=0)
            feature_sets = {
                "actualUnstyled": actual,
                "predictedUnstyled": predicted_up,
                "primaryAsUnstyled": styled,
                "shuffledUnstyled": shuffled,
            }
            predictions: dict[str, Any] = {}
            for name, unstyled in feature_sets.items():
                features = torch.cat((styled, unstyled, styled - unstyled), dim=1)
                matrix = torch.tensor([[0.2126, 0.7152, 0.0722], [-0.114572, -0.385428, 0.5], [0.5, -0.454153, -0.045847]], device=device)
                ycbcr = torch.einsum("oc,bchw->bohw", matrix, styled - unstyled)
                features = torch.cat((features, ycbcr), dim=1)
                ids = torch.tensor([profile_ids.get(str(r.get("Model") or "unknown"), unknown) for r in batch_records], device=device)
                predictions[name] = ((1.0 - args.alpha) * baseline(features, ids) + args.alpha * candidate(features))
            for idx, record in enumerate(batch_records):
                row = {"model": str(record.get("Model") or "unknown"), "session": str(record["captureSession"]), "target": targets[idx], "mask": masks[idx]}
                row["directUniversal"] = universal_output["key1"][idx].detach().cpu().numpy()
                for name, prediction in predictions.items():
                    row["pairedOracle" if name == "actualUnstyled" else name] = prediction[idx].detach().cpu().numpy()
                rows.append(row)
    report = {
        "schema": "xdremux-universal-paired-cascade-evaluation-v1",
        "device": device,
        "split": {"name": "heldout", "sampleCount": len(heldout), "sessionCount": len({r["captureSession"] for r in heldout}), "manifestSHA256": __import__("hashlib").sha256(args.universal_manifest.resolve().read_bytes()).hexdigest()},
        "alpha": args.alpha,
        "unstyledUpsample": args.unstyled_upsample,
        "checkpoints": {"universal": str(args.universal_checkpoint.resolve()), "pairedBaseline": str(args.paired_baseline.resolve()), "pairedCandidate": str(args.paired_candidate.resolve())},
        "metrics": {name: _metrics(rows, name, scales) for name in ("directUniversal", "pairedOracle", "predictedUnstyled", "primaryAsUnstyled", "shuffledUnstyled")},
        "claimBoundary": "Held-out iPhone cascade diagnostic only; no OPPO locked-set, Photos, or runtime promotion claim.",
    }
    args.output.resolve().parent.mkdir(parents=True, exist_ok=True)
    args.output.resolve().write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
