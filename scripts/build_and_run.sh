#!/usr/bin/env bash
#
# Build and run the XDRemux macOS App.
#
# Only an explicit debug build uses the Debug configuration. Everything else —
# including the plain `run` workflow — builds Release, because the Photographic
# Styles solver is dominated by floating-point work that a debug build slows by
# roughly an order of magnitude. Attach a debugger with `debug` when you need
# symbols more than speed.
#
# usage: scripts/build_and_run.sh [run|build|debug|logs|telemetry|verify|clean] [--verbose]

set -euo pipefail

APP_NAME="XDRemuxApp"
SCHEME="XDRemuxApp"
PROJECT_PATH="apps/macos/XDRemuxApp/XDRemuxApp.xcodeproj"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DERIVED_DATA="${XDREMUX_DERIVED_DATA:-$ROOT_DIR/.build/xcode}"
LOG_DIRECTORY="$DERIVED_DATA/Logs/XDRemux"
BUILD_LOG="$LOG_DIRECTORY/build.log"

COMMAND="run"
VERBOSE=0

while (($# > 0)); do
  case "$1" in
    run|build|debug|logs|telemetry|verify|clean|--debug|--logs|--telemetry|--verify)
      COMMAND="$1"
      ;;
    --verbose)
      VERBOSE=1
      ;;
    -h|--help|help)
      sed -n '2,11p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      echo "usage: $0 [run|build|debug|logs|telemetry|verify|clean] [--verbose]" >&2
      exit 2
      ;;
  esac
  shift
done

case "$COMMAND" in
  --debug|debug)
    CONFIGURATION="Debug"
    ;;
  run|--logs|logs|--telemetry|telemetry|--verify|verify)
    CONFIGURATION="Release"
    ;;
  *)
    CONFIGURATION="Release"
    ;;
esac

APP_BUNDLE="$DERIVED_DATA/Build/Products/$CONFIGURATION/$APP_NAME.app"
APP_EXECUTABLE="$APP_BUNDLE/Contents/MacOS/$APP_NAME"

cd "$ROOT_DIR"

if [[ "$COMMAND" == "clean" ]]; then
  rm -rf "$DERIVED_DATA"
  echo "removed $DERIVED_DATA"
  exit 0
fi

if [[ "$COMMAND" == "--logs" || "$COMMAND" == "logs" ]]; then
  if [[ ! -f "$BUILD_LOG" ]]; then
    echo "no build log yet at $BUILD_LOG" >&2
    exit 1
  fi
  tail -200 "$BUILD_LOG"
  exit 0
fi

build_app() {
  mkdir -p "$LOG_DIRECTORY"
  local status=0
  set +e
  xcodebuild \
    -project "$PROJECT_PATH" \
    -scheme "$SCHEME" \
    -configuration "$CONFIGURATION" \
    -derivedDataPath "$DERIVED_DATA" \
    CODE_SIGNING_ALLOWED=NO \
    build >"$BUILD_LOG" 2>&1
  status=$?
  set -e
  if ((status != 0)); then
    echo "build failed ($CONFIGURATION); last 80 log lines:" >&2
    tail -80 "$BUILD_LOG" >&2
    echo "full log: $BUILD_LOG" >&2
    return "$status"
  fi
  if ((VERBOSE == 1)); then
    cat "$BUILD_LOG"
  fi
  echo "built $CONFIGURATION -> $APP_BUNDLE"
}

build_app

case "$COMMAND" in
  build)
    ;;
  --verify|verify)
    cargo fmt --all -- --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    swift build --product xdremux-apple-adapter
    python3 -m unittest discover -s Tests -p "test_*.py"
  ;;
  --telemetry|telemetry)
    log show --predicate 'subsystem == "com.proxdr.XDRemuxApp"' --last 15m --info || true
    ;;
  *)
    if [[ ! -x "$APP_EXECUTABLE" ]]; then
      echo "app executable missing: $APP_EXECUTABLE" >&2
      exit 1
    fi
    open "$APP_BUNDLE"
    ;;
esac
