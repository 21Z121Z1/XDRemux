#!/usr/bin/env bash
# Convert every Find X9 Ultra fixture to Standard Photographic Styles and
# render the old and new Key 1 polynomials on the linear primary.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

package="${XDREMUX_MACOS_PACKAGE:-$RUNNER_TEMP/staging/XDRemux-macOS-arm64}"
cli="$package/bin/xdremux"
adapter="$package/libexec/xdremux-apple-adapter"
prefix="${XDREMUX_NATIVE_PREFIX:-$RUNNER_TEMP/xdremux-native/prefix}"
out="${XDREMUX_KEY1_COLOR_DIR:-$RUNNER_TEMP/key1-color}"

test -x "$cli"
test -x "$adapter"
test -d "$prefix/lib"

export PKG_CONFIG_PATH="$prefix/lib/pkgconfig"
export PKG_CONFIG_LIBDIR="$PKG_CONFIG_PATH"
export DYLD_LIBRARY_PATH="$prefix/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
export XDREMUX_APPLE_ADAPTER="$adapter"
export XDREMUX_KEY1_COLOR_DIR="$out"
export XDREMUX_KEY1_STYLES_DIR="$out/heic"

mkdir -p "$XDREMUX_KEY1_STYLES_DIR"
shopt -s nullglob
fixtures=(fixtures/proxdr/oppo/find-x9-ultra/*.heic)
test "${#fixtures[@]}" -eq 7

for fixture in "${fixtures[@]}"; do
  stem="$(basename "$fixture" .heic)"
  output="$XDREMUX_KEY1_STYLES_DIR/${stem}-standard-styles.heic"
  echo "convert $fixture -> $output"
  "$cli" --lang en convert --input "$fixture" --output "$output" --apple-styles
  test -s "$output"
done

cargo test --locked --release --target aarch64-apple-darwin -p xdremux-runtime --lib \
  key1_color_evidence_renders_proxdr_fixtures -- --nocapture

python3 - "$out/identity-key1.bin" "$XDREMUX_KEY1_STYLES_DIR" <<'PY'
import sys
from pathlib import Path

identity = Path(sys.argv[1]).read_bytes()
if len(identity) < 64:
    raise SystemExit("identity Key 1 payload is too short")
styles = sorted(Path(sys.argv[2]).glob("*-standard-styles.heic"))
if len(styles) != 7:
    raise SystemExit(f"expected 7 Styles HEIC files, found {len(styles)}")
# The payload begins with zeros, so search for a later window that includes
# the identity diagonal instead of scanning every full-length alignment.
signature = identity[6:38]
for path in styles:
    data = path.read_bytes()
    start = 0
    found = False
    while True:
        at = data.find(signature, start)
        if at < 0:
            break
        origin = at - 6
        if origin >= 0 and data[origin:origin + len(identity)] == identity:
            found = True
            break
        start = at + 1
    if not found:
        raise SystemExit(f"{path.name} does not contain the identity Key 1 payload")
    print(f"identity-key1 {path.name}")
PY
