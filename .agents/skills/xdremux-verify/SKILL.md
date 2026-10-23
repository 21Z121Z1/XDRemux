---
name: xdremux-verify
description: Verify XDRemux through its real Rust CLI, fixture conversion, output validation, and applicable Apple/app consumers using existing repository commands. Use after a behavior change or before a completion claim. Keep portable, native, device, and exact-HEAD evidence distinct.
---

# Verify XDRemux

Read [AGENTS.md](../../../AGENTS.md), [testing policy](../../../docs/quality/testing.en.md), the [validation runbook](../../../docs/validation/README.en.md), the relevant [CLI contract](../../../docs/cli.en.md), and [Apple feature limits](../../../docs/apple-features.en.md). Use the [feature index](features/README.md) to select the affected entry points. The existing scripts, CI, and completion gate remain the verification owners. Do not create another runner, result ledger, installer, or test framework.

## Launch

The primary product is a short-lived Rust CLI. From the repository root, use the existing documented build:

```bash
export CARGO_TARGET_DIR="$PWD/target"
cargo build --locked -p xdremux-cli
export XDREMUX_VERIFY_CLI="$CARGO_TARGET_DIR/debug/xdremux"
```

Use a separate Cargo target for each worktree; the launch command overrides an inherited shared target. Completion plans that set a custom target must bind every Cargo check and CLI path to that worktree's own directory. Use the session's already configured Rust/libheif environment. Do not install or upgrade dependencies without existing task authority. If the user or lead supplies an existing binary, set `XDREMUX_VERIFY_CLI` to its absolute path and record its hash and unverified build binding. That binary can prove the exercised behavior of those bytes; it cannot prove that the latest source or final committed HEAD produced them.

No server or interactive session must remain running. Give each run its own temporary output directory. Never reuse the input path for a verification conversion, even when ordinary ProXDR supports intentional in-place conversion. Do not drive a shared app/helper instance that the run does not own.

## Doctor

Run `"$XDREMUX_VERIFY_CLI" --help` and `"$XDREMUX_VERIFY_CLI" inspect fixtures/motion-photo/samsung/heif-ultrahdr-01.heic --json`. Require successful exit, the documented commands, and a Motion Photo inspection. Check fixture bytes against `fixtures/SHA256SUMS`. Capture binary hash, source HEAD/dirty state, actual commands, stdout, stderr, and exit codes. A failed doctor is a failed prerequisite; repair or report the actual issue before driving a feature.

On macOS, read the feature's host requirement and use `bash scripts/apple/check_apple_adapter_handshake.sh` before relying on the adapter. Record `sw_vers`, architecture, SDK/Xcode, and helper identity. A successful generic handshake does not establish availability of every private feature API.

## Drive

Start with the real Samsung HEIF [Motion Photo recipe](features/motion-photo.md) for a cheap conversion plus independent validation of both pair members. Read [inspect and validate](features/inspect-validate.md), [batch and categorize](features/batch-categorize.md), or [HDR and Apple editing](features/hdr-apple.md) when the change reaches those paths. Run the smallest complete evidence set, not every recipe for every change.

## Evidence

Capture the action and resulting files/state, not just a final success line. Retain fixture identity, source preservation, output hashes, validation results, command statuses, and build-binding limits. The Motion Photo recipe writes its proof directory outside its disposable output directory and prints that proof path.

A parser, a portable codec, a helper mock, a native framework, and a device UI prove different things. Media/file/helper/native/final-byte values remain validation boundaries. Missing required resources, capability, or validation must fail before publication. For changed error or publication behavior, also run the owning negative regression and observe that existing bytes or both pair members remain safe.

When committed-HEAD evidence is required, use `scripts/ci/agent_completion_gate.py run --base <actual-base> --plan <reviewed-plan>` and its `verify` command as documented in the runbook. The plan must reflect the actual changed paths and every required check. A dirty tree or later source change prevents a final exact-HEAD claim. This skill does not authorize commits, pushes, workflow dispatch, or external publication.

## Cleanup

Remove only the output directories and processes this run created. Never kill by process name. Retain proof logs and manifests outside scratch state and confirm they still exist after cleanup. The Motion Photo recipe handles its own cleanup even after failure. Other native scripts may retain fixture outputs as evidence; inspect their cleanup contract before removing anything.

## Helpers and maintenance

This skill ships no executable helper. Use the existing repository commands named in the feature map. Keep the map as navigation to current owners, not a copied acceptance definition. When a command or interface changes, update its recipe after exercising that path. Record a missing native/device environment as a gap rather than weakening a journey. Do not schedule future upkeep unless requested.

Adapted using pstack `create-verification-skill` at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`. See [sources.json](sources.json) and [LICENSE](LICENSE).
