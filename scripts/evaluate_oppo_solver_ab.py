#!/usr/bin/env python3
"""Summarize cached OPPO consumer A/B artifacts without rerunning the solver.

Historical scenes were used for model/scale selection; this report marks them
unlocked and audits reusable artifacts rather than claiming generalization.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def probe_metrics(path: Path, index: int = 1) -> dict[str, float] | None:
    if not path.is_file():
        return None
    candidates = read_json(path).get("candidates")
    if not isinstance(candidates, list) or len(candidates) <= index:
        return None
    metrics = candidates[index].get("targetMetrics")
    if not isinstance(metrics, dict):
        return None
    return {key: float(metrics[key]) for key in ("mae8", "rmse8") if key in metrics}


def summarize_scene(scene: Path) -> dict[str, Any]:
    bounded_path = scene / "seeded-neutral" / "solver-result.json"
    full_path = scene / "baseline-solver" / "solver-result.json"
    bounded = read_json(bounded_path) if bounded_path.is_file() else {}
    full = read_json(full_path) if full_path.is_file() else {}
    model_probe = scene / "semantic-proxy-precomputed-coreml" / "probe.json"
    if not model_probe.is_file():
        model_probe = scene / "semantic-apply" / "probe.json"
    envelope = scene / "baseline-solver" / "response-envelope" / "response-envelope.json"
    envelope_data = read_json(envelope) if envelope.is_file() else {}
    return {
        "scene": scene.name,
        "locked": False,
        "lockedReason": "historical model/scale/solver selection artifacts are present",
        "paths": {
            "directModelProposal": probe_metrics(model_probe),
            "boundedOneStepResidual": bounded.get("bestMetrics"),
            "fullSolver": full.get("bestMetrics") or bounded.get("bestMetrics"),
            "identity": full.get("identityMetrics") or bounded.get("identityMetrics"),
        },
        "solver": {
            "boundedArtifact": str(bounded_path),
            "boundedArtifactSHA256": digest(bounded_path) if bounded_path.is_file() else None,
            "boundedSeconds": (bounded.get("timing") or {}).get("totalSeconds"),
            "fullArtifact": str(full_path),
            "fullArtifactSHA256": digest(full_path) if full_path.is_file() else None,
            "fullSeconds": (full.get("timing") or {}).get("totalSeconds"),
            "nativeResponseValidated": full.get("nativeResponseValidated") or bounded.get("nativeResponseValidated"),
            "renderRequestCount": full.get("renderRequestCount") or bounded.get("renderRequestCount"),
        },
        "responseEnvelope": {
            "available": envelope.is_file(),
            "schema": envelope_data.get("schema"),
            "passed": envelope_data.get("passed"),
            "falseAcceptCount": envelope_data.get("falseAcceptCount"),
            "directionReversalCount": envelope_data.get("directionReversalCount"),
            "claimBoundary": envelope_data.get("claimBoundary"),
        },
        "modelProbe": {
            "path": str(model_probe),
            "sha256": digest(model_probe) if model_probe.is_file() else None,
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    scenes = sorted(path for path in args.root.iterdir() if path.is_dir())
    report = {
        "schema": "xdremux-oppo-solver-ab-audit-v1",
        "root": str(args.root.resolve()),
        "lockedSet": {
            "status": "unavailable",
            "sceneCount": len(scenes),
            "reason": "all discovered scenes contain historical tuning artifacts; no untouched provenance was found",
        },
        "comparison": [summarize_scene(scene) for scene in scenes],
        "claimBoundary": "Historical cached consumer comparison only; no locked-set or Photos acceptance claim.",
    }
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
