#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC_DIR="$ROOT/scripts/research"
OUT_DIR="${XDREMUX_TEXTURE_PROBE_OUT:-/tmp/xdremux-texture-style-person-probe-$(date +%Y%m%d-%H%M%S)}"
BUILD_DIR="$OUT_DIR/build"
mkdir -p "$BUILD_DIR" "$OUT_DIR/cases"

CLANG="${CLANG:-$(xcrun --find clang)}"
SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"

PERSON_PROBE="$BUILD_DIR/texture_style_person_input_probe"
TRACE_DYLIB="$BUILD_DIR/libXDRemuxTextureStyleTrace.dylib"

"$CLANG" -fobjc-arc -fmodules -isysroot "$SDKROOT" \
  -framework Foundation -framework CoreGraphics -ldl \
  "$SRC_DIR/texture_style_person_input_probe.m" \
  -o "$PERSON_PROBE"

"$CLANG" -dynamiclib -fobjc-arc -fmodules -isysroot "$SDKROOT" \
  -framework Foundation -ldl \
  -Wl,-install_name,@rpath/libXDRemuxTextureStyleTrace.dylib \
  "$SRC_DIR/texture_style_input_trace.m" \
  -o "$TRACE_DYLIB"

cat >"$OUT_DIR/BUILD.txt" <<EOF
built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
sw_vers=$(sw_vers -productVersion)
person_probe=$PERSON_PROBE
trace_dylib=$TRACE_DYLIB
EOF

usage() {
  cat <<EOF
usage: $0 [--build-only] [--case NAME] [--all] [--normalize x y w h]

Builds two local-only macOS Texture Style probes:
  1. texture_style_person_input_probe
     Black-boxes +[CMITextureStylesPersonInputDataUtilities personInputDataArrayFromDetectedFaces:].
  2. libXDRemuxTextureStyleTrace.dylib
     Observes the real PhotoImaging/CMImaging person-data contracts when injected into a local harness.

Output: $OUT_DIR

Trace example for a non-hardened local harness:
  XDREMUX_TEXTURE_TRACE=1 \\
  XDREMUX_TEXTURE_TRACE_FILE="$OUT_DIR/trace.jsonl" \\
  DYLD_INSERT_LIBRARIES="$TRACE_DYLIB" \\
  /path/to/your/local/texture-style-harness <args>
EOF
}

mode="all"
case_name=""
normalize_args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --build-only)
      mode="build"
      shift
      ;;
    --case)
      case_name="${2:?missing case name}"
      mode="case"
      shift 2
      ;;
    --all)
      mode="all"
      shift
      ;;
    --normalize)
      [[ $# -ge 5 ]] || { echo "--normalize needs x y w h" >&2; exit 64; }
      normalize_args=(--normalize "$2" "$3" "$4" "$5")
      shift 5
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 64
      ;;
  esac
done

if [[ "$mode" == "build" ]]; then
  echo "$OUT_DIR"
  exit 0
fi

run_case() {
  local name="$1"
  local stdout_file="$OUT_DIR/cases/$name.json"
  local stderr_file="$OUT_DIR/cases/$name.stderr.log"
  set +e
  "$PERSON_PROBE" --case "$name" "${normalize_args[@]}" >"$stdout_file" 2>"$stderr_file"
  local status=$?
  set -e
  printf '%s\t%s\n' "$name" "$status" >>"$OUT_DIR/status.tsv"
  if [[ $status -ne 0 ]]; then
    echo "[probe] case '$name' exited $status; see $stderr_file" >&2
  fi
}

: >"$OUT_DIR/status.tsv"
printf 'case\tstatus\n' >>"$OUT_DIR/status.tsv"

if [[ "$mode" == "case" ]]; then
  run_case "$case_name"
else
  while IFS= read -r name; do
    [[ -n "$name" ]] && run_case "$name"
  done < <("$PERSON_PROBE" --list)
fi

cat >"$OUT_DIR/TRACE_USAGE.txt" <<EOF
The trace dylib is intentionally passive: it only logs method inputs/outputs and forwards to the original IMP.

Use it with the same local executable that already exercises the real PhotoImaging/CMImaging Texture Style path:

XDREMUX_TEXTURE_TRACE=1 \\
XDREMUX_TEXTURE_TRACE_FILE="$OUT_DIR/trace.jsonl" \\
DYLD_INSERT_LIBRARIES="$TRACE_DYLIB" \\
/path/to/local/harness <native-texture-style-heic-or-existing-args>

Expected events include:
- CMITextureStylesPersonInputDataUtilities.personInputDataArrayFromDetectedFaces.input/output
- PITextureStyleProcessorKernel.personInputDataFromStillProperties.input/output
- CMITextureStylesProcessor.setInputPersonData
- CMITextureStylesProcessor.setInputSkinSmoothingFaceDetections

If DYLD injection is blocked by hardened runtime/library validation, do not weaken system security. Rebuild the local research harness without hardened runtime/library validation, or load this dylib from that harness with dlopen before it opens PhotoImaging.
EOF

echo "$OUT_DIR"
