# Testing Policy

English | [简体中文](testing.md)

A change is complete only when its evidence matches the behavior that changed.

For commands, see [Tests/README.en.md](../../Tests/README.en.md). For completion-gate plans, see the [validation runbook](../validation/README.en.md).

## Evidence layers

| Layer | Typical command or source | What it proves |
| --- | --- | --- |
| Unit and contract | `swift test` | Swift model, parser, format, and Apple-feature contracts. |
| Repository policy | `python3 -m unittest discover -s Tests -v` | Cross-file policy, Python behavior, documentation, and architecture contracts. |
| Real fixture | `fixtures/` and `Tests/validation/` | Behavior on versioned or supplied real media. |
| Native framework | macOS validation jobs | ImageIO, PhotoKit, or other tested Apple framework behavior. |
| Device | manual or recorded real-device validation | Behavior that depends on a specific gallery, Photos version, display, or device. |
| Completion receipt | `scripts/agent_completion_gate.py` | The selected checks passed for the exact commit. |

A static policy test is not functional conversion evidence.

A parser test is not a device test.

A container parser pass is not proof that a gallery renders the result correctly.

## Minimum evidence

Use the smallest complete evidence set.

- Documentation-only change: documentation policy and link checks.
- CLI parser or message change: targeted parser/output regression.
- HDR or container change: unit tests plus a real conversion or equivalent functional fixture.
- Motion Photo change: parser/writer tests plus the applicable real Motion Photo fixture gates.
- Apple feature change: structural tests plus the applicable native-framework or device evidence.
- App change: app build and the affected model/UI test.
- Verification-framework change: completion-gate self-tests and a real gate run.

Every source bug fix should add a regression assertion that would fail for the original defect when that is practical.

## Public Motion Photo fixtures

The repository contains versioned real Motion Photo fixtures under `fixtures/`.

Their exact bytes are part of the test contract. `fixtures/SHA256SUMS` is the identity manifest.

Strict Swift and pure-Python CI gates use these fixtures to test multiple JPEG and HEIC/HEIF Motion Photo layouts.

Do not describe all real samples as private. Some older ProXDR, device-only, and Apple-feature samples can still be external or private.

## Device-dependent claims

A claim about Apple Photos editing, OPPO Gallery display, or another device UI requires evidence from that environment.

If the environment is unavailable, narrow the claim to the behavior that was actually tested.

Do not mark an untested device-dependent claim as passed.

## Documentation is testable behavior

User-visible command names, options, defaults, output-safety rules, and support boundaries are product contracts.

When code changes one of these contracts, update the English and Chinese documentation in the same change.

Current technical documentation follows the [technical writing guide](../style-guide.en.md).
