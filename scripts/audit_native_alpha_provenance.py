#!/usr/bin/env python3
"""Audit a small no-cache alpha provenance set without rerendering it."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    ap = argparse.ArgumentParser(); ap.add_argument("--directory", type=Path, required=True); ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args(); root = args.directory
    rows = []
    for tag, seed_name, heic_name in [("0", "alpha-0.f16.bin", "nocache-alpha-0.heic"), ("625", "selfpair-key1.f16.bin", "selfpair-fast-v2.heic"), ("1", "alpha-1.f16.bin", "nocache-alpha-1.heic")]:
        render = root / f"nocache-render-{tag}"
        manifests = [json.loads((render / f"{state}.json").read_text()) for state in ("neutral", "tone_plus")]
        rows.append({"alpha": tag, "seedSHA256": sha(root / seed_name), "heicSHA256": sha(root / heic_name), "renderRequests": [{"state": state, "photo": manifest["photo"], "output": manifest["output"], "outputSHA256": sha(Path(manifest["output"])), "stageMilliseconds": manifest.get("stageMilliseconds")} for state, manifest in zip(("neutral", "tone_plus"), manifests)]})
    if len({row["seedSHA256"] for row in rows}) != 3 or len({row["heicSHA256"] for row in rows}) != 3:
        raise SystemExit("provenance audit failed: alpha inputs are not distinct")
    for state in ("neutral", "tone_plus"):
        outputs = [next(x["outputSHA256"] for x in row["renderRequests"] if x["state"] == state) for row in rows]
        # Equal consumer pixels are valid evidence of insensitivity; only reuse
        # of the same artifact path would be a provenance failure.
        paths = [next(x["output"] for x in row["renderRequests"] if x["state"] == state) for row in rows]
        if len(set(paths)) != 3: raise SystemExit(f"provenance audit failed: {state} response paths were reused")
    result = {"schema":"xdremux-native-alpha-provenance-v1","noCache":True,"rows":rows,"consumerPixelsEqualByState":{state:len({next(x["outputSHA256"] for x in row["renderRequests"] if x["state"] == state) for row in rows}) == 1 for state in ("neutral","tone_plus")}}
    args.output.parent.mkdir(parents=True, exist_ok=True); args.output.write_text(json.dumps(result,indent=2,sort_keys=True)+"\n"); print(json.dumps(result,indent=2))
if __name__ == "__main__": main()
