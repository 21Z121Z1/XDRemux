#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CASES="Tests/fixtures/conversion_plan_cases.json"
RUST_OUTPUT="$(mktemp)"
SWIFT_OUTPUT="$(mktemp)"
trap 'rm -f "$RUST_OUTPUT" "$SWIFT_OUTPUT"' EXIT

cargo run --quiet --locked -p xdremux-engine --bin xdremux-engine-plan-oracle -- "$CASES" >"$RUST_OUTPUT"
swift run -c release EnginePlanOracle "$CASES" >"$SWIFT_OUTPUT"

python3 - "$SWIFT_OUTPUT" "$RUST_OUTPUT" <<'PY'
from __future__ import annotations

import difflib
import json
import pathlib
import sys

swift_path = pathlib.Path(sys.argv[1])
rust_path = pathlib.Path(sys.argv[2])
swift_plan = json.loads(swift_path.read_text(encoding="utf-8"))
rust_plan = json.loads(rust_path.read_text(encoding="utf-8"))

if swift_plan != rust_plan:
    swift_text = json.dumps(swift_plan, ensure_ascii=False, indent=2, sort_keys=True).splitlines()
    rust_text = json.dumps(rust_plan, ensure_ascii=False, indent=2, sort_keys=True).splitlines()
    print("Swift/Rust conversion plan mismatch", file=sys.stderr)
    print(
        "\n".join(
            difflib.unified_diff(
                swift_text,
                rust_text,
                fromfile="swift-plan",
                tofile="rust-plan",
                lineterm="",
            )
        ),
        file=sys.stderr,
    )
    raise SystemExit(1)

print(json.dumps(rust_plan, ensure_ascii=False, sort_keys=True))
PY

echo 'PASS Swift/Rust conversion plan conformance'
