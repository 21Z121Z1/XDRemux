#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fixtures=(
  "fixtures/motion-photo/samsung/heif-ultrahdr-01.heic"
  "fixtures/motion-photo/samsung/heif-ultrahdr-02.heic"
)

cargo build --locked -p xdremux-format --bin xdremux-format-inspect
inspector="$repo_root/target/debug/xdremux-format-inspect"

if [[ ! -x "$inspector" ]]; then
  echo "Rust format inspector was not built at $inspector" >&2
  exit 1
fi

for fixture in "${fixtures[@]}"; do
  if [[ ! -f "$fixture" ]]; then
    echo "missing Rust format fixture: $fixture" >&2
    exit 1
  fi

  summary="$($inspector "$fixture")"
  for required in $'box\t66747970' $'box\t6d657461' $'box\t6d646174' $'primary\t' $'iloc\t' $'iinf\t' $'ipma\t' $'property\t'; do
    if [[ "$summary" != *"$required"* ]]; then
      echo "Rust format fixture $fixture did not emit required semantic record: $required" >&2
      exit 1
    fi
  done
  echo "PASS Rust format real fixture: $fixture"
done
