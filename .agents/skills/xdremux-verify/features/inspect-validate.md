# Inspect and validate

Inspect source facts and independently validate published output through human and machine-readable interfaces.

## Sub-features

Check source kind, relevant ranges/timing or HDR facts, JSON schema, success/failure exit codes, and validation from both Live Photo members. Feature-specific `validate --apple-styles` is a separate native contract.

## How to get to it (user POV)

Run `xdremux inspect INPUT`, `xdremux inspect INPUT --json`, `xdremux validate OUTPUT`, or `xdremux validate OUTPUT --json`. Help and the CLI reference define the supported argument contract.

## Driving it with the Rust CLI

Preconditions: use a fixture whose SHA matches the manifest and outputs owned by this run. Use the Motion Photo recipe for ready-to-validate paired output. Run both human and JSON forms when wording or output schema changes. Capture stdout, stderr, and exit status separately. Assert documented success 0, runtime failure 1, or usage failure 2 only for a case designed to reach that outcome.

Use `bash scripts/validation/verify_error_messages.sh` for relevant error contracts and existing CLI parser/validation tests for the touched behavior. On macOS 27, `validate STYLES_OUTPUT --apple-styles` must reach the actual adapter and final required state. A plain HDR file must fail that feature check.

## Gotchas

Inspection reports facts and does not imply convertibility. Plain `validate` proves its structural contract, not gallery/device editing. A feature validation or lower-level parser that accepts one resource does not establish every other required resource, final MakerApple state, native GPU completion, or input preservation.
