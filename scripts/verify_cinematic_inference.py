#!/usr/bin/env python3
"""Exercise independent Cinematic inference with known RGB zoom and pan videos."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess

from verify_cinematic_envelope import compare_packets, digest, run


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--adapter", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--run-dir", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    out = args.run_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    cli, adapter, probe = (path.resolve(strict=True) for path in [args.cli, args.adapter, args.probe])
    texture = out / "texture.png"
    run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "mandelbrot=size=1920x1080",
         "-frames:v", "1", "-update", "1", str(texture)], out / "texture-create.log")
    transforms = {
        "zoom": "zoompan=z='pow(2,max(0,min(1,(on-12)/30)))':x='iw/2-iw/zoom/2':y='ih/2-ih/zoom/2':d=60:s=960x540:fps=30",
        "pan": "zoompan=z=1.5:x='on*5':y=100:d=60:s=960x540:fps=30",
    }
    report = {"schema": "xdremux-cinematic-rgb-inference-v1",
              "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
              "cli_sha256": digest(cli), "adapter_sha256": digest(adapter), "probe_sha256": digest(probe),
              "texture_sha256": digest(texture), "cases": []}
    for name, transform in transforms.items():
        source, pending = out / f"{name}.mp4", out / f"{name}-pending.mov"
        run(["ffmpeg", "-v", "error", "-loop", "1", "-i", str(texture), "-vf", transform,
             "-frames:v", "60", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-an", str(source)],
            out / f"{name}-create.log")
        source_hash = digest(source)
        run([str(cli), "cinematic", "--input", str(source), "--output", str(pending),
             "--focal-length-35mm", "24", "--aperture", "2.8"], out / f"{name}-generate.log",
            os.environ | {"XDREMUX_APPLE_ADAPTER": str(adapter)})
        consumer_path = out / f"{name}-consumer.json"
        run([str(probe), "verify", str(source), str(pending), str(consumer_path), "2.8"],
            out / f"{name}-consumer.log")
        consumer = json.loads(consumer_path.read_text())
        media = compare_packets(source, pending)
        if digest(source) != source_hash:
            raise RuntimeError("inference changed the source video")
        focal = [row["focal_length_35mm"] for row in consumer["focal_length_35mm_timeline"]]
        if len(focal) != 60 or consumer["decoded_frames"] != 60:
            raise RuntimeError("inference did not cover the complete fixture")
        initial, final = statistics.median(focal[:12]), statistics.median(focal[-8:])
        if abs(initial - 24) > 0.01:
            raise RuntimeError("inference changed the initial focal assumption")
        if name == "zoom" and not 1.85 <= final / initial <= 2.15:
            raise RuntimeError(f"known 2x RGB zoom was not recovered: {final / initial}")
        if name == "pan" and max(focal) / min(focal) > 1.02:
            raise RuntimeError("pure RGB translation was interpreted as zoom")
        report["cases"].append({"case": name, "rgb_transform": transform, "source_sha256": source_hash,
                                "source_unchanged": True, "pending_sha256": digest(pending),
                                "initial_focal_length_35mm": initial, "final_focal_length_35mm": final,
                                "focal_range": [min(focal), max(focal)], "compressed_media": media,
                                "consumer": consumer})
        (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"{name}: focal={initial:.3f}->{final:.3f}, 60 frames, public pending consumer accepted", flush=True)


if __name__ == "__main__":
    main()
