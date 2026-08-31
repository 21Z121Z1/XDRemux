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
  swift test --filter ContainerRustConformanceOracleTests/testEmitContainerConformanceCorpus

CASES="$SWIFT_ROOT/cases.tsv"
if [[ ! -f "$CASES" ]]; then
  echo "Swift container oracle did not emit cases.tsv" >&2
  exit 1
fi

matched=0
lhdr=0
uhdr=0

while IFS=$'\t' read -r snapshot_name input_path mode case_name; do
  [[ -n "$snapshot_name" ]] || continue
  swift_dir="$SWIFT_ROOT/$snapshot_name"
  rust_dir="$RUST_ROOT/$snapshot_name"
  rust_err="$TMP_ROOT/$snapshot_name.rust.err"
  mkdir -p "$RUST_ROOT"

  if ! "$RUST_ORACLE" "$input_path" "$rust_dir" 2>"$rust_err"; then
    echo "Rust failed a Swift conformance corpus case: $case_name" >&2
    cat "$rust_err" >&2
    exit 1
  fi

  if ! diff -rq "$swift_dir" "$rust_dir"; then
    echo "Swift/Rust container snapshot mismatch: $case_name" >&2
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
      echo "Unexpected Swift extraction mode '$mode' for $case_name" >&2
      exit 1
      ;;
  esac
  matched=$((matched + 1))
  echo "PASS container conformance: $case_name ($mode)"
done < "$CASES"

if (( matched != 4 || lhdr != 2 || uhdr != 2 )); then
  echo "Container corpus coverage changed unexpectedly: matched=$matched lhdr=$lhdr uhdr=$uhdr" >&2
  exit 1
fi

echo "PASS Swift/Rust container corpus: matched=$matched lhdr=$lhdr uhdr=$uhdr"
