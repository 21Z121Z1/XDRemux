---
name: xdremux-verify
description: Verify XDRemux through its real Rust CLI, fixture conversion, output validation, and applicable Apple/app consumers using existing repository commands. Use after a behavior change or before a completion claim. Keep portable, native, device, and exact-HEAD evidence distinct.
---

# Verify XDRemux

Verify the affected behavior through the actual product entry point. Keep portable, native, device, and exact-HEAD evidence separate.

Read [AGENTS.md](../../../AGENTS.md), [testing policy](../../../docs/quality/testing.en.md), and the [validation runbook](../../../docs/validation/README.en.md). Check the relevant [CLI contract](../../../docs/cli.en.md) and [Apple feature limits](../../../docs/apple-features.en.md). Use the [feature index](features/README.md) to select the affected entry points.

The existing scripts, CI, and completion gate own verification. Do not create another runner, result ledger, installer, or test framework.

## Build or identify the CLI

The primary product is a Rust CLI that exits after each command. From the repository root, use the documented build:

```bash
export CARGO_TARGET_DIR="$PWD/target"
cargo build --locked -p xdremux-cli
export XDREMUX_VERIFY_CLI="$CARGO_TARGET_DIR/debug/xdremux"
```

Use a separate Cargo target for each worktree. The launch command overrides an inherited shared target. If a completion plan uses a custom target, bind every Cargo check and CLI path to that worktree's target directory.

Use the session's configured Rust/libheif environment. Do not install or upgrade dependencies without existing task authority.

If the user or lead supplies an existing binary, set `XDREMUX_VERIFY_CLI` to its absolute path. Record the binary hash and state that its build binding is unverified. That binary can prove the behavior of the exercised bytes. It cannot prove that the latest source or final committed HEAD produced them.

Give each run its own temporary output directory. Never use the input path as the conversion output during verification. This restriction also applies when ordinary ProXDR supports intentional in-place conversion. Do not operate a shared app or helper instance that the run does not own. No server or interactive session needs to remain running.

## Check the prerequisites

1. Run `"$XDREMUX_VERIFY_CLI" --help`. Require a successful exit and the documented commands.
2. Run `"$XDREMUX_VERIFY_CLI" inspect fixtures/motion-photo/samsung/heif-ultrahdr-01.heic --json`. Require a successful exit and Motion Photo recognition.
3. Check the fixture bytes against `fixtures/SHA256SUMS`.
4. Capture the binary hash, source HEAD, dirty state, actual commands, stdout, stderr, and exit codes.
5. If a prerequisite fails, repair or report the actual issue before exercising a feature.

On macOS, read the feature's host requirements. Before relying on the adapter, run `bash scripts/apple/check_apple_adapter_handshake.sh`. Record `sw_vers`, architecture, SDK/Xcode, and helper identity. A successful generic handshake does not establish that every private feature API is available.

## Exercise the affected feature

Start with the real Samsung HEIF [Motion Photo recipe](features/motion-photo.md). It provides a cheap conversion and independent validation of both pair members.

If the change affects another path, read its recipe: [inspect and validate](features/inspect-validate.md), [batch and categorize](features/batch-categorize.md), or [HDR and Apple editing](features/hdr-apple.md). Run the smallest complete evidence set. Do not run every recipe for every change.

## Retain evidence

Capture the action and the resulting files or state. A final success line alone is insufficient. Retain fixture identity, source preservation, output hashes, validation results, command statuses, and build-binding limits. The Motion Photo recipe stores proof outside its disposable output directory. It prints the retained proof path.

Parsers, portable codecs, helper mocks, native frameworks, and device interfaces establish different evidence. Media, files, helper results, native facts, and final bytes remain validation boundaries. If required resources, capability, or validation are missing, fail before publication.

If error or publication behavior changes, run the owning negative regression. Observe that existing bytes or both pair members remain safe.

If committed-HEAD evidence is required, follow the runbook. Use `scripts/ci/agent_completion_gate.py run --base <actual-base> --plan <reviewed-plan>` and its `verify` command. The plan must cover the actual changed paths and all required checks. A dirty tree or later source change prevents a final exact-HEAD claim. This skill does not authorize commits, pushes, workflow dispatch, or external publication.

## Clean up owned state

Remove only output directories and processes created by this run. Never kill processes by name.

Keep proof logs and manifests outside the disposable scratch state. After cleanup, confirm that they still exist. The Motion Photo recipe cleans its output directory even after failure. Other native scripts can retain fixture outputs as evidence. Before removing those outputs, read the script's cleanup contract.

## Maintain the feature map

This skill contains no executable helper. Use the existing repository commands named in the feature map. Keep the map as navigation to current owners. Do not copy their acceptance definitions into a second owner.

If a command or interface changes, exercise the affected path before updating its recipe. Record an unavailable native or device environment as a verification gap. Do not weaken the required journey to remove that gap. Do not schedule future upkeep unless requested.

Adapted using pstack `create-verification-skill` at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`. See [sources.json](sources.json) and [LICENSE](LICENSE).
