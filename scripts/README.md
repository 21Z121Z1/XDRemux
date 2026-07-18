# Scripts

This directory contains checked-in helper scripts for local development, build, and verification workflows.

## Available scripts

- `build_and_run.sh` manages the macOS `XDRemuxApp` development build. Use `build`, `run`, `debug`, `verify`, `logs`, `logs --all`, or `clean`; add `--verbose` to stream the complete `xcodebuild` log.
- `agent_completion_gate.py` executes a committed change's declared checks and writes a HEAD-bound acceptance receipt. Production code changes cannot pass without regression plus functional/integration/device evidence.

Keep reusable automation here. One-off scratch scripts should stay outside the repository or under an explicitly named experiment directory.
