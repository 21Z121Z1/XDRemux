# PVT, native Texture carrier and Tone consumer integration

Source **000e74b329d7f4f37a60da3523ccec1887e29469**, tree
`f738fcbdd67254195e3933d749a53c6cec57b47b`. This is a tested portable candidate,
not completed native acceptance. Merge/deletion remain **NOT APPROVED**.

## Changes and review evidence

| Demonstrated problem | Repair | Actual evidence and boundary |
| --- | --- | --- |
| Optional Live Photo packaging was absent; recursively treating a package as ordinary files could split it or mutate its contents. | `587f253`: typed default-pair/optional-PVT intent, common validated pair construction, real three-member directory with string-version plist, exclusive directory publication and recoverable complete stage. CLI/App agree; scanners, output/checkpoint owners, aliases and package CWD preserve existing packages. | [Integration review](pvt-parent-review.md), [Rust/CLI freeze](pvt-final-manifest.json), [App freeze](app-pvt-FREEZE.json). Actual binary race proof has five case groups; its earlier dirty build is recorded rather than relabeled final HEAD. Native Photos import/wallpaper not established. |
| A temporary native codec can carry a distinct alpha in addition to the measured skin matte. The old untyped count also accepted alpha-only as a mask. | `711213c`: select exactly one strict native skin carrier by auxC/auxl; independently validate/discard only the disposable codec primary/known alpha graph. Keep unknown/ambiguous rejection and original source resources. | [HEIF review](texture-native-skin-carrier-review.txt), real RED and 61 unit +2 fail-closed GREEN. Source alpha/gain/tmap payloads, raw properties, flags/order and references remain exact. The extra resource in the original native failure was not captured, so alpha identity remains an inference pending the new native run. |
| c379 now exposes the real Tone extractor failure: the pinned research parser reads a legal iinf v1 u32 count as u16 and loses all item declarations. | `6d24577`: check the complete pinned source and exact patch hashes, derive only that parser function in an owned extraction launcher, retain original sweep/native/MSL execution and all missing-data gates. No upstream or product metadata change. | [Full parser/real Rust writer experiment](tone-iinf-probe/REVIEW.md), [byte and role facts](tone-iinf-probe/complete-parser-proof.json), 37 focused tests. Original Styles payload/ref survives; fixed parser recovers 3 items and one linked Styles item. Actual native rendering is still required. |
| c379 Styles exceeds its absolute budget; a broad scene span cannot distinguish reconstruction from reduction. | `000e74b`: two opt-in nested timings, with complete/finite/unique receipts and explicit overlap. Formal samples, numerical algorithms and budgets stay unchanged. | [Performance review](000e74b-performance-profile-review.md), [old failed metrics](c379aa7-performance-summary.json), [cross-job/source review](c379-performance-readonly-review.md). No performance fix or new native timing is claimed. |

The historical Snoopy sample remained inaccessible. Independently inspected
makelive format evidence is linked in the [previous appendix](../c379aa7/pvt-output-research.public.md).
This is not byte-level reuse of that missing sample. Wrapping the validated
pair performs no additional codec/remux operation; the preceding conversion
can still transcode a still or report omitted vendor resources.

## Committed-head verification

All7 checks in [the exact committed-head receipt](000e74b-portable-receipt.json)
passed, and positional `agent_completion_gate.py verify` exited0. The
[plan](pvt-texture-tone-portable-plan.json) runs locked metadata, fmt, workspace
all-target clippy, workspace all-target regressions, all275 Python tests,
current CLI build, and real pair/PVT conversion, structural readback, byte-exact
unwrapping and duplicate rejection. [The final CLI proof](final-head-cli-proof/manifest.json)
records a clean source head, binary/fixture identity, actual commands, preserved
source and surviving proof after scratch cleanup. Its recipe conservatively
labels its binary unbound; the immediately preceding current-HEAD build and
completion receipt provide the separate build evidence.

The Linux workspace command explicitly filters two repeated JPEG CLI encodes:
`live_photo_batch_reports_direct_conversion_omissions_and_preserves_source_media`
and `convert_command_executes_full_rust_jpeg_ultrahdr_live_photo_pipeline`.
This is not an unfiltered Rust-suite claim. The native product workflow keeps
its full suite, dedicated vendor CLI routes, App builds and all30 model calls.
The earlier deliberately terminated broad PVT run and failed recipe sequencing
are retained, not counted as passes. Darwin publication, Windows cfg, GPU,
native System HEIF/Tone/App, performance and relocated arm64 packaging require
their own actual new-head jobs. Photos import and wallpaper remain separate.

## Previous native observations and new runs

The [completed c379 snapshot](c379aa7-completed-native-observation.json) and raw
logs retain its actual results: product/App and exact-head passed, including
both App builds and 25 model calls; arm64/macOS27 relocated package and all3
portable hosts passed. System HEIF's small HDR roundtrip passed but the actual
Styles fixture failed the untyped mask count. Tone failed extraction and the
absolute Styles budget failed. These are not new-head acceptance receipts.

Automatic **000e74b** runs were queued/running at [capture](000e74b-initial-workflows.json):
[product/App](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326147),
[exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326263),
[research System HEIF/Tone](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326078),
[performance](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326241),
[portable core/arm64 package](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326082),
[real ProXDR fixtures](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326105).
No duplicate dispatch or interrupted preceding native diagnosis was used.

## Follow-up native failures observed before publication

[The follow-up receipt](000e74b-followup-observations.json) preserves three
distinct faults. macOS PVT resume compared /var and /private/var strings for
the same directory; a real Linux symlink alias reproduces that RED, and local
commit7c7773a now verifies canonical identity and every member byte (all8 PVT
tests pass). This is not a new native pass. Product Rust testing hit the same
assertion, so both App steps were explicitly skipped; all30 new model calls
still require execution after the repair.

The real System fixture now converts through Texture, but its old measurement
shell bypasses the shared consumer adapter and reaches the scorer with no
tone-1 render. That path did not export its internal extractor stderr; its
precise first internal exception is not invented. Separately, the main Tone
job exposes a new launcher regression: runpy omitted the original script's
directory from module search, so all5 Apple references cannot import
heif_standalone and product sweep is never reached. The prior stub tests did
not cover this integration. Both consumer repairs are in progress using the
same boundary and the actual pinned extractor/helper. Strict missing-data
failure stays in place; no upstream or legal product container is changed.
The performance job is still running at this follow-up capture.

## Recovery and convergence gate

Both711 and final000e have persistent backup tags with independent exact
commit/tree restore and full fsck0; final000e was backed up before the ordinary
source fast-forward. [The fresh25-ref manifest](all-25-backup-manifest.json)
and [restore log](all-25-backup-restore.log) prove a non-shallow complete restore
and cover all9 current remote heads. [The source push record](integration-source-push-000e74b.json)
shows the other8 heads unchanged. Prior v7/v8/v9 evidence and the original PR
heads remain reachable through their durable refs.

The proposed action is still one ordinary merge commit for#71, expected head
000e74b, only after complete new-head functional gates, fresh rules/reviews/heads
and explicit parent execution approval. The four original PR tips remain
ancestors. Inspect actual indirect merged states and automatic source deletion,
then approve a refreshed exact remaining-ref cleanup set. Do not reuse old
disabled hook/operation manifests. Unknown unique research commits remain
recoverable in tags; original CI failures are not relabeled green. No source
branch was deleted, protection/security/billing changed, upstream written or
release published. The earlier PR metadata update returned an unretained tool
error and readback showed it unchanged; its reason remains unknown and no
alternate write route was attempted.
