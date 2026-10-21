#!/usr/bin/env python3
"""Diagnose Styles costs with two exact revisions interleaved on one runner.

This is separate evidence from the unchanged absolute performance gate. Each
timed conversion uses the real adapter directly. Native validation runs only
after every timed conversion, so its work cannot enter a sample's elapsed time.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import statistics
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path("fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic")
TIME = Path("/usr/bin/time")
WARMUP = 1
ITERATIONS = 5
CARGO_RELEASE_COMMAND = ["cargo", "build", "--locked", "--workspace", "--all-targets", "--release"]
PROFILE_ENV = "XDREMUX_STYLES_PROFILE"
PROFILE_STAGES = (
    "source_analysis", "base_conversion", "semantic_production", "resource_decode",
    "scene_hdr_reconstruction", "scene_linear_area_reduction", "scene_derivation",
    "thumbnail_encode", "key1_fit", "style_metadata",
    "native_style_properties", "style_assembly", "texture_generation",
    "final_validation", "publication",
)


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def git(root: Path, *arguments: str) -> str:
    return subprocess.check_output(["git", "-C", str(root), *arguments], text=True).strip()


def commit(root: Path, reference: str) -> str:
    return git(root, "rev-parse", "--verify", "--end-of-options", reference + "^{commit}")


def write_report(directory: Path, report: dict) -> None:
    pending = directory / "report.pending.json"
    pending.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    pending.replace(directory / "report.json")


def checked_fixture(root: Path) -> str:
    entries = dict(
        (name.strip(), checksum)
        for checksum, name in (
            line.split(None, 1)
            for line in (root / "fixtures/SHA256SUMS").read_text().splitlines()
            if line.strip()
        )
    )
    actual = digest(root / FIXTURE)
    expected = entries[str(FIXTURE.relative_to("fixtures"))]
    if actual != expected:
        raise RuntimeError(f"fixture checksum differs from manifest in {root}")
    return actual


def run_process(command: list[str], cwd: Path, env: dict, stdout, stderr, timeout: int) -> int:
    with subprocess.Popen(command, cwd=cwd, env=env, stdout=stdout, stderr=stderr,
                          start_new_session=True) as child:
        try:
            return child.wait(timeout=timeout)
        except BaseException:
            # A timed-out /usr/bin/time or cargo process can own descendants.
            # Terminate only the fresh process group created for this command.
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.wait()
            raise


def run_logged(command: list[str], cwd: Path, env: dict, log: Path, timeout: int) -> None:
    with log.open("w", encoding="utf-8") as stream:
        status = run_process(command, cwd, env, stream, subprocess.STDOUT, timeout)
    if status:
        raise RuntimeError(f"command exited {status}: {command[0]}; see {log}")


def binary(path: Path) -> dict:
    path = path.resolve(strict=True)
    return {"path": str(path), "sha256": digest(path), "bytes": path.stat().st_size}


def feature_graph(source: Path, env: dict, directory: Path, name: str) -> dict:
    command = ["cargo", "tree", "--locked", "--workspace", "--edges", "features"]
    log = directory / f"{name}-cargo-features.log"
    run_logged(command, source, env, log, 120)
    normalized = log.read_text()
    # Darwin temporary paths can be spelled /var/... while Cargo prints the
    # canonical /private/var/... path. Normalize both without masking features.
    for prefix in sorted({str(source), str(source.resolve())}, key=len, reverse=True):
        normalized = normalized.replace(prefix, "<source>")
    return {"command": command, "log": log.name, "sha256": digest(log),
            "source_path_normalized_sha256": hashlib.sha256(normalized.encode()).hexdigest()}


@contextmanager
def reference_build(root: Path, sha: str, owned: Path, logs: Path, report: dict):
    source = owned / "reference-source"
    cargo_target = owned / "reference-cargo-target"
    swift_scratch = owned / "reference-swift-scratch"
    # Every path is inside this invocation's fresh TemporaryDirectory. Never
    # prune the shared registry or remove a caller's existing worktree.
    added = False
    try:
        run_logged(["git", "worktree", "add", "--detach", str(source), sha], root,
                   os.environ.copy(), logs / "worktree-add.log", 120)
        added = True
        env = {**os.environ, "CARGO_TARGET_DIR": str(cargo_target)}
        run_logged(CARGO_RELEASE_COMMAND, source, env, logs / "reference-rust-build.log", 1800)
        swift = ["swift", "build", "--package-path", str(source), "--scratch-path",
                 str(swift_scratch), "-c", "release"]
        run_logged([*swift, "--product", "xdremux-apple-adapter"], source, env,
                   logs / "reference-swift-build.log", 1800)
        bin_path = subprocess.check_output([*swift, "--show-bin-path"], cwd=source,
                                           env=env, text=True).strip()
        yield {
            "sha": commit(source, "HEAD"),
            "tracked_status": git(source, "status", "--porcelain=v1", "--untracked-files=no"),
            "cli": binary(cargo_target / "release/xdremux"),
            "adapter": binary(Path(bin_path) / "xdremux-apple-adapter"),
            "fixture_sha256": checked_fixture(source),
            "build_commands": [CARGO_RELEASE_COMMAND, [*swift, "--product", "xdremux-apple-adapter"]],
            "feature_graph": feature_graph(source, env, logs, "reference"),
        }
    finally:
        registered = any(line == f"worktree {source}" for line in git(root, "worktree", "list", "--porcelain").splitlines())
        if added or registered:
            result = subprocess.run(["git", "worktree", "remove", "--force", str(source)],
                                    cwd=root, capture_output=True, text=True, check=False)
            (logs / "worktree-remove.log").write_text(result.stdout + result.stderr)
            report["owned_worktree_cleanup"] = {"path": str(source), "returncode": result.returncode}
            if result.returncode:
                report["cleanup_error"] = "failed to remove this diagnostic's owned worktree"
                if sys.exc_info()[0] is None:
                    raise RuntimeError(report["cleanup_error"])


def sample_order() -> list[tuple[bool, int, str]]:
    result = [(True, index, arm) for index in range(WARMUP) for arm in ("baseline", "candidate")]
    for index in range(ITERATIONS):
        arms = ("baseline", "candidate") if index % 2 == 0 else ("candidate", "baseline")
        result.extend((False, index, arm) for arm in arms)
    return result


def time_facts(stderr: str) -> dict:
    cpu = re.search(r"(?m)^\s*([0-9.]+)\s+real\s+([0-9.]+)\s+user\s+([0-9.]+)\s+sys\s*$", stderr)
    rss = re.search(r"(?m)^\s*(\d+)\s+maximum resident set size\s*$", stderr)
    if not cpu or not rss:
        raise RuntimeError("missing Darwin /usr/bin/time CPU or RSS receipt")
    return {"time_real_seconds": float(cpu[1]), "user_seconds": float(cpu[2]),
            "sys_seconds": float(cpu[3]), "peak_rss_bytes": int(rss[1])}


def profile_facts(stderr: str, *, require_complete: bool = True) -> list[dict]:
    stages = []
    for line in stderr.splitlines():
        if not line.startswith("xdremux_styles_profile"):
            continue
        match = re.fullmatch(r"xdremux_styles_profile stage=([a-z0-9_]+) seconds=([0-9]+(?:\.[0-9]+)?)", line)
        if not match or not math.isfinite(float(match[2])):
            raise RuntimeError("malformed Styles profile receipt")
        stages.append({"stage": match[1], "wall_seconds": float(match[2])})
    expected = PROFILE_STAGES if require_complete else PROFILE_STAGES[:len(stages)]
    if tuple(stage["stage"] for stage in stages) != expected:
        raise RuntimeError("incomplete, duplicate, or out-of-order Styles profile receipt")
    return stages


def conversion_environment(arm: dict, *, profile: bool = False) -> dict:
    env = {**os.environ, "XDREMUX_APPLE_ADAPTER": arm["adapter"]["path"]}
    env.pop(PROFILE_ENV, None)
    if profile:
        env[PROFILE_ENV] = "1"
    return env


def measure(arm: dict, fixture: Path, output: Path, logs: Path, *, profile: bool = False) -> dict:
    env = conversion_environment(arm, profile=profile)
    command = [str(TIME), "-l", arm["cli"]["path"], "convert", "--input", str(fixture),
               "--output", str(output), "--apple-styles"]
    started = time.perf_counter()
    with (logs / "convert.stdout.log").open("w") as stdout, (logs / "convert.stderr.log").open("w") as stderr:
        status = run_process(command, ROOT, env, stdout, stderr, 1200)
    elapsed = time.perf_counter() - started
    if status:
        raise RuntimeError(f"Styles conversion exited {status}; see {logs}")
    if not output.is_file() or not output.stat().st_size:
        raise RuntimeError(f"Styles conversion did not produce a nonempty output; see {logs}")
    stderr = (logs / "convert.stderr.log").read_text()
    result = {"wall_seconds": elapsed, **time_facts(stderr),
              "output_bytes": output.stat().st_size, "output_sha256": digest(output)}
    if profile:
        result["stages"] = profile_facts(stderr)
    return result


def collect(arms: dict, fixture: Path, owned: Path, directory: Path, report: dict) -> None:
    outputs = []
    for ordinal, (warmup, iteration, name) in enumerate(sample_order()):
        label = f"{ordinal:02d}-{name}-{'warmup' if warmup else iteration}"
        logs = directory / label
        logs.mkdir()
        work = owned / label
        work.mkdir()
        output = work / "styles.heic"
        row = {"ordinal": ordinal, "arm": name, "warmup": warmup,
               "iteration": iteration, "logs": str(logs.relative_to(directory)), "status": "running"}
        report["samples"].append(row)
        write_report(directory, report)
        try:
            row.update(measure(arms[name], fixture, output, logs))
        except Exception as error:
            row.update(status="failed", error=str(error))
            write_report(directory, report)
            raise
        row["status"] = "converted"
        print(json.dumps({"sample": {key: row[key] for key in (
            "ordinal", "arm", "warmup", "iteration", "wall_seconds", "time_real_seconds",
            "user_seconds", "sys_seconds", "peak_rss_bytes")}}, sort_keys=True), flush=True)
        outputs.append((arms[name], output, logs, row))
        write_report(directory, report)
    # These are real native validations, deliberately outside all timed samples.
    for arm, output, logs, row in outputs:
        env = conversion_environment(arm)
        try:
            run_logged([arm["cli"]["path"], "validate", str(output), "--apple-styles"],
                       ROOT, env, logs / "validate.log", 300)
            if digest(output) != row["output_sha256"]:
                raise RuntimeError("validation changed the measured output bytes")
        except Exception as error:
            row.update(status="validation_failed", error=str(error))
            write_report(directory, report)
            raise
        row["status"] = "validated"
        write_report(directory, report)


def profile_candidate(arm: dict, fixture: Path, owned: Path, directory: Path, report: dict) -> None:
    logs = directory / "candidate-profile"
    work = owned / "candidate-profile-output"
    logs.mkdir(); work.mkdir()
    output = work / "styles.heic"
    row = {"arm": "candidate", "status": "running", "logs": logs.name,
           "outside_budget_and_comparison_samples": True,
           "limits": "One candidate-only diagnostic conversion; no baseline stage comparison. "
                     "The two scene detail spans overlap scene_derivation; do not add them to stage totals."}
    report["candidate_profile"] = row
    write_report(directory, report)
    try:
        row.update(measure(arm, fixture, output, logs, profile=True))
        row["status"] = "converted"
        write_report(directory, report)
        run_logged([arm["cli"]["path"], "validate", str(output), "--apple-styles"],
                   ROOT, conversion_environment(arm), logs / "validate.log", 300)
        if digest(output) != row["output_sha256"]:
            raise RuntimeError("profile validation changed the measured output bytes")
    except Exception as error:
        row.update(status="validation_failed" if row["status"] == "converted" else "failed", error=str(error))
        stderr_log = logs / "convert.stderr.log"
        if stderr_log.is_file():
            stderr = stderr_log.read_text()
            try:
                row["partial_stages"] = profile_facts(stderr, require_complete=False)
            except RuntimeError as receipt_error:
                row["profile_receipt_error"] = str(receipt_error)
            try:
                row.update(time_facts(stderr))
            except RuntimeError:
                pass
        write_report(directory, report)
        print(json.dumps({"candidate_profile": row}, indent=2), flush=True)
        raise
    row["status"] = "validated"
    write_report(directory, report)


def summarize(report: dict) -> dict:
    summary = {}
    for name in ("baseline", "candidate"):
        rows = [r for r in report["samples"] if r["arm"] == name and not r["warmup"]]
        if len(rows) != ITERATIONS or any(r["status"] != "validated" for r in rows):
            raise RuntimeError(f"{name} comparison is missing validated samples")
        summary[name] = {
            "median_wall_seconds": statistics.median(r["wall_seconds"] for r in rows),
            "median_user_seconds": statistics.median(r["user_seconds"] for r in rows),
            "median_sys_seconds": statistics.median(r["sys_seconds"] for r in rows),
            "max_peak_rss_bytes": max(r["peak_rss_bytes"] for r in rows),
        }
    summary["candidate_over_baseline_median"] = (
        summary["candidate"]["median_wall_seconds"] / summary["baseline"]["median_wall_seconds"]
    )
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--apple-adapter", type=Path, required=True)
    parser.add_argument("--baseline-ref", required=True)
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    directory = args.output_dir.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    report = {"schema_version": 1, "diagnostic_only": True, "absolute_budget_unchanged": True,
              "status": "running", "warmup_per_arm": WARMUP, "iterations_per_arm": ITERATIONS,
              "baseline_ref": args.baseline_ref, "samples": [],
              "comparison_limits": [
                  "Cross-revision costs include all real producer and validation differences.",
                  "A main/candidate ratio does not establish the cause of a f9/ba2 cross-run change.",
                  "Independent validation is outside timing and uses each revision's own CLI/adapter.",
                  "The additional candidate profile is outside both the budget and interleaved samples.",
              ]}
    try:
        if platform.system() != "Darwin":
            raise RuntimeError("requires macOS and the real Apple adapter")
        candidate_sha = commit(ROOT, "HEAD")
        baseline_sha = commit(ROOT, args.baseline_ref)
        report.update(candidate_sha=candidate_sha, baseline_sha=baseline_sha,
                      distinct_revisions=baseline_sha != candidate_sha,
                      candidate_tracked_status=git(ROOT, "status", "--porcelain=v1", "--untracked-files=no"))
        if candidate_sha != args.expected_head or report["candidate_tracked_status"]:
            raise RuntimeError("candidate HEAD differs from expected SHA or tracked files are dirty")
        if not report["distinct_revisions"]:
            report["comparison_limits"].append("Both arms are the same revision; no code-change attribution is possible.")
        report["fixture"] = str(FIXTURE)
        report["fixture_sha256"] = checked_fixture(ROOT)
        facts = {}
        for command in (["sw_vers"], ["uname", "-m"], ["xcodebuild", "-version"],
                        ["swift", "--version"], ["cargo", "--version"], ["rustc", "--version"],
                        ["pkg-config", "--modversion", "libheif"],
                        ["sysctl", "hw.model", "machdep.cpu.brand_string", "hw.physicalcpu", "hw.logicalcpu", "hw.memsize"]):
            facts[" ".join(command)] = subprocess.check_output(command, text=True).strip()
        report["toolchain_and_host"] = facts
        write_report(directory, report)
        candidate = {"sha": candidate_sha, "tracked_status": report["candidate_tracked_status"],
                     "cli": binary(args.cli), "adapter": binary(args.apple_adapter),
                     "fixture_sha256": report["fixture_sha256"],
                     "build_commands": [CARGO_RELEASE_COMMAND,
                                        ["swift", "build", "-c", "release", "--product", "xdremux-apple-adapter"]],
                     "build_owner": "performance.yml existing candidate release build",
                     "feature_graph": feature_graph(ROOT, os.environ.copy(), directory, "candidate")}
        report["arms"] = {"candidate": candidate}
        write_report(directory, report)
        with tempfile.TemporaryDirectory(prefix="xdremux-styles-compare-") as raw:
            owned = Path(raw)
            with reference_build(ROOT, baseline_sha, owned, directory, report) as baseline:
                report["arms"] = {"baseline": baseline, "candidate": candidate}
                if baseline["sha"] != baseline_sha or baseline["tracked_status"]:
                    raise RuntimeError("reference build changed its exact revision")
                if baseline["fixture_sha256"] != report["fixture_sha256"]:
                    raise RuntimeError("reference and candidate fixture identities differ")
                print(json.dumps({"arms": report["arms"]}, indent=2), flush=True)
                collect(report["arms"], ROOT / FIXTURE, owned, directory, report)
                report["summary"] = summarize(report)
                profile_candidate(candidate, ROOT / FIXTURE, owned, directory, report)
        report["status"] = "completed"
        print(json.dumps({"distinct_revisions": report["distinct_revisions"],
                          "diagnostic_only": True, "summary": report["summary"],
                          "candidate_profile": report["candidate_profile"]}, indent=2))
        return 0
    except Exception as error:
        report["status"] = "failed"
        report["error"] = str(error)
        print(f"Styles comparison failed: {error}", file=sys.stderr)
        return 1
    finally:
        write_report(directory, report)


if __name__ == "__main__":
    raise SystemExit(main())
