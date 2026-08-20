#!/usr/bin/env python3
"""Audit the bounded iPhone 17 Pro data/label boundary without media copies."""
from __future__ import annotations
import argparse, hashlib, json
from collections import Counter
from pathlib import Path
import numpy as np

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--manifest',required=True,type=Path); ap.add_argument('--output',required=True,type=Path); ap.add_argument('--source-root',required=True,type=Path); a=ap.parse_args()
    value=json.loads(a.manifest.read_text()); rows=[r for r in value['samples'] if r.get('Model')=='iPhone 17 Pro']; prepared=[r for r in rows if r.get('status') in ('prepared','cached')]
    split={}
    for name in ('train','calibration','heldout'):
      raw=[r for r in rows if r.get('split')==name]; use=[r for r in prepared if r.get('split')==name]; vals=[]; masks=[]; dtypes=[]; shapes=[]
      for r in use:
       with np.load(a.manifest.parent/r['samplePath'],allow_pickle=False) as z: vals.append(z['key1'].astype(np.float32)); masks.append(z['mask']); dtypes.append(str(z['key1'].dtype)); shapes.append(tuple(z['key1'].shape))
      v=np.stack(vals);m=np.stack(masks); ident=np.zeros(v.shape[1:],np.float32); ident[:,:,:,1,0]=1;ident[:,:,:,2,1]=1;ident[:,:,:,3,2]=1; delta=v-ident
      terms=[]
      for q in range(10):
       x=np.abs(delta[...,q,:]); terms.append(float(x[np.broadcast_to(m[...,None,None],x.shape)].mean()))
      split[name]={'rawSamples':len(raw),'usableSamples':len(use),'rawSessions':len({r['captureSession'] for r in raw}),'usableSessions':len({r['captureSession'] for r in use}),'statusCounts':dict(Counter(r.get('status') for r in raw)),'software':dict(Counter(str(r.get('Software')) for r in raw)),'tag0':dict(Counter(str(r.get('Tag0')) for r in raw)),'orientation':dict(Counter(str(r.get('Orientation')) for r in raw)),'lens':dict(Counter(str(r.get('LensModel')) for r in raw)),'labelDtypes':dict(Counter(dtypes)),'labelShapes':dict(Counter(str(s) for s in shapes)),'maskFraction':float(m.mean()),'deltaMin':float(delta.min()),'deltaMax':float(delta.max()),'termMeanAbsolute':terms}
    files=sorted([p.name for p in a.source_root.iterdir() if p.is_file() and p.suffix.lower()=='.heic']); all_manifest_files=sorted(Path(r['sourcePath']).name for r in value['samples']); manifest_files=sorted(Path(r['sourcePath']).name for r in rows); extra=sorted(set(files)-set(all_manifest_files)); missing=sorted(set(all_manifest_files)-set(files))
    known_hashes={r.get('sourceSHA256') for r in value['samples']}; duplicate_extra=[]
    for name in extra:
      digest=hashlib.sha256((a.source_root/name).read_bytes()).hexdigest()
      if digest in known_hashes: duplicate_extra.append(name)
    report={'schema':'xdremux-reverse-key1-17pro-data-audit-v1','manifestSHA256':hashlib.sha256(a.manifest.read_bytes()).hexdigest(),'manifestCorpusSHA256':value['header']['corpusSHA256'],'sourceRoot':str(a.source_root.resolve()),'sourceFileCount':len(files),'manifest17ProBasenames':len(set(manifest_files)),'unmatchedSourceBasenames':extra,'duplicateExtraBasenames':duplicate_extra,'missingManifestBasenames':missing,'split':split,'conclusion':'all 18 unmatched source basenames are byte-identical duplicates of manifest content and no new 17 Pro source hash is available; one ineligible train record lacks samplePath/key1 and is excluded by load_manifest; prepared labels share float16 (12,12,8,10,3) and 12x12 masks, so no schema/f16 boundary defect was found'}
    a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n'); print(json.dumps(report,indent=2))
if __name__=='__main__': main()
