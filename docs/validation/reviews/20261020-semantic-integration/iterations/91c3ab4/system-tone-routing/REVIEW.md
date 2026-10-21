# System/Tone consumer routing and original extractor imports

Two confirmed research-boundary failures are repaired locally; native System/Tone success remains unverified until the next source-head jobs render actual product outputs.

## Actual first faults

- `/workspace/xdremux-audit/000e74b-system-job118965588063.log:860–870`: the actual fixture conversion succeeds and produces the System HEIF/report, then the separate scorer finds zero tone -1 textures. `measure_system_heif_hdr.sh` still ran the fixed sweep directly, bypassing the shared iinf compatibility repair, row completeness checks and first-fault receipt. The old System log does not retain its extractor stderr, so it does not prove the internal extraction exception.
- `/workspace/xdremux-audit/000e74b-tone-job118965588509.log:834–892`: all five Apple reference extractions fail at the real original `extract_inputs.py:8` import of sibling `heif_standalone`. This is a defect introduced by the previous owned launcher: `runpy.run_path` does not install the original script directory as `sys.path[0]`. This run failed before reaching product samples.

## Small repair in existing owners

`check_style_tone_response.py:153–215` now accepts an explicit finite, nonempty, unique subset of the existing eleven Tone values. Default execution remains all eleven. The receipt, upstream invocation, completeness checks and filtered diagnostics use the exact selected tones. `measure_system_heif_hdr.sh:34–38` calls this same owner with exactly `--tones -1 0 1`; it does not add eight unnecessary native renders.

Before any completed consumer receipt, the existing native texture loader verifies actual requested RGB textures, typed trace status observations, positive dimensions, exact byte counts and finite RGB. The owner also requires equal geometry within each photo and native/model component-count agreement. Missing or duplicate rows, unexpected tones, nonzero tools, missing buffers, nonfinite samples and incomplete comparisons remain failed operations. The System scorer's shape/luminance mathematics and thresholds were not modified. The eleven-tone analyzer and metadata comparison retain their full original request requirements.

The existing launcher now adds the **original** `research/vm-native` directory before executing the original extractor (`check_style_tone_response.py:110–114`). The extractor's own ROOT insertion then preserves direct-script precedence: original root before original script directory, ahead of an external conflicting helper. Upstream files are not rewritten. The fixed parser byte hash and checked iinf repair remain unchanged.

Only five authorized repository files changed: checker, its tests, the System shell, research workflow and its configuration tests. The workflow gives both focused-test steps the existing pinned checkout through `RESEARCH_ROOT`; this ensures the real-source integration group executes in CI. The System upload whitelist adds only `tone/consumer-run.json` and `tone/diagnostics/*.log`. Tests reject upload of media, raw traces/facts, pixel buffers and the ephemeral derived parser.

## Real fixed-source integration; no missing-env skip

Exact local source layout for final verification:

```text
RESEARCH_ROOT=/workspace/xdremux-audit/tone-pinned-checkout
  tools/heif_dump.py
  research/vm-native/extract_inputs.py
  research/vm-native/heif_standalone.py
```

All three are complete original files from revision `37dd551a949047ed14580efa0c0a2556922791b5`. The integration group's setup validates their original Git blob IDs, copies them into its own temporary layout and verifies source bytes remain unchanged afterward. No source checkout is modified.

| Original file | Git blob SHA1 | SHA256 |
| --- | --- | --- |
| `tools/heif_dump.py` | `d65da02fb2ffadc334d4d00199d063d26bf7eb94` | `24ef91b177d6346fb3952036e8e2b770c4491c7aad5bea85244a768794336e10` |
| `research/vm-native/extract_inputs.py` | `9bd353c651a93c7479af2902cbe23057dae4f720` | `faf134ce5f57620c01828ba99831b2ed4311ab380a4948c6d1527ee4b8d3497e` |
| `research/vm-native/heif_standalone.py` | `42968e5aa336b80e5d2cafade45bb65256dc5a40` | `0abb9f3a3cbba724b83c29ba2168dfb4abec01e1ccde8c77dc9830a623be9587` |

The tests actually run the original pinned extractor and real sibling helper as subprocesses on synthetic structural containers with binary plists and two linked auxiliary resources. They compare:

- v0 direct-script success and compatibility-launcher success, with byte-identical extracted plist, two standalone auxiliary wrappers and key buffers;
- v1 direct-script failure at the known Styles lookup, then compatibility recovery;
- a missing Linear Thumbnail relationship, which fails through both paths;
- an external conflicting helper, which both valid paths must ignore, and a root helper sentinel, which both must select according to the original script's search order;
- unchanged input and original source bytes.

The structural resources carry synthetic coded markers and no real image pixels. This proves Python import, extraction and container handling, not HEVC decodability, native rendering, actual System Tone shape, or Photos editing. Missing `RESEARCH_ROOT` is an explicit skip in a generic checkout; the reported run below supplied the exact root and did **not** skip either integration test.

## RED/GREEN and frozen state

RED before implementation: `/workspace/xdremux-audit/system-tone-routing/red.log`. Four methods produced two real original-extractor assertion failures (wrong external helper / missing sibling module) and nine errors (eight unsupported-tone API subcases plus missing System consumer receipt). The concrete original-source import failures reproduce the observed Tone regression; the errors also expose the bypassed/new API boundaries.

Final command:

```sh
env RESEARCH_ROOT=/workspace/xdremux-audit/tone-pinned-checkout \
  python3 -m unittest tests.test_style_tone_response tests.validation.test_workflow_configuration -v
python3 scripts/apple/check_style_tone_response.py --self-test
bash -n scripts/apple/measure_system_heif_hdr.sh scripts/apple/run_style_tone_sweep.sh
git diff --check
```

42 PASS with no skips: 21 Tone boundary tests, 2 complete fixed-source integration tests, 19 workflow tests. Self-test, shell syntax and diff check pass. Full output: `green.log`. New three-tone subprocess tests exercise complete data, missing/duplicate/unexpected rows, nonfinite/missing native buffers, geometry and component mismatch. A real POSIX System-shell test confirms a zero-exit upstream with an extractor error leaves a failed receipt and first stderr before scoring; Windows explicitly skips only that POSIX-shell test.

| Frozen repository file | SHA256 |
| --- | --- |
| `scripts/apple/check_style_tone_response.py` | `e2d45ed365576a52e346d2ef7159aa89049c75451a9d8b20367cf96a584a24b3` |
| `tests/test_style_tone_response.py` | `374b5da3643eb94c48c2fd03b2967c994ac5a3387af67ea0233d02b8c7e75366` |
| `scripts/apple/measure_system_heif_hdr.sh` | `048d716fa1f9d8ec8c46a149941507b1a01ab7f0a2c2a2a0c3e91f67ac96ee7a` |
| `.github/workflows/research.yml` | `4de441a2bfc8dde8df15a92889088e0e3ff9085f04d5ccaa7c9642292a728768` |
| `tests/validation/test_workflow_configuration.py` | `74460491b6ba4942a7f37fc5f34c3d8e457af08dbbbac2ebef85ce8daf1c3a10` |

Exact five-file patch: `frozen.patch`. No product/native/HEIF changes, score changes, plist-version changes, budget adjustments, upstream writes, commits, push, dispatch or media upload by this agent. The old failed native receipts remain failures. The next head must independently demonstrate actual product renders and complete finite measurements; any next failure must preserve its real first stderr.
