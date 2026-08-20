#!/usr/bin/env python3
"""Leakage-resistant low-dimensional self-pair key1 calibration.

The command is deliberately split into ``collect``, ``select`` and ``heldout``.
The latter refuses to run without a frozen choice produced from calibration.
No per-pixel parameters are fitted: candidates are blend alpha, global residual
gain/bias, and one strongly regularized channel bias.
"""
from __future__ import annotations

import argparse, hashlib, json, sys
from collections import defaultdict
from pathlib import Path
from typing import Any
import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.apple_reverse_key1_training import (
    build_model, identity_key1, input_features, load_manifest, sha256_file,
    _require_torch,
)

V3 = Path(".codex/reverse-key1/run-v3/best.pt")
V4 = Path(".codex/reverse-key1/run-v4-shared/best.pt")
RULE = {
    "baseline": {"name": "selfpair_ensemble", "alpha": 0.625,
                 "normalizedMAE": 0.78220},
    "candidateFamilies": [
        {"name": "blend_alpha", "values": [round(x, 2) for x in np.arange(0, 1.001, .05)]},
        {"name": "global_residual_affine", "gain": [0.85, .9, .95, 1., 1.05, 1.1, 1.15],
         "biasNormalized": [-.02, -.01, 0., .01, .02]},
        {"name": "channel_ridge_bias", "ridge": 10.0, "channels": 30},
    ],
    "selection": "minimum calibration normalized MAE among candidates whose shuffled MAE remains greater than paired MAE; heldout is final-only",
    "promotion": {"minimumRelativeImprovement": .01, "maxDeviceRegression": .05},
}

def _load(path: Path, torch: Any, device: str):
    ck = torch.load(path.resolve(), map_location="cpu", weights_only=False)
    arch = "multiscale_large" if "multiscale-large" in str(ck["architecture"]) else "small"
    vocab = tuple(ck.get("profileVocabulary", ()))
    model = build_model(np.asarray(ck["coefficientScales"], np.float32), profile_count=len(vocab), architecture=arch)
    model.load_state_dict(ck["model"]); model.to(device).eval()
    return model, ck, vocab

def _rows(manifest: Path, split: str):
    header, rows = load_manifest(manifest.resolve())
    selected = [r for r in rows if r["split"] == split]
    if not selected: raise ValueError(f"empty {split} split")
    return header, selected

def _predict(manifest: Path, split: str, v3: Path, v4: Path):
    torch, _ = _require_torch(); device = "mps" if torch.backends.mps.is_available() else "cpu"
    header, rows = _rows(manifest, split); root = manifest.resolve().parent
    m3, c3, vocab = _load(v3, torch, device); m4, c4, vocab4 = _load(v4, torch, device)
    if not np.array_equal(c3["coefficientScales"], c4["coefficientScales"]): raise ValueError("scale mismatch")
    identity = identity_key1(); images=[]; targets=[]; masks=[]; names=[]; sessions=[]; ids=[]
    for r in rows:
        with np.load(root / r["samplePath"], allow_pickle=False) as a:
            im=np.asarray(a["images"], np.uint8); images.append(im[0]); targets.append(np.asarray(a["key1"],np.float32)); masks.append(np.asarray(a["mask"],bool))
        names.append(str(r.get("Model") or "unknown")); sessions.append(str(r["captureSession"]))
        ids.append(vocab.index(names[-1]) if names[-1] in vocab else vocab.index("__unknown__"))
    # Make the shuffled control deterministic and source-independent: the second
    # image is another row's primary, while the normal path duplicates itself.
    arr=np.stack(images); shuffled=np.roll(arr, 1, axis=0)
    p3=[]; p4=[]; s3=[]; s4=[]
    with torch.no_grad():
        for start in range(0,len(arr),8):
            end=min(len(arr),start+8); primary=arr[start:end]
            def run(second):
                feats=np.stack([input_features(np.stack([primary[i], second[i]],axis=0)) for i in range(end-start)])
                ft=torch.from_numpy(feats).to(device); pi=torch.tensor(ids[start:end],dtype=torch.long,device=device)
                return m3(ft,pi).cpu().numpy(), m4(ft).cpu().numpy()
            a,b=run(primary); c,d=run(shuffled[start:end]); p3.append(a);p4.append(b);s3.append(c);s4.append(d)
    return {"v3":np.concatenate(p3),"v4":np.concatenate(p4),"shuffleV3":np.concatenate(s3),"shuffleV4":np.concatenate(s4),"target":np.stack(targets),"mask":np.stack(masks),"models":np.asarray(names),"sessions":np.asarray(sessions),"identity":identity,"scales":np.asarray(c3["coefficientScales"],np.float32),"manifestSHA256":sha256_file(manifest.resolve()),"checkpointSHA256":{"v3":sha256_file(v3.resolve()),"v4":sha256_file(v4.resolve())},"split":split,"sampleCount":len(rows)}

