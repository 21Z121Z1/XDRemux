#!/usr/bin/env python3
"""Short, calibration-selected mixed true/self-pair fine-tune experiment."""
from __future__ import annotations
import argparse, json, sys
from pathlib import Path
import numpy as np
if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from xdremux_py.apple_reverse_key1_training import _CachedDataset, _require_torch, build_model, identity_key1, load_manifest, sha256_file

def load_model(torch, checkpoint, device):
    c=torch.load(checkpoint,map_location="cpu",weights_only=False)
    m=build_model(np.asarray(c["coefficientScales"],np.float32), architecture="multiscale_large")
    m.load_state_dict(c["model"]); m.to(device)
    return m,c

def loss_for(torch, pred, target, mask, scales):
    n=((pred-target)/scales); selected=n[mask[:,:,:,None,None,None].expand_as(n)]
    return torch.nn.functional.smooth_l1_loss(selected,torch.zeros_like(selected),beta=1.)

def evaluate(torch, model, loader, device, scales, mode):
    model.eval(); vals=[]; models=[]
    with torch.no_grad():
      for features,target,mask,names,_sessions,_ids in loader:
        features,target,mask=features.to(device),target.to(device),mask.to(device)
        if mode=="selfpair": features=torch.cat((features[:,3:6],features[:,3:6],torch.zeros_like(features[:,6:12])),1)
        pred=model(features); e=((pred-target)/scales).abs()
        for i,n in enumerate(names): vals.append(float(e[i][mask[i,:,:,None,None,None].expand_as(e[i])].mean())); models.append(str(n))
    return {"normalizedMAE":float(np.mean(vals)),"perDevice":{n:float(np.mean([v for v,m in zip(vals,models) if m==n])) for n in sorted(set(models))}}

def main():
  ap=argparse.ArgumentParser(); ap.add_argument("--manifest",required=True,type=Path); ap.add_argument("--checkpoint",required=True,type=Path); ap.add_argument("--output",required=True,type=Path); ap.add_argument("--selfpair-probability",type=float,required=True); ap.add_argument("--consistency-weight",type=float,required=True); ap.add_argument("--epochs",type=int,default=1); ap.add_argument("--learning-rate",type=float,default=1e-6); ap.add_argument("--seed",type=int,default=260819); args=ap.parse_args()
  torch,_=_require_torch(); torch.manual_seed(args.seed); np.random.seed(args.seed); device="mps" if torch.backends.mps.is_available() else "cpu"
  header,samples=load_manifest(args.manifest.resolve()); root=args.manifest.resolve().parent; scales=torch.from_numpy(np.asarray(torch.load(args.checkpoint,map_location="cpu",weights_only=False)["coefficientScales"],np.float32)).to(device)
  _,ck=load_model(torch,args.checkpoint,device); model,_=load_model(torch,args.checkpoint,device)
  for n,p in model.named_parameters(): p.requires_grad_(n.startswith("head."))
  train=[x for x in samples if x["split"]=="train"]; cal=[x for x in samples if x["split"]=="calibration"]
  ds=_CachedDataset(root,train,(),single_image_self_pair=False); loader=torch.utils.data.DataLoader(ds,batch_size=8,shuffle=True)
  cds=_CachedDataset(root,cal,(),single_image_self_pair=False); cloader=torch.utils.data.DataLoader(cds,batch_size=8,shuffle=False)
  opt=torch.optim.AdamW([p for p in model.parameters() if p.requires_grad],lr=args.learning_rate,weight_decay=1e-4); history=[]
  for epoch in range(args.epochs):
    model.train(); losses=[]
    for features,target,mask,_names,_sessions,_ids in loader:
      features,target,mask=features.to(device),target.to(device),mask.to(device)
      true=model(features); self_features=torch.cat((features[:,3:6],features[:,3:6],torch.zeros_like(features[:,6:12])),1); self_pred=model(self_features)
      choose=(torch.rand((len(features),),device=device)<args.selfpair_probability).view(-1,1,1,1,1,1)
      selected=torch.where(choose,self_pred,true); primary=loss_for(torch,selected,target,mask,scales)
      consistency=torch.nn.functional.smooth_l1_loss(((true-self_pred)/scales)[mask[:,:,:,None,None,None].expand_as(true)],torch.zeros_like(((true-self_pred)/scales)[mask[:,:,:,None,None,None].expand_as(true)]))
      total=primary+args.consistency_weight*consistency; opt.zero_grad(); total.backward(); torch.nn.utils.clip_grad_norm_(model.parameters(),1.); opt.step(); losses.append(float(total.detach().cpu()))
    history.append({"epoch":epoch+1,"loss":float(np.mean(losses)),"truePairCalibration":evaluate(torch,model,cloader,device,scales,"truepair"),"selfPairCalibration":evaluate(torch,model,cloader,device,scales,"selfpair")})
  report={"schema":"xdremux-reverse-key1-mixed-selfpair-v1","inputMode":"mixed_true_pair_and_single_image_self_pair","manifestSHA256":sha256_file(args.manifest.resolve()),"checkpointSHA256":sha256_file(args.checkpoint.resolve()),"splitCounts":header["splitCounts"],"selfPairProbability":args.selfpair_probability,"consistencyWeight":args.consistency_weight,"learningRate":args.learning_rate,"epochs":args.epochs,"device":device,"trainableParameters":sum(p.numel() for p in model.parameters() if p.requires_grad),"history":history,"status":"calibration-only"}
  args.output.mkdir(parents=True,exist_ok=True); torch.save({"model":model.state_dict(),"coefficientScales":ck["coefficientScales"],"architecture":ck["architecture"],"manifestSHA256":sha256_file(args.manifest.resolve()),"inputMode":"mixed_true_pair_and_single_image_self_pair"},args.output/"best.pt"); (args.output/"report.json").write_text(json.dumps(report,indent=2,sort_keys=True)+"\n"); print(json.dumps(report,indent=2))
if __name__=="__main__": main()
