# Inspect and validate

Inspect source facts and independently validate published output. Check both human-readable and machine-readable interfaces when the change affects them.

## Select the affected behavior

Check source kind, relevant ranges or timing, HDR facts, JSON schema, and success or failure exit codes. Validate both Live Photo members. Feature-specific `validate --apple-styles` has a separate native contract.

## Use the public entry points

Run the command that reaches the affected interface:

- `xdremux inspect INPUT`
- `xdremux inspect INPUT --json`
- `xdremux validate OUTPUT`
- `xdremux validate OUTPUT --json`

Help and the CLI reference define the supported arguments.

## Run the checks

Before starting, verify that the fixture SHA matches the manifest. Use outputs owned by this run. The Motion Photo recipe produces a pair ready for validation.

1. If wording or the output schema changes, run both human and JSON forms.
2. Capture stdout, stderr, and exit status separately.
3. Select cases that reach the documented exit outcomes. Assert success 0, runtime failure 1, or usage failure 2 only for the corresponding case.
4. For relevant error contracts, run `bash scripts/validation/verify_error_messages.sh`. Use the existing CLI parser and validation tests for the affected behavior.
5. On macOS 27, use `validate STYLES_OUTPUT --apple-styles` to reach the actual adapter and required final state. Confirm that a plain HDR file fails the feature check.

## Interpret the evidence

Inspection reports facts. It does not establish that conversion is possible. Plain `validate` proves its structural contract. It does not establish gallery or device editing behavior.

Acceptance of one resource by a feature check or parser does not establish all required resources. It also does not establish final MakerApple state, native GPU completion, or input preservation.
