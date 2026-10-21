# App PVT presentation and preservation review

The six delegated App files are complete and frozen for parent review, on source HEAD `c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5` plus dirty App changes. No staging, commit, push, workflow dispatch, new session, deployment, packaging, Rust, Cargo, HEIF, root documentation, or cleanup-guard edit was performed.

## Contract and implementation

The Rust PVT owner confirmed `--live-photo-output pair|pvt`, default `pair`; ordinary HDR keeps HEIC even when the global PVT option is selected. PVT validation retains the current `live-photo` JSON shape with the outer input and explicit inner image/video paths. Batch outputs list only the outer package. App conversion continues to use the single-file `convert` entry, rather than adding a batch receipt decoder or reproducing package layout policy.

`LivePhotoOutput` is a typed `pair`/`pvt` presentation value. The default still produces `[]` from `productArgumentsForTesting`; explicit PVT selection forwards `--live-photo-output pvt`, preserving independent existing product flags. Only queue items classified by Rust as `live-photo` receive `.pvt` output names. Ordinary rows, mixed queues' HDR rows, and default pair naming retain their prior HEIC paths. Pending output paths refresh when the picker changes. Swift creates no PVT members, metadata plist, media encoding, or package assembly.

The four requested AppStrings keys and exact Chinese strings are wired to the picker and explanation. The explanation retains the unverified dynamic-wallpaper limit. The skip help also states that PVT packages are not automatically reused.

The existing thumbnail path reads the queue item's source image; this was retained. Finder reveal and output paths retain the outer `.pvt` URL. The app does not guess member names or copy Rust's PVT reader/writer rules.

If Rust classification is unavailable, the existing app fallback stays a static/unreadable row. The app does not guess Motion Photo identity or silently switch the selected container back to pair. A real Motion Photo subsequently rejected by Rust for an incompatible explicit output path remains a conversion failure.

## Preservation boundaries

An existing PVT path cannot authorize skip/adoption through `RustCLIClient.isValidOutput`, even if generic Rust validation would succeed. The App has no durable source provenance for that existing package. Output planning and execution preparation reject any pre-existing `.pvt` entry: regular file, directory, file alias, directory alias, or dangling symbolic link. The existence check explicitly probes symlink destinations instead of relying only on `fileExists`.

The root review then found one omitted current-node case: `foo.heic` could itself link directly to `existing.pvt`. The initial predicate inspected the lexical current name and the resolved path's parents, but missed the resolved current node. The repaired predicate checks both current names as well as ancestors, before validation or replacement preparation. An ordinary-name alias therefore blocks for both skip settings, without relying on a later directory replacement error. The initial review artifacts are retained under `app-pvt-before-alias-repair/`.

The former pre-prepare skip in `convertOne` was removed. Reuse decisions now pass through preparation, so an existing valid HEIC inside a package cannot be adopted before the package boundary is checked. Existing ordinary skip and pair replacement contracts remain in their original preparation logic.

The initial review found two additional reachable preservation problems, reported to and accepted by the parent:

- Directory import would recurse into an existing PVT and treat its still image as a new input. PVT roots, explicit members, and subdirectories are now opaque to import. The scanner skips the directory descendants, including an ordinary-looking alias to a package.
- The output picker can select an existing PVT as an output directory. Both pair and PVT output plans/preparation now reject paths beneath a `.pvt` ancestor, including nested output directories and resolved aliases. This prevents adding `foo.heic` or another package to an unknown package. The same name boundary also rejects a not-yet-created `.pvt` ancestor; it does not create a general filesystem policy framework.

Queued-original path/inode identity protection remains in place. Existing packages, symlinks, members and source bytes are left untouched by the App's planning/preparation. Final media validation and no-clobber package publication remain owned by Rust.

## Regression coverage and actual execution

All 25 old model-test functions remain. Five new functions are registered in the custom `@main` runner:

1. Default empty arguments, explicit typed forwarding, mixed live/static names, existing category paths, and switching back to pair.
2. Process forwarding for a serialized mixed queue with spaces in input/output paths; only the test CLI writes the package marker. Both originals remain unchanged.
3. Both skip settings across file/directory/file-link/directory-link/dangling-link PVT entries, and an ordinary `foo.heic` output linked directly to `existing.pvt`; no validate/convert calls, no adoption/replacement, exact member sets, exact marker bytes, unchanged links and queued-original aliases.
4. Opaque PVT discovery and blocked output directories beneath the package, nested directories and aliases; exact member count/bytes remain unchanged for pair and PVT choices.
5. Actual ImageIO thumbnail loading from a source image while the outer PVT directory is empty; the outer reveal URL and directory contents remain unchanged.

These Swift model tests have **not been compiled or executed here**. The host is Linux, and neither `swift` nor `xcodebuild` is installed. No failing-before/passing-after Swift execution is claimed. The test CLI is a transport fixture, not proof of PVT media validity or wallpaper support.

Actually executed:

```sh
python3 -m unittest tests.test_app_architecture -v
git diff --check -- <the six App files>
```

All six existing architecture checks passed; whitespace validation passed. A portable inventory check confirmed all 25 prior model functions remain and all 30 current functions are registered in the custom runner. That inventory is a structural registration check, not a Swift behavior pass. No extra static test was added to impersonate native behavior, and the Rust agent's tests were not rerun.

`app-pvt-verification.json` records exact commands, exits, HEAD/dirty state, unavailable tools and SHA-256 of the six files. `app-pvt-static-0.log`, `app-pvt-static-1.log`, `app-pvt-model-test-inventory.json`, and `app-pvt-change.patch` are the inspectable evidence.

## Author review and remaining gate

The scoped workflow, engineering principles, TDD limitations, verification and review guidance were applied. The two initial package-preservation findings and the root review's resolved-current-node finding were acted on within the delegated owner. Raw CLI flags remain the canonical boundary; no Swift media or PVT metadata implementation was introduced. The arm64/macOS 27 deployment and help contracts remain intact.

The parent must commit the combined source and run the existing App build/model scheme on the new exact head. The local launcher for that native check is `apps/macos/XDRemuxApp/scripts/verify_model_tests.sh`, which builds `XDRemuxAppModelTests` with Xcode and runs its executable. The app itself still needs its `XDRemuxApp` build. Native PVT conversion/validation is the Rust owner's evidence; Photos, dynamic wallpaper and UI interaction claims need their own environment. Prior CI on c379aa7 does not validate these new dirty App edits.
