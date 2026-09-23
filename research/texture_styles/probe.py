"""Bounded, synthetic-only native research probes; never produces a HEIF."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import time
from typing import Sequence

ROOT = Path(__file__).resolve().parent
CASES = (
    "rect-dict", "rect-value", "rect-pose", "rect-value-pose",
    "rect-pose-landmarks-faceLandmarks", "rect-pose-landmarks-features",
    "rect-pose-landmarks-landmarks", "rect-pose-landmarks-values",
    "rect-pose-landmarks-all-aliases",
)
SOURCES = ("PersonInputProbe.m", "InputTrace.m", "RuntimeInventory.m")


class ProbeError(RuntimeError):
    """Invalid probe request or incomplete research evidence."""


def crop_values(values: Sequence[float] | None) -> tuple[float, ...] | None:
    if values is None:
        return None
    if len(values) != 4 or not all(math.isfinite(v) for v in values):
        raise ProbeError("crop needs four finite x/y/width/height values")
    x, y, width, height = values
    if x < 0 or y < 0 or width <= 0 or height <= 0 or x + width > 1 or y + height > 1:
        raise ProbeError("crop must have positive area within normalized [0,1] bounds")
    return tuple(values)


def fresh_directory(path: Path) -> Path:
    # Do not resolve a final symlink: even a dangling symlink must be rejected.
    path = Path(os.path.abspath(path))
    try:
        path.mkdir(mode=0o700)
    except OSError as exc:
        raise ProbeError(f"a new output directory with an existing parent is required: {path}") from exc
    return path


def run_case(command: Sequence[str], directory: Path, name: str, timeout: float) -> dict:
    """One process per case. Preserve failures/timeouts without masking later cases."""
    if name not in (*CASES, "abi-self-test", "runtime", "build-person", "build-trace", "build-runtime"):
        raise ProbeError("unknown case name")
    if not math.isfinite(timeout) or not 0 < timeout <= 300:
        raise ProbeError("timeout must be finite and within (0,300] seconds")
    stdout = directory / f"{name}.stdout"
    stderr = directory / f"{name}.stderr"
    started = time.monotonic()
    result = {"name": name, "command": list(command), "timedOut": False, "exitCode": None}
    with stdout.open("xb") as out, stderr.open("xb") as err:
        try:
            process = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
        except OSError as exc:
            result["launchError"] = str(exc)
        else:
            try:
                result["exitCode"] = process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                result["timedOut"] = True
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                result["exitCode"] = process.wait()
    result["elapsedSeconds"] = time.monotonic() - started
    result["logs"] = {}
    for label, path in (("stdout", stdout), ("stderr", stderr)):
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        result["logs"][label] = {"name": path.name, "size": path.stat().st_size, "sha256": digest.hexdigest()}
    result["succeeded"] = result["exitCode"] == 0 and not result["timedOut"]
    return result


def write_report(path: Path, report: dict) -> None:
    temporary = path.with_suffix(".tmp")
    try:
        with temporary.open("x", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True, allow_nan=False)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def build(output: Path, report: dict) -> dict[str, Path]:
    clang = subprocess.check_output(["xcrun", "--find", "clang"], timeout=10, text=True).strip()
    sdk = subprocess.check_output(["xcrun", "--sdk", "macosx", "--show-sdk-path"], timeout=10, text=True).strip()
    build_directory = output / "build"
    build_directory.mkdir()
    products = {"person": build_directory / "PersonInputProbe",
                "trace": build_directory / "libXDRemuxTextureStyleTrace.dylib",
                "runtime": build_directory / "RuntimeInventory"}
    report["compiler"] = {"path": clang, "sdk": sdk}
    for role, source in zip(products, SOURCES):
        command = [clang, "-fobjc-arc", "-fblocks", "-fmodules", "-Wall", "-Wextra", "-Werror",
                   "-isysroot", sdk, "-framework", "Foundation"]
        if role == "person":
            command += ["-framework", "CoreGraphics"]
        if role == "trace":
            command += ["-dynamiclib", "-Wl,-install_name,@rpath/libXDRemuxTextureStyleTrace.dylib"]
        command += [str(ROOT / source), "-o", str(products[role])]
        result = run_case(command, output, "build-" + role, 120)
        report["cases"].append(result)
        write_report(output / "report.json", report)
        if not result["succeeded"]:
            raise ProbeError(f"{source} did not compile; see {output / result['logs']['stderr']['name']}")
    return products


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("build", "self-test", "person", "runtime"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--case", choices=CASES)
    parser.add_argument("--normalize", nargs=4, type=float)
    parser.add_argument("--timeout", type=float, default=30)
    args = parser.parse_args(argv)
    output = None
    report = {"schema": 1, "mode": args.mode, "platform": platform.platform(),
              "architecture": platform.machine(), "productAccepted": False,
              "deviceValidated": False, "cases": []}
    try:
        crop = crop_values(args.normalize)
        if (args.case or crop) and args.mode != "person":
            raise ProbeError("case/crop options apply only to the person matrix")
        if not math.isfinite(args.timeout) or not 0 < args.timeout <= 300:
            raise ProbeError("timeout must be finite and within (0,300] seconds")
        if sys.platform != "darwin":
            raise ProbeError("native probe build requires macOS and the public Xcode SDK")
        output = fresh_directory(args.output)
        report["sources"] = {s: hashlib.sha256((ROOT / s).read_bytes()).hexdigest() for s in SOURCES}
        products = build(output, report)
        work = []
        if args.mode == "self-test":
            work = [("abi-self-test", [str(products["person"]), "--self-test"])]
        elif args.mode == "runtime":
            work = [("runtime", [str(products["runtime"])])]
        elif args.mode == "person":
            for name in (args.case,) if args.case else CASES:
                command = [str(products["person"]), "--case", name]
                if crop:
                    command += ["--normalize", *(str(v) for v in crop)]
                work.append((name, command))
        for name, command in work:
            report["cases"].append(run_case(command, output, name, args.timeout))
            write_report(output / "report.json", report)
        report["succeeded"] = all(case["succeeded"] for case in report["cases"])
        write_report(output / "report.json", report)
        return 0 if report["succeeded"] else 2
    except (ProbeError, OSError, subprocess.SubprocessError) as exc:
        report.update(succeeded=False, error=str(exc))
        if output is not None:
            write_report(output / "report.json", report)
        print(f"texture-style research: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
