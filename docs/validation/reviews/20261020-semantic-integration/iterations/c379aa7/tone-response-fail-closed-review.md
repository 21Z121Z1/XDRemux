# Tone response: reject missing measurements and retain the first fault

## Outcome and evidence limit

The local Tone owner now rejects missing, failed, partial, corrupt, or nonfinite product measurements instead of inheriting the Apple reference host gate. It retains text diagnostics and a typed failure receipt before analysis. This repairs false successful acceptance and observability. It does **not** repair or establish the underlying product Tone rendering failure.

Observed source revision after the parent committed the frozen files: `c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5` (`Fail closed on missing native Tone measurements and retain first faults`). The working tree was clean and all four file hashes below matched their pre-commit freeze. This agent did not stage, commit, push, dispatch CI, edit upstream, or change native product metadata.

All execution in this report is Linux Python/shell testing. No macOS renderer, Apple framework, ARM64 behavior, Photos editing admission, or device acceptance was executed here.

## Proven defect and actual first failure boundary

Saved job log: `/workspace/xdremux-audit/4b-tone-job118919132030.log`, job `118919132030`.

- Lines 1052–1054 show successful CLI conversions of the three requested XDRemux fixture outputs.
- Lines 1063–1094 show missing native textures, `None` Tone luminance statistics, and three `extract_inputs returncode 1` rows.
- Lines 1098 onward nevertheless report an Apple asymmetry range and product rows with `None` values. The job mechanically succeeded.

The pinned consumer appends an error row and continues after a failed extractor, then writes the sweep JSON without a failing exit. The old local checker admitted the product from the Apple host gate and treated missing product measurements as concerns. This allowed reference data to mask the absence of product data. The immutable-baseline regression below reproduces that acceptance defect.

The first **known** product failure is `extract_inputs.py` exiting 1 for all three product outputs. Its internal stderr/exception was not exported by the old job; the saved job log cannot identify the first failing Python expression or HEIF precondition. Source inspection alone does not establish a version, layout, linkage, parser, or product-metadata cause. No version 15→16 substitution, upstream consumer edit, or invented Tone/Color interpretation was made.

## Fixed consumer contract, read-only source evidence

Repository: `21Z121Z1/Photographic_Style_Research`; exact ref: `37dd551a949047ed14580efa0c0a2556922791b5`.

Snapshots and source/blob provenance are retained under `/workspace/xdremux-audit/tone-consumer-37dd551/`. These were read-only file fetches, with no upstream checkout or mutation.

1. `consumer-model/harness/run_tone_sweep.py:32` captures each subprocess's stdout and stderr into its stage log. Lines 49–50 append an extractor error and continue. Lines 56–57 record the renderer process return code; lines 59–68 compute model/native comparison metrics from real half-float RGB buffers. Line 73 prints only the comparison/error row; line 74 writes the sweep JSON and does not derive a failing exit from failed rows.
2. `research/vm-native/extract_inputs.py:53` writes a pretty styles-summary **file**. Lines 58 and 67 construct/write inventory; line 68 prints `json.dumps(inventory)` on one line. The diagnostic copy excludes that inventory line, while preserving traceback and stderr. It does not upload the summary, container, inventory, or raw resources.
3. `consumer-model/harness/native_renderer_trace.m:112–116` calls the original process implementation and records `process_status` and `finish_status`. Line 155 returns the process status from an internal interposed callback, **not the executable main**. The source proves those integer fields exist in a produced trace; it does not prove both private success domains equal zero.
4. `consumer-model/harness/native_filter_probe.m:234–238` catches exceptions into `facts.json`, prints `facts.description` as a multiline dictionary, and returns 6 for the top-level exception or 0 otherwise. The per-tone log therefore cannot be uploaded unfiltered. The current diagnostic copy removes that final multiline dictionary, retaining preceding stderr and the fixed `Trace exception:`/`NATIVE ASSERTION:` stderr tags. Only typed exception strings and a bool-or-null `use_style_engine` field are copied from the local facts file into the receipt.

Consequently the new reader requires typed integer process/finish observations but does not invent `== 0` status rules. It separately requires actual tool return code 0, valid nonempty native textures, finite RGB, and all required comparison fields. Existing product-native GPU receipt gates are unchanged.

## Minimal implementation

`scripts/apple/check_style_tone_response.py` remains the owner; there is no additional general runner or result ledger.

