#!/usr/bin/env python3
"""Freeze small calibration/held-out native-iPhone consumer cohorts."""
from __future__ import annotations
import argparse, datetime as dt, hashlib, json
from pathlib import Path

def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--dataset", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--head", required=True)
    p.add_argument("--model-hash", action="append", default=[])
    p.add_argument("--checkpoint-hash", action="append", default=[])
    a = p.parse_args()
    value = json.loads(a.dataset.read_text())
    selected = []
    for split in ("calibration", "heldout"):
        rows = [r for r in value["samples"] if r.get("split") == split and Path(r["sourcePath"]).is_file()]
        for model in sorted({str(r["Model"]) for r in rows}):
            group = [r for r in rows if str(r["Model"]) == model]
            row = min(group, key=lambda r: (str(r["sourceSHA256"]), str(r["captureSession"])))
            selected.append({
                "split": split, "model": model, "session": row["captureSession"],
                "sourcePath": str(Path(row["sourcePath"]).resolve()),
                "sourceSHA256": row["sourceSHA256"], "samplePath": row["samplePath"],
                "relativePath": row["relativePath"], "displayWidth": row["displayWidth"],
                "displayHeight": row["displayHeight"],
            })
    if len(selected) != 8 or len({r["session"] for r in selected}) != 8:
        raise SystemExit("native consumer freeze requires 8 distinct available sessions")
    created = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    manifest = {
        "schema": "xdremux-native-consumer-calibration-v1", "createdAt": created,
        "currentHEAD": a.head,
        "selectionRule": "fixed dataset split; per split/model choose lexicographically smallest sourceSHA256 among existing files; no image/metric inspection",
        "splitBoundary": {"calibration": "4 devices x 1 session; alpha/gain selection only", "heldout": "4 devices x 1 untouched session; final reveal only"},
        "candidate": {"pairedAlphaGrid": [0, .25, .5, .625, .75, 1], "residualGainGrid": [.5, .75, 1, 1.25, 1.5], "universalTuning": False, "modelHashes": sorted(a.model_hash), "checkpointHashes": sorted(a.checkpoint_hash)},
        "sampleCount": len(selected), "samples": selected,
    }
    payload = json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    a.output.mkdir(parents=True, exist_ok=True)
    (a.output / "manifest.json").write_text(payload)
    print(json.dumps({"manifest": str(a.output / "manifest.json"), "sha256": hashlib.sha256(payload.encode()).hexdigest(), "sampleCount": len(selected), "createdAt": created}, sort_keys=True))
if __name__ == "__main__": main()
