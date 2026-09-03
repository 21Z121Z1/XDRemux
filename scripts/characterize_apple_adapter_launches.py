#!/usr/bin/env python3
"""Count Apple adapter process launches without contaminating timed benchmarks.

The canonical benchmark measures the real adapter directly. This script runs one
additional, untimed Portrait and Styles conversion through a tiny exec wrapper.
Every invocation appends one record before replacing itself with the real Swift
helper, so the result characterizes transport lifecycle rather than framework
or Python timing.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"


def git_head() -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()


def write_wrapper(path: Path) -> None:
    path.write_text(
        "#!/bin/sh\n"
        "set -eu\n"
        ': "${XDREMUX_REAL_APPLE_ADAPTER:?}"\n'
        ': "${XDREMUX_ADAPTER_LAUNCH_LOG:?}"\n'
        "printf 'launch\\n' >> \"$XDREMUX_ADAPTER_LAUNCH_LOG\"\n"
        "exec \"$XDREMUX_REAL_APPLE_ADAPTER\"\n",
        encoding="utf-8",
    )
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def count_launches(cli: Path, adapter: Path, flag: str) -> int:
    with tempfile.TemporaryDirectory(prefix="xdremux-adapter-launches-") as raw:
        work = Path(raw)
        wrapper = work / "adapter-wrapper"
        log = work / "launches.log"
        output = work / "output.heic"
        write_wrapper(wrapper)

        env = os.environ.copy()
        env["XDREMUX_APPLE_ADAPTER"] = str(wrapper)
        env["XDREMUX_REAL_APPLE_ADAPTER"] = str(adapter)
        env["XDREMUX_ADAPTER_LAUNCH_LOG"] = str(log)
        subprocess.run(
            [
                str(cli),
                "convert",
                "--input",
                str(FIXTURE),
                "--output",
                str(output),
                flag,
            ],
            cwd=ROOT,
            env=env,
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        )
        if not output.is_file() or output.stat().st_size == 0:
            raise RuntimeError(f"{flag} did not produce a non-empty HEIC")
        if not log.is_file():
            raise RuntimeError(f"{flag} did not launch the Apple adapter")
        return sum(1 for line in log.read_text(encoding="utf-8").splitlines() if line == "launch")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--apple-adapter", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    cli = args.cli.resolve()
    adapter = args.apple_adapter.resolve()
    if not cli.is_file():
        parser.error(f"--cli is not a file: {cli}")
    if not adapter.is_file():
        parser.error(f"--apple-adapter is not a file: {adapter}")
    if not FIXTURE.is_file():
        raise RuntimeError(f"required fixture is missing: {FIXTURE}")

    cases = {
        "apple-portrait": count_launches(cli, adapter, "--apple-portrait"),
        "apple-styles": count_launches(cli, adapter, "--apple-styles"),
    }
    report = {
        "schema_version": 1,
        "head": git_head(),
        "metric": "apple_adapter_process_launches_per_conversion",
        "cases": cases,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for name, count in cases.items():
        print(f"{name}: {count} Apple adapter launches")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
