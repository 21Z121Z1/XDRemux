#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fixtures=(
  "fixtures/motion-photo/samsung/heif-ultrahdr-01.heic"
  "fixtures/motion-photo/samsung/heif-ultrahdr-02.heic"
)

for fixture in "${fixtures[@]}"; do
  if [[ ! -f "$fixture" ]]; then
    echo "missing conformance fixture: $fixture" >&2
    exit 1
  fi
done

cargo build --locked -p xdremux-format --bin xdremux-format-inspect

rust_inspector="$repo_root/target/debug/xdremux-format-inspect"

if [[ ! -x "$rust_inspector" ]]; then
  echo "Rust conformance inspector was not built at $rust_inspector" >&2
  exit 1
fi

for fixture in "${fixtures[@]}"; do
  name="$(basename "$fixture")"
  output="$($rust_inspector "$fixture")"
  for required in $'box\t66747970' $'box\t6d657461' $'box\t6d646174' $'primary\t' $'iloc\t' $'iinf\t' $'ipma\t' $'property\t'; do
    if [[ "$output" != *"$required"* ]]; then
      echo "Rust format conformance missing $required for $fixture" >&2
      exit 1
    fi
  done
  if [[ -z "$name" ]]; then
    echo "empty fixture name" >&2
    exit 1
  fi
  echo "PASS format conformance: $fixture"
done
