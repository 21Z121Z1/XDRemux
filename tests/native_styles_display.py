#!/usr/bin/env python3
"""Exercise the public display outcome on explicitly supplied real HEIFs.

The CLI performs native decode, EXR/HEIC roundtrip and SDR identity checks.
This runner also checks the requested outcome, source/output integrity and
refusal to replace an existing bundle. It is not a physical display test.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--adapter", type=Path, required=True)
    parser.add_argument("--input", type=Path, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-edge", type=int, default=1024)
    parser.add_argument("--headrooms", type=int, nargs="+", choices=[2, 4, 8], default=[2, 4, 8])
    args = parser.parse_args()
    binary, adapter = args.binary.resolve(), args.adapter.resolve()
    inputs = [path.resolve() for path in args.input]
    before = {str(path): sha(path) for path in inputs}
    args.output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, XDREMUX_APPLE_ADAPTER=str(adapter))
    rows = []
    first_command = None
    for headroom in args.headrooms:
        command = [str(binary), "reconstruct-styles", "--output-dir", str(args.output),
                   "--max-edge", str(args.max_edge), "--display-headroom", str(headroom)]
        for path in inputs:
            command.extend(["--input", str(path)])
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=1200)
        (args.output / f"h{headroom}.stdout.json").write_text(result.stdout)
        (args.output / f"h{headroom}.stderr.log").write_text(result.stderr)
        assert result.returncode == 0, result.stderr
        batch = json.loads(result.stdout)
        assert batch["failures"] == 0 and len(batch["results"]) == len(inputs)
        for item in batch["results"]:
            report = item["report"]
            display = report["display"]
            carrier = report["outputs"]["hdr.heic"]
            assert display["headroom"] == headroom
            assert display["runtime_raw_inputs"] == 0
            assert not display["native_editor_parameters_used"]
            assert not display["native_output_ratio_used"]
            assert display["target_max_rgb"] <= headroom + 2e-6
            assert carrier["compressed_sdr_preserved"]
            assert carrier["sdr_readback_max_rgb_error"] == 0
            assert abs(carrier["consumer_content_headroom"] - headroom) <= 1e-5
            assert carrier["hdr_target_mae_ev"] <= .15
            assert display["exr"]["readback_mae_ev"] <= .005
            assert report["source_sha256"] == before[report["source"]]
            path = Path(item["output"]) / "hdr.heic"
            assert sha(path) == carrier["sha256"]
            for mask in display["masks"]:
                assert "sky" not in mask.get("uri", "").lower()
                if mask.get("foreground_proxy"):
                    assert mask["role"] == "foreground"
                    assert mask["uri"] == "urn:com:apple:photo:2018:aux:portraiteffectsmatte"
            rows.append(dict(input=report["source"], headroom=headroom,
                             output=str(path), sha256=carrier["sha256"],
                             scene=report["scene"], display=display, carrier=carrier))
        first_command = first_command or command
    published = {row["output"]: sha(row["output"]) for row in rows}
    repeated = subprocess.run(first_command, env=env, capture_output=True, text=True, timeout=120)
    assert repeated.returncode != 0
    assert json.loads(repeated.stdout)["failures"] == len(inputs)
    assert all(sha(path) == digest for path, digest in published.items())
    assert all(sha(path) == digest for path, digest in before.items())
    result = dict(passed=True, head=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                  binary_sha256=sha(binary), adapter_sha256=sha(adapter),
                  max_edge=args.max_edge, original_hashes=before, rows=rows,
                  existing_bundle_refusal=True, physical_HDR_visual_acceptance=False)
    (args.output / "validation.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(dict(passed=True, photos=len(inputs), profiles=len(rows),
                          max_edge=args.max_edge, validation=str(args.output / "validation.json"))))


if __name__ == "__main__":
    main()
