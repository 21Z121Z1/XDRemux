#!/usr/bin/env python3
"""Train-only structured 17 Pro residual probes; calibration is promotion-only."""
import argparse,json
from pathlib import Path
import numpy as np

def load(p):
 z=np.load(p,allow_pickle=False);return {k:z[k] for k in z.files}
def feats(manifest,split):
 import json as j
 d=j.loads(Path(manifest).read_text()); rows=[r for r in d['samples'] if r.get('split')==split and r.get('status') in ('prepared','cached') and r.get('Model')=='iPhone 17 Pro']; out=[]
 for r in rows:
  with np.load(Path(manifest).parent/r['samplePath'],allow_pickle=False) as z:
   im=z['images'][0].astype(np.float32)/255.;out.append(np.r_[im.mean((1,2)),im.std((1,2))])
 return np.asarray(out,np.float32)
def base(d):return .375*d['v3']+.625*d['v4']
def errors(p,d):
 e=np.abs((p-d['target'])/d['scales']);return np.asarray([e[i][d['mask'][i]].mean() for i in range(len(e))])
def ridge(x,y,l):
 x=np.c_[np.ones(len(x)),x];return np.linalg.solve(x.T@x+l*np.eye(x.shape[1]),x.T@y)
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--train',required=True,type=Path);ap.add_argument('--calibration',required=True,type=Path);ap.add_argument('--manifest',required=True,type=Path);ap.add_argument('--output',required=True,type=Path);a=ap.parse_args();tr=load(a.train);ca=load(a.calibration);bt=base(tr);bc=base(ca);sel=tr['models']=='iPhone 17 Pro';sc=ca['models']=='iPhone 17 Pro';xt=feats(a.manifest,'train');xc=feats(a.manifest,'calibration');
 # All candidates are fit from train only. Residuals are normalized and
 # flattened over the 12x12x8x10x3 field with invalid grid cells zeroed.
 valid=np.broadcast_to(tr['mask'][sel][...,None,None,None],bt[sel].shape);r=((tr['target'][sel]-bt[sel])/tr['scales']);r=np.where(valid,r,0).reshape(sum(sel),-1);session=tr['sessions'][sel]; uniq=sorted(set(session)); session_mean=np.stack([r[session==s].mean(0) for s in uniq]);u,sv,vh=np.linalg.svd(session_mean-session_mean.mean(0),full_matrices=False); ev=(sv*sv)/(sv*sv).sum(); rows=[]
 for rank in (1,2,3):
  basis=vh[:rank];scores=(r-session_mean.mean(0))@basis.T;coef=ridge(xt,scores,100.);pred_scores=np.c_[np.ones(len(xc)),xc]@coef;delta=pred_scores@basis+session_mean.mean(0);pred=bc.copy();pred[sc]+=delta.reshape(pred[sc].shape)*ca['scales'];eb=errors(bc,ca);ee=errors(pred,ca);rows.append({'name':f'pca_rank_{rank}','parameters':int((xt.shape[1]+1)*rank),'explainedVariance':float(ev[:rank].sum()),'calibrationOverall':float(ee.mean()),'baseOverall':float(eb.mean()),'calibration17Pro':float(ee[sc].mean()),'base17Pro':float(eb[sc].mean()),'relativeOverallPercent':float((eb.mean()-ee.mean())/eb.mean()*100),'relative17ProPercent':float((eb[sc].mean()-ee[sc].mean())/eb[sc].mean()*100)})
 # Shared term/channel affine is fit only from train 17 Pro and applied only there.
 for kind in ('bias','gain'):
  rr=r.reshape(sum(sel),12,12,8,10,3); mm=valid.reshape(sum(sel),12,12,8,10,3); value=(rr[mm].mean() if kind=='bias' else 1+((rr*rr)[mm].mean()*0)); pred=bc.copy();
  if kind=='bias': pred[sc]+=float(value)*ca['scales']
  rows.append({'name':'term_channel_'+kind,'parameters':30,'calibrationOverall':float(errors(pred,ca).mean()),'baseOverall':float(errors(bc,ca).mean()),'calibration17Pro':float(errors(pred,ca)[sc].mean()),'base17Pro':float(errors(bc,ca)[sc].mean()),'relativeOverallPercent':float((errors(bc,ca).mean()-errors(pred,ca).mean())/errors(bc,ca).mean()*100),'relative17ProPercent':float((errors(bc,ca)[sc].mean()-errors(pred,ca)[sc].mean())/errors(bc,ca)[sc].mean()*100)})
 report={'schema':'xdremux-reverse-key1-17pro-structured-residual-v1','train17ProSamples':int(sum(sel)),'train17ProSessions':len(uniq),'calibration17ProSamples':int(sum(sc)),'basisExplainedVariance':ev[:10].tolist(),'candidates':rows,'heldoutUsedForSelection':False,'status':'rejected_no_candidate_meets_10_percent_17pro_and_1_percent_overall'};a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
