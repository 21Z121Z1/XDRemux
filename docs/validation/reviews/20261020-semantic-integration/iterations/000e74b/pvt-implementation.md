# Optional PVT implementation receipt

Frozen Rust/CLI source list and SHA-256: `pvt-final-files.sha256` (16 files). Machine-readable evidence: `pvt-final-manifest.json`. Parent owns App changes, HEIF changes and bilingual product docs. This agent did not edit, stage or commit those files. No commit, push or workflow dispatch was performed.

## Behavior and boundary

- `--live-photo-output pair|pvt` defaults to the existing pair behavior. The engine owns the typed intent; ordinary HDR stills and ordinary video retain their existing media policy. A mixed batch writes ordinary HDR `.heic` and Motion Photo `.pvt`. An incompatible explicit destination fails before publication.
- PVT is a directory containing a HEIF still, a MOV, and XML `metadata.plist`; `PFVideoComplementMetadataVersionKey` is a string `1`. Member names use the destination stem. The reader supports exactly one regular HEIF/JPEG still plus one regular MOV and that bounded XML recipe; it checks the actual pairing identifier and presentation metadata rather than requiring matching filenames. JPEG read support is tested using a real Apple MakerNote from pinned LibLivePhoto. The product writer explicitly rejects JPEG bytes rather than labeling them HEIC.
- The existing conversion constructs and validates the pair once. The package writer takes the parsed pair, no codec/remux provider, and copies every resource byte. Whole-file equality plus parsed identifier, exact presentation time, auxiliary resources and encoded payload equality are covered.
- All members are written and synced in an operation-owned hidden `.pvt` staging directory. `TempDir.keep()` disarms deletion before one exclusive directory rename. Linux uses `renameat2` through safe rustix; Darwin rustix maps NOREPLACE to RENAME_EXCL and does not fall back to replacing rename if its weak symbol is absent. Windows and other unsupported publication backends fail closed. Filesystems that reject the operation return the original I/O error and a typed retained-stage path. Neither success nor failure unlinks a final destination.
- A publication failure retains the complete `.pvt` staging path, which is directly readable by `xdremux validate`. A directory rename gives one visible package claim; this is not a power-loss durability guarantee or protection against a third party subsequently editing package members.
- Durable batch provenance records an optional package path only for PVT; pair checkpoint/receipt output remains unchanged. Reuse requires source hash, requested container, member paths, metadata shape and actual matching asset identity. Unknown or tampered packages are preserved and rejected, never adopted or replaced.
- Convert/batch/categorize discovery treats existing PVT packages as opaque, rejects explicit packages/members, and prunes recursive scans. Runtime output and checkpoint owners reject existing package ancestors, including symlink aliases and relative output from a package working directory. This scope adds output/validation support, not PVT conversion input or package migration.

## Executed evidence

Linux, Rust toolchain and libheif/plugins under `/tmp/xdremux-toolchain`; `CARGO_BUILD_JOBS=2`.

| Evidence | Result |
| --- | --- |
| `pvt-cli-red.log` | Baseline parser/router tests compiled; 2 failed because the option was absent, 1 preservation check passed. |
| `pvt-categorize-red.log` | Existing categorizer copied 2 inputs instead of pruning the package to 1 outside input. |
| `pvt-cwd-red.log` | Relative output from inside an existing package incorrectly succeeded before the parent normalization fix. |
| `pvt-core-green.log` | Engine 94 and runtime 36 unit tests passed, including 8 package/filesystem tests. |
| `pvt-cli-final-green.log` | All 8 PVT tests passed; tests primarily use `run_from`; the working-directory test spawns the actual binary. |
| `pvt-cli-regression-green.log` | 56 selected CLI tests passed, 2 slow repeated JPEG tests explicitly filtered; locale tests use actual OS subprocesses and verify byte-identical JSON. |
| `pvt-legacy-negative.log` | 5 legacy Live Photo negatives passed; slow SDR JPEG conversion explicitly filtered. |
| `pvt-clippy.log`, `pvt-fmt.log` | All-target engine/runtime/CLI clippy with `-D warnings`, workspace formatting, and `git diff --check` passed. |
| `pvt-bound-build.json` | `cargo build --locked -p xdremux-cli` exit 0; all recorded Rust/Cargo sources stable before/after build. |
| `pvt-cli-proof-t6crm39t/manifest.json` | Five real binary publication case groups passed, all frozen owned source hashes match the bound build, all fixture bytes unchanged. |

The final OS-process prototype used binary SHA-256 `a1df49511eea37af489f97d219da83b690666200712e883025be031556a740f0`. Its one-shot audited scheduler pauses before the first owned image member is opened, after conversion preflight, without changing the product binary. Two cooperating publishers to one destination yield a complete preserved winner, loser exit 1, and a separately valid retained package. Different destinations both succeed. A file, empty directory or dangling symlink appearing after preflight is preserved; each failure retains a package that the actual CLI validates. Independent Python `plistlib` readback confirms the string `1`. The fixture, product-source and binary hashes match before/after execution.

The earlier `pvt-cli-proof-ajv6r_4m` also passed but precedes final categorize/CWD changes. A later attempted snapshot was deliberately rejected before execution because concurrent HEIF source edits no longer matched its bound build; the final manifest above replaced that attempted binding. The initial broad CLI run was deliberately terminated during repeated ColorOS JPEG encoding and is not a full-suite PASS receipt.

## Remaining evidence limits

The code and tests are frozen on an uncommitted worktree based on `c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5`, alongside other owners' changes. The parent must commit and run the exact committed-head gate. This Linux receipt does not execute the Darwin backend or Windows negative cfg, and does not prove Apple Photos import, native preview behavior or dynamic wallpaper usability. The inaccessible historical Library sample and the independently inspected public makelive source remain distinct evidence; no historical sample bytes or unverified plist keys were invented, and no public implementation code was copied into product source.
