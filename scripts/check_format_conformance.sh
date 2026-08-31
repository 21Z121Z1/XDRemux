#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fixtures=(
  "fixtures/20260312_135609..heic"
  "fixtures/20260312_135610..heic"
)

for fixture in "${fixtures[@]}"; do
  if [[ ! -f "$fixture" ]]; then
    echo "missing conformance fixture: $fixture" >&2
    exit 1
  fi
done

cargo build --locked -p xdremux-format --bin xdremux-format-inspect
swift build --target FormatConformanceOracle

rust_inspector="$repo_root/target/debug/xdremux-format-inspect"
swift_bin_dir="$(swift build --show-bin-path)"
swift_oracle="$swift_bin_dir/FormatConformanceOracle"

if [[ ! -x "$rust_inspector" ]]; then
  echo "Rust conformance inspector was not built at $rust_inspector" >&2
  exit 1
fi
if [[ ! -x "$swift_oracle" ]]; then
  echo "Swift conformance oracle was not built at $swift_oracle" >&2
  exit 1
fi

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-format-conformance.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

for fixture in "${fixtures[@]}"; do
  name="$(basename "$fixture")"
  rust_output="$work_dir/$name.rust.txt"
  swift_output="$work_dir/$name.swift.txt"

  "$rust_inspector" "$fixture" > "$rust_output"
  "$swift_oracle" "$fixture" > "$swift_output"

  if ! diff -u "$swift_output" "$rust_output"; then
    echo "format conformance failed for $fixture" >&2
    exit 1
  fi
  echo "PASS format conformance: $fixture"
done
