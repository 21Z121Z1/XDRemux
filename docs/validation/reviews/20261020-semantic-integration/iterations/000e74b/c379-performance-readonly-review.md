# c379 versus 4b: Styles performance review

**Conclusion:** c379 remains a real absolute-budget failure: Styles median 12.829s > 11.732s (1.097s over); p95, RSS, and one-adapter-launch requirements pass. The evidence does not support attributing that 1.097s to the newly added HVC1/graph validation. It shows a large cross-job slowdown affecting both exact main and candidate, alongside a persistent candidate/main gap already present in 4b. The shared slowdown's mechanism is unmeasured; neither this comparison nor the initial load snapshot identifies it as host contention, throttling, scheduling, or framework behavior. No gate adjustment or successful reclassification follows.

Read-only review of exact commits `4b04fb3168cdc9bf8f54d7755db94fd3677eebb6` and `c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5`, following AGENTS and pstack review/verification. No source modifications, builds, native execution, CI dispatch, commits, or remote writes. All native figures below are read from the saved CI logs, not Linux measurements.

## Comparable inputs and measurement owners

Sources:

- `/workspace/xdremux-audit/4b-performance-job118919126778.log`, job 118919126778; product lines 747–763 and budget lines 863–886; interleaved samples 1011–1022 and validated profile 1041–1107.
- `/workspace/xdremux-audit/c379aa7-performance-job118944009773.log`, job 118944009773; product lines 803–819 and failed budget lines 919–944; arm identities 969–1065, samples 1069–1080, summary/profile 1081–1166.
- Parsed numeric/binary/command/sample evidence: `/workspace/xdremux-audit/c379-performance-readonly-evidence.json`. This is a transcription/calculation companion for these two saved logs, not a new runner or gate.

Both jobs use the unchanged `.github/workflows/performance.yml:89–176` owner: `cargo build --locked --workspace --all-targets --release`, release Swift adapter, one warmup and five product samples, the unchanged absolute budget, then the diagnostic comparison even after an absolute failure. `scripts/performance/benchmark_rust_product.py:217–233` defines Styles as `convert --apple-styles` on `fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic`. Lines 259–272 create a fresh owned output per process. The compare owner uses the same fixture and command (`compare_apple_styles_revisions.py:28,200–214`), rotates baseline/candidate order (`:160–167`), removes inherited profiling for formal samples (`:192–197`), validates all outputs outside timing (`:243–258`), then runs one extra candidate profile and validates unchanged bytes (`:261–285`). The candidate profile is not a baseline stage comparison and does not enter either formal median.

Both main arms resolve to exact `89d806c8aec961a7991eb3e6d6e2b1370aed8791`, tracked status empty; both candidates are exact/clean. Both arms/jobs record fixture SHA256 `32a2c5df1d3ca14e64d24ad291bc68067edcaaec6a7f9fc760d331bbd6576cbc`. Main builds use an owned detached worktree, independent Cargo target and Swift scratch, then the same input path as candidate. Each profile reports output 2,894,706 bytes, SHA256 `41b854e57e8954cc04465ee57b195554de8932a0b2af63e4dc3e346f401bb493`: these two particular final profile outputs are byte-identical.

The main Rust executable hash is identical across jobs: `4f774a7262f35fcb1c55d2672ff81f33caf5dc1fdc7cb675199a5aef74e9a1c7`. Main adapter hashes differ across fresh builds despite equal size (805,352 bytes); the logs do not explain that binary difference. Do not claim all main executable bytes are identical. Candidate CLI/adapter hashes differ as expected and are retained in the evidence JSON.

The normalized feature graph hashes match **across jobs for each arm**: main `dfd23f0509793a5a6d2531f09545bb2e5ce29fa6915180da8ef4f520023f6e87`; candidate `f86fe060cf4f7a668a03444fb1a042f6008245266c0547847d1501a3ea8b7e94`. Main/candidate graphs differ from one another; no claim of cross-revision feature identity is made. There is no new Cargo feature/lock/build-scope difference in 4b→c379.

