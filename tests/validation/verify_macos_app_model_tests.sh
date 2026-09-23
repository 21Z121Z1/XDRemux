#!/usr/bin/env bash
#
# Builds and runs the macOS app's model test bundle, the same way ci.yml does.
# Covers XDRemuxViewModel behavior that the SwiftPM test suite cannot reach.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DERIVED_DATA="${XDREMUX_APP_MODEL_DERIVED_DATA:-$(mktemp -d "${TMPDIR:-/tmp}/xdremux-app-model-XXXXXX")}"

cd "$ROOT_DIR"

xcodebuild \
  -quiet \
  -project apps/macos/XDRemuxApp/XDRemuxApp.xcodeproj \
  -scheme XDRemuxAppModelTests \
  -configuration Debug \
  -derivedDataPath "$DERIVED_DATA" \
  CODE_SIGNING_ALLOWED=NO \
  build

"$DERIVED_DATA/Build/Products/Debug/XDRemuxAppModelTests"
