#!/usr/bin/env python3
"""Compatibility entry point for the Python CLI.

The implementation moved to the ``xdremux_py`` package at the repository root.
This forwards ``convert``, ``batch``, and ``categorize`` unchanged, so existing
automation calling ``python3 xdremux/python/XDRemux.py`` keeps working. New
callers should use the installed ``xdremux-py`` command or
``python3 -m xdremux_py``.

This mirrors ``xdremux/swift-cli/XDRemux.swift``, which forwards the legacy
``swift <file>`` entry point to the root package executable.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from xdremux_py.cli import main  # noqa: E402

if __name__ == "__main__":
    sys.exit(main())
