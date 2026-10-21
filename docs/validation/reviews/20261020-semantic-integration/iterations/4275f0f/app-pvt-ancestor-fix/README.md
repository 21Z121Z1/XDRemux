# App PVT ancestor boundary fix

Scope: only `apps/macos/XDRemuxApp/Sources/XDRemuxViewModel.swift` and `apps/macos/XDRemuxApp/Tests/XDRemuxAppModelTests.swift`. No commit, staging, push or workflow dispatch. `frozen-files.sha256` binds those two files.

Native RED: job `118972766877`, exact source `91c3ab45310adb05d5e9e3a3d1c23e1345542b06`, compiled both App targets and then failed the existing package-boundary assertion, exit 133. The helper and original test are unchanged between that source and the current base `04c09aac1b0cabdbd412ab10d071f74120b7bff6`. The original log does **not** identify the failed subcase, so alias resolution is a supported defect path, not a proven identification of that particular failure.

The shared helper now checks lexical package parents and individually resolves every existing node/ancestor, including the current node. New output leaves no longer require whole-path resolution to expose a package behind an existing directory alias. Both planning and preparation already consume the same helper. New `.pvt` leaves outside existing packages stay allowed. Original-file alias identity checks and their higher-priority `.overwritesInput` status are unchanged.

The existing regression retains its blocking/preparation assertions and expands to 8 named directory cases × pair/PVT × both skip settings = 32 checks. It covers direct paths, missing children, multiple missing components below directory aliases, aliases pointing inside PVT and a chain of such aliases. Each check prints the existing root and its actual Foundation resolution, full planned URL and resolution, output/queue status and `canStart`. The same bounded context accompanies failures. Tests call `startConversion()` only after asserting that it is blocked, then require failed/nonbusy state and an unchanged sentinel CLI-command log. Permitted discovery/classification is snapshotted separately; validate/convert calls are forbidden. Source bytes, root/interior member sets and bytes, and symlink destinations must remain intact.

Local executed checks:

- `python3 -m unittest tests.test_app_architecture`: 6 passed.
- `git diff --check`: passed.
- `python3 ancestor_semantics.py`: 16 modeled negative paths, 4 ordinary/new-PVT positive controls and an existing ordinary-name alias to an interior member passed using real Linux filesystem entries. Package bytes and member sets were unchanged.

The filesystem model explicitly tests two hypothetical whole-path resolver behaviors. Under the missing-leaf failure hypothesis, the old algorithm misses 12 modeled cases; under existing-prefix resolution it does not. This is **not Swift or Darwin execution**, does not establish Darwin's actual behavior and is not native GREEN. No `swift` or `xcrun` is available locally. The parent must run both Xcode builds and the full 30-entry App model harness on the new exact head; the 32 emitted case diagnostics will establish the real Foundation behavior even when all assertions pass.
