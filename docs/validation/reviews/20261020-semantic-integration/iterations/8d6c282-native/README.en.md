# Exact-head native closure: whole-PVT admission remains blocked

Source **8d6c2823f015f06ca28d35ef80e4b3e7e2a4971a**, tree **24128c312feb507a95672c168c43bb1b0c07c805**. [All20 exact-head checks](8d6c282-final-checks.json)
are terminal: **19success,1failure**. No source changes or manual reruns followed
the one instrumented native PVT attempt. This dossier updates the initial
pending snapshot in the [source/recovery dossier](../8d6c282/README.en.md).

| Current-source validation | Actual evidence |
| --- | --- |
| [Strict exact-head](8d6c282-exact-observation.json) | 14/14 actual native subchecks plus receipt verifier. Full17232-byte artifact12326814848 not independently materialized locally. |
| [Product/App](8d6c282-app-observation.json) | Both builds;30 sequential registered model calls;32 distinct existing-package/ancestor/alias combinations blocked before CLI, preserving source/package bytes. |
| [System HEIF](8d6c282-system-observation.json) | HDR roundtrip1/1 and three successful nonempty Tone -1/0/+1 native renders,168960 components each. |
| [Full Tone](8d6c282-tone-observation.json) | 88 successful nonempty renders:5reference+3product x11; finite monotonic comparisons for all3products. |
| [Relocated arm64 package](8d6c282-package-observation.json) | Pinned dependencies/archive and real conversions outside checkout in system-only environment passed. Full relocated artifact not independently materialized locally. |
| [Performance](8d6c282-performance-observation.json) | Actual native release RGB8 consumer1passed/0failed/0ignored and unchanged absolute budgets passed. |
| Three-platform Rust/CLI and CodeQL | All pass in the complete source-bound check snapshot. |
| [PVT native receipt](../8d6c282/8d6c282-pvt-native-receipt.json) | FAILED/blocked at installation-service inventory, install0. No grant/baseline/whole-package sharing/independent Live asset readback. |

The App registration list is checked against this exact source's actual main;
the pass sentinel follows all30 calls.32 concrete native boundary rows are
distinct and all reject before CLI. Tone counts/finite data are extracted from
the executed rows, not inferred from a green job. Full original native console
logs accompany the observations. The [clean local4/4 receipt](../8d6c282/8d6c282-portable-receipt.json)
separately binds312Python, format, current CLI and real pair/PVT/source-preserving
roundtrip/no-clobber. Its30 focused models are not Apple acceptance.

## Diagnostic finding, actual host boundary and next valid test

The acted-on review finding was the diagnostic dependency: the old device-side
`simctl spawn log` used the same unresponsive simulator as the failed inventory.
Only the probe, four regressions and recipe changed. The new host collector
retains bounded no-follow owned file facts, selected host process names/CPU/RSS,
VM/load and owned-UUID host logd output. It does not replace readiness or product
policy. Device selection, three20s inventory budgets, at most one120s install,
installed-byte reconciliation and all media/admission assertions are unchanged.
No global reset, service mutation, permission or privilege workaround occurred.

The [actual new host observation](../8d6c282/8d6c282-pvt-native-observation.json)
preserves SDK5.917s/reader1.202s/signing success, bootstatus85.300s exit0 but only
already-booted output, followed by inventory22.841/21.892/22.038s timeouts.
Installation attempts0. Owned plist matches UUID/runtime/type with state3;
its393-byte system log contains boot/syslog startup only. Runtime metadata
directly lists the selected iPhone11 among62 supported iOS27 types.

Host load432.994/168.835/69.361 and VM page counts were observed. Host ps itself
timed out at5s, so PID/CPU state and a particular failing daemon remain unknown.
Host logd also timed out; its own client group stopped at5.259s, exit-15.
No pressure/cause is proven from one load/VM snapshot. This invocation did not
provide responsive application-service or native admission evidence. It does
not establish Photos rejection or package acceptance. Source/package hashes
were unchanged and exact-owned shutdown/delete exited0. No replay followed.

Complete-PVT URL/file representation, exact system save callback, one new Live
asset and independent original still/video bytes, pairing ID, cover time and
hash readback remain required. Decomposed PhotoKit pair creation is not a
substitute. No alternate-host import has been executed in this task. This invocation did
not provide a responsive simulator for the required path; that acceptance
remains **blocked**, not passed. The next feasible
test is the same exact probe on an available responsive arm64/macOS27/Xcode27
host, with a fresh owned simulator/library and configured project dependencies:

