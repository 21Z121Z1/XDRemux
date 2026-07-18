#!/usr/bin/env python3
"""Verify that an interactive XDRemux process restores the cursor on SIGINT."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import pty
import select
import signal
import subprocess
import sys
import time


HIDE_CURSOR = b"\x1b[?25l"
SHOW_CURSOR = b"\x1b[?25h"


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--input", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    binary = arguments.binary.expanduser().resolve()
    input_path = arguments.input.expanduser().resolve()
    if not binary.is_file() or not input_path.is_file():
        print("binary and input must both exist", file=sys.stderr)
        return 2

    master, slave = pty.openpty()
    process = subprocess.Popen(
        [
            str(binary),
            "convert",
            "--input",
            str(input_path),
            "--output",
            f"/tmp/xdremux-sigint-{os.getpid()}.heic",
            "--language",
            "en",
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=slave,
        close_fds=True,
    )
    os.close(slave)
    output = bytearray()
    deadline = time.monotonic() + 10
    interrupted = False
    try:
        while time.monotonic() < deadline:
            readable, _, _ = select.select([master], [], [], 0.1)
            if readable:
                try:
                    chunk = os.read(master, 65_536)
                except OSError:
                    break
                if not chunk:
                    break
                output.extend(chunk)
            if not interrupted and HIDE_CURSOR in output:
                process.send_signal(signal.SIGINT)
                interrupted = True
            if process.poll() is not None:
                break
        if not interrupted:
            process.kill()
            raise RuntimeError("interactive process never hid the cursor")
        process.wait(timeout=5)
        while True:
            readable, _, _ = select.select([master], [], [], 0)
            if not readable:
                break
            try:
                chunk = os.read(master, 65_536)
            except OSError:
                break
            if not chunk:
                break
            output.extend(chunk)
    finally:
        os.close(master)
        if process.poll() is None:
            process.kill()
            process.wait()

    if SHOW_CURSOR not in output or output.rfind(SHOW_CURSOR) < output.find(HIDE_CURSOR):
        raise RuntimeError("SIGINT output did not restore the terminal cursor")
    if process.returncode not in (-signal.SIGINT, 128 + signal.SIGINT):
        raise RuntimeError(f"unexpected SIGINT return code: {process.returncode}")
    print("SIGINT restored the interactive terminal cursor")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
