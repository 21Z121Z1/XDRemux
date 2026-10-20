# arm64/macOS 27 distribution convergence

The delegated distribution change is complete and frozen for parent review. It is an uncommitted change on `4b04fb3168cdc9bf8f54d7755db94fd3677eebb6`, alongside other agents' dirty changes. It is not a final committed-HEAD receipt or a native macOS result. No staging, commit, push, workflow dispatch, protection, payment, global security setting, or cleanup-guard change was performed.

## Scope and actual consumer path

The parent relayed explicit user authorization at 2026-10-20 22:58:19 UTC to target complete product delivery at arm64/macOS 27, remove the newly added Intel package and its product acceptance gate, and retain useful cross-platform Rust core tests. The parent separately authorized only the corresponding product-package method in `tests/validation/test_workflow_configuration.py` and CLI support method in `tests/test_public_documentation.py`.

The current caller chain is `.github/workflows/rust-cli-core.yml` → `scripts/distribution/build_native.py` → `macos.py stage/archive` → `smoke_macos.py`. The workflow builds the root `xdremux-apple-adapter` product. The root package currently lists both `XDCinematicNative` and `XDTextureNative` as adapter dependencies. This change did not edit that package, adapter, app, runtime, media code, other CI owners, or any cleanup guard.

The existing package matrix had arm64/xcode-27 and x86_64/macos-26-intel entries, with `MACOSX_DEPLOYMENT_TARGET=15.0`. The CMake dependencies also targeted 15.0. Mach-O inspection recorded `minimum_os` but accepted it without checking. These were the distribution-side decisions changed here.

## Resulting behavior

- The product job is a single `macos-package (arm64)` on `xcode-27`, with `MACOSX_DEPLOYMENT_TARGET=27.0` and Rust target `aarch64-apple-darwin`. There is no product architecture matrix or Intel artifact path.
- The preceding Linux, Windows, and macOS portable Rust job, triggers, permissions, exact-head checkout, and commands are byte-for-byte unchanged. Removing an Intel product package does not remove cross-platform core code or tests.
- Product build, staging, audit, archive entry and relocated smoke require a native arm64 Darwin process on macOS 27 or later. All explicit architecture options accept only `arm64`. These are distribution-tool checks; no platform restriction was added to the Rust core.
- Every packaged executable and pinned dylib must have exactly the arm64 architecture and Mach-O minimum OS `27.0` (the equivalent `27.0.0` spelling is accepted). Missing, older, or higher deployment targets fail. Build, final archive audit, and extracted-package audit remain separate artifact boundaries.
- `MINIMUM_MACOS` and `PRODUCT_ARCH` live in the existing `macos.py` owner and are reused by the native builder and smoke tool. The dependency CMake target, package manifest, linkage report and extracted identity check agree on 27.0.
- Full dylib closure, all dynamic load kinds, forbidden host rpaths/dyld environment, strict signatures, pinned source archives/checksums, license notices, reproducible archives, and safe extraction remain in place. `native-dependencies.json` is byte-for-byte unchanged from the starting HEAD.
- Checkout-external scratch execution still uses `env -i`, system-only PATH, real Samsung HEIF Motion Photo and OPPO ProXDR fixtures, both HEIC/MOV validation for Live Photo, correct current validation kinds, original-byte checks, and existing-pair rejection. This change does not substitute a parser test for that CI smoke.
- English support/distribution paragraphs were updated before the corresponding Chinese paragraphs. They distinguish the complete arm64/macOS 27 product target from the cross-platform Rust core. Package smoke and compilation floors do not establish Styles SPI, native feature acceptance, Photos editing, or app behavior. Existing experimental Cinematic and device-evidence limits remain unchanged.

The maintained obligation removed is the newly introduced Intel product build/acceptance path. The strongest reason to retain Intel was Rust core portability, which remains covered by the existing three-OS job. No alternate workflow, wrapper, release system, or native policy owner was introduced.

## Failing-before and passing-after evidence

The first test run was captured before changing distribution production code. `arm64-macos27-distribution-red.log` and `.json` record nine methods with 11 failing subcases: old minimum OS facts were accepted (missing/15/26/28), incorrect Linux/Intel/old-macOS host facts were accepted by audit, the three tools still accepted Intel argument syntax, and the old CI matrix violated the new single-product contract. Existing wrong-Mach-O-architecture rejection already passed and remains covered.

The first broader run then found one old documentation assertion requiring the literal phrase “one cross-platform product entry point”. This was a stale support contract, not a native product failure. The original failure is retained in `arm64-macos27-distribution-green-0.log` and `.json`. With separate parent authorization, that method now positively requires the English/Chinese arm64/macOS 27 product scope, Linux/Windows/macOS Rust core support, and the Styles SPI limitation. Other documentation methods were not changed.

The final command was:

```sh
python3 -m unittest tests.test_macos_distribution tests.validation.test_workflow_configuration tests.validation.test_repository_layout tests.test_public_documentation -v
```

All 46 methods passed in 0.431 seconds. `git diff --check` on the 14 owned/authorized files also passed. The actual Linux invocation of `build_native.py --arch arm64` exited 2 before creating the requested work directory, proving the distribution entry fails before downloads/build effects on this host. Logs are `arm64-macos27-distribution-final-0.log`, `-final-1.log` and `arm64-macos27-linux-host-negative-final.log`.

`arm64-macos27-distribution-final.json` contains exact commands, exits, actual HEAD/dirty state, and SHA-256 for all 14 final files. Those files were unchanged during the checks. Host and Mach-O observation injection in portable regressions is explicitly test data; it is not native execution. No Rust test suite or build was repeated because no Rust code changed and the parent owns the final complete gate.

## Review and remaining gates

The author reread the relevant canonical owners and the project-adapted TDD, engineering-principles, verification, review rubric/lead judgment/code quality, and unslop skills. Review found no additional in-scope defect after the focused fix. The concrete simplification is removing the unused product architecture choice while preserving artifact, media, helper and native capability validation boundaries.

Linux cannot compile/run Mach-O, execute `otool`/codesign, build the Swift targets, or validate the extracted Mac package. This task does not claim those checks passed. The parent still owns root Package.swift/app deployment and adapter capability alignment, then a fresh committed-HEAD full CI run including the arm64 package and its actual relocated conversions, native/System/App, performance and CodeQL results. Any real failure remains a failure. Durable recovery evidence and separate parent merge/delete approval remain required.

The current package source still showed the old `.macOS(.v15)` when read during this audit; that is the parent's pending platform edit, not a retained product support promise. The new strict package audit will reject an adapter that actually still emits an older deployment target. No fallback, continue-on-error, skipped feature acceptance, relaxed threshold, or fabricated native capability was added to make the platform transition look successful.
