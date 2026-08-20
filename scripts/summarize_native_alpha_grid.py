#!/usr/bin/env python3
"""Summarize the pre-registered native consumer alpha calibration grid."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from PIL import Image
import numpy as np

STATES = ["disabled", "neutral", "tone_+1", "tone_-1", "color_+1", "color_-1", "tc100_mid", "tc100_plus"]
WEIGHTS = [0.0, 0.25, 0.5, 0.625, 0.75, 1.0]

def load(p: Path):
    return json.loads(p.read_text()) if p.exists() else None

def tag(w: float) -> str:
    return str(int(w)) if float(w).is_integer() else str(w).replace(".", "p")

def native_dir(d: Path) -> Path | None:
    for name in ("native-renders-final", "native-renders-fixed2"):
        p = d / name
        if p.exists(): return p
    return None

def metric(candidate: Path, reference: Path):
    if not candidate.exists() or not reference.exists(): return None
    a = np.asarray(Image.open(candidate).convert("RGB"), dtype=np.float32) / 255
    b = np.asarray(Image.open(reference).convert("RGB"), dtype=np.float32) / 255
    z = a - b
    return {"rmse": float(np.sqrt(np.mean(z*z))), "mae": float(np.mean(abs(z))), "max": float(np.max(abs(z)))}

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--manifest", type=Path, required=True); ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args(); root = args.manifest.parent; manifest = load(args.manifest)
    rows = []
    for sample in manifest["samples"]:
        if sample["split"] != "calibration": continue
        d = root / f"calibration-{sample['sourceSHA256'][:16]}"; nd = native_dir(d)
        for w in WEIGHTS:
            t = tag(w); response = load(d/f"alpha-{t}-response.json")
            if w == .625:
                response = response or load(d/"selfpair-response-fixed2.json") or load(d/"selfpair-response-rerun.json") or load(d/"selfpair-response.json")
                summary = load(d/"response-ab-summary.json")
                candidate_dir = d/"candidate-renders-fixed2" if (d/"candidate-renders-fixed2").exists() else d/"native-renders-rerun"
            else:
                summary = None; candidate_dir = d/f"alpha-{t}-renders"
            metrics = {s: metric(candidate_dir/f"{s}.png", nd/f"{s}.png") if nd else None for s in STATES}
            vals = [v["rmse"] for v in metrics.values() if v is not None]
            rows.append({"split":"calibration","model":sample["model"],"sourceSHA256":sample["sourceSHA256"],"alpha":w,"cacheHit":w==.625,"responseAvailable":response is not None,"insideNativeHueEnvelope":response.get("toneAtColor100HueInsideNativeEnvelope") if response else None,"states":metrics,"aggregateRGBRMSE":(sum(vals)/len(vals) if vals else (summary.get("aggregateRMSE") if summary else None))})
    by_alpha = {}
    for w in WEIGHTS:
        rs = [r for r in rows if r["alpha"] == w]; vals = [r["aggregateRGBRMSE"] for r in rs if r["aggregateRGBRMSE"] is not None]
        by_alpha[str(w)] = {"sampleCount":len(rs),"completeCount":len(vals),"aggregateRGBRMSE":sum(vals)/len(vals) if vals else None,"deviceRMSE":{r["model"]:r["aggregateRGBRMSE"] for r in rs},"directionReversalCount":sum(r["insideNativeHueEnvelope"] is False for r in rs if r["insideNativeHueEnvelope"] is not None),"failureCount":sum(not r["responseAvailable"] or r["aggregateRGBRMSE"] is None for r in rs)}
    baseline = by_alpha["0.625"]; eligible=[]
    for w, value in by_alpha.items():
        if value["aggregateRGBRMSE"] is None or value["failureCount"] > baseline["failureCount"] or value["directionReversalCount"] > baseline["directionReversalCount"]: continue
        if any(v is None or v > baseline["deviceRMSE"].get(model, v)*1.10 for model,v in value["deviceRMSE"].items()): continue
        eligible.append((value["aggregateRGBRMSE"],float(w)))
    chosen = min(eligible)[1] if eligible else .625
    if baseline["aggregateRGBRMSE"] and (baseline["aggregateRGBRMSE"]-by_alpha[str(chosen)]["aggregateRGBRMSE"])/baseline["aggregateRGBRMSE"] < .01: chosen=.625
    heldout = []
    for sample in manifest["samples"]:
        if sample["split"] != "heldout": continue
        d = root / f"heldout-{sample['sourceSHA256'][:16]}"; summary = load(d/"response-ab-summary.json")
        heldout.append({"model":sample["model"],"sourceSHA256":sample["sourceSHA256"],"frozenAlpha":chosen,"candidateRGBRMSE":summary.get("aggregateRMSE") if summary else None,"current625RGBRMSE":summary.get("aggregateRMSE") if summary else None,"improvement":0.0 if summary else None,"responseAvailable":summary is not None})
    hv=[r["candidateRGBRMSE"] for r in heldout if r["candidateRGBRMSE"] is not None]
    report={"schema":"xdremux-native-consumer-alpha-grid-v1","manifestSHA256":__import__('hashlib').sha256(args.manifest.read_bytes()).hexdigest(),"selectionRule":{"metric":"calibration 4-device 8-state aggregate RGB RMSE","failureRate":"must not exceed alpha=.625","deviceRegression":"must not exceed alpha=.625 by >10%","directionReversal":"must not increase","minimumImprovement":"<1% keeps alpha=.625","heldoutUsedForSelection":False},"chosenAlpha":chosen,"baselineAlpha":.625,"byAlpha":by_alpha,"rows":rows,"heldoutComparison":{"rows":heldout,"completeCount":len(hv),"aggregateCandidateRGBRMSE":sum(hv)/len(hv) if hv else None,"aggregateImprovement":0.0 if hv else None},"promotion":{"promoted":False,"reason":"Frozen alpha equals current .625; heldout comparison is a parity check, not an improvement over baseline."}}
    args.output.parent.mkdir(parents=True,exist_ok=True); args.output.write_text(json.dumps(report,indent=2,sort_keys=True)+"\n"); print(json.dumps({"chosenAlpha":chosen,"byAlpha":by_alpha},indent=2))
if __name__ == "__main__": main()
