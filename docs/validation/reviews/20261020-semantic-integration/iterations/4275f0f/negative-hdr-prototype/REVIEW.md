# HDR immutable-validation prototype — archived, not recommended

The Linux/x86_64 measurement did **not** demonstrate a speedup. Stop this prototype here; do not transfer its patch into the product on this evidence. It does confirm that repeated immutable validation exists in the inspected release codegen, and that this narrowly factored alternative preserves all tested output/error bytes. It does not explain the persistent main/candidate native CPU gap.

## Scope and exact input

Baseline is the HDR Rust source at `0c5789ac3891f5bbfaad629d2500abd417231ff7`, exported with `git archive` into `source0c`; baseline and candidate are isolated external workspaces and targets under this directory. The HDR crate has no external dependencies. Both use the same default release flags, rustc 1.99.1 / LLVM 23.1.1, and identical probe source. No LTO/profile overrides or RUSTFLAGS were set. Preparation uses 55 registry packages whose name/version/source/checksum tuples all occur in the original repository Cargo.lock (`verification.json`). Full commands are in `commands.txt`; all builds and tests were offline. Product HEAD advanced independently to `04c09aac1b0cabdbd412ab10d071f74120b7bff6` while this ran, and product HDR source still exactly equals the baseline export. At final verification the repository was clean. The intermediate provenance receipt records root's concurrently created untracked PVT probe; the source export is committed 0c, unaffected by that work. This agent did not change, stage, commit, push, or dispatch product work.

The public `uhdr-portrait-01.heic` SHA256 is `32a2c5df1d3ca14e64d24ad291bc68067edcaaec6a7f9fc760d331bbd6576cbc`. Decode/preparation occurs once, outside timing: its 3064×4080 primary HEIF is decoded to RGB8 and linearized using the current runtime sRGB expression; its 1532×2040 private Gain Map JPEG is decoded as RGB8; the application is derived by the exact source container/resolve/iso_gain_application_for_alternate functions. This source resolves gamma 1, zero offsets, weight 1. A separate executed source-block check confirms the extracted metadata bytes and Gain Map JPEG bytes exactly match their original source blocks, all required metadata checks are valid, and the extraction fallback conditions are false (`source-metadata-evidence.log`). `public-request.bin` is 159,389,472 bytes, SHA256 `38a8eafc02ddd8c603f722c063bd45a41a14f73b0eeea65d07b2fab6c89106fe`.

This is real source-derived kernel input, **not** the product's semantic-base primary / HEVC-reencoded Gain Map raster. It does not perform Apple native rendering, Texture, publication, or full E2E validation. No ARM64 or macOS claim follows from it.

## Code and invariant evidence

The root-provided original `.s` has outer calls to `apply_iso_gain_sample` at 3581, 3666, 3748. The scalar at 620–849 validates source/code finiteness, all five immutable channel fields including positive gamma, and weight finiteness/range on each call. Fresh baseline codegen reproduces those three call sites. LLVM has not lifted these immutable checks out of the pixel-component loop in this Linux release library build.

`candidate.patch` adds one private application helper. The outer function retains its original ordered validation of the whole SDR raster, Gain Map, all three channels, then weight. It calls the private helper after those immutable checks. The public scalar retains its original first source/code finite guard, then channel validation, then weight validation, then delegates; the private helper retains source/code finite and reconstructed-f64 finite guards. The outer f32 finite guard remains. Source arithmetic, parentheses, bilinear code interpolation, gamma/weight/offset semantics, allocation, iteration, and FP precision are unchanged. There is no new cache, unsafe path, sampling, approximation, or public API. The public scalar and private helper both retain the sample guard; public-call codegen still has that extra delegation, so this patch should not be described as cost-free API refactoring.

Candidate `.s` outer calls at 3514/3599/3681 target the private helper, whose body 3904–4025 checks sample/result finiteness and performs the original clamp → inverse gamma → log gain → exp2 → offsets sequence. Its body no longer validates immutable channel fields/weight. It remains an out-of-line call. These facts show the intended change in work, not that this work dominates time. Full assembly paths/hashes and call lines are in `codegen.json`.

## Behavioral evidence

