# Validated-parameter HDR kernel prototype

The independent candidate is **baa763e4cea14ee79953912b50c90d877ab9d352**, tree
**c0f0b24f3cd77336049da50a499a0867f46153fa**, saved under the non-triggering tag
`review/hdr-validated-kernel-20261021-v1`. It is a four-file child of unchanged
e107. It has not been put on PR71/main or run on macOS. **Speed benefit is unknown.**

The [production patch](production-source.patch) adds one private numeric kernel
and one small finite-sample validator. The public scalar entry still validates
sample, channel, then weight, in that order. Whole-image reconstruction already
validates all immutable channels and weight before its pixel loop, so it calls
the private kernel without rechecking those parameters per component. The kernel
still checks every SDR/gain sample and f64 result; the original loop still rejects
f32 overflow. The scalar path deliberately rechecks finite samples in the kernel
after its public ordering checks. No public signature, interpolation, precision,
pixel order, HDR-before-reduction, runtime policy or Apple SPI path changes.

[Source invariants](source-invariants.json) prove the gain math/f64-result guard
is byte-identical. Renaming only the reconstruction callee and removing two new
comments recovers the exact old image-function body. This includes its original
validation ordering, iteration, sampling and f32 checks. This is a source review
proof, not proof that the compiler emits fewer instructions or that Mac is faster.

## Actual Rust evidence

The frozen oracle is the exact16,424-byte non-test prefix of e107 scene_linear.rs,
SHA256 `a795cd6d0f9f85ce9d8bff9f3911926d455fa4b316cc361d28f16edfb1843053`.
The original full file is30957d37…; exact source commit/blob/hash and cut point
are in [the candidate map](prototype-map.json) and the candidate's
`tests/oracles/provenance.json`. Old validation, sampling, arithmetic and loop
live in that frozen file; they do not import the new kernel. Only unchanged
error/result/scale representation types are shared. Final Git-byte comparison
verifies the frozen prefix. rustfmt skips that frozen module.

Before modifying production code, the new differential tests and16 existing
unit tests passed against e107. That is a behavioral baseline, not a fabricated
failing-before bug reproduction. On committed candidate baa763e4, actual Linux
Rust1.99.1 **debug and release each pass17 unit +6 integration tests** (zero doc
tests), rustfmt and git diff checks. [Final commands/receipt](rust-tests/final/report.json),
[debug log](rust-tests/final/debug.log), [release log](rust-tests/final/release.log)
bind those results to the exact head/tree and source hashes.

The integration tests cover1,600 scalar boundary combinations,720 whole-image
RGB/mono/padding/resampling/channel/weight combinations, invalid data/geometry/
overflow, exact first errors at both public entries, signed zero/subnormals,
nonfinite reconstruction and finite-f64-to-f32 overflow. Valid values compare
f64/f32 bits, not tolerances; invalid results compare exact errors to e107.
The additional private-kernel unit test directly protects per-sample/result
guards. Two owned synthetic negative controls deliberately alter gamma math or
public error ordering. Each is rejected by the frozen tests with the expected
exit101; [their reports](rust-tests/negative-control-final-gamma/report.json) and
[order report](rust-tests/negative-control-final-public-order/report.json) retain
raw failure logs and patches. Neither mutation touched the candidate source.

The normal workspace `cargo test --locked --offline -p xdremux-hdr` stopped in
resolution of an unrelated LibLivePhoto Git dependency, before Rust tests. Its
[raw failure](rust-tests/before/cargo-test.log) is retained; no network checkout
or dependency rewrite was used. xdremux-hdr itself has no dependencies. The
isolated [Cargo manifest](rust-tests/isolated-Cargo.toml) points directly to the
real candidate lib/test source with the same edition2021 and unsafe-code-forbid
lint. It runs the real crate unit/integration/doc tests, but is **not a full
workspace test pass**. Absolute paths in that generated receipt describe this
execution; the maintained Rust tests and oracle use relative paths.

The cloud container initially had no Rust in PATH. Official standalone rustc,
std, cargo and rustfmt components were verified against the actual official
release manifest and materialized only in an owned workspace prefix. No remote
installer/bootstrap ran or system/global configuration changed. [Toolchain
receipts](toolchain/preparation-receipt.json), hashes, actual compiler version,
manifest bytes and formatter receipt are retained; archives/binaries are not
published. Source: [official standalone installation methods](https://forge.rust-lang.org/infra/other-installation-methods.html).

## Review, persistence and next experiment

This is an author review under the project pstack workflow, engineering
principles and review rubric; no independent agent/model session was created.
No additional demonstrated behavior regression remains in the bounded diff.
Noted limits are intentional scalar finite rechecking, compiler-hoisting
uncertainty, dependency-resolution scope and missing Mac execution. No unrelated
SIMD, numerical approximation, feature deletion or framework was introduced.

[Prototype publication/restore raw commands](prototype-publication-restore-raw/report.json)
record exact main/prototype workflow inventories before one additive tag push:
six workflows each, main-only push filters and no tag/create/workflow_run match.
The tag then restores independently at the exact commit/tree, non-shallow/no
alternates, full strict fsck0. [The all-event query](hdr-kernel-actions-none.json)
found zero Actions runs at the prototype SHA. Main/PR71 and all9 heads are
unchanged; the old dirty-file hash is preserved. No ordinary/native CI was
triggered by publishing the review candidate.

The parent's v29 raw-evidence gap is closed here: [v29 original commands](v29-publication-restore-raw/report.json),
[v29 restore/fsck stdout](evidence-v29-restore.log) and
[v29 ref/tree/index receipt](evidence-v29-publication.json) explicitly target
**6e8e77d0db1b318726007447460da619cf3551ee**, tree
**2f364eb0831a9fd2c9675099099d4fbfe925bfd9**,2189 indexed files and
SHA256SUMS `c919c2d1cfb2d68fd41dc98b459057bcb7f275cd6c88ba389c25cecb925a56a3`.
These are the original exact-v29 records, not the older v28 recovery attachment.

[The unexecuted native comparison plan](same-host-native-comparison-plan.json)
fixes baseline e107 and candidate baa763e4 in one free standard macOS27 arm64
allocation. Fresh separate CLI builds, one exact shared e107 adapter, feature
graphs/byte receipts, arm64 frozen-oracle tests and12 unprofiled ABBA conversions
are required. At most two separate locator profiles stay outside E2E timing.
All outputs are validated and original hashes preserved, with text-only uploads.
The old33bbe86/1a169 entrypoint compares e107 against main89 and **cannot run this
plan as-is**. A new bounded control/harness revision and ref must first be
implemented, fixed and reviewed; none is created or dispatched this round.
Budgets and required gates are not relaxed, and failure must retain partial data.

Artifact12367047255 remains unread independently after its prior URLError. Its
underlying reason was not retained, so no403/authorization denial is claimed;
no retry, alternate download route or native rerun occurred. Actual run192
hashes/results remain in inherited v29; its full candidate feature graph,
checkout/export manifests and build receipt are still a separate gap. That gap
does not turn these real Rust tests into Mac evidence or block this code-only
prototype. e107 remains18green/2red with all old failures. operation_enabled=false;
no merge/delete/force push, upstream/release/security/budget change occurred.
