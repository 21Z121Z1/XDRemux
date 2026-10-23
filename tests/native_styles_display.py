#!/usr/bin/env python3
"""Exercise the public display outcome on explicitly supplied real HEIFs.

The CLI performs native decode, EXR/HEIC roundtrip and SDR identity checks.
This runner also checks the requested outcome, source/output integrity and
refusal to replace an existing bundle. It is not a physical display test.
"""
import argparse
import array
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import tempfile


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_target(adapter, path, directory, width, height, headroom):
    request = dict(schema_version=2, operation="styles-readback", input_path=str(path),
                   output_path=str(directory), width=width, height=height,
                   target_headroom=headroom)
    result = subprocess.run([str(adapter)], input=json.dumps(request), text=True,
                            capture_output=True, check=True, timeout=300)
    facts = json.loads(result.stdout)
    assert facts["target"]["width"] == width and facts["target"]["height"] == height
    values = array.array("f")
    values.frombytes((directory / "published_target.f32").read_bytes())
    assert len(values) == width * height * 4
    assert all(math.isfinite(v) for v in values)
    return values


def compare_luminance(a, b):
    errors = []
    for i in range(0, len(a), 4):
        ya = a[i]*.22897456 + a[i+1]*.69173855 + a[i+2]*.07928691
        yb = b[i]*.22897456 + b[i+1]*.69173855 + b[i+2]*.07928691
        if ya > 1e-4:
            errors.append(abs(math.log2(max(yb, 1e-8) / ya)))
    assert errors
    errors.sort()
    return dict(mean_abs_ev=sum(errors)/len(errors), p99_abs_ev=errors[int((len(errors)-1)*.99)],
                max_abs_ev=errors[-1], pixels=len(errors), global_exposure_removed=False)


def check_guidance(report):
    assert report["algorithm"] == "embedded-styles-relative-scene-v2"
    facts = report["scene"]["guidance_stabilization"]
    assert facts["algorithm"] == "bounded-weak-structure-log-gain-v1"
    assert max(facts["width"], facts["height"]) <= 512
    assert 0 <= facts["solver_relative_residual"] <= 1e-5
    assert 0 <= facts["maximum_adjustment_ev"] <= facts["limit_ev"] == .25
    assert 0 <= facts["adjustment_rms_ev"] <= facts["maximum_adjustment_ev"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--adapter", type=Path, required=True)
    parser.add_argument("--input", type=Path, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-edge", type=int, default=1024)
    parser.add_argument("--headrooms", type=int, nargs="+", choices=[2, 4, 8], default=[2, 4, 8])
    parser.add_argument("--check-coupling", action="store_true",
                        help="Compare separately encoded 2x/4x endpoints to the same 8x file")
    parser.add_argument("--check-scene", action="store_true",
                        help="Also validate the default scene outcome on the same inputs")
    args = parser.parse_args()
    if args.check_coupling:
        assert sorted(args.headrooms) == [2, 4, 8]
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
            check_guidance(report)
            display = report["display"]
            carrier = report["outputs"]["hdr.heic"]
            assert display["headroom"] == headroom
            assert display["algorithm"] == "scene-gap-coupled-log-family-v2"
            assert display["family_master_headroom"] == 8
            assert display["runtime_raw_inputs"] == 0
            assert not display["native_editor_parameters_used"]
            assert not display["native_output_ratio_used"]
            assert display["target_max_rgb"] <= headroom + 2e-6
            assert carrier["compressed_sdr_preserved"]
            assert carrier["sdr_readback_max_rgb_error"] == 0
            assert abs(carrier["consumer_content_headroom"] - headroom) <= 1e-5
            assert carrier["hdr_target_mae_ev"] <= .15
            assert display["exr"]["readback_mae_ev"] <= .005
            family = display["headroom_validation"]
            assert family["same_encoded_file"]
            assert [r["requested_headroom"] for r in family["rows"]] == [1, 1.5, 2, 3, 4, 6, 8]
            previous = 1
            for row in family["rows"]:
                assert row["target_headroom"] == min(headroom, row["requested_headroom"])
                assert 0 <= row["retained_local"] <= previous
                previous = row["retained_local"]
                assert abs(row["carrier_weight"] - math.log2(row["target_headroom"])/math.log2(headroom)) < 1e-6
                assert row["transport_and_projection"]["mean_abs_ev"] <= .05
                assert row["transport_and_projection"]["nonfinite_rgb_components"] == 0
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
    coupling = []
    if args.check_coupling:
        with tempfile.TemporaryDirectory(prefix=".coupling-", dir=args.output) as scratch:
            directory = Path(scratch)
            for source in inputs:
                variants = {r["headroom"]: r for r in rows if r["input"] == str(source)}
                width, height = variants[8]["scene"]["width"], variants[8]["scene"]["height"]
                for h in [2, 4]:
                    own = read_target(adapter, Path(variants[h]["output"]), directory, width, height, h)
                    same = read_target(adapter, Path(variants[8]["output"]), directory, width, height, h)
                    metrics = compare_luminance(own, same)
                    # Codec quantization and range projection remain in this
                    # comparison. No median exposure is subtracted.
                    assert metrics["mean_abs_ev"] <= .03, metrics
                    coupling.append(dict(source=str(source), headroom=h, metrics=metrics,
                                         same_file_sha256=variants[8]["sha256"]))
    scene_rows = []
    if args.check_scene:
        command = [str(binary), "reconstruct-styles", "--output-dir", str(args.output),
                   "--max-edge", str(args.max_edge)]
        for path in inputs:
            command.extend(["--input", str(path)])
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=1200)
        (args.output / "scene.stdout.json").write_text(result.stdout)
        (args.output / "scene.stderr.log").write_text(result.stderr)
        assert result.returncode == 0, result.stderr
        batch = json.loads(result.stdout)
        assert batch["failures"] == 0 and len(batch["results"]) == len(inputs)
        for item in batch["results"]:
            report = item["report"]
            check_guidance(report)
            assert report["display"] is None
            carrier = report["outputs"]["hdr.heic"]
            assert carrier["compressed_sdr_preserved"]
            assert carrier["sdr_readback_max_rgb_error"] == 0
            assert carrier["hdr_target_mae_ev"] <= .15
            assert report["outputs"]["scene_linear.exr"]["readback_mae_ev"] <= .005
            assert report["source_sha256"] == before[report["source"]]
            path = Path(item["output"]) / "hdr.heic"
            assert sha(path) == carrier["sha256"]
            scene_rows.append(dict(input=report["source"], output=str(path),
                                   scene=report["scene"], carrier=carrier))
    published = {row["output"]: sha(row["output"]) for row in rows + scene_rows}
    repeated = subprocess.run(first_command, env=env, capture_output=True, text=True, timeout=120)
    assert repeated.returncode != 0
    assert json.loads(repeated.stdout)["failures"] == len(inputs)
    assert all(sha(path) == digest for path, digest in published.items())
    assert all(sha(path) == digest for path, digest in before.items())
    result = dict(passed=True, head=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                  binary_sha256=sha(binary), adapter_sha256=sha(adapter),
                  max_edge=args.max_edge, original_hashes=before, rows=rows,
                  coupling=coupling, scene_rows=scene_rows,
                  existing_bundle_refusal=True, physical_HDR_visual_acceptance=False)
    (args.output / "validation.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(dict(passed=True, photos=len(inputs), profiles=len(rows),
                          max_edge=args.max_edge, validation=str(args.output / "validation.json"))))


if __name__ == "__main__":
    main()