Both isolated HDR crates pass their 16 existing unit tests in release (`*-hdr-test.log`). The deterministic cross-arm probe emits complete success dimensions/f32 or f64 bit patterns and complete Display+Debug error strings. `baseline-matrix.bin` and `candidate-matrix.bin` were read and compared as bytes, not only hashed: all **10,885,558 bytes** match (SHA256 `d7a9ba30061c9420d1bada08b3e1979e69af1212bbe747071cfa7a48adabccb7`).

The matrix contains 607 reconstruction requests (198 success / 409 error) and 66,096 public scalar requests (9,716 success / 56,380 error). It covers mono/RGB, padded strides, odd and 1-pixel dimensions, unequal raster/gain dimensions, deterministic generated inputs, negative and signed-zero/subnormal SDR values, gamma 1/0.5/2/2.4/tiny/huge, offsets, weight boundaries, clamped scalar codes, invalid dimensions/counts/channels/stride/truncation/overflow, NaN/infinities/nonpositive gamma, f64 reconstruction overflow and f32 storage overflow. Combined invalid source+Gain Map+channel/weight requests and scalar cross-products preserve the original public error precedence. This finite matrix is regression evidence, not a proof for every possible f64 input or another architecture.

The public-source reconstruction is successful in all 12 fresh process invocations (warmup + 5 measured per arm). Each serializes the complete result **after** the kernel timer. All outputs are 150,013,493 bytes with SHA256 `d22c1b504da3965bbe1be8ca8cdb47d4fcc018baee614feda9744f002f51a9fa`; 11 actual full chunk-by-chunk cross-arm comparisons cover the first baseline via the subsequent candidate comparison and every later invocation. Input and fixture hashes remain unchanged after sampling.

## Interleaved result

One warmup per arm was excluded. Each measured sample is a new process using the same input path; order rotates by pair. Kernel timing includes reconstruction validation, allocation, sampling, math and f32 storage; excludes decoding, input deserialization, result serialization, hashing and byte comparison.

| Pair | Order | Baseline kernel wall (s) | Candidate kernel wall (s) | C/B |
| --- | --- | ---: | ---: | ---: |
| 1 | C → B | 2.078117477 | 1.518363945 | 0.7306 |
| 2 | B → C | 1.341398556 | 1.046846333 | 0.7804 |
| 3 | C → B | 1.191034613 | 1.386091103 | 1.1638 |
| 4 | B → C | 1.219452830 | 1.467680364 | 1.2036 |
| 5 | C → B | 1.236143002 | 1.038568333 | 0.8402 |

Baseline kernel median **1.236143002s**, candidate **1.386091103s**, ratio **1.121303199**. Only 3 of 5 pairs favor candidate and sample ranges overlap (baseline 1.1910–2.0781s; candidate 1.0386–1.5184s). Whole-process user medians are 1.308243 / 1.240979s and sys medians 0.386296 / 0.405229s; these include untimed input parsing/output writing and must not be presented as kernel CPU timing or a proven speedup. No significance or native estimate is claimed. Raw per-sample receipts, host/flags, binary hashes and commands are in `results.json` / `measure.log`.

The candidate removes repeated validation in codegen but the bounded measurement fails the stated benefit criterion. Do not repeat runs until green or promote on instruction-count intuition. Archive the source/patch/text evidence and stop; no new CI, budgets, numerical changes or generic runner are warranted. If root later chooses a separate ARM experiment, first require a focused full-byte/error comparison on arm64, then retain existing native/E2E and absolute budget gates; this Linux result does not justify scheduling that work now or delaying PVT verification.

## Privacy and frozen evidence

Keep `public-request.bin`, `*-public-output.bin`, compiled probe binaries, source-derived media and all pixel buffers local only. They are not uploadable evidence and must not enter repository documents or artifacts. Shareable evidence here is code/patches, hashes, statistics and plain text logs. No media was uploaded.

The earlier `perf-review-000/REVIEW.md` and `evidence.json` remain byte-identical at SHA256 `35b0d72833c41dd1281d97081e91fb9abb4ddff6fd2f130088b2fc135a39f47f` and `7799879d682406955ae500c5fbec9fef96081b26497255a5e393808906f1faf9`. No source or target cleanup was performed. Root owns any later recovery/cleanup decision.