Host class/toolchain match: image `xcode-27-arm64` version `20261012.0260.1`; macOS 27.0.1/26A434; VirtualMac2,1 Apple M1 (Virtual), 3 physical/logical CPUs, 7,516,192,768 bytes RAM; Xcode 27.0/27A266a, Swift 6.4.0.34.1/clang 2100.3.34.1; Rust 1.99.1 `58cf8f9c3`; Homebrew libheif 1.23.6. Worker IDs differ. Initial load snapshots are 1.50/1.78/3.64 versus 1.79/2.52/6.21, taken minutes before sampling, not measurements of resource contention during samples (4b log 7,18–20,179–196; c379 log 7,18–20,216–233). Matching versions do not prove matching effective CPU/GPU/VM service rate or identical dynamically loaded native bytes.

## Two separate effects

| Diagnostic metric | 4b | c379 | Change |
| --- | ---: | ---: | ---: |
| Main median wall | 7.912647s | 12.175517s | +53.87% |
| Candidate median wall | 9.143782s | 14.176075s | +55.04% |
| Candidate / main | 1.155591 | 1.164310 | +0.872 percentage points |
| Main median user / sys | 7.88 / 0.53s | 11.09 / 1.68s | both increase |
| Candidate median user / sys | 8.99 / 0.55s | 12.76 / 2.02s | both increase |

Every non-warmup candidate sample is slower than every main sample within its own job (4b candidate 9.115–9.387 versus main 7.817–7.972; c379 candidate 13.794–14.679 versus main 12.049–12.765). Therefore the persistent revision gap cannot be discarded as just the new job's shared slowdown. It also predates the HVC1 changes.

Conversely, a 55% candidate cross-job increase is not evidence of 55% new candidate work: the exact main CLI also increases 54%, its user time 41%, and its sys time more than triples. The ratio changes only 0.75% relative. Descriptively, applying 4b's ratio to c379's main predicts 14.069913s, just 0.106161s below c379's candidate. This is arithmetic for scale, not a causal estimate, confidence bound, or alternative budget.

The absolute product workload owner also records broad cross-job increases: LHDR 1.377→2.090s; UHDR 6.797→9.897s; OPPO LHDR 1.505→2.134s; OPPO UHDR 5.872→8.334s; Motion JPEG 6.170→9.419s; Motion HEIF 0.075→0.107s; batch 16.044→22.353s; Portrait 3.200→4.682s; Styles 9.142→12.829s. Styles grows 40.3%, within the 39.3–52.7% span of these other workloads. This corroborates a common execution-rate change, while leaving its cause unresolved. The later c379 interleaved candidate median is itself 10.5% slower than that job's earlier absolute Styles median; those are different sample windows, not interchangeable outputs of one measurement.

## HVC1 change and real stage placement

4b→c379 contains three commits: `eca4bf9` direct HEVC Gain Map validation, `2ea21ed` arm64/macOS 27 delivery scope, and `c379aa7` Tone failure handling. The runtime Styles/Texture files, engine/HDR/codecs/source, semantic graph owner, Cargo manifests/lock, performance owner, and Texture native implementation are unchanged between those exact endpoints. Tone scripts/workflow steps are outside this performance command. The native product change is `Package.swift:9` deployment target 15→27 plus a host support check in `Sources/XDRemuxAppleAdapter/main.swift:11,759`; deployment target/codegen effects are not isolated by these logs.

`crates/xdremux-heif/src/styles.rs:1004` adds a complete source Gain Map validation to the appender, replacing looser local root inference and reusing validated roots for references. `validation.rs:430–493` resolves ordered tmap roots; `:538–572` admits grid or direct hvc1; `:612–643` adds coded-role alias, dimg, and nonempty-extent checks. Those checks traverse graph metadata and extent lengths. They do not decode or encode HEVC or scan every pixel. The default benchmark still exercises the normal standard HDR base; accepting hvc1 is not itself a switch to the opt-in system HDR producer.

Stage placement in `crates/xdremux-runtime/src/apple_styles.rs`:

- The **new complete appender validation** is inside `style_assembly`: assembly at lines 411–417, source staging at 418–422, timer at 423. c379's entire stage is **0.001836083s**, including that validation, parsing, assembly and file write; 4b's is 0.001085625s. The whole c379 stage is roughly 598 times smaller than the absolute 1.097s overage. Removing this required boundary cannot be a justified 1.1s remedy.
- The existing post-semantic structural check at line 244 is inside `resource_decode`; the entire c379 stage, including two raster decodes and resource extraction, is **0.402163625s**. The final structural check at 427 and ImageIO MakerApple readback through 457 are inside `final_validation`, **0.018905958s** total. Neither total is a measured validation-only cost.
- `base_conversion` also contains pre-existing structural validations after container rewrites and the artifact boundary (`runtime/lib.rs:622,987` at c379; `heif/native.rs:1309`; `heif/canonical.rs:161,482,599,665–667`). Changes to checks in those calls are not separately timed. This prevents a strict claim that all HVC1-related overhead was measured at 1.8ms. Still, the only added full invocation lies in the measured millisecond appender stage, while the altered existing loops check graph metadata. There is no positive evidence of 1.1s new validation CPU work.

