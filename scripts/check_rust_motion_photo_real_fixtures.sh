#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_ROOT="${1:-$ROOT/fixtures}"
OUTPUT_ROOT="${2:-$(mktemp -d "${TMPDIR:-/tmp}/xdremux-rust-motion-photo-real.XXXXXX")}"
CLI="${XDREMUX_CLI:-$ROOT/target/debug/xdremux}"

cd "$ROOT"

if [[ ! -x "$CLI" ]]; then
  cargo build --locked -q -p xdremux-cli
fi
if [[ ! -x "$CLI" ]]; then
  echo "Rust CLI was not built at $CLI" >&2
  exit 1
fi

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

mkdir -p "$OUTPUT_ROOT"
manifest="$FIXTURE_ROOT/SHA256SUMS"
if [[ ! -f "$manifest" ]]; then
  echo "fixture identity manifest is missing: $manifest" >&2
  exit 1
fi

count=0
while read -r expected relative; do
  [[ -n "${relative:-}" ]] || continue
  [[ "$relative" == motion-photo/* ]] || continue
  source="$FIXTURE_ROOT/$relative"
  if [[ ! -f "$source" ]]; then
    echo "Motion Photo fixture is missing: $source" >&2
    exit 1
  fi
  actual="$(sha256 "$source")"
  if [[ "$actual" != "$expected" ]]; then
    echo "Motion Photo fixture identity mismatch: $relative ($actual != $expected)" >&2
    exit 1
  fi

  filename="${relative##*/}"
  stem="${filename%.*}"
  case_root="$OUTPUT_ROOT/$stem"
  mkdir -p "$case_root"
  still="$case_root/$stem.heic"
  movie="$case_root/$stem.mov"
  "$CLI" convert --input "$source" --output "$still" >"$case_root/convert.log" 2>&1
  [[ -s "$still" && -s "$movie" ]] || {
    echo "Rust CLI did not publish a complete Live Photo pair: $relative" >&2
    exit 1
  }
  "$CLI" validate "$still" >"$case_root/still-validation.txt"
  "$CLI" validate "$movie" >"$case_root/movie-validation.txt"
  grep -q '^valid: live-photo$' "$case_root/still-validation.txt"
  grep -q '^valid: live-photo$' "$case_root/movie-validation.txt"
  count=$((count + 1))
  echo "PASS Rust Motion Photo fixture: $relative"
done < "$manifest"

if ((count != 14)); then
  echo "expected 14 versioned Motion Photo fixtures, converted $count" >&2
  exit 1
fi

echo "PASS Rust Motion Photo real-fixture gate: fixtures=$count output=$OUTPUT_ROOT"