```bash
cargo build --locked -p xdremux-cli
python3 scripts/apple/pvt_native_import/check_native_import.py \
  --repo "$PWD" --cli "$PWD/target/debug/xdremux" \
  --expected-head 8d6c2823f015f06ca28d35ef80e4b3e7e2a4971a \
  --output-root /tmp/xdremux-pvt-proof --execute-owned-simulator
```

It still must prove the whole-package path; this command is a reproducible next
test, not an executed alternate-host success. Host Photos is untouched.
macOS Photos, physical-device Photos and wallpaper qualification remain distinct
untested paths. The old restricted artifact download was not retried/bypassed;
current host facts were read from the complete console receipt.

## Performance: exact observations, no cross-runner cause claim

Attempt1 only: Styles median9.260s/p959.937s/RSS1083.172MiB pass original
11.732s/15.406s/1361.367MiB limits. The same-runner interleaved comparison is:

```json
{
  "baseline": {
    "median_wall_seconds": 8.54638629200008,
    "median_user_seconds": 8.15,
    "median_sys_seconds": 0.77,
    "max_peak_rss_bytes": 1141506048
  },
  "candidate": {
    "median_wall_seconds": 9.598370207999892,
    "median_user_seconds": 9.07,
    "median_sys_seconds": 0.81,
    "max_peak_rss_bytes": 1135788032
  },
  "candidate_over_baseline_median": 1.1230910796747546
}
```

Candidate remains **+12.3091%**, user8.15/9.07s and sys0.77/0.81s. The earlier
86+13.0422%,392+33.1235%, all failed budgets and the one reasoned7a retry remain
in inherited raw evidence. Product bytes are unchanged by this diagnostic-only
commit. No cross-runner improvement, complete E2E speedup, host-load excuse,
full relative-cost attribution, threshold change or favorable-only retry is
claimed. The candidate-only stage profile is outside the timed comparison and
has overlapping scene spans; it is not a baseline stage comparison.

## Exact PR handling, recovery and disabled execution

| PR | Exact head | Actual state | Candidate ancestor |
| --- | --- | --- | --- |
| #58 | `0688ea7d6270df52cc5968e2c36dd016f13bd31b` | open, draft=True, merged=False | True |
| #65 | `bc4d54753f18905dbe73f595a06a6eaf21aaa014` | open, draft=True, merged=False | True |
| #68 | `1f29e3cdc1b8fd49d56718ceecf96b8de80ec817` | open, draft=False, merged=False | True |
| #69 | `7952d0b19c6b47738cc0722acae57465a243186f` | open, draft=False, merged=False | True |
| #71 | `8d6c2823f015f06ca28d35ef80e4b3e7e2a4971a` | open, draft=True, merged=False | True |

Original4tips retain ordinary merge ancestry plus reviewed semantic corrections;
their old-head CI is not relabeled independently green. Earlier feature/branch
dispositions, native-source preservation and architecture/check mappings remain
in inherited dossiers. No old PR was closed or falsely marked merged.

The [38-ref manifest](all-38-backup-manifest.json), [full restoration log](all-38-backup-restore.log)
and [fresh equality/non-shallow/full strict fsck0](8d6c282-recovery-refresh.json)
bind every remote commit/tree and all9 exact heads. Main remains89d806c8.
[Seven local worktrees](8d6c282-local-worktree-inventory.json) retain the one
preexisting dirty file unchanged. [Durable reconstruction proof](8d6c282-local-dirty-recovery.json)
matches the complete uncommitted file and patch hash31be7f073e7ac635492d010910121789faa33e8c69f3b01b91e226b4207aa358
from the current persisted tag and its ancestor, without removing local bytes.

Rules21932863 remain active/never-bypass, strict exact-head, CodeQL high/errors
and resolved threads required. PR71 remains draft and has no review threads.
The [convergence plan](8d6c282-convergence-plan.json) is disabled: valid final
acceptance and parent execution review precede ordinary merge71, actual
indirect-merged/automatic deletion inspection, durable restored backup of
resulting main, then fresh exact remaining deletion-set/guard review.
No merge/delete, force push, release, upstream write, paid/billing, protection
or global security change occurred. Nine remote heads remain; main-only
convergence is not claimed complete.
