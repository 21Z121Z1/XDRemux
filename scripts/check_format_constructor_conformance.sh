#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build --locked -p xdremux-format --bin xdremux-format-vectors

rust_vectors="$repo_root/target/debug/xdremux-format-vectors"

if [[ ! -x "$rust_vectors" ]]; then
  echo "Rust constructor vector tool was not built at $rust_vectors" >&2
  exit 1
fi

rust_output="$($rust_vectors)"

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
  if ! grep -q $'^vector\t'"$name"$'\t' <<<"$rust_output"; then
    echo "Rust constructor vector is missing: $name" >&2
    exit 1
  fi
done

echo "PASS Rust constructor byte conformance: ${#expected_vectors[@]} vectors"
