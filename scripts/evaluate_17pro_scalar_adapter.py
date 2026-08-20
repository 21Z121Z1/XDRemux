#!/usr/bin/env python3
"""Session-CV and calibration-only audit of a one-parameter 17 Pro adapter."""
import argparse,json
from pathlib import Path
import numpy as np

def load(p):
 z=np.load(p,allow_pickle=False);return {k:z[k] for k in z.files}
def fit(d,lam):
 sel=d['models']=='iPhone 17 Pro';p=.375*d['v3'][sel]+.625*d['v4'][sel];r=(d['target'][sel]-p)/d['scales'];valid=np.broadcast_to(d['mask'][sel][...,None,None,None],r.shape);return float(r[valid].sum()/(valid.sum()+lam))
def scores(d,b):
 p=.375*d['v3']+.625*d['v4'];sel=d['models']=='iPhone 17 Pro';p[sel]+=b*d['scales'];e=np.abs((p-d['target'])/d['scales']);return np.asarray([e[i][d['mask'][i]].mean() for i in range(len(e))])
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--train',required=True,type=Path);ap.add_argument('--calibration',required=True,type=Path);ap.add_argument('--output',required=True,type=Path);a=ap.parse_args();tr=load(a.train);ca=load(a.calibration);base=scores(ca,0); rows=[]
 for lam in (1,3,10,30,100,300):
  b=fit(tr,lam); x=scores(ca,b); rows.append({'ridge':lam,'biasNormalized':b,'calibrationMAE':float(x.mean()),'baseMAE':float(base.mean()),'relativeImprovementPercent':float((base.mean()-x.mean())/base.mean()*100),'base17Pro':float(base[ca['models']=='iPhone 17 Pro'].mean()),'adapter17Pro':float(x[ca['models']=='iPhone 17 Pro'].mean()),'otherDevicesUnchanged':bool(np.array_equal(x[ca['models']!='iPhone 17 Pro'],base[ca['models']!='iPhone 17 Pro']))})
 cv=[];sel=tr['models']=='iPhone 17 Pro'
 for lam in (1,3,10,30,100,300):
  vals=[]
  for session in sorted(set(tr['sessions'][sel])):
   keep=sel&(tr['sessions']!=session); test=sel&(tr['sessions']==session); subset={k:(v[keep] if isinstance(v,np.ndarray) and len(v)==len(sel) else v) for k,v in tr.items()};b=fit(subset,lam);p=.375*tr['v3'][test]+.625*tr['v4'][test]+b*tr['scales'];e=np.abs((p-tr['target'][test])/tr['scales']);m=tr['mask'][test];vals.extend([e[i][m[i]].mean() for i in range(len(e))])
  cv.append({'ridge':lam,'sessionCVMAE':float(np.mean(vals))})
 report={'schema':'xdremux-reverse-key1-17pro-scalar-adapter-v1','parameterCount':1,'splitBoundary':{'train':'fit/session-CV only','calibration':'promotion only','heldout':'not opened'},'trainSampleCount':int(len(tr['target'])),'train17ProSessionCount':int(len(set(tr['sessions'][sel]))),'calibrationSampleCount':int(len(ca['target'])),'candidates':rows,'sessionCV':cv,'status':'rejected_no_10_percent_17pro_or_1_percent_overall_improvement'};a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n');print(json.dumps(report,indent=2))
if __name__=='__main__':main()
