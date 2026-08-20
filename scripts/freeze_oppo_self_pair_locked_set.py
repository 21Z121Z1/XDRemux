#!/usr/bin/env python3
"""Freeze a prospective, provenance-first OPPO self-pair consumer set."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
from pathlib import Path
from typing import Any


def _walk_sha256(value: Any, result: set[str]) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key.lower().endswith("sha256") and isinstance(child, str):
                result.add(child)
            _walk_sha256(child, result)
    elif isinstance(value, list):
        for child in value:
            _walk_sha256(child, result)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--historical-audit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--model-hash", action="append", default=[])
    parser.add_argument("--checkpoint-hash", action="append", default=[])
    args = parser.parse_args()

    inventory = json.loads(args.inventory.read_text(encoding="utf-8"))
    audit = json.loads(args.historical_audit.read_text(encoding="utf-8"))
    historical_names = {
        str(item.get("scene")) for item in audit.get("comparison", [])
    }
    historical_hashes: set[str] = set()
    _walk_sha256(audit, historical_hashes)
    rows = inventory.get("rows", [])
    eligible: list[dict[str, Any]] = []
    for row in rows:
        path = Path(str(row["path"]))
        digest = str(row["sha256"])
        if not path.is_file():
            continue
        if path.name.split(".", 1)[0] in historical_names:
            continue
        if digest in historical_hashes:
            continue
        eligible.append(row)
    by_model: dict[str, list[dict[str, Any]]] = {}
    for row in eligible:
        by_model.setdefault(str(row["model"]), []).append(row)
    selected: list[dict[str, Any]] = []
    for model in sorted(by_model):
        selected.append(min(by_model[model], key=lambda row: str(row["sha256"])))
    if not selected:
        raise SystemExit("no eligible prospective rows")
    created_at = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    manifest = {
        "schema": "xdremux-oppo-self-pair-consumer-locked-v1",
        "createdAt": created_at,
        "currentHEAD": args.head,
        "selectionRule": {
            "required": ["source file exists", "not historical solver-ab scene/hash"],
            "groupBy": "canonical model",
            "pick": "lexicographically smallest source sha256",
            "lookedAt": False,
            "usedForSelection": ["model", "source sha256", "path existence"],
        },
        "historicalBoundary": {
            "sceneNames": sorted(historical_names),
            "historicalArtifactSha256Count": len(historical_hashes),
            "oodInventoryWasLabelFreeObserved": True,
            "notUsedFor": ["iPhone training", "paired/self-pair weights", "solver tuning", "consumer tuning"],
        },
        "candidate": {
            "kind": "zero_shot_self_pair",
            "pairedEnsembleAlpha": 0.625,
            "modelHashes": sorted(args.model_hash),
            "checkpointHashes": sorted(args.checkpoint_hash),
        },
        "sampleCount": len(selected),
        "samples": [
            {
                "model": str(row["model"]),
                "sourcePath": str(Path(str(row["path"])).resolve()),
                "sha256": str(row["sha256"]),
                "suffix": str(row["suffix"]),
                "sourceInventoryFields": {
                    "make": row.get("make"),
                    "hasGainMap": row.get("hasGainMap"),
                    "hasRAW": row.get("hasRAW"),
                },
            }
            for row in selected
        ],
    }
    payload = json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    args.output.mkdir(parents=True, exist_ok=True)
    destination = args.output / "manifest.json"
    destination.write_text(payload, encoding="utf-8")
    print(json.dumps({"manifest": str(destination), "sha256": hashlib.sha256(payload.encode()).hexdigest(), "sampleCount": len(selected), "createdAt": created_at}, sort_keys=True))


if __name__ == "__main__":
    main()
