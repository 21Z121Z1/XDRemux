# Testing policy

English | [简体中文](testing.md)

What a change has to pass before it counts as done. For how to run the tests, see [Tests/README.md](../../Tests/README.md); for the acceptance flow, see the [validation guide](../validation/README.md).

## Four layers

| Layer | Command | Covers |
| --- | --- | --- |
| Unit and contract | `swift test` | Conversion models, HEIF bounds, error text, CLI parsing, Apple feature contracts |
| Policy | `python3 -m unittest discover -s Tests` | Architecture boundaries, documentation consistency, categorization behaviour, Python tail policy |
| Real samples | the harnesses in `Tests/validation/` | Run a full conversion on real OPPO photos and assert the result |
| Acceptance | `scripts/agent_completion_gate.py` | Bind the selected checks to a specific commit and emit a receipt |

The policy tests are purely static — they read source and documentation and assert on it. They never run a conversion. **Do not count a static check as functional evidence.**

## Passing

`swift test` and the Python suites must be green. A change that affects conversion behaviour also needs at least one piece of real-sample evidence; type-checking is not enough.

## Adding tests

1. A bug fix needs a regression assertion, and that assertion should fail against the unfixed code.
2. A change to text users see — error messages, help output, command output — needs an assertion pinning it.
3. If something cannot be automated, say in the commit message what could not be verified and why.

## Known gaps

- The strict Samsung/Xiaomi/OPPO/vivo Motion Photo fixtures are versioned under `fixtures/`, so those CI gates do not require a private archive or repository secret. Other legacy ProXDR / Apple-feature regression samples remain separate and may still be private.
- There is no automated acceptance against Photos on a real device. The import-edit-save-reopen round trip for Apple Photographic Styles and portrait is still manual.
- How a photo actually displays in OPPO Gallery cannot be verified in CI.
