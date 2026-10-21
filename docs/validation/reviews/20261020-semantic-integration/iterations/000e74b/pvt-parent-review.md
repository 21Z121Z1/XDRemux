# Optional PVT: integration review

Local source commits: `587f2530653cfefc3f266ed4ed9df4d77ab96db6` and its verification-recipe correction `530f3ef`. Neither has been pushed at this capture. The final committed integration head still needs one clean-head receipt and its own native CI. Merge and branch deletion remain unapproved.

## Reviewed behavior

The engine owns `LivePhotoOutput::{Pair,Pvt}`; Pair remains the default. CLI convert/batch and the App forward that choice. Ordinary HDR output stays HEIF. The runtime first constructs and strictly validates the existing pair, then gives its parsed media to a private package writer with no codec/remux capability. The new wrapper therefore cannot independently re-encode resources. Whole member-byte comparisons, parsed identity/timing, auxiliary resources and MOV media payloads cover preservation of that validated pair; this does not claim that the preceding vendor conversion retains every vendor resource or never needs to transcode a still.

PVT is a directory with one still, one MOV and a bounded XML metadata.plist whose version value is string `1`. The reader supports real JPEG and HEIF pairs; the current writer requires its actual HEIF output. No ZIP suffix substitution or invented plist keys are used. The inaccessible historical Snoopy sample was not recovered. Independently inspected makelive revision `40abfa4a5bc458609ddb9087d3a2743a826103a4` supplies format evidence, not proof of historical byte reuse. Existing MOV construction already places pairing metadata at moov/meta.

## Findings addressed

| Risk | Enforced boundary | Discriminating evidence |
| --- | --- | --- |
| A second publisher or late unknown output could replace a package. | Owned destination-filesystem staging; synced members; disarm old-path cleanup before exclusive directory rename. Darwin maps to RENAME_EXCL, Linux to renameat2 NOREPLACE; unsupported publication fails closed. No final destination unlink or replacing fallback. | Runtime filesystem negatives and a real, unchanged CLI binary paused after preflight. Same target has one winner and a complete separately valid loser recovery package; different targets both succeed. Late file, empty directory and dangling link survive. |
| A structurally valid unrelated package could be adopted during resume. | Checkpoint binds source signature, container choice, actual member paths, plist shape and pair identity. Pair receipts omit the new optional field. | PVT batch resume, unknown-package, tampering and mixed-output tests. App has no such durable provenance and rejects all existing PVT destinations. |
| Recursive consumers could split package members or mutate a package through an output/CWD alias. | Convert/batch/categorize prune packages and reject explicit package/member inputs. Runtime publication/checkpoint owners reject existing package ancestors, canonical aliases and relative paths from package CWD. | Existing categorizer initially copied two inputs instead of pruning one package; CWD-relative output initially succeeded. Both reproduced as RED and were repaired. App tests include ordinary output-name symlink to a PVT and both skip settings. |
| UI selection could silently disagree with Rust or affect unrelated HDR media. | Swift forwards the typed CLI choice; only Rust-classified Motion Photos plan `.pvt`. Swift does not write media or infer package member filenames. | Existing 25 App model functions retained, five added and registered (30 calls). Six portable App architecture checks passed; native execution is still required. |
| A verification recipe could validate an incomplete unwrap or read the wrong JSON layer. | Copy both pair members before validating either; read the existing `details.value` schema. | Initial recipe execution failed with missing sibling MOV; retained `/tmp/xdremux-verify-proof-c2zxhp3t` and repaired execution `/tmp/xdremux-verify-proof-mmgifzq8` passes. The failure belongs to recipe sequencing, not product conversion. |

## Actual checks and limits

The Rust/CLI owner's frozen 16-file hashes match the committed product files. The six App hashes match its freeze. Root reviewed the production diffs, package reader/writer, common conversion split, checkpoint/planner behavior, recursive consumers, App changes and tests; no additional blocking finding remains in that scope.

Owner evidence: `pvt-final-manifest.json`, `pvt-final-files.sha256`, `pvt-implementation.md`, `app-pvt-FREEZE.json`, `app-pvt-verification.json`, and real-process `pvt-cli-proof-t6crm39t/manifest.json`. Engine94/runtime36, 56 selected CLI tests, five legacy negatives, scoped all-target clippy and format pass. The deliberately stopped initial broad CLI run and its two later repeated-JPEG exclusions are explicit; they are not a full-suite pass. Root's repaired pair/PVT/unwrap/duplicate recipe also passed against the recorded existing binary, without claiming final-HEAD binding.

Linux does not execute Darwin publication, App builds/models, Photos import or dynamic wallpaper. Windows has an explicit unsupported publisher and preservation test, which has not been executed here. Complete new-head macOS27 product CI must run App builds and all 30 model calls; portable hosts must cover their actual cfg. The one-directory exclusive claim does not promise power-loss durability or prevent a later external edit to package contents. No current Photos import or wallpaper pass is claimed.

The old c379 native product/App and exact-head jobs passed before PVT; they do not accept these commits. c379 System HEIF and Tone failures and its failed absolute Styles budget remain failures. Their independent repairs and final-head validation are still in progress.
