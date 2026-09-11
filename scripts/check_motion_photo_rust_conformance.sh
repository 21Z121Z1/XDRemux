#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --locked -q -p xdremux-motion-photo \
  --example motion_photo_conformance \
  --example payload_conformance
cargo test --locked -p xdremux-motion-photo

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-motion-photo-conformance.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

SOURCE="$ROOT/fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"
OPPO_SOURCE="$ROOT/fixtures/motion-photo/oppo/coloros15-ultrahdr-01.jpg"
test -s "$SOURCE"
test -s "$OPPO_SOURCE"

cargo run --locked --quiet -p xdremux-motion-photo \
  --example motion_photo_conformance -- sources \
  >"$TMP_ROOT/sources.json"
cargo run --locked --quiet -p xdremux-motion-photo \
  --example motion_photo_conformance -- android "$SOURCE" \
  >"$TMP_ROOT/android.json"
cargo run --locked --quiet -p xdremux-motion-photo \
  --example motion_photo_conformance -- oppo "$OPPO_SOURCE" \
  >"$TMP_ROOT/oppo.json"
cargo run --locked --quiet -p xdremux-motion-photo \
  --example motion_photo_conformance -- lpex "$OPPO_SOURCE" \
  >"$TMP_ROOT/lpex.json"

PAYLOAD="$TMP_ROOT/payload.bin"
cargo run --locked --quiet -p xdremux-motion-photo \
  --example payload_conformance -- "$SOURCE" 0 128 "$PAYLOAD" 1024 32 \
  >"$TMP_ROOT/payload.json"

python3 - "$TMP_ROOT" <<'PY'
import json
from pathlib import Path
import sys

root = Path(sys.argv[1])
sources = json.loads((root / "sources.json").read_text())
assert sources["presentationSources"] == [
    "androidXMP",
    "legacyMicroVideoXMP",
    "oppoCoverFrame",
    "timelineFallback",
], sources

android = json.loads((root / "android.json").read_text())
assert android["status"] == "asset", android
assert android["sourceKind"] == "androidMotionPhotoV1", android
assert android["presentationSource"] == "androidXMP", android
assert android["video"]["upper"] > android["video"]["lower"], android

oppo = json.loads((root / "oppo.json").read_text())
assert oppo["status"] == "asset", oppo
assert oppo["sourceKind"] == "oppoLivePhoto", oppo
assert oppo["vendorMetadata"]["matrixCount"] > 0, oppo

lpex = json.loads((root / "lpex.json").read_text())
assert lpex["matrixCount"] > 0, lpex

payload = json.loads((root / "payload.json").read_text())
assert payload == {"status": "ok"}, payload
assert (root / "payload.bin").stat().st_size == 128
PY

echo "PASS Rust Motion Photo parser, payload, and presentation contracts"
