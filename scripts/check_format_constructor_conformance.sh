#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build --locked -p xdremux-format --bin xdremux-format-vectors
swift build --target FormatConformanceOracle

rust_vectors="$repo_root/target/debug/xdremux-format-vectors"
swift_bin_dir="$(swift build --show-bin-path)"
swift_oracle="$swift_bin_dir/FormatConformanceOracle"

if [[ ! -x "$rust_vectors" ]]; then
  echo "Rust constructor vector tool was not built at $rust_vectors" >&2
  exit 1
fi
if [[ ! -x "$swift_oracle" ]]; then
  echo "Swift conformance oracle was not built at $swift_oracle" >&2
  exit 1
fi

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-format-constructors.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

rust_output="$work_dir/rust.txt"
swift_output="$work_dir/swift.txt"

"$rust_vectors" > "$rust_output"
"$swift_oracle" --constructor-vectors > "$swift_output"

expected_vectors=(
  pitm-v0
  pitm-v1
  infe-v2
  iinf-v0
  iinf-v1
  iloc-v1-44
  ipma-v0-narrow
  ipma-v1-wide
  iref-v0
  iref-v1
  ispe
  irot
)

for name in "${expected_vectors[@]}"; do
  if ! grep -q $'^vector\t'"$name"$'\t' "$rust_output"; then
    echo "Rust constructor vector is missing: $name" >&2
    exit 1
  fi
  if ! grep -q $'^vector\t'"$name"$'\t' "$swift_output"; then
    echo "Swift constructor vector is missing: $name" >&2
    exit 1
  fi
done

if ! diff -u "$swift_output" "$rust_output"; then
  echo "Swift/Rust constructor byte conformance failed" >&2
  exit 1
fi

echo "PASS Swift/Rust constructor byte conformance: ${#expected_vectors[@]} vectors"
