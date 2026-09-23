"""Write a finite research state ZIP from one image, never a product HEIC."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
from .inference import load_universal_image, load_universal_model, predict_universal_state, candidate_state_resources, publish_candidate_archive


def main() -> None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input',required=True,type=Path)
    parser.add_argument('--checkpoint',required=True,type=Path,help='provenance-checked PyTorch checkpoint')
    parser.add_argument('--output',required=True,type=Path,help='new ZIP path; existing files are never replaced')
    parser.add_argument('--device',choices=('auto','mps','cpu'),default='auto')
    parser.add_argument('--exiftool',default='exiftool')
    parser.add_argument('--linear-rgb-sidecar',type=Path)
    parser.add_argument('--gain-map-sidecar',type=Path)
    args=parser.parse_args()
    image=load_universal_image(args.input,exiftool=args.exiftool,
        linear_rgb_sidecar=args.linear_rgb_sidecar,gain_map_sidecar=args.gain_map_sidecar)
    model,checkpoint,device=load_universal_model(args.checkpoint,args.device)
    predicted,elapsed=predict_universal_state(image,model,device=device)
    resources=candidate_state_resources(image,predicted)
    report=publish_candidate_archive(args.output,image,resources,checkpoint,device=device,model_seconds=elapsed)
    print(json.dumps(report,indent=2,sort_keys=True,allow_nan=False))

if __name__=='__main__':main()