- `run_consumer` invokes the pinned consumer unchanged, with the existing eleven Tone values and explicit requested photos. It creates a fresh owned output directory and writes `consumer-run.json` before launch. Receipt fields include the request, command, consumer script hash, process code, measured wall time, diagnostics hashes/omission counts, and errors.
- Nonzero exit, missing or invalid sweep, missing/duplicate/unexpected photo-Tone rows, absent metrics, malformed status/texture shape, truncated buffers, and nonfinite RGB all fail. Exceptions preserve a failed receipt. Timeout/cancellation reaps the owned subprocess/session; cancellation propagates.
- Known build/extract/tone text diagnostics are copied before scoring. Unfiltered trace/facts/pixels dictionaries, images, HEIC, and raw buffers stay runner-local. Direct consumer stdout is limited by the fixed script to metric/error rows.
- `analyze` requires the completed request receipt, every requested photo at all eleven tones, actual native texture data, consistent geometry, and model comparison component counts. A trusted reference cannot replace any product render.
- Signed zero is preserved: the upstream metric counts half-bit differences, while PSNR uses numerical subtraction. Thus nonzero differing bits with zero numerical error may legitimately have infinite PSNR. The stricter Tone-0 host gate continues to require bit equality.
- `write_asymmetry` requires completed, trusted, finite measurements for both groups and actual metadata inputs. Missing/damaged GTC/light-map data fails instead of yielding an empty comparison. Existing luminance weights, tone values, epsilon, score mathematics, HDR values, and model/native metrics are retained.
- Complete measurements do not imply the curves fall within Apple's observed range or establish Photos Tone/Color editing admission. Monotonicity, asymmetry, and range membership remain measured outputs rather than newly invented semantic acceptance rules.

`scripts/apple/run_style_tone_sweep.sh` routes both groups through that owner; existing fail-fast shell behavior stops on the failed consumer before a successful analysis can be produced.

Only the Tone job in `.github/workflows/research.yml` changed: a focused Python test step and exact upload whitelist entries for `consumer-run.json` plus `diagnostics/*.log`, under the job's existing failure-capable upload step. The whitelist excludes media, raw resource dictionaries, and pixel buffers. Failure receipts/logs remain available even when extraction prevents later reports.

## Behavioral RED and GREEN

Baseline source: `git show 4b04fb3168cdc9bf8f54d7755db94fd3677eebb6:scripts/apple/check_style_tone_response.py`, saved as `/workspace/xdremux-audit/tone-checker-baseline-4b.py`. The current tests were loaded against this separate immutable module; shared product source was never overwritten.

Reproduction:

```sh
python3 /workspace/xdremux-audit/reproduce-tone-baseline-red.py
```

Result: exit 1; 4 selected behavioral test methods, 6 assertion failures, no setup errors. They prove the baseline trusts empty product data, permits absent requested Tone/photo data, reuses present data despite failed process/comparison receipts, and accepts a failed/empty host comparison. Full output: `/workspace/xdremux-audit/tone-response-red-baseline-4b.log`. The initial broader RED against the unchanged owner is retained in `/workspace/xdremux-audit/tone-response-red.log` (8 failures and 4 errors, including missing new runner API); the immutable-baseline run is the cleaner acceptance-regression proof.

Final GREEN command, run in `/workspace/XDRemux-integration`:

```sh
python3 -m unittest tests.test_style_tone_response tests.validation.test_workflow_configuration -v
python3 scripts/apple/check_style_tone_response.py --self-test
bash -n scripts/apple/run_style_tone_sweep.sh
git diff --check
```

Result: 34 tests pass (15 focused Tone tests and 19 workflow tests); checker self-test, shell syntax, and diff check pass. Test log: `/workspace/xdremux-audit/tone-response-green.log`.

Focused tests cover real subprocess nonzero/timeout behavior and retained streams; an upstream-like zero exit with extractor error rows; omitted multiline facts/inventory; typed native exceptions; empty/partial/duplicate/malformed sweeps; failed process/model rows with stale present renders; corrupt/truncated/nonfinite native buffers; invalid/missing status observations; signed-zero PSNR; complete synthetic render measurements; and both complete and damaged metadata comparisons. Synthetic buffers verify the local acceptance logic; they are not native-render evidence.

## Frozen deliverables

| File | SHA256 |
| --- | --- |
| `scripts/apple/check_style_tone_response.py` | `97d83ecf7f0e207a5a8aae4dae6996208a16bd82306864c75a91c6ef57115d81` |
| `scripts/apple/run_style_tone_sweep.sh` | `efd9a8a1bd5d9aa028bd8d2c73ed9203b342419fc66e4621a60a38b22c1f8075` |
| `tests/test_style_tone_response.py` | `44dac4a7a28ff1e752424841fbc35eacfaa1690828f2ccac66b8a85ef3bc923a` |
| `.github/workflows/research.yml` | `aec20a8afd0ecfe38e875c96bbfba3c2218dd7d69995ab7e03fb255cb5ea58f4` |

## Next observation; remaining blocker

Real product Tone acceptance remains blocked pending the next native run. It must produce actual product native textures, finite measured Tone curves, and complete comparisons. If extraction fails again, the new receipt and filtered extract log should expose the first actual stderr/traceback. That concrete error must be reconciled with the product v15 native consumer contract before any semantic fix is proposed. An upstream hardcoded assumption, if found, does not justify declaring the product successful, rewriting fixed upstream, changing the product metadata version by guess, or replacing #70 native Texture with fabricated metadata.
