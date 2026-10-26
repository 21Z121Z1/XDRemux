# Testing policy

English | [简体中文](testing.md)

A change is complete only when the evidence covers the behavior that changed. Select checks from the layers below. Commands are in the [test guide](../../tests/README.en.md). Receipt procedures are in the [validation runbook](../validation/README.en.md).

## Match the evidence to the claim

| Evidence | Command or source | Scope of the result |
| --- | --- | --- |
| Unit and contract tests | `cargo test --workspace --locked` | Rust product, parser, format, Apple-feature policy, and transaction behavior. |
| Repository policy tests | `python3 -m unittest discover -s tests -t . -v` | Cross-file rules, Python behavior, documentation, and architecture contracts. |
| Real media tests | `fixtures/` and `scripts/validation/` | Behavior on the versioned or supplied media. |
| Native framework tests | macOS validation jobs | The ImageIO, PhotoKit, or other Apple framework behavior exercised by the test. |
| Device tests | Recorded or manual device validation | Behavior on the tested gallery, Photos version, display, and device. |
| Completion receipt | `scripts/ci/agent_completion_gate.py` | The declared checks passed for the specified commit. |

Source-inspection tests establish static contracts. They do not establish conversion behavior. A parser pass does not establish device behavior or correct gallery rendering.

## Select the minimum complete checks

The complete product gate targets arm64/macOS 27. It includes the app, adapter, relocated package, source preservation, HDR, Styles, and performance checks. Rust core tests remain enabled on Linux, Windows, and macOS. Intel product packages are outside the supported distribution scope.

An unavailable native environment, a failed operation, or an untested device behavior remains a gap. The platform scope does not make those checks pass.

| Change | Minimum evidence |
| --- | --- |
| Documentation only | Documentation policy and link checks. |
| CLI parser or messages | Tests for the affected parser and output behavior. |
| HDR or containers | Unit tests and a real conversion or an equivalent functional fixture. |
| Motion Photo | Parser and writer tests, plus the applicable real-fixture checks. |
| Apple features | Structural tests and the applicable native framework or device checks. |
| App behavior | App build and the affected model or UI tests. |
| Verification framework | Completion-gate self-tests and a real gate run. |

When practical, add a regression assertion that fails for the original source defect. Use the smallest set of checks that covers the changed behavior.

### Preserve performance reference values

The [macOS 27 budget](../../benchmarks/macos27-arm64-product-baseline.json) retains its original measurements. The `apple-styles` case permits twice the historical median and p95 wall times. This allowance covers full-resolution HDR reconstruction before Linear Thumbnail reduction.

The RSS and helper-launch limits remain unchanged. Other workloads use the shared wall-time tolerance. The [measurement guide](evals.en.md) defines current reports and the limits of performance conclusions.

## Preserve fixtures and restrict uploaded media

The `fixtures/` directory contains versioned real Motion Photo files. Their exact bytes are part of the test contract. `fixtures/SHA256SUMS` records their identities. Strict Rust CI checks use these files across JPEG and HEIC/HEIF layouts.

Some older ProXDR, Apple-feature, and device-only samples can remain private or outside the repository. Do not describe every real sample as private.

Native checks can create media and pixel buffers on the runner. Automatic Actions uploads must contain only the named diagnostic reports, hashes, and logs. Do not upload source photos, converted portraits, rendered comparisons, or pixel dictionaries.

Keep media generation, native readback, and failure assertions in the tests. Restrict uploads without skipping those checks. For device validation, produce Photos import files locally.

## Report device limits

Claims about Photos editing, OPPO Gallery display, or another device UI need evidence from that environment. If the environment is unavailable, report only the behavior actually tested. Do not mark untested device behavior as passed.

## Keep documentation consistent with code

Command names, options, defaults, output-safety rules, and support limits are product contracts. When code changes a contract, update both language versions in the same change. Follow the [technical writing guide](../style-guide.en.md).

## Keep research evidence separate

