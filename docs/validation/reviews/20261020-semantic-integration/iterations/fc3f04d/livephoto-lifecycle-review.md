# Live Photo publication lifecycle review

Reviewed source HEAD: `b049b7a4bf41934bacf93514e5899567a54000ca`.
No product source, tests, Cargo metadata, CI, index or refs were changed by this
review. One-shot prototypes and retained proof are under `xdremux-audit` only.

## Demonstrated findings

### P1 · Act on: filename cleanup can delete the current source and another conversion's parent

`xdremux-runtime/src/live_photo.rs:518-526` rejects existing finals, then calls
`reconcile_live_photo_pair`. That function creates the parent and calls
`remove_stale_artifacts` (`xdremux-motion-photo/src/publication.rs:160-162`).
The latter unconditionally removes the legacy directory (101-107), then trusts
name prefixes/suffixes to identify old files or directories (116-134). It has no
operation ownership, source provenance, age, active-worker or lock evidence.

Actual CLI case: copy the public Samsung HEIF to
`P/.capture.unowned.tmp.heic/source.heic`, then convert that input to
`P/capture.heic`. Conversion exits **0**, both output members exist and independent
`validate` succeeds, but the input file and its entire parent have been deleted.
The input is read into memory before cleanup, so conversion can continue after
losing its on-disk source. Only disposable fixture copies were used.

A two-CLI interleave also reproduces the parent's exact error text: B prepares
an output under `P/.capture.unowned.tmp.heic/`, pauses before first temporary
create, and A converts to `P/capture.heic`. A's production cleanup deletes B's
parent. B resumes and fails with `Live Photo temporary create: No such file or
directory (os error 2)`.

This demonstrates a product path to parent deletion **when its name matches the
cleanup grammar**. It does not establish that this caused the macOS batch test
failure in an ordinary `xdremux-cli-batch-*` directory.

### P1 · Act on: different output names can delete active staging files

Temporary names are `.<stem>.<UUID>.tmp.heic/.mov`
(`live_photo.rs:103-115`). Cleanup checks only `starts_with(".<stem>.")`.

Actual two-CLI interleave: B converts to `P/capture.variant.heic`, writes/syncs
its image temporary, then pauses before creating its MOV temporary. A converts
to `P/capture.heic`; A's `reconcile` deletes B's active
`.capture.variant.<UUID>.tmp.heic`. B resumes, writes the movie temporary, then
fails publication with `validated Live Photo temporary pair is incomplete`.
A's resulting pair validates. Distinct final paths therefore do not establish
independent lifecycle ownership when stems overlap.

### P1 · Act on: late competitors are overwritten; failure rollback deletes a competitor's valid pair

The runtime existence check is preflight only. The publisher subsequently checks
existence again and backs up whatever is now there, then uses replacing renames
(`publication.rs:219-234`). This accepts a new competitor as replacement state,
despite the runtime's no-overwrite contract.

Two real CLI conversions, same explicit output:

1. A passes source/output validation and pauses before its first temporary create.
2. B runs to completion, exits **0**, and its pair independently validates.
3. A resumes, exits **0**, and replaces both B output hashes with its own pair.

The rollback path is more destructive (`publication.rs:238-249`): its boolean
`image_installed` proves only that A installed that name earlier. It does not
prove the name still belongs to A when unlink executes.

Two real CLI conversions plus one injected filesystem failure:

1. B passes preflight and pauses before its first temporary create.
2. A starts, installs its HEIC, and pauses before final MOV rename.
3. B resumes, publishes its own pair, exits **0**; independent `validate` succeeds.
4. A's final MOV rename is made to return `EIO`, a legitimate filesystem failure.
5. A exits **1**, removes B's final HEIC, and leaves B's exact MOV bytes behind.

This proves a failed conversion can damage another successfully published pair.
No invalid media or synthetic pairing metadata was needed.

## Checks, controls and retained evidence

Commands actually run:

```sh
cc -shared -fPIC -Wall -Wextra -Werror \
  /workspace/xdremux-audit/lifecycle_interpose.c \
  -o /workspace/xdremux-audit/lifecycle_interpose.so -ldl
python3 /workspace/xdremux-audit/lifecycle_cli_probe.py
python3 /workspace/xdremux-audit/lifecycle_competition_probe.py
```

The temporary `LD_PRELOAD` shim only schedules the specified filesystem boundary
and injects the stated error/competitor operation. The second prototype uses two
actual CLI processes for competitor publication, not synthetic product receipts.
It snapshots the existing binary so a concurrent parent rebuild cannot change
the bytes being exercised. Every child is reaped; scratch output is removed.
No workflow, external message, persistent service or interactive session starts.

