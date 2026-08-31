#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --locked -q -p xdremux-motion-photo --example motion_photo_conformance
swift test --filter MotionPhotoRustConformanceTests/testSwiftAndRustPureMotionPhotoContractsMatch
swift test --filter MotionPhotoAndroidRustConformanceTests/testSwiftAndRustAndroidParserContractsMatch
python3 -m unittest discover -s Tests -p 'test_python_motion_photo.py' -v
