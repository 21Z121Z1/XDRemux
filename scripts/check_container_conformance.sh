#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-container-conformance.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

SWIFT_ROOT="$TMP_ROOT/swift"
RUST_ROOT="$TMP_ROOT/rust"

cargo build --locked -q -p xdremux-container --bin xdremux-container-extract
RUST_ORACLE="$ROOT/target/debug/xdremux-container-extract"
if [[ ! -x "$RUST_ORACLE" ]]; then
  echo "Rust container oracle was not built at $RUST_ORACLE" >&2
  exit 1
fi

XDREMUX_CONTAINER_ORACLE_ROOT="$SWIFT_ROOT" \
  swift test --filter ContainerRustConformanceOracleTests/testEmitRepositoryFixtureSnapshots

ACCEPTED="$SWIFT_ROOT/accepted.tsv"
REJECTED="$SWIFT_ROOT/rejected.tsv"
if [[ ! -f "$ACCEPTED" ]]; then
  echo "Swift container oracle did not emit accepted.tsv" >&2
  [[ -f "$REJECTED" ]] && cat "$REJECTED" >&2
  exit 1
fi

matched=0
lhdr=0
uhdr=0

while IFS=$'\t' read -r snapshot_name relative mode; do
  [[ -n "$snapshot_name" ]] || continue
  swift_dir="$SWIFT_ROOT/$snapshot_name"
  rust_dir="$RUST_ROOT/$snapshot_name"
  rust_err="$TMP_ROOT/$snapshot_name.rust.err"
  mkdir -p "$RUST_ROOT"

  if ! "$RUST_ORACLE" "$relative" "$rust_dir" 2>"$rust_err"; then
    echo "Rust failed a fixture accepted by Swift: $relative" >&2
    cat "$rust_err" >&2
    exit 1
  fi

  if ! diff -rq "$swift_dir" "$rust_dir"; then
    echo "Swift/Rust container snapshot mismatch: $relative" >&2
    echo "--- Swift summary ---" >&2
    cat "$swift_dir/summary.tsv" >&2
    echo "--- Rust summary ---" >&2
    cat "$rust_dir/summary.tsv" >&2
    exit 1
  fi

  case "$mode" in
    lhdr) lhdr=$((lhdr + 1)) ;;
    uhdr) uhdr=$((uhdr + 1)) ;;
    *)
      echo "Unexpected Swift extraction mode '$mode' for $relative" >&2
      exit 1
      ;;
  esac
  matched=$((matched + 1))
  echo "PASS container fixture: $relative ($mode)"
done < "$ACCEPTED"

if (( matched < 2 )); then
  echo "Container conformance needs at least two Swift-accepted repository fixtures; matched=$matched" >&2
  [[ -f "$REJECTED" ]] && cat "$REJECTED" >&2
  exit 1
fi

rejected=0
if [[ -f "$REJECTED" ]]; then
  rejected="$(awk 'NF { count += 1 } END { print count + 0 }' "$REJECTED")"
fi

echo "PASS Swift/Rust container fixtures: matched=$matched lhdr=$lhdr uhdr=$uhdr rejected=$rejected"
