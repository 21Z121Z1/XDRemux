#!/usr/bin/env python3
"""Run zero-shot self-pair proposals and auditable native-render probes.

This intentionally does not substitute a semantic proxy for consumer output. If
the private renderer cannot materialize the required target/style contract, each
scene is durably recorded as blocked with the exact command and traceback.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageOps

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from scripts.predict_reverse_key1 import _load_model, _predict, _read_fitted_rgb
from xdremux_py.apple_reverse_key1_training import input_features, _require_torch


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--manifest", type=Path, required=True)
    p.add_argument("--baseline", type=Path, required=True)
    p.add_argument("--candidate", type=Path, required=True)
    p.add_argument("--helper", required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    torch, _ = _require_torch()
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    baseline, baseline_ckpt = _load_model(torch, args.baseline.resolve(), device)
    candidate, candidate_ckpt = _load_model(torch, args.candidate.resolve(), device)
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    args.output.mkdir(parents=True, exist_ok=True)
    rows = []
    for sample in manifest["samples"]:
        source = Path(sample["sourcePath"])
        scene = sample["sha256"][:16]
        scene_dir = args.output / scene
        scene_dir.mkdir(parents=True, exist_ok=True)
        styled, width, height = _read_fitted_rgb(source)
        features = torch.from_numpy(
            input_features(np.stack((styled, styled), axis=0))
        ).unsqueeze(0)
        baseline_prediction = _predict(torch, baseline, baseline_ckpt, features, "__unknown__", device)
        candidate_prediction = _predict(torch, candidate, candidate_ckpt, features, "__unknown__", device)
        prediction = 0.375 * baseline_prediction + 0.625 * candidate_prediction
        prediction_path = scene_dir / "self-pair-key1.f32.npy"
        np.save(prediction_path, prediction.astype(np.float32))
        render_path = scene_dir / "disabled.png"
        render_manifest = scene_dir / "disabled.json"
        command = [args.helper, "--render-style", str(source), str(render_path), str(render_manifest), "0", "0", "1", "false", "1024", "Standard"]
        started = time.monotonic()
        completed = subprocess.run(command, capture_output=True, text=True, timeout=120, check=False)
        render = {
            "command": command,
            "exitCode": completed.returncode,
            "stdout": completed.stdout[-4000:],
            "stderr": completed.stderr[-4000:],
            "seconds": time.monotonic() - started,
            "outputExists": render_path.is_file(),
            "outputSHA256": digest(render_path) if render_path.is_file() else None,
        }
        response_path = scene_dir / "response-envelope.json"
        response_command = [
            sys.executable,
            str(Path(__file__).with_name("measure_style_editor_response.py")),
            "--input", str(source),
            "--output", str(response_path),
            "--helper", args.helper,
        ]
        response_started = time.monotonic()
        response_completed = subprocess.run(
            response_command, capture_output=True, text=True, timeout=180, check=False
        )
        response_probe = {
            "command": response_command,
            "exitCode": response_completed.returncode,
            "stdout": response_completed.stdout[-4000:],
            "stderr": response_completed.stderr[-4000:],
            "seconds": time.monotonic() - response_started,
            "outputExists": response_path.is_file(),
        }
        row = {
            "model": sample["model"],
            "sourcePath": str(source),
            "sourceSHA256": sample["sha256"],
            "inputWidth": width,
            "inputHeight": height,
            "inputMode": "single_image_self_pair",
            "proposal": {"path": str(prediction_path), "sha256": digest(prediction_path), "ensembleAlpha": 0.625},
            "routes": {
                "identity": {"status": "blocked", "reason": "no native target/consumer comparison"},
                "selfPairDirect": {"status": "proposal-only", "proposalPath": str(prediction_path)},
                "selfPairOneStep": {"status": "blocked", "reason": "bounded residual requires native target/response"},
                "fullSolver": {"status": "blocked", "reason": "native target/solver input contract unavailable"},
            },
            "nativeRendererProbe": render,
            "responseEnvelopeProbe": response_probe,
            "consumerMetrics": None,
            "responseEnvelope": None,
        }
        (scene_dir / "result.json").write_text(json.dumps(row, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        rows.append(row)
    report = {
        "schema": "xdremux-oppo-self-pair-consumer-locked-report-v1",
        "manifest": str(args.manifest.resolve()),
        "manifestSHA256": digest(args.manifest),
        "claimBoundary": "Prospective locked set; proposal and renderer capability evidence only; no consumer A/B promotion.",
        "candidate": {"baselineSHA256": digest(args.baseline), "candidateSHA256": digest(args.candidate), "ensembleAlpha": 0.625},
        "sampleCount": len(rows),
        "rows": rows,
        "aggregate": {"identity": None, "selfPairDirect": None, "selfPairOneStep": None, "fullSolver": None, "reason": "all five consumer comparisons blocked by missing native target/solver contract"},
    }
    (args.output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"report": str(args.output / "report.json"), "sampleCount": len(rows), "manifestSHA256": report["manifestSHA256"]}, sort_keys=True))


if __name__ == "__main__":
    main()
