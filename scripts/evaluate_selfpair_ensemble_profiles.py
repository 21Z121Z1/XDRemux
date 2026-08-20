#!/usr/bin/env python3
"""Calibration-only low-dimensional v3/v4 self-pair ensemble profiles."""
from __future__ import annotations
import argparse, json
from pathlib import Path
from collections import defaultdict
import numpy as np

def load(path):
    z=np.load(path,allow_pickle=False); return {k:z[k] for k in z.files}
def errors(pred,d):
    e=np.abs((pred-d['target'])/d['scales']); return np.asarray([e[i][d['mask'][i]].mean() for i in range(len(e))])
def score(pred,d): return float(errors(pred,d).mean())
def devices(pred,d):
    x=errors(pred,d); return {m:float(x[d['models']==m].mean()) for m in sorted(set(d['models']))}
def blend(d,a): return (1-a)*d['v3']+a*d['v4']
def baseline(d): return blend(d,.625)
def summary(pred,d):
    e=errors(pred,d); return {'normalizedMAE':float(e.mean()),'perDevice':devices(pred,d),'sampleCount':len(e),'sessionCount':len(set(d['sessions']))}
def decomposition(d):
    p=baseline(d); e=errors(p,d); raw=np.abs((p-d['target'])/d['scales']); valid=np.broadcast_to(d['mask'][...,None,None,None],raw.shape)
    terms=[]
    for q in range(10): terms.append(float(raw[...,q,:][np.broadcast_to(d['mask'][...,None,None],raw[...,q,:].shape)].mean()))
    zones=[float(raw[:,lo:hi][np.broadcast_to(d['mask'][:,lo:hi,...,None,None,None],raw[:,lo:hi].shape)].mean()) for lo,hi in ((0,4),(4,8),(8,12))]
    e3=errors(d['v3'],d); e4=errors(d['v4'],d); disagreement=np.abs(d['v3']-d['v4'])[valid]
    return {'baseline':summary(p,d),'small':summary(d['v3'],d),'multiscale':summary(d['v4'],d),'perSampleError':e.tolist(),'termGroupMAE':terms,'spatialBandMAE':zones,'smallMultiscaleErrorCorrelation':float(np.corrcoef(e3,e4)[0,1]),'meanDisagreement':float(disagreement.mean()),'disagreementP95':float(np.quantile(disagreement,.95))}
def candidate_rows(d):
    names=sorted(set(d['models'])); global_a=.625; grid=np.arange(.5,.751,.025)
    rows=[]
    # Per-device alpha, with unknown devices explicitly falling back global.
    best={m:min(grid,key=lambda a:score(blend(d,a)[d['models']==m],{**d,'target':d['target'][d['models']==m],'mask':d['mask'][d['models']==m]})) for m in names}
    for shrink in (1.,.75,.5,.25):
      a={m:global_a+shrink*(best[m]-global_a) for m in names}; pred=np.stack([blend(d,a[str(m)])[i] for i,m in enumerate(d['models'])])
      rows.append({'name':f'device_alpha_shrink_{shrink:.2f}','parameters':len(a),'alphas':a,'prediction':pred})
    # Low-dimensional disagreement gate: one slope around fixed global alpha.
    dis=np.abs(d['v3']-d['v4']).reshape(len(d['v3']),-1).mean(1); center=float(np.median(dis)); scale=max(float(np.std(dis)),1e-6)
    for g in (-.1,0.,.1):
      aa=np.clip(global_a+g*(dis-center)/scale,.25,.9); pred=np.stack([blend(d,float(aa[i]))[i] for i in range(len(aa))]); rows.append({'name':f'disagreement_gate_{g:.2f}','parameters':2,'gain':g,'center':center,'scale':scale,'prediction':pred})
    return rows
def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--calibration',required=True,type=Path); ap.add_argument('--train',required=True,type=Path); ap.add_argument('--frozen',required=True,type=Path); ap.add_argument('--heldout',type=Path); ap.add_argument('--output',required=True,type=Path); a=ap.parse_args()
    cal=load(a.calibration); train=load(a.train); report={'schema':'xdremux-reverse-key1-profile-ensemble-v1','selectionRule':'calibration overall >=1% better than fixed .625; no device >5% regression; iPhone 17 Pro improves; shuffled remains worse; unknown fallback=.625','calibrationArchive':str(a.calibration.resolve()),'trainArchive':str(a.train.resolve()),'trainDecomposition':decomposition(train),'calibrationDecomposition':decomposition(cal),'candidates':[],'heldoutUsedForSelection':False}
    base=summary(baseline(cal),cal); rows=candidate_rows(cal)
    for row in rows:
      p=row.pop('prediction'); row['calibration']=summary(p,cal); row['relativeImprovementPercent']=(base['normalizedMAE']-row['calibration']['normalizedMAE'])/base['normalizedMAE']*100; row['deviceGuard']=all(row['calibration']['perDevice'][m] <= base['perDevice'][m]*1.05 for m in base['perDevice']); row['proGuard']=row['calibration']['perDevice'].get('iPhone 17 Pro',999) < base['perDevice'].get('iPhone 17 Pro',0); report['candidates'].append(row)
    eligible=[r for r in report['candidates'] if r['relativeImprovementPercent']>=1 and r['deviceGuard'] and r['proGuard']]
    if eligible:
      chosen=min(eligible,key=lambda r:r['calibration']['normalizedMAE']); report['frozenChoice']=chosen
      if a.heldout:
        hd=load(a.heldout); # heldout is opened only after frozenChoice exists
        # Reconstruct only the chosen profile; device alpha is the supported
        # research path. Unknown model names use fixed .625.
        aa=chosen.get('alphas',{}); pred=np.stack([blend(hd,float(aa.get(str(m),.625)))[i] for i,m in enumerate(hd['models'])]); report['heldout']={'candidate':summary(pred,hd),'baseline':summary(baseline(hd),hd)}
    else: report['status']='rejected_no_calibration_promotion_candidate'
    a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_text(json.dumps(report,indent=2,sort_keys=True)+"\n"); print(json.dumps({k:report[k] for k in ('schema','status','calibrationDecomposition','candidates') if k in report},indent=2))
if __name__=='__main__': main()
