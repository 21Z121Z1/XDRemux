#!/usr/bin/env bash
set -euo pipefail

APP_NAME="XDRemuxApp"
BUNDLE_ID="com.proxdr.XDRemuxApp"
PROJECT_PATH="apps/macos/XDRemuxApp/XDRemuxApp.xcodeproj"
SCHEME="XDRemuxApp"
CONFIGURATION="Debug"
DERIVED_DATA="${XDREMUX_DERIVED_DATA:-/tmp/xdremuxapp-derived}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$DERIVED_DATA/Build/Products/$CONFIGURATION/$APP_NAME.app"
APP_EXECUTABLE="$APP_BUNDLE/Contents/MacOS/$APP_NAME"
HELPER_DIRECTORY="$APP_BUNDLE/Contents/Helpers"
LOG_DIRECTORY="$DERIVED_DATA/Logs/XDRemux"
BUILD_LOG="$LOG_DIRECTORY/build.log"
RESULT_BUNDLE="$LOG_DIRECTORY/build-$(date +%Y%m%d-%H%M%S).xcresult"
HELPERS=(XDRemuxSemanticHelper XDRemuxHEVCEncoderHelper XDRemuxStyleValidationHelper)

MODE="run"
VERBOSE=0
LOGS_ALL=0

usage() {
  echo "usage: $0 [run|build|debug|logs [--all]|verify|clean] [--verbose]" >&2
}

while (($# > 0)); do
  case "$1" in
    run|build|debug|logs|verify|clean)
      MODE="$1"
      ;;
    --verbose)
      VERBOSE=1
      ;;
    --all)
      LOGS_ALL=1
      ;;
    --debug)
      MODE="debug"
      ;;
    --logs)
      MODE="logs"
      ;;
    --verify)
      MODE="verify"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
  shift
done

cd "$ROOT_DIR"

if [[ "$MODE" != "logs" && "$LOGS_ALL" -eq 1 ]]; then
  echo "--all is valid only with logs" >&2
  exit 2
fi

show_build_failure() {
  echo "Build failed." >&2
  if command -v rg >/dev/null 2>&1; then
    rg -n -C 3 'error:|BUILD FAILED|The following build commands failed' "$BUILD_LOG" >&2 || tail -80 "$BUILD_LOG" >&2
  else
    tail -80 "$BUILD_LOG" >&2
  fi
  echo "Full log: $BUILD_LOG" >&2
  echo "Result bundle: $RESULT_BUNDLE" >&2
}

build_app() {
  mkdir -p "$LOG_DIRECTORY"
  local obsolete_source_resources="$APP_BUNDLE/Contents/Resources/ApplePlatform"
  if [[ -d "$obsolete_source_resources" ]]; then
    rm -rf "$obsolete_source_resources"
  fi
  local command=(
    xcodebuild
    -project "$PROJECT_PATH"
    -scheme "$SCHEME"
    -configuration "$CONFIGURATION"
    -derivedDataPath "$DERIVED_DATA"
    -resultBundlePath "$RESULT_BUNDLE"
    build
  )
  if [[ "$VERBOSE" -eq 1 ]]; then
    set +e
    "${command[@]}" 2>&1 | tee "$BUILD_LOG"
    local status=${PIPESTATUS[0]}
    set -e
    if [[ "$status" -ne 0 ]]; then
      show_build_failure
      return "$status"
    fi
  elif ! "${command[@]:0:1}" -quiet "${command[@]:1}" >"$BUILD_LOG" 2>&1; then
    show_build_failure
    return 1
  fi
}

verify_signatures() {
  /usr/bin/codesign --verify --strict --verbose=2 "$APP_BUNDLE" >/dev/null 2>&1
  for helper in "${HELPERS[@]}"; do
    /usr/bin/codesign --verify --strict --verbose=2 "$HELPER_DIRECTORY/$helper" >/dev/null 2>&1
  done
}

verify_bundle() {
  [[ -x "$APP_EXECUTABLE" ]]
  for helper in "${HELPERS[@]}"; do
    [[ -x "$HELPER_DIRECTORY/$helper" ]]
  done
  if find "$APP_BUNDLE" -type f \( -name '*.swift' -o -name '*.m' -o -name '*.h' \) -print -quit | grep -q .; then
    echo "Bundle verification failed: source files are present in $APP_BUNDLE" >&2
    return 1
  fi
}

stop_current_bundle() {
  local pid command_path expected_path
  expected_path="$(canonical_app_executable)"
  while read -r pid; do
    [[ -n "$pid" ]] || continue
    command_path="$(ps -p "$pid" -o command= 2>/dev/null || true)"
    if [[ "$command_path" == "$expected_path"* ]]; then
      kill "$pid" 2>/dev/null || true
    fi
  done < <(pgrep -x "$APP_NAME" 2>/dev/null || true)
}

launch_app() {
  stop_current_bundle
  /usr/bin/open -n "$APP_BUNDLE"
}

canonical_app_executable() {
  local directory
  directory="$(cd "$(dirname "$APP_EXECUTABLE")" && pwd -P)"
  echo "$directory/$(basename "$APP_EXECUTABLE")"
}

verify_process() {
  local attempt pid command_path expected_path
  expected_path="$(canonical_app_executable)"
  for attempt in {1..40}; do
    while read -r pid; do
      [[ -n "$pid" ]] || continue
      command_path="$(ps -p "$pid" -o command= 2>/dev/null || true)"
      if [[ "$command_path" == "$expected_path"* ]]; then
        return 0
      fi
    done < <(pgrep -x "$APP_NAME" 2>/dev/null || true)
    sleep 0.25
  done
  echo "Launch verification failed for $APP_EXECUTABLE" >&2
  return 1
}

run_build_stages() {
  local total="$1"
  echo "XDRemux macOS Development Build"
  echo "[1/$total] Building XDRemuxApp"
  build_app
  echo "[2/$total] Signing application"
  verify_signatures
  echo "[3/$total] Verifying bundle"
  verify_bundle
}

case "$MODE" in
  build)
    run_build_stages 3
    echo "✓ XDRemuxApp built"
    ;;
  run)
    run_build_stages 4
    echo "[4/4] Launching application"
    launch_app
    echo "✓ XDRemuxApp launched"
    ;;
  verify)
    run_build_stages 4
    echo "[4/4] Launching and checking process"
    launch_app
    verify_process
    echo "✓ XDRemuxApp bundle and process verified"
    ;;
  debug)
    run_build_stages 4
    echo "[4/4] Starting LLDB"
    stop_current_bundle
    exec lldb -- "$APP_EXECUTABLE"
    ;;
  logs)
    if [[ "$LOGS_ALL" -eq 1 ]]; then
      exec /usr/bin/log stream --level debug --style compact --predicate "process == \"$APP_NAME\""
    fi
    exec /usr/bin/log stream --style compact --predicate "subsystem == \"$BUNDLE_ID\""
    ;;
  clean)
    if [[ -d "$(dirname "$APP_EXECUTABLE")" ]]; then
      stop_current_bundle
    fi
    case "$DERIVED_DATA" in
      /tmp/xdremuxapp-derived|"$ROOT_DIR"/.build/xdremuxapp-derived)
        rm -rf "$DERIVED_DATA"
        ;;
      *)
        echo "Refusing to clean unexpected DerivedData path: $DERIVED_DATA" >&2
        exit 2
        ;;
    esac
    echo "✓ Removed XDRemux DerivedData: $DERIVED_DATA"
    ;;
esac
