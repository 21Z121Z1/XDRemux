#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# The Rust crate owns the normalized classification contract. The fixture and
# the crate-level assertions are the regression source of truth.
cargo test --locked -p xdremux-classification matches_shared_swift_python_golden_contract

# Asset grouping/planning remains a separate behavioral contract from tag parsing.
# One occupied still destination must advance the whole pair to the same sequence.
cargo test --locked -p xdremux-classification asset_planning_contract_keeps_validated_live_pair_on_shared_collision_sequence
