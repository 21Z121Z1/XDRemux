# Scripts

This directory contains checked-in helper scripts for local development, build, and verification workflows.

## Available scripts

- `build_and_run.sh` builds and launches the macOS `XDRemuxApp` Xcode project from `apps/macos/XDRemuxApp`. It supports `run`, `--debug`, `--logs`, `--telemetry`, and `--verify`.
- `agent_completion_gate.py` executes a committed change's declared checks and writes a HEAD-bound acceptance receipt. Production code changes cannot pass without regression plus functional/integration/device evidence.

Keep reusable automation here. One-off scratch scripts should stay outside the repository or under an explicitly named experiment directory.