Candidate-only profiles locate the cross-job wall increase mainly in unchanged producers:

| Stage | 4b | c379 | Delta |
| --- | ---: | ---: | ---: |
| Base conversion | 2.326812s | 3.008598s | +0.681786s |
| Semantic production | 2.031062s | 3.800641s | +1.769579s |
| Resource decode | 0.225591s | 0.402164s | +0.176573s |
| Scene derivation | 1.011504s | 1.819929s | +0.808425s |
| Thumbnail encode | 0.172149s | 0.400405s | +0.228256s |
| Key-1 fit | 0.366361s | 0.457362s | +0.091001s |
| Texture generation | 2.818307s | 4.113587s | +1.295280s |
| Entire profile | 9.289624s | 14.505147s | +5.215522s |

Semantic + Texture + scene derivation account for 3.873284s, about 74.3% of the profile wall increase, with no source change in their owners across 4b→c379. Stage wall is not stage CPU time; the process user/sys totals do not identify which stage consumes it. Both profiles passed independent native validation and retained the same final output hash.

## Judgments and smallest useful next evidence

- **Act on:** retain c379's failed absolute receipt; the median gate has not passed. No threshold change or automatic retry is justified by this review.
- **Noted:** a persistent 15–16% total candidate/main cost exists in both jobs. Main→4b has real semantic algorithm changes, including the local spatial key-1 fit and ISO reconstruction/linear area reduction (`apple_styles.rs:635–802,963`). Main's original global solver and scene math differ. The existing comparison deliberately includes such changes; it does not quantify their separate costs or authorize restoring old semantics. Vision singleton scheduling is unchanged even relative to this main and is not a demonstrated cause.
- **Dismissed as supported attribution:** “the newly added complete HVC1 validation explains the 1.1s failure.” Its actual new call is inside a measured 1.836ms stage; source, ratio, and broad-workload evidence point elsewhere. Possible small costs in existing calls remain unisolated, so the review does not claim mathematically zero overhead.
- **Consider:** if distinguishing these three new commits is necessary, use the existing comparator on one authorized future native job with exact 4b as baseline and exact candidate, retaining the existing absolute gate and 5+5 order. This tests 4b→candidate directly; it needs no third-arm framework, baseline source patch, or new budget. It cannot explain main→4b's older gap on its own. No run was dispatched here.
- **Consider before a performance patch:** use a small opt-in split of an existing candidate boundary: semantic Vision requests versus ImageIO scaffold/transplant, or scene HDR reconstruction versus the two linear area reductions. Current semantic includes adapter startup/capabilities, Vision and scaffold work (`apple_styles.rs:166–236`); Texture spans many native operations and final readback (`:424–426`). Sampling one broad wall span again will not distinguish them. Preserve real native results and failures; avoid a wrapper or persistent diagnostic owner.
- **No prototype recommended yet:** `reconstruct_scene_linear_hdr` validates application channels once then calls public `apply_iso_gain_sample` per component, which repeats channel/weight checks (`hdr/scene_linear.rs:135–165,176–214`); paired SDR/HDR area reductions also repeat footprint geometry (`runtime/apple_styles.rs:788–791`, `hdr/scene_linear.rs:257–327`). These are concrete potential repeated computations, but LLVM may already hoist checks, and neither is measured as the hotspot within the 1.820s scene span. Inspect optimized code or a bounded audit-only exact-output kernel probe only after that stage split supports it. Do not drop public input checks, change accumulation order, approximate gain/area math, or remove final validation to pursue the budget.

The present evidence narrows the claim: a shared rate change and an older revision gap coexist. It does not establish why the common rate changed, whether deployment-target codegen adds a small cost, or which older revision change causes the persistent gap. Native profiling/authorized direct revision isolation remains the missing evidence; Linux inspection cannot supply it.