Retained proof, including individual commands, exit codes, stdout/stderr and
output-state/hash observations:

- [First proof manifest](lifecycle-proof-m27chyyj/manifest.json)
- [Source-directory conversion](lifecycle-proof-m27chyyj/source-directory.stdout)
- [Parent ENOENT](lifecycle-proof-m27chyyj/removed-parent.stderr)
- [Active staging deletion](lifecycle-proof-m27chyyj/active-prefix.stderr)
- [Two-real-CLI proof manifest](lifecycle-competition-proof-b22ddlu7/manifest.json)
- [B's valid pair before A failure](lifecycle-competition-proof-b22ddlu7/rollback-b-valid-before-a-failure.stdout)
- [A's final rename error](lifecycle-competition-proof-b22ddlu7/rollback-a.stderr)

Controls:

- Finals existing at runtime entry: exit **1**, both original sentinel files kept.
- Ordinary different stems `alpha` / `beta`, including one active staging pair:
  both exit **0**, both pairs validate, B's hashes remain unchanged.
- Successful outputs from the destructive and overwrite probes validate through
  the real CLI, establishing that their success paths reached publication.
- Both proof manifests record identical source-fixture hashes before/after and
  successful scratch cleanup.

Binary SHA-256: `944301ac1ce06f322dbb4dda8eeb388be7320ee5dbbb20981b58626765c1a4c0`.
Public Samsung HEIF SHA-256:
`06eb244bc69ae464bd7b0a60b769f4fc3429dc543451481f5331586a7536b8d0`, matching
`fixtures/SHA256SUMS`. The existing binary's build-to-current-HEAD binding was
not established, so these are executable-binary plus inspected-source findings,
not an exact-HEAD completion receipt. Source HEAD and dirty state are recorded
in the second manifest. This was Linux filesystem/portable validation, not
macOS, Apple framework, Photos or device acceptance.

## Smallest coherent repair proposal — approved and implemented

1. In runtime, replace the `reconcile` call with explicit parent creation.
   Existing finals are already rejected, so the only routine effect of current
   `reconcile` is unauthenticated sidecar deletion. Stage under an operation-owned
   private directory using the existing `tempfile` dependency. Remove only that
   operation's owned staging, never anything selected by basename/glob.
2. Keep the public `reconcile_live_photo_pair` signature, but make it conservative:
   no finals → create/check parent and succeed; complete validated pair → retain
   and succeed; partial/invalid/unknown finals → error, retain bytes. Remove
   filename-based cleanup and unconditional legacy-directory/lock deletion.
3. Keep the public publication call shape and underlying I/O error chain. Replace
   backup/replacing rename/rollback-unlink with same-filesystem no-clobber
   publication, using an existing primitive (`hard_link`, or tempfile's
   `persist_noclobber`). Claim the shared MOV destination first so `.heic` and
   `.heif` variants that share one MOV cannot both win. Success requires both
   members to exist as the planned pair; preflight is not the safety mechanism.
4. If the second final entry cannot be installed, never unlink either final name.
   Retain the full owned staged pair and report the partial final plus its staged
   recovery location with the original I/O error. This is explicit recovery from
   preserved bytes, not successful conversion or two-file atomic publication.

Compatibility/tradeoffs:

- The only in-tree production caller is runtime. It already forbids replacement
  and cannot reach old automatic partial-final recovery because its initial
  existence gate rejects that state. Runtime therefore loses no reachable
  successful replacement capability.
- Public helper signatures can remain source-compatible, but helper behavior
  narrows: implicit existing-pair replacement and deletion of unproven invalid
  finals must become errors. The old unit tests currently encode both behaviors
  and need replacement with ownership/no-clobber/failure assertions.
- A private staging directory remains locally owned; orphan staging after a
  process crash is left intact, never classified as safe to delete by its name.
  Error-time full staged bytes allow explicit recovery. Automatic restoration
  of an arbitrary old partial pair would require additional durable provenance;
  it cannot safely be inferred from the present four-path API.
- Two flat directory entries cannot be committed together with the existing
  filesystem primitives. The proposed contract is per-entry no-clobber plus
  success only after both entries, and explicit recoverable failure otherwise.
  Power-loss durability would require separately specified directory syncing;
  neither the current code nor these probes establishes that guarantee.
- Cooperating writers can arbitrate through the same no-clobber MOV claim,
  without another lock lifecycle. A writer that ignores the protocol may still
  overwrite or mutate files itself; the proposed implementation never replaces
  or deletes its final names. A repeated existence check or advisory lock alone
  cannot provide that property. An inode check followed by unlink still has a
  race against a noncooperating replacement, so it is not the recommended
  rollback design.

This proposal intentionally gives up automatic deletion and immediate cleanup
of an owned partial final after a second-entry error to preserve unrelated and
competing media. If immediate removal of all partial finals is retained instead,
the contract must explicitly assume all writers cooperate with a reservation/
lock; it cannot honestly promise safety against arbitrary competing writers.


## Implemented contract and red/green evidence

Parent approved the safety contract and file scope before edits. No changes were
staged, committed or pushed by this agent. Product edits are confined to
`crates/xdremux-motion-photo/src/publication.rs`,
`crates/xdremux-runtime/src/live_photo.rs`, the existing runtime negative test,
and the English/Chinese architecture and LibLivePhoto integration contracts.

The public helper signatures are preserved. `PairPublicationError` adds a
read-only `partial_output()` accessor while retaining its original I/O source.
Its partial path records that this attempt created an entry; it does not grant
authority to unlink a path another writer may have replaced.

Runtime creates its destination explicitly and stages into its own `TempDir`.
It no longer invokes reconciliation, enumerates sibling files, or deletes any
legacy lock/transaction directory. Both staged resources use the same stem
`capture.heic` / `capture.mov`, so the retained pair is independently consumable
by CLI validation. Before the first final claim, failures drop only that private
directory. After the first claim, a typed partial error retains the whole private
directory and identifies both the final MOV and staged recovery location.

Publication atomically claims MOV with a no-clobber hard link, then links the
still. No final path is ever renamed over or unlinked, on success or failure.
The shared MOV name arbitrates both same-still-path writers and `.heic`/`.heif`
variants. Reconciliation checks only: empty or complete validated pair succeeds;
partial/invalid pair errors without deleting final entries or sidecars. Both
helpers reject identical final paths.

### Red before production edits

The production behavior was still the old implementation at parent HEAD
`ab3756746b6482da704f921de207fb12c5bc86a6`.

- `cargo test --locked -p xdremux-motion-photo --lib publication::tests`
  exited 101: **1 passed, 7 failed**. The failures demonstrate replacement of
  existing/dangling entries, acceptance of identical finals, deletion of partial
  and stale-looking user entries, rejection of owned child staging and loss of
  recoverable partial data. Log: [publication RED](lifecycle-publication-red.log).
- `cargo test --locked -p xdremux-runtime --test live_photo_negative -- --skip known_timestamp --skip sef_`
  exited 101: **0 passed, 2 failed**. The real source directory disappeared;
  old filename-derived staging failed before reaching recoverable publication.
  Log: [runtime RED](lifecycle-runtime-red.log).
- The original helper partial regression used a Unix NAME_MAX boundary. It was
  subsequently replaced with a smaller deterministic real filesystem state:
  existing competitor still + absent MOV makes the first link succeed and the
  second link fail with AlreadyExists. The runtime recovery regression remains
  Unix-only, explicitly probes legal MOV and overlong HEIC names, and asserts the
  typed partial marker so a first-link failure cannot satisfy it.

### Green

Commands use Rust 1.99.1 and the existing staged libheif/plugins on Linux, with
`CARGO_BUILD_JOBS=2` and the toolchain/pkg-config variables recorded in parent
context. They do not exercise Apple frameworks or a device.

- `cargo test --locked -p xdremux-motion-photo --lib`: **16 passed**, including
  all **9 publication tests**. [Full crate log](lifecycle-motion-photo-all-green.log),
  [publication-only log](lifecycle-publication-green.log).
- `cargo test --locked -p xdremux-runtime --test live_photo_negative -- --skip sef_fallback_with_real_sdr`:
  **5 passed**, the prior expensive SDR JPEG positive excluded. This includes
  source directory preservation, typed partial/recovery, missing video track,
  missing sample table and unmapped HDR SEF rejection.
  [Runtime log](lifecycle-runtime-green.log).
- `cargo clippy --locked -p xdremux-motion-photo -p xdremux-runtime --all-targets -- -D warnings`:
  passed. [Clippy log](lifecycle-clippy-green.log).
- `python3 -m unittest discover -s tests -p test_public_documentation.py`:
  **9 passed**. [Documentation log](lifecycle-docs-green.log).
- `cargo fmt --all --check` and `git diff --check`: passed.
  [Formatting log](lifecycle-fmt-green.log).
- `cargo build --locked -p xdremux-cli`: passed before the CLI prototypes.
  [Build log](lifecycle-cli-build-green.log).

The runtime negative test's final cleanup/readability refinement was performed
after the CLI proof; it does not change either product source file. The final
runtime negative test was rerun. The source/binary binding below is a local
uncommitted build, not a committed-HEAD or CI completion receipt.

### Real CLI green prototypes

[Green manifest](lifecycle-green-proof-ywva836u/manifest.json),
[summary log](lifecycle-green-probe.log),
[one-shot audit script](lifecycle_green_probe.py),
[audit-only interposer](lifecycle_interpose_green.c).
The manifest records the current base HEAD, dirty source paths, product source
hashes, exact commands/exits, binary SHA-256 and source fixture hashes before
and after. It copies the freshly built binary to prevent a concurrent rebuild
changing the executed bytes. All children were reaped; scratch outputs were
removed. It is disposable audit evidence, not a product runner or new CI owner.

All **12 cases** satisfied the intended observable state:

| Case | CLI exits | Observed result |
| --- | --- | --- |
| Existing final pair | 1 | Both original sentinel byte arrays preserved. |
| Post-preflight competitor pair | 1 | Both competitor members preserved; only own staging removed. |
| Source in `.capture.unowned.tmp.heic` directory | 0 | Input, its parent and exact source bytes remain; output pair validates. |
| Same-output real CLI competition | A 1, B 0 | B's strict-valid pair and both hashes preserved. |
| `.heic` / `.heif` shared MOV | A 1, B 0 | B's strict-valid pair preserved; no losing still installed. |
| Parallel prefix `capture` / `capture.variant` | 0, 0 | Active stage preserved; both outputs validate. |
| Parallel output under a stale-looking nested parent | 0, 0 | Parent/stage preserved; both outputs validate. |
| Ordinary parallel `alpha` / `beta` | 0, 0 | Both outputs validate; no staging survives success. |
| Late still-only competitor; second link AlreadyExists | 1 | Competitor still preserved; partial MOV equals staged MOV; complete stage validates directly. |
| Cooperative writer races an already claimed MOV while first writer's second link fails | 1, 1 | No replacement; first writer's complete staged pair validates; partial MOV retained. |
| Noncooperating writer replaces finals with a valid seed pair, then original second link returns EIO | 1 | Both competitor hashes preserved and validate; original full stage still validates. |
| First link returns EOPNOTSUPP | 1 | No final members, user file preserved, own stage removed; hard-link requirement and original OS error reported. |

The EIO and EOPNOTSUPP cases inject those errors at the real CLI filesystem call;
they do not represent a physically failed disk or a mounted FAT/exFAT volume.
The source HEIF remains SHA-256
`06eb244bc69ae464bd7b0a60b769f4fc3429dc543451481f5331586a7536b8d0`.

### Filesystem compatibility and recovery boundaries

This minimal fix deliberately requires same-filesystem hard links. Unsupported
filesystems fail closed; neither FAT/exFAT nor any unexecuted filesystem is marked
supported. The error context names the requirement and preserves `io::Error`.

The installed tempfile 3.27 implementation was read directly:
`src/file/imp/unix.rs:94` attempts `renameat_with(NOREPLACE)` on Linux/Apple and
falls back to hard_link + unlink; `windows.rs:92` uses MoveFileExW without replace.
`TempPath::persist_noclobber` consumes the staging pathname on success and returns
a deletion-owning guard on error. Wrapping the public helper's arbitrary input
paths would therefore violate the retained-staging contract or add incorrect
delete authority. Supporting other filesystems while retaining a complete pair
would need a separate owned copy + no-replace-move design, with its own errors,
ownership and partial-state verification. That is deferred rather than adding an
untested fallback or weakening no-clobber safety.

Two final entries are not a system-atomic pair. A partial MOV can remain after a
second-member error or crash; the operation returns failure, retains the complete
pair on normal error, and requires explicit inspection/recovery. This change
provides no automatic recovery after process death, no durable directory-entry
or power-loss guarantee, and no protection against an external writer actively
replacing/mutating paths. It does prevent this implementation from deleting that
writer's paths during its own failure cleanup. No check-inode-then-unlink race is
introduced.

The original macOS direct-conversion ENOENT observation remains unbound to these
Linux scheduling proofs. The old code demonstrably could produce the same error
by deleting a stale-looking parent. Ordinary batch temporary directory names did
not match that path; independent test directory collisions and guard lifetimes
remain separate concerns. Parent owns the macOS/native reruns and final committed
HEAD/CI gates.

Final owned-file snapshot: [SHA-256 list](lifecycle-final-files.sha256). Files are frozen for parent staging. The final clippy run includes the final test refinement.

A direct `git diff 89d806c8 ab3756746b6482da704f921de207fb12c5bc86a6 -- crates/xdremux-motion-photo/src/publication.rs` produced no output. The destructive publication helper was pre-existing on main; these P1s must not be described as newly introduced by the LibLivePhoto consumer PR.
