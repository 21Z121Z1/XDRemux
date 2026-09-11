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
    echo "missing metadata conformance fixture: $fixture" >&2
    exit 1
  fi
done

cargo build --locked -p xdremux-metadata --bins

rust_vectors="$repo_root/target/debug/xdremux-metadata-vectors"
rust_fixture="$repo_root/target/debug/xdremux-metadata-fixture"

for binary in "$rust_vectors" "$rust_fixture"; do
  if [[ ! -x "$binary" ]]; then
    echo "metadata conformance binary was not built: $binary" >&2
    exit 1
  fi
done

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/xdremux-metadata-conformance.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

rust_vectors_output="$work_dir/rust-vectors.txt"
"$rust_vectors" > "$rust_vectors_output"

routing_count="$(grep -c $'^routing\t' "$rust_vectors_output")"
metadata_count="$(grep -c $'^metadata\t' "$rust_vectors_output")"
comment_count="$(grep -c $'^comment\t' "$rust_vectors_output")"
patch_count="$(grep -c $'^patch\t' "$rust_vectors_output")"
extent_count="$(grep -c $'^extent\t' "$rust_vectors_output")"
if [[ "$routing_count" -ne 14 || "$metadata_count" -ne 10 || "$comment_count" -ne 7 || "$patch_count" -ne 1 || "$extent_count" -ne 4 ]]; then
  echo "metadata vector coverage changed unexpectedly: routing=$routing_count metadata=$metadata_count comment=$comment_count patch=$patch_count extent=$extent_count" >&2
  exit 1
fi

echo "PASS Rust metadata vectors: routing=$routing_count metadata=$metadata_count comment=$comment_count patch=$patch_count extent=$extent_count"

for fixture in "${fixtures[@]}"; do
  name="$(basename "$fixture")"
  rust_output="$work_dir/$name.rust.txt"
  "$rust_fixture" "$fixture" > "$rust_output"
  echo "PASS metadata fixture conformance: $fixture"
done
