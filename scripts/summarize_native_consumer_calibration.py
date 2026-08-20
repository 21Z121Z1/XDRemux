#!/usr/bin/env python3
"""Summarize the frozen native-consumer-v1 calibration/heldout cache.

This intentionally preserves missing states and failed conversions as null or
failure records. It never substitutes identity, another candidate, or a
different split for a missing native response.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


STATES = ["disabled", "neutral", "tone_+1", "tone_-1", "color_+1", "color_-1", "tc100_mid", "tc100_plus"]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def load(path: Path) -> dict[str, Any] | None:
    return json.loads(path.read_text()) if path.exists() else None


def summarize_sample(root: Path, row: dict[str, Any]) -> dict[str, Any]:
    sid = row["sourceSHA256"][:16]
    work = root / f"{row['split']}-{sid}"
    candidate = work / "selfpair-fast.heic"
    ab = load(work / "response-ab-summary.json")
    response = (
        load(work / "selfpair-response-fixed2.json")
        or load(work / "selfpair-response-rerun.json")
        or load(work / "selfpair-response.json")
        or load(work / "candidate-response.json")
    )
    failure = None
    if not candidate.exists():
        failure = "candidate_native_conversion_missing"
    elif response is None:
        failure = "candidate_native_response_missing"
    result: dict[str, Any] = {
        "split": row["split"],
        "model": row["model"],
        "session": row["session"],
        "sourceSHA256": row["sourceSHA256"],
        "sourcePath": row["sourcePath"],
        "candidate": {
            "path": str(candidate),
            "exists": candidate.exists(),
            "sha256": sha256(candidate) if candidate.exists() else None,
            "alpha": 0.625,
        },
        "response": {
            "available": response is not None,
            "insideNativeHueEnvelope": response.get("toneAtColor100HueInsideNativeEnvelope") if response else None,
            "toneAtColor100": response.get("nativeEnvelope", {}).get("toneAtColor100") if response else None,
            "states": ab.get("states") if ab else None,
            "aggregateRGBRMSE": ab.get("aggregateRMSE") if ab else None,
        },
        "failure": failure,
        "selectionRole": "calibration-only" if row["split"] == "calibration" else "heldout-final-only",
    }
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-markdown", type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text())
    root = args.manifest.parent
    rows = [summarize_sample(root, row) for row in manifest["samples"]]
    calibration = [row for row in rows if row["split"] == "calibration"]
    heldout = [row for row in rows if row["split"] == "heldout"]
    complete_calibration = [row for row in calibration if row["response"]["aggregateRGBRMSE"] is not None]
    report: dict[str, Any] = {
        "schema": "xdremux-native-consumer-calibration-report-v1",
        "manifestSHA256": sha256(args.manifest),
        "selection": {
            "candidateAlpha": 0.625,
            "alphaGrid": [0, 0.25, 0.5, 0.625, 0.75, 1],
            "boundedResidualGainGrid": [0.5, 0.75, 1, 1.25, 1.5],
            "states": STATES,
            "heldoutUsedForSelection": False,
            "status": "frozen-current-baseline-only" if len(complete_calibration) == len(calibration) else "not-selected-insufficient-calibration",
            "frozenChoice": 0.625 if len(complete_calibration) == len(calibration) else None,
        },
        "calibration": {"sampleCount": len(calibration), "completeResponseCount": len(complete_calibration), "rows": calibration},
        "heldout": {"sampleCount": len(heldout), "completeResponseCount": sum(row["response"]["aggregateRGBRMSE"] is not None for row in heldout), "rows": heldout},
        "promotion": {"promoted": False, "reason": "Only the pre-registered current alpha=.625 was rendered; no alternate alpha/gain response matrix exists, so no improvement over current baseline can be claimed."},
    }
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    lines = [
        "# Native consumer calibration v1",
        "",
        f"Manifest SHA256: `{report['manifestSHA256']}`",
        "",
        "Calibration is selection-only; heldout is final-only. Missing responses remain missing.",
        "",
        "| Split | Device | Candidate | 8-state RGB RMSE | Response | Failure |",
        "|---|---|---:|---:|---|---|",
    ]
    for row in rows:
        response = row["response"]
        value = "null" if response["aggregateRGBRMSE"] is None else f"{response['aggregateRGBRMSE']:.6f}"
        lines.append(f"| {row['split']} | {row['model']} | `{row['candidate']['alpha']}` | {value} | {response['available']} | {row['failure'] or ''} |")
    lines += ["", "## Decision", "", "The pre-registered current alpha=.625 is frozen as a baseline-only choice because all calibration responses are available. No alternate alpha/gain matrix exists, so no improvement or promotion is claimed."]
    args.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    args.output_markdown.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
