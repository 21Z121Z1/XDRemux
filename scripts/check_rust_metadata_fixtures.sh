#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fixtures=(
  "fixtures/20260312_135609..heic"
  "fixtures/20260312_135610..heic"
)

cargo build --locked -p xdremux-metadata --bin xdremux-metadata-fixture
inspector="$repo_root/target/debug/xdremux-metadata-fixture"

for fixture in "${fixtures[@]}"; do
  if [[ ! -f "$fixture" ]]; then
    echo "missing metadata fixture: $fixture" >&2
    exit 1
  fi
  output="$($inspector "$fixture")"
  line_count="$(printf '%s\n' "$output" | grep -c $'^fixture\t')"
  if [[ "$line_count" -ne 7 ]]; then
    echo "metadata fixture inspector produced $line_count mode rows for $fixture" >&2
    exit 1
  fi
  echo "PASS Rust metadata fixture parse: $fixture"
done
