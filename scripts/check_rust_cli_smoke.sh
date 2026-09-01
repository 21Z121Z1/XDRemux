#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMPDIR_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
WORKDIR="$(mktemp -d "$TMPDIR_ROOT/xdremux-rust-cli.XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT

FIXTURE="$WORKDIR/motion.jpg"
REPORT="$WORKDIR/report.json"
CATEGORIZE_INPUT="$WORKDIR/portrait.heic"
CATEGORIZE_OUTPUT="$WORKDIR/categorized"
CATEGORIZE_REPORT="$WORKDIR/categorize.json"

python3 - "$FIXTURE" <<'PY'
from pathlib import Path
import struct
import sys

out = Path(sys.argv[1])

def box(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I4s", len(payload) + 8, kind) + payload

video = box(b"ftyp", b"isom\x00\x00\x02\x00") + box(b"mdat", b"")
xmp = f'''<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1" Camera:MotionPhotoPresentationTimestampUs="1417000"><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{len(video)}" Item:Padding="0"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>'''.encode()
out.write_bytes(b"\xff\xd8" + xmp + b"\xff\xd9" + video)
PY

cargo run --locked --quiet -p xdremux-cli -- inspect "$FIXTURE" --json > "$REPORT"

python3 - "$REPORT" <<'PY'
import json
from pathlib import Path
import sys

report = json.loads(Path(sys.argv[1]).read_text())
assert report["schema_version"] == 1, report
assert report["kind"] == "motion-photo", report
assert report["source_kind"] == "androidMotionPhotoV1", report
assert report["presentation_timestamp_us"] == 1417000, report
assert report["video"]["length"] > 0, report
assert report["stream_count"] == 1, report
PY

printf 'synthetic metadata Oplus_16 payload' > "$CATEGORIZE_INPUT"
cargo run --locked --quiet -p xdremux-cli -- categorize \
  --input "$CATEGORIZE_INPUT" \
  --output-dir "$CATEGORIZE_OUTPUT" \
  --dry-run \
  --json > "$CATEGORIZE_REPORT"

python3 - "$CATEGORIZE_REPORT" "$CATEGORIZE_OUTPUT" <<'PY'
import json
from pathlib import Path
import sys

report = json.loads(Path(sys.argv[1]).read_text())
output = Path(sys.argv[2])
assert report["schema_version"] == 1, report
assert report["command"] == "categorize", report
assert report["processed"] == 1, report
assert report["dry_run"] == 1, report
assert report["failed"] == 0, report
assert report["items"][0]["classification"]["primary_capture_mode"] == "portrait", report
assert not output.exists(), output
PY

cargo run --locked --quiet -p xdremux-cli -- --help | grep -F "canonical Rust runtime" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- inspect --help | grep -F "Inspect one input" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- convert --help | grep -F -- "--input <INPUT>" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- convert --help | grep -F "Convert one supported source" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- batch --help | grep -F "Convert a deterministic batch" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- batch --help | grep -F -- "--input-dir <DIR>" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- batch --help | grep -F -- "--recursive" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- categorize --help | grep -F "Classify photo assets" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- categorize --help | grep -F -- "--output-dir <DIR>" >/dev/null
cargo run --locked --quiet -p xdremux-cli -- categorize --help | grep -F -- "--dry-run" >/dev/null