The reusable [`research.yml`](../../.github/workflows/research.yml) workflow contains portable research tests, native carrier readback, and the pinned Photographic Styles tone sweep. It also contains the provisional system HEIF HDR measurement and PVT admission probe.

Jobs use the exact PR or `main` SHA with read-only permissions. Manual replay accepts no alternate source or secrets. Native tone and system HEIF jobs use `xcode-27` and retain measurement artifacts for one day.

Research results do not supply the `exact-head` product gate. They do not establish Photos editing, slider response, save/reopen behavior, or gallery acceptance.

### Interpret the PVT probe

The readiness control and product probe use one clean checkout. CI requires the installed iOS 27.0 runtime and iPhone 11 device type. A missing or incompatible pair stops the control before simulator creation. Direct runs that omit both pin options retain catalog-based selection.

Before simulator creation, the job checks that the selected runtime supports the chosen iPhone type. It also checks the device type's runtime version bounds and independently verifies membership in the supported list.

The product probe runs at most once. It starts only after the control simulator passes and the job verifies deletion of that simulator. The control passes its exact pair through `--required-runtime` and `--required-device-type`. The product checks that pair against fresh catalogs without selecting a replacement. After build and preparation, it checks the same pair again immediately before simulator creation. A changed or incompatible pair blocks creation.

The control submits one application-inventory query with a 120-second deadline.
A timeout keeps readiness blocked and collects bounded host diagnostics for the
owned simulator before cleanup. These diagnostics do not establish readiness or
Photos admission.

The product probe permits at most three startup inventory queries, each with a
120-second deadline. A real inventory reply is required before its single reader
installation. The registration query after that command has a 20-second deadline;
matching installed bytes and both owned containers must still be verified.

The file promise uses the package interchange type declared readable by
`PHLivePhoto`, rather than assuming the filesystem's content-type alias is a
Photos reader type. Before sharing, a separate provider must load the exact
whole directory through that native reader, with positive dimensions and
unchanged member hashes. UIKit receives that native decoder result, with its
object class and the opaque whole-package origin bound to the input receipt.
Receipts distinguish native decoding from the later system save callback and
independent library readback; decoding alone is not admission. They retain both
the filesystem/provider facts and the consumer type, native decode and hashes.
Neither a Live Photo object type label nor assembly from extracted resources can
substitute for whole-package decoding, system save and independent readback.

All system share sheets use the invoking button as their popover source view
and its bounds as the source rectangle. The image availability control and both
whole-package routes use the same presentation path, including regular-width
windows.

When the UI test fails, the probe collects a bounded native log for its reader
inside the owned simulator before deletion. A failed diagnostic query preserves
the original test failure and does not prevent cleanup.

The product receipt must identify the same source SHA, runtime, and device type. A passing control does not prove PVT admission. Admission requires whole-package identity and independent PhotoKit readback.

The control and product boot monitors require a zero process exit and an owned-device `Finished` terminal record. Migration failure, an already-booted shortcut, or unknown output blocks inventory or installation. This check does not replace inventory, Photos Save, or independent PhotoKit readback.

Stage observations identify the source and session. Each stage retains monotonic times within its named process clock. An unstarted or timed-out stage leaves admission unobserved.

Cleanup command results and independent observations that resources are absent are separate facts. Later absence does not change a failed or uncertain original receipt. The control wrapper stores overall acceptance in `status` and native facts in `product_observations`. A cleanup failure does not erase verified admission observations.

Portable event and history replays test the probe's reporting behavior only. To claim that a changed native runner works, execute it with a simulator created for that run. Photos admission still requires the complete save and independent readback sequence. See the [Motion Photo verification recipe](../../.agents/skills/xdremux-verify/features/motion-photo.md).

Keep experiment commands and promotion criteria beside their code. Do not create a workflow for each hypothesis. Product orientation readback runs in `ci.yml` at the same SHA as Rust product tests. ImageIO reads the synthetic oriented HEIF and checks its expected geometry.
