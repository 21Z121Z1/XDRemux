#!/usr/bin/env python3
"""Measure generic-image coverage, confidence, and latency without copying media."""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.apple_reverse_key1_training import (
    GRID_LONG,
    GRID_SHORT,
    ReverseKey1Error,
    _atomic_json,
    _require_torch,
    sha256_file,
)
from xdremux_py.universal_photographic_style import (
    SUPPORTED_IMAGE_SUFFIXES,
    load_universal_image,
    load_universal_model,
    predict_universal_state,
)
from xdremux_py.universal_photographic_style_training import (
    _UniversalDataset,
    load_universal_manifest,
)


def calibration_threshold(
    model: object,
    device: str,
    manifest: Path,
    quantile: float,
    batch_size: int,
) -> tuple[float, int]:
    torch, _ = _require_torch()
    _, records = load_universal_manifest(manifest)
    calibration = [record for record in records if record["split"] == "calibration"]
    loader = torch.utils.data.DataLoader(
        _UniversalDataset(manifest, calibration),
        batch_size=batch_size,
        num_workers=0,
    )
    values: list[float] = []
    with torch.no_grad():
        for batch in loader:
            output = model(batch[0].to(device), batch[1].to(device), batch[2].to(device))
            values.extend(
                output["key1LogVariance"]
                .exp()
                .mean(dim=(1, 2, 3))
                .detach()
                .cpu()
                .numpy()
                .tolist()
            )
    return float(np.quantile(values, quantile)), len(values)


def _summary(values: list[float]) -> dict[str, float]:
    if not values:
        return {}
    return {
        "median": float(np.median(values)),
        "p95": float(np.quantile(values, 0.95)),
        "maximum": float(np.max(values)),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", action="append", required=True, type=Path)
    parser.add_argument("--checkpoint", required=True, type=Path)
    parser.add_argument("--calibration-manifest", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--maker-contains", default="")
    parser.add_argument("--uncertainty-quantile", type=float, default=0.95)
    parser.add_argument("--batch-size", type=int, default=6)
    parser.add_argument("--device", choices=("auto", "mps", "cpu"), default="auto")
    parser.add_argument("--exiftool", default="exiftool")
    args = parser.parse_args()
    if not 0.5 <= args.uncertainty_quantile < 1.0:
        raise ValueError("uncertainty quantile must be in [0.5, 1)")

    roots = [root.resolve() for root in args.root]
    paths = sorted(
        {
            path.resolve()
            for root in roots
            for path in root.rglob("*")
            if path.is_file() and path.suffix.lower() in SUPPORTED_IMAGE_SUFFIXES
        },
        key=str,
    )
    model, checkpoint, device = load_universal_model(args.checkpoint, args.device)
    threshold, calibration_count = calibration_threshold(
        model,
        device,
        args.calibration_manifest.resolve(),
        args.uncertainty_quantile,
        args.batch_size,
    )
    rows: list[dict[str, object]] = []
    failures: list[dict[str, str]] = []
    seen_hashes: set[str] = set()
    duplicate_count = 0
    maker_filter = args.maker_contains.casefold()
    for index, path in enumerate(paths, 1):
        started = time.perf_counter()
        try:
            image = load_universal_image(path, exiftool=args.exiftool)
            if maker_filter and maker_filter not in (image.source_make or "").casefold():
                continue
            if image.source_sha256 in seen_hashes:
                duplicate_count += 1
                continue
            seen_hashes.add(image.source_sha256)
            prediction, model_seconds = predict_universal_state(image, model, device=device)
            uncertainty = float(np.exp(prediction["key1LogVariance"]).mean())
            landscape = image.display_width >= image.display_height
            width_slots = GRID_LONG if landscape else GRID_SHORT
            height_slots = GRID_SHORT if landscape else GRID_LONG
            normalized = (prediction["key1"] - model.identity.detach().cpu().numpy()[0]) / (
                model.key1_scale.detach().cpu().numpy()[0]
            )
            selected = normalized[:height_slots, :width_slots]
            residual_rms = float(np.sqrt(np.mean(selected * selected)))
            rows.append(
                {
                    "path": str(path),
                    "sha256": image.source_sha256,
                    "suffix": path.suffix.lower(),
                    "make": image.source_make,
                    "model": image.source_model,
                    "hasRAW": image.has_raw,
                    "hasGainMap": image.has_gain_map,
                    "uncertainty": uncertainty,
                    "normalizedResidualRMS": residual_rms,
                    "modelSeconds": model_seconds,
                    "totalSeconds": time.perf_counter() - started,
                    "fastPathEligible": uncertainty <= threshold,
                }
            )
        except Exception as error:
            failures.append({"path": str(path), "error": str(error)[:1000]})
        if index % 25 == 0:
            print(
                json.dumps(
                    {"scanned": index, "eligibleInputs": len(rows), "failures": len(failures)}
                ),
                flush=True,
            )

    uncertainties = [float(row["uncertainty"]) for row in rows]
    model_durations = [float(row["modelSeconds"]) for row in rows]
    total_durations = [float(row["totalSeconds"]) for row in rows]
    report = {
        "schema": "xdremux-universal-photographic-style-ood-evaluation-v1",
        "roots": [str(root) for root in roots],
        "checkpoint": {
            "path": str(args.checkpoint.resolve()),
            "sha256": sha256_file(args.checkpoint.resolve()),
            "architecture": checkpoint.get("architecture"),
            "epoch": checkpoint.get("epoch"),
        },
        "calibration": {
            "manifest": str(args.calibration_manifest.resolve()),
            "sampleCount": calibration_count,
            "uncertaintyQuantile": args.uncertainty_quantile,
            "uncertaintyThreshold": threshold,
        },
        "inventory": {
            "candidateFileCount": len(paths),
            "evaluatedUniqueCount": len(rows),
            "duplicateCount": duplicate_count,
            "failureCount": len(failures),
            "formatCounts": dict(Counter(str(row["suffix"]) for row in rows)),
            "modelCounts": dict(Counter(str(row["model"]) for row in rows)),
            "gainMapCount": sum(bool(row["hasGainMap"]) for row in rows),
            "rawCount": sum(bool(row["hasRAW"]) for row in rows),
        },
        "coverage": {
            "fastPathEligibleCount": sum(bool(row["fastPathEligible"]) for row in rows),
            "fastPathEligibleFraction": (
                sum(bool(row["fastPathEligible"]) for row in rows) / len(rows) if rows else 0.0
            ),
            "uncertainty": _summary(uncertainties),
        },
        "performance": {
            "device": device,
            "modelSeconds": _summary(model_durations),
            "stateConstructionSeconds": _summary(total_durations),
        },
        "rows": rows,
        "failures": failures,
        "claimBoundary": (
            "Label-free OOD coverage and latency only. Eligibility predicts when to try the "
            "fast path; it does not prove Apple response or Photos consumer correctness."
        ),
    }
    _atomic_json(args.output.resolve(), report)
    print(json.dumps({key: report[key] for key in ("inventory", "coverage", "performance")}, indent=2))


if __name__ == "__main__":
    main()
