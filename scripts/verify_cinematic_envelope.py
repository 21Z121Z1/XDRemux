#!/usr/bin/env python3
"""Strip native Cinematic samples and regenerate pending envelopes with the CLI.

Private input samples stay external to Git. Only caller-selected inputs are read.
The generated report records source integrity and public Apple consumer results.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def run(command: list[str], log: Path, env: dict[str, str] | None = None) -> None:
    with log.open("w") as output:
        result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, env=env, timeout=1200)
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}): {command[0]}; {log.read_text()[-4000:]}")


def media_packets(path: Path) -> list[dict]:
    # AVAssetReader can group the same audio packets into different buffers.
    # Compare individual container packets, including AAC priming/trim data.
    document = json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-show_streams", "-show_packets", "-show_data_hash", "sha256",
        "-show_entries", "stream=index,codec_type,codec_name,codec_tag_string,time_base:packet=stream_index,pts_time,dts_time,duration_time,size,data_hash",
        "-of", "json", str(path)
    ], text=True))
    media = []
    for stream in document["streams"]:
        if stream["codec_type"] in {"video", "audio"}:
            packets = [{key: value for key, value in packet.items() if key != "stream_index"}
                       for packet in document["packets"] if packet["stream_index"] == stream["index"]]
            media.append({"type": stream["codec_type"], "codec": stream.get("codec_name", stream.get("codec_tag_string", "unknown")), "packets": packets})
    return media


def compare_packets(a: Path, b: Path) -> list[dict]:
    source, output = media_packets(a), media_packets(b)
    if source != output:
        raise RuntimeError(f"compressed media packet hash/timing mismatch: {a} -> {b}")
    return [{"type": track["type"], "codec": track["codec"], "packets": len(track["packets"]),
             "packet_bytes_timestamps_and_trim_identical": True} for track in source]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", action="append", type=Path, required=True)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--swift-build-dir", type=Path, required=True)
    parser.add_argument("--rust-build-dir", type=Path, required=True)
    parser.add_argument("--focal-length-35mm", type=float, default=24.0)
    parser.add_argument("--aperture", type=float, default=2.8)
    parser.add_argument("--synthetic", action="store_true", help="Test independent 1080p H.264 rendering and a separate 360p native compatibility probe")
    parser.add_argument("--compare-render", action="store_true", help="Preprocess, render and reload public edits; compare native references when available")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    out = args.run_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    rust_env = os.environ | {"CARGO_TARGET_DIR": str(args.rust_build_dir.resolve())}
    run(["cargo", "build", "--offline", "-p", "xdremux-cli"], out / "rust-build.log", rust_env)
    run(["swift", "build", "--product", "xdremux-apple-adapter", "--scratch-path", str(args.swift_build_dir.resolve())], out / "swift-build.log")
    adapter_candidates = [args.swift_build_dir / "out/Products/Debug/xdremux-apple-adapter", args.swift_build_dir / "debug/xdremux-apple-adapter"]
    adapter = next(path.resolve() for path in adapter_candidates if path.is_file())
    cli = (args.rust_build_dir / "debug/xdremux").resolve()
    probe = out / "cinematic-consumer-probe"
    run(["swiftc", "-parse-as-library", str(repo / "scripts/verify_cinematic_envelope.swift"),
         "-module-cache-path", str(out / "module-cache"), "-o", str(probe)], out / "probe-build.log")
    renderer = out / "cinematic-render-comparison"
    if args.compare_render:
        run(["swiftc", "-O", "-parse-as-library", str(repo / "scripts/compare_cinematic_render.swift"),
             "-module-cache-path", str(out / "module-cache"), "-o", str(renderer)], out / "render-probe-build.log")
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    report: dict = {"schema": "xdremux-cinematic-envelope-verification-v1", "head": head,
                    "runtime": subprocess.check_output(["sw_vers", "-productVersion"], text=True).strip(),
                    "cli_sha256": digest(cli), "adapter_sha256": digest(adapter), "assets": []}
    inputs = [(path.resolve(), True, False) for path in args.input]
    if args.synthetic:
        for label, size, compatibility in [("1080", "1920x1080", False), ("360", "640x360", True)]:
            generated = out / f"synthetic-h264-{label}.mp4"
            run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", f"testsrc2=size={size}:rate=30000/1001", "-t", "2",
                 "-c:v", "libx264", "-pix_fmt", "yuv420p", str(generated)], out / f"synthetic-{label}-create.log")
            inputs.append((generated, False, compatibility))
    for index, (source, needs_strip, compatibility) in enumerate(inputs):
        case = out / f"{index + 1:02d}-{source.stem}"
        case.mkdir()
        original_hash = digest(source)
        ordinary = case / "ordinary.mov" if needs_strip else source
        if needs_strip:
            run([str(probe), "strip", str(source), str(ordinary), str(case / "strip.json")], case / "strip.log")
            strip_packets = compare_packets(source, ordinary)
        ordinary_hash = digest(ordinary)
        pending = case / "pending.mov"
        environment = os.environ | {"XDREMUX_APPLE_ADAPTER": str(adapter)}
        run([str(cli), "cinematic", "--input", str(ordinary), "--output", str(pending),
             "--focal-length-35mm", str(args.focal_length_35mm), "--aperture", str(args.aperture)], case / "generate.log", environment)
        run([str(probe), "verify", str(ordinary), str(pending), str(case / "consumer.json"), str(args.aperture)], case / "consumer.log")
        consumer = json.loads((case / "consumer.json").read_text())
        pending_packets = compare_packets(ordinary, pending)
        processed = next(int(line.split(":", 1)[1]) for line in (case / "generate.log").read_text().splitlines() if line.startswith("processed-frames:"))
        if processed != consumer["decoded_frames"]:
            raise RuntimeError("CLI processed-frame count differs from consumer frame count")
        if digest(source) != original_hash or digest(ordinary) != ordinary_hash:
            raise RuntimeError("a source video changed")
        record = {"source": str(source), "source_sha256": original_hash, "source_unchanged": True,
                  "ordinary": str(ordinary), "ordinary_sha256": ordinary_hash, "ordinary_unchanged": True,
                  "pending": str(pending), "pending_sha256": digest(pending), "processed_frames": processed,
                  "consumer": consumer, "compressed_media": pending_packets}
        if needs_strip:
            record["strip"] = json.loads((case / "strip.json").read_text())
            record["stripped_compressed_media"] = strip_packets
        if args.compare_render:
            if needs_strip:
                run([str(renderer), str(source), str(pending), str(case / "headless")], case / "headless.log")
                record["headless_comparison"] = json.loads((case / "headless/comparison.json").read_text())
            elif compatibility:
                run([str(renderer), "--probe-compatibility", str(pending), str(case / "headless")], case / "headless.log")
                record["headless_compatibility"] = json.loads((case / "headless/compatibility.json").read_text())
            else:
                run([str(renderer), "--verify-generated", str(pending), str(case / "headless")], case / "headless.log")
                record["headless_generated"] = json.loads((case / "headless/generated-verification.json").read_text())
            if digest(source) != original_hash or digest(pending) != record["pending_sha256"]:
                raise RuntimeError("a Cinematic input changed during headless comparison")
        report["assets"].append(record)
        (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"{source.name}: none -> needsPreprocessing, ready, frames={processed}, compressed media unchanged", flush=True)
        if "headless_comparison" in record:
            print(f"  headless native/generated pixel identity={record['headless_comparison']['all_compared_frames_identical']}", flush=True)
        if "headless_compatibility" in record:
            print(f"  compatibility only: native preprocessing supported={record['headless_compatibility']['native_preprocessing_supported']}", flush=True)
    print(f"verified={len(report['assets'])}; report={out / 'report.json'}", flush=True)


if __name__ == "__main__":
    main()
