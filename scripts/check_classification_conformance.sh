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

# Asset grouping/planning remains a separate behavioral contract from tag parsing.
# Exercise the same validated Live Photo collision scenario in Rust, Swift, and Python:
# one occupied still destination must advance the whole pair to the same sequence.
cargo test --locked -p xdremux-classification asset_planning_contract_keeps_validated_live_pair_on_shared_collision_sequence
swift test --filter CoreContractTests/testPhotoCategorizationKeepsValidatedLivePhotoResourcesTogether
python3 -m unittest Tests.test_photo_classification_contract.PhotoClassificationContractTests.test_asset_planning_keeps_validated_live_pair_on_shared_collision_sequence