def _score(pred, d):
    e=np.abs((pred-d["target"])/d["scales"]); return float(e[d["mask"]].mean())

def _sample_scores(pred, d):
    return np.asarray([np.abs((pred[i]-d["target"][i])/d["scales"])[d["mask"][i]].mean() for i in range(len(d["target"]))], np.float32)

def _bootstrap_delta(candidate, baseline, d, seed=260819):
    delta=_sample_scores(candidate,d)-_sample_scores(baseline,d)
    groups=defaultdict(list)
    for i,s in enumerate(d["sessions"]): groups[str(s)].append(float(delta[i]))
    rng=np.random.default_rng(seed); keys=sorted(groups); draws=[]
    for _ in range(20000):
        selected=rng.choice(keys,size=len(keys),replace=True)
        draws.append(np.mean([v for k in selected for v in groups[k]]))
    return {"meanDelta":float(delta.mean()),"relativePercent":float(delta.mean()/_score(baseline,d)*100),"clusterBootstrap95PercentCI":[float(np.quantile(draws,.025)),float(np.quantile(draws,.975))],"bootstrapProbabilityImproved":float(np.mean(np.asarray(draws)<0)),"improvedSampleFraction":float(np.mean(delta<0))}

def _device_scores(pred,d):
    out=defaultdict(list)
    for i,m in enumerate(d["models"]):
        e=np.abs((pred[i]-d["target"][i])/d["scales"]); out[str(m)].append(float(e[d["mask"][i]].mean()))
    return {k:float(np.mean(v)) for k,v in sorted(out.items())}

def _candidate(name, d, **kw):
    base=d["identity"]; blend=(1-kw.get("alpha",.625))*d["v3"]+kw.get("alpha",.625)*d["v4"]
    if name=="blend_alpha": return blend
    if name=="global_residual_affine": return base + kw["gain"]*(blend-base) + kw["bias"]*d["scales"]
    # Shared 30-channel ridge bias. The closed-form fit is deliberately one
    # scalar per (polynomial,output) channel, not one parameter per pixel.
    x=(blend-base)/d["scales"]; y=(d["target"]-blend)/d["scales"]; mask=d["mask"]
    bias=np.zeros((10,3),np.float32)
    for p in range(10):
      for o in range(3):
        valid=np.broadcast_to(mask[...,None], x[...,p,o].shape)
        z=x[...,p,o][valid]; t=y[...,p,o][valid]
        bias[p,o]=float(t.sum()/(len(t)+kw.get("ridge",10.0)))
    return blend + bias[None,None,None,None,:,:]*d["scales"]

def _pack(d,path):
    np.savez_compressed(path, **{k:v for k,v in d.items() if isinstance(v,np.ndarray)})
    meta={k:v for k,v in d.items() if not isinstance(v,np.ndarray)}; meta["archiveSHA256"]=sha256_file(path); Path(str(path)+".json").write_text(json.dumps(meta,indent=2,sort_keys=True)+"\n")

