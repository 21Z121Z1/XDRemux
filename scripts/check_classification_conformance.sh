#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# All three implementations consume Tests/fixtures/photo_classification_cases.json.
# A pass therefore means the Rust port agrees with the already-established Swift
# and Python normalized contract rather than with a Rust-only copy of the data.
cargo test --locked -p xdremux-classification matches_shared_swift_python_golden_contract
swift test --filter PhotoClassificationContractTests/testCanonicalGoldenContract
python3 -m unittest Tests.test_photo_classification_contract.PhotoClassificationContractTests.test_canonical_golden_contract
