#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-container-conformance.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

swift build --target ContainerConformanceOracle >/dev/null
SWIFT_BIN_DIR="$(swift build --show-bin-path)"
SWIFT_ORACLE="$SWIFT_BIN_DIR/ContainerConformanceOracle"

cargo build --locked -q -p xdremux-container --bin xdremux-container-extract
RUST_ORACLE="$ROOT/target/debug/xdremux-container-extract"

if [[ ! -x "$SWIFT_ORACLE" ]]; then
  echo "Swift container oracle was not built at $SWIFT_ORACLE" >&2
  exit 1
fi
if [[ ! -x "$RUST_ORACLE" ]]; then
  echo "Rust container oracle was not built at $RUST_ORACLE" >&2
  exit 1
fi

matched=0
skipped=0
lhdr=0
uhdr=0

while IFS= read -r fixture; do
  [[ -n "$fixture" ]] || continue
  relative="${fixture#./}"
  key="$(printf '%s' "$relative" | shasum -a 256 | awk '{print $1}')"
  swift_dir="$TMP_ROOT/$key/swift"
  rust_dir="$TMP_ROOT/$key/rust"
  swift_err="$TMP_ROOT/$key/swift.err"
  rust_err="$TMP_ROOT/$key/rust.err"
  mkdir -p "$TMP_ROOT/$key"

  if ! "$SWIFT_ORACLE" "$relative" "$swift_dir" 2>"$swift_err"; then
    skipped=$((skipped + 1))
    continue
  fi

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

  mode="$(awk -F '\t' '$1 == "mode" { print $2; exit }' "$swift_dir/summary.tsv")"
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
done < <(
  find fixtures -maxdepth 1 -type f \
    \( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.heic' -o -iname '*.heif' \) \
    -print | LC_ALL=C sort
)

if (( matched < 2 )); then
  echo "Container conformance needs at least two Swift-accepted repository fixtures; matched=$matched skipped=$skipped" >&2
  exit 1
fi

echo "PASS Swift/Rust container fixtures: matched=$matched lhdr=$lhdr uhdr=$uhdr skipped=$skipped"
