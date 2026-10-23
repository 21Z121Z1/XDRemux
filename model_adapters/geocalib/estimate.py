#!/usr/bin/env python3
"""GeoCalib model primitive: return pinhole observations to the Rust runtime."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import io
import json
import math
import os
import selectors
import tempfile
from pathlib import Path
import subprocess
import sys
import time

GEOCALIB_WEIGHTS_SHA256 = "86d6aeacd8bbd974c59ce39f61854e00d36911c732ad89be471476fd708722ac"
GEOCALIB_REVISION = "1dedb29f94fc11b896c5f572005978aeebccd67f"


PROBE_OUTPUT_LIMIT = 64 * 1024
FRAME_OUTPUT_LIMIT = 16 * 1024 * 1024
DIAGNOSTIC_OUTPUT_LIMIT = 64 * 1024


def run_bounded(command: list[str], *, stdout_limit: int, stderr_limit: int = DIAGNOSTIC_OUTPUT_LIMIT,
                timeout: float = 60) -> subprocess.CompletedProcess:
    """Drain both POSIX pipes within byte and wall-clock limits, always reaping."""
    if stdout_limit < 0 or stderr_limit < 0 or not math.isfinite(timeout) or timeout <= 0:
        raise ValueError("invalid subprocess limits")
    deadline = time.monotonic() + timeout
    process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE)
    stdout = bytearray()
    stderr = bytearray()
    try:
        with selectors.DefaultSelector() as selector:
            for stream, name, buffer, limit in [(process.stdout, "stdout", stdout, stdout_limit),
                                                 (process.stderr, "stderr", stderr, stderr_limit)]:
                os.set_blocking(stream.fileno(), False)
                selector.register(stream, selectors.EVENT_READ, (name, buffer, limit))
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise subprocess.TimeoutExpired(command, timeout)
                for key, _ in selector.select(remaining):
                    name, buffer, limit = key.data
                    chunk = os.read(key.fd, min(65536, limit - len(buffer) + 1))
                    if not chunk:
                        selector.unregister(key.fileobj)
                    elif len(buffer) + len(chunk) > limit:
                        raise ValueError(f"{Path(command[0]).name} {name} exceeded {limit} bytes")
                    else:
                        buffer.extend(chunk)
            # EOF alone does not mean the child exited; retain the same deadline.
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise subprocess.TimeoutExpired(command, timeout)
            process.wait(timeout=remaining)
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        process.stdout.close()
        process.stderr.close()
    result = subprocess.CompletedProcess(command, process.returncode, bytes(stdout), bytes(stderr))
    result.check_returncode()
    return result


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def source_duration(path: Path) -> float:
    result = run_bounded(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "stream=duration,sample_aspect_ratio,color_transfer:stream_disposition=attached_pic:format=duration",
         "-of", "json", str(path)],
        stdout_limit=PROBE_OUTPUT_LIMIT,
    )
    facts = json.loads(result.stdout)
    streams = facts.get("streams", [])
    if len(streams) != 1:
        raise ValueError("input must have a decodable first video stream")
    video = streams[0]
    if video.get("sample_aspect_ratio", "1:1") not in {"1:1", "N/A"}:
        raise ValueError("camera estimation currently requires square pixels")
    if video.get("color_transfer") in {"smpte2084", "arib-std-b67"}:
        raise ValueError("camera estimation currently requires SDR video")
    if video.get("disposition", {}).get("attached_pic", 0):
        raise ValueError("first video stream must not be an attached picture")
    duration = float(video.get("duration", facts.get("format", {}).get("duration", 0)))
    if not math.isfinite(duration) or duration <= 0:
        raise ValueError("video duration must be finite and positive")
    return duration


def check_package_revision() -> None:
    distribution = importlib.metadata.distribution("geocalib")
    direct = json.loads(distribution.read_text("direct_url.json") or "{}")
    expected = f"https://github.com/cvg/GeoCalib/archive/{GEOCALIB_REVISION}.tar.gz"
    if direct.get("url") != expected:
        raise ValueError("install GeoCalib from the pinned revision in README.en.md")


def shared_focal_uncertainty(covariance: list[list[float]], count: int, vertical_scale: float) -> float:
    dimensions = 2 * count + 1
    if len(covariance) != dimensions or any(len(row) != dimensions for row in covariance):
        raise ValueError("unexpected GeoCalib shared pinhole covariance dimensions")
    if not math.isfinite(vertical_scale) or vertical_scale <= 0:
        raise ValueError("invalid model preprocessing scale")
    # This revision's shared-mode accessor indexes a gravity parameter at 2.
    # The joint system puts the single focal parameter after 2 * count gravity parameters.
    variance = covariance[-1][-1]
    if not math.isfinite(variance) or variance < 0:
        raise ValueError("invalid shared focal covariance")
    return math.sqrt(variance) / 2 / vertical_scale


def camera_observation(camera, uncertainty: float) -> dict:
    return {
        "image_size": [round(value) for value in camera.size[0].tolist()],
        "focal_length_pixels": camera.f[0].tolist(),
        "principal_point_pixels": camera.c[0].tolist(),
        "focal_uncertainty_pixels": uncertainty,
    }


def estimate(input_path: Path, weights: Path, count: int) -> dict:
    import numpy as np
    from PIL import Image
    import torch
    from geocalib import GeoCalib

    check_package_revision()
    if not 3 <= count <= 128:
        raise ValueError("sample count must be between 3 and 128")
    source_hash = digest(input_path)
    weights_hash = digest(weights)
    if weights_hash != GEOCALIB_WEIGHTS_SHA256:
        raise ValueError("weights SHA-256 does not match the pinned GeoCalib release")
    duration = source_duration(input_path)
    torch.set_num_threads(4)
    torch.manual_seed(0)
    model = GeoCalib(weights=str(weights)).to("cpu")
    samples = []
    images = []
    for index in range(count):
        time = duration * (index + 0.5) / count
        decoded = run_bounded(
            ["ffmpeg", "-v", "error", "-ss", str(time), "-i", str(input_path),
             "-map", "0:v:0", "-frames:v", "1", "-vf",
             "scale=w='min(1280,iw)':h='min(1280,ih)':force_original_aspect_ratio=decrease",
             "-f", "image2pipe", "-vcodec", "png", "pipe:1"],
            stdout_limit=FRAME_OUTPUT_LIMIT,
        )
        with Image.open(io.BytesIO(decoded.stdout)) as frame:
            image = torch.from_numpy(np.array(frame.convert("RGB"), copy=True)).permute(2, 0, 1).float() / 255
        images.append(image)
        prediction = model.calibrate(image)
        samples.append({
            "time_seconds": time,
            "intrinsics": camera_observation(prediction["camera"], prediction["focal_uncertainty"][0].item()),
        })
        print(f"sample {index + 1}/{count}", file=sys.stderr, flush=True)
    batch = torch.stack(images)
    shared = model.calibrate(batch, shared_intrinsics=True)
    scales = model.image_processor(batch)["scales"]
    uncertainty = shared_focal_uncertainty(shared["covariance"][0].tolist(), count, scales[1].item())
    if digest(input_path) != source_hash or digest(weights) != weights_hash:
        raise ValueError("input or model weights changed during estimation")
    return {
        "schema_version": 1,
        "source_sha256": source_hash,
        "estimator": {
            "name": "geocalib-pinhole",
            "revision": GEOCALIB_REVISION,
            "weights_sha256": weights_hash,
        },
        "samples": samples,
        "shared_intrinsics": camera_observation(shared["camera"], uncertainty),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--weights", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=5)
    args = parser.parse_args()
    try:
        if args.output.exists():
            raise ValueError("refusing to replace an existing camera estimate")
        result = estimate(args.input.resolve(strict=True), args.weights.resolve(strict=True), args.samples)
        encoded = json.dumps(result, indent=2, allow_nan=False) + "\n"
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=args.output.parent,
                                             prefix=".camera-estimate-", delete=False) as stream:
                temporary = Path(stream.name)
                stream.write(encoded)
            os.link(temporary, args.output)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
    except (ValueError, OSError, subprocess.SubprocessError, ImportError) as error:
        print(f"camera estimation failed: {error}", file=sys.stderr)
        return 1
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
