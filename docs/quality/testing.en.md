# Testing Policy

English | [简体中文](testing.md)

A change is complete only when its evidence matches the behavior that changed.

For commands, see [tests/README.en.md](../../tests/README.en.md). For completion-gate plans, see the [validation runbook](../validation/README.en.md).

## Evidence layers

| Layer | Typical command or source | What it proves |
| --- | --- | --- |
| Unit and contract | `cargo test --workspace --locked` | Rust product, parser, format, Apple-feature policy, and transaction contracts. |
| Repository policy | `python3 -m unittest discover -s tests -t . -v` | Cross-file policy, Python behavior, documentation, and architecture contracts. |
| Real fixture | `fixtures/` and `scripts/validation/` | Behavior on versioned or supplied real media. |
| Native framework | macOS validation jobs | ImageIO, PhotoKit, or other tested Apple framework behavior. |
| Device | manual or recorded real-device validation | Behavior that depends on a specific gallery, Photos version, display, or device. |
| Completion receipt | `scripts/ci/agent_completion_gate.py` | The selected checks passed for the exact commit. |

A static policy test is not functional conversion evidence.

A parser test is not a device test.

A container parser pass is not proof that a gallery renders the result correctly.

## Minimum evidence

The complete product gate targets arm64/macOS 27: App, adapter, relocated package, media preservation, HDR/Styles and performance checks remain required. Linux, Windows and macOS Rust core tests remain enabled. Intel product-package checks are outside the supported delivery scope. This scope does not turn missing native data, failed operations or untested device behavior into a pass.

The macOS 27 performance reference preserves its original measurements. The
`apple-styles` case allows twice the historical median and p95 wall times to
accommodate full-resolution HDR reconstruction before Linear Thumbnail reduction.
Its RSS and helper-launch limits remain unchanged. Other workloads retain the
shared wall-time tolerance. See the [versioned budget](../../benchmarks/macos27-arm64-product-baseline.json).

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

Strict Rust CI gates use these fixtures to test multiple JPEG and HEIC/HEIF Motion Photo layouts.

Do not describe all real samples as private. Some older ProXDR, device-only, and Apple-feature samples can still be external or private.

Native checks may create media and pixel buffers on their runner. Automatic
Actions uploads contain named diagnostic reports, hashes and logs, not source
photos, converted portraits, rendered comparisons or pixel dictionaries. Keep
media generation, native readback and failure assertions in the test; restrict
the upload boundary rather than skipping those checks. Produce Photos import
files locally when device validation is required.

## Device-dependent claims

A claim about Apple Photos editing, OPPO Gallery display, or another device UI requires evidence from that environment.

If the environment is unavailable, narrow the claim to the behavior that was actually tested.

Do not mark an untested device-dependent claim as passed.

## Documentation is testable behavior

User-visible command names, options, defaults, output-safety rules, and support boundaries are product contracts.

When code changes one of these contracts, update the English and Chinese documentation in the same change.

Current technical documentation follows the [technical writing guide](../style-guide.en.md).

## Research regression evidence

A single reusable `research.yml` workflow owns portable research tests, native
carrier readback, the pinned Photographic Styles tone sweep, and the provisional
system HEIF HDR measurement and PVT admission probe. Its jobs use the exact pull-request or main SHA
with read-only permissions. A manual replay accepts no alternate source or secrets.
The native tone and system HEIF jobs use the standard `xcode-27` runner and retain
measurement artifacts for one day. Their results do not supply `exact-head` or
establish Photos editing, slider response, save/reopen, or Gallery acceptance.

The PVT job uses one clean checkout for its readiness control and product probe.
It selects an iPhone that the chosen runtime explicitly supports and that meets
the device type's runtime version bounds. An independent membership check runs
before simulator creation. The product probe runs at most once, after the owned
control simulator has passed and its deletion is verified. Its receipt must bind
the same source SHA, runtime and device type. A passing control does not prove
PVT admission. Admission requires whole-package identity and independent PhotoKit
readback. See the [Motion Photo verification recipe](../../.agents/skills/xdremux-verify/features/motion-photo.md).

The control submits one application-inventory query with a 120-second deadline.
A timeout keeps readiness blocked and collects bounded host diagnostics for the
owned simulator before cleanup. These diagnostics do not establish readiness or
Photos admission.

PVT stage observations bind the source and session and retain monotonic times
within each named process clock. Unstarted or timed-out stages leave admission
unobserved. Cleanup command outcomes and independently observed resource absence
are separate facts; later absence does not rewrite a failed or uncertain original
receipt. The control wrapper keeps overall acceptance in `status` and native
facts in `product_observations`; a cleanup failure does not erase verified
admission observations. Portable event and history replays establish harness reporting only.
An owned run of the changed native runner is required for a runner-working claim;
Photos admission still requires the complete save and independent readback path.

Keep experiment commands and promotion criteria beside their code rather than
creating a workflow per hypothesis. Product orientation readback runs inside
`ci.yml` against the same exact SHA as the Rust product tests. It reads the
synthetic oriented HEIF and its expected geometry with ImageIO.