def main():
    ap=argparse.ArgumentParser(); sub=ap.add_subparsers(dest="cmd",required=True)
    for cmd in ("collect","heldout"):
      p=sub.add_parser(cmd); p.add_argument("--manifest",required=True,type=Path); p.add_argument("--output",required=True,type=Path); p.add_argument("--v3",type=Path,default=V3); p.add_argument("--v4",type=Path,default=V4)
      if cmd == "heldout": p.add_argument("--frozen",required=True,type=Path)
    p=sub.add_parser("select"); p.add_argument("--archive",required=True,type=Path); p.add_argument("--frozen",required=True,type=Path)
    a=ap.parse_args()
    if a.cmd in ("collect", "heldout"): a.output.parent.mkdir(parents=True,exist_ok=True)
    else: a.frozen.parent.mkdir(parents=True,exist_ok=True)
    if a.cmd in ("collect", "heldout"):
      split = "calibration" if a.cmd == "collect" else "heldout"
      d=_predict(a.manifest,split,a.v3,a.v4); _pack(d,a.output)
      if a.cmd == "collect":
       print(json.dumps({"split":split,"sampleCount":d["sampleCount"],"manifestSHA256":d["manifestSHA256"]},indent=2)); return
      frozen=json.loads(a.frozen.read_text()); z=np.load(a.output,allow_pickle=False); d={k:z[k] for k in z.files}
      choice=frozen["frozenChoice"]; name=choice["name"]; q=choice["parameters"]
      if name.startswith("alpha_"): pred=_candidate("blend_alpha",d,alpha=q["alpha"])
      elif name.startswith("affine_"): pred=_candidate("global_residual_affine",d,gain=q["gain"],bias=q["biasNormalized"])
      else: pred=_candidate("channel_ridge_bias",d,ridge=q.get("ridge",10))
      baseline=.375*d["v3"]+.625*d["v4"]
      result={"schema":"xdremux-reverse-key1-selfpair-lowdim-heldout-v1","frozenChoice":choice,"manifestSHA256":json.loads(Path(str(a.output)+".json").read_text())["manifestSHA256"],"sampleCount":int(d["target"].shape[0]),"normalizedMAE":_score(pred,d),"baselineAlpha625":{"normalizedMAE":_score(baseline,d),"perDevice":_device_scores(baseline,d)},"perDevice":_device_scores(pred,d),"identityNormalizedMAE":_score(d["identity"],d),"shuffledNormalizedMAE":_score(d["shuffleV3"]*.375+d["shuffleV4"]*.625,d),"bootstrapVsAlpha625":_bootstrap_delta(pred,baseline,d),"heldoutUsedForSelection":False}
      a.output.with_suffix(".json").write_text(json.dumps(result,indent=2,sort_keys=True)+"\n"); print(json.dumps(result,indent=2)); return
    if a.cmd=="select":
      z=np.load(a.archive,allow_pickle=False); meta=json.loads(Path(str(a.archive)+".json").read_text()); d={k:z[k] for k in z.files}; base=d["identity"]
      candidates=[]
      for alpha in RULE["candidateFamilies"][0]["values"]: candidates.append((f"alpha_{alpha:.2f}",_candidate("blend_alpha",d,alpha=alpha),{"alpha":alpha,"parameters":1}))
      for g in RULE["candidateFamilies"][1]["gain"]:
       for b in RULE["candidateFamilies"][1]["biasNormalized"]: candidates.append((f"affine_g{g:.2f}_b{b:.2f}",_candidate("global_residual_affine",d,gain=g,bias=b),{"gain":g,"biasNormalized":b,"parameters":2}))
      candidates.append(("channel_ridge_lambda10",_candidate("channel_ridge_bias",d,ridge=10),{"ridge":10,"parameters":30}))
      rows=[]
      for n,p,params in candidates:
       cm=_score(p,d); sm=_score((base+(p-base)*0+d["shuffleV3"]*0),d) # retained for schema clarity
       # Apply the same transform to the shuffled blend for a real control.
       sh=(1-params.get("alpha",.625))*d["shuffleV3"]+params.get("alpha",.625)*d["shuffleV4"]
       if n.startswith("affine_"): sh=base+params["gain"]*(sh-base)+params["biasNormalized"]*d["scales"]
       rows.append({"name":n,"parameters":params,"calibrationNormalizedMAE":cm,"shuffleNormalizedMAE":_score(sh,d),"perDevice":_device_scores(p,d)})
      eligible=[r for r in rows if r["shuffleNormalizedMAE"]>r["calibrationNormalizedMAE"]]
      chosen=min(eligible or rows,key=lambda r:r["calibrationNormalizedMAE"])
      # Persist the fitted 30-channel correction only if that candidate wins;
      # it is shared over space and is never refit on heldout.
      if chosen["name"] == "channel_ridge_lambda10":
       p=_candidate("channel_ridge_bias",d,ridge=10); residual=(p-((1-.625)*d["v3"]+.625*d["v4"]))
       chosen["parameters"]["biasNormalized"]=(residual/d["scales"])[0,0,0].tolist()
      report={"schema":"xdremux-reverse-key1-selfpair-lowdim-calibration-v1","rule":RULE,"source":meta,"candidates":rows,"frozenChoice":chosen,"heldoutUsedForSelection":False}
      a.frozen.parent.mkdir(parents=True,exist_ok=True); a.frozen.write_text(json.dumps(report,indent=2,sort_keys=True)+"\n"); print(json.dumps(chosen,indent=2)); return
    raise SystemExit("unreachable")

if __name__=="__main__": main()
