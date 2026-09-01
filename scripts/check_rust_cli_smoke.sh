#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMPDIR_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
WORKDIR="$(mktemp -d "$TMPDIR_ROOT/xdremux-rust-cli.XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT

FIXTURE="$WORKDIR/motion.jpg"
REPORT="$WORKDIR/report.json"

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

cargo run --locked --quiet -p xdremux-cli -- --help | grep -F "inspect <INPUT> [--json]" >/dev/null
