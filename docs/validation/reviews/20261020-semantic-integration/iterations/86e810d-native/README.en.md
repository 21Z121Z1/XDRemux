# Current-head native results and exact remaining blockers

Source **86e810dfb52cbe2516d816bc0ac11b94de84d0e7**, tree **0997860e6554abedc9999d5ea63ca7936df2e144**. No source change after86e810d.
All20 checks have finished: **{"failure": 1, "success": 19}**. Non-success contexts:
**pvt-native-import**. See the [complete check snapshot](86e810d-final-checks.json).
These are current-head results, distinct from the earlier392/af223 evidence.

| Current native evidence | Actual observed result |
| --- | --- |
| [Exact-head](86e810d-exact-observation.json) | 14/14 native subchecks, followed by completion-receipt verifier success. Full receipt artifact not independently materialized locally. |
| [Product/App](86e810d-app-observation.json) | Both App builds and all30 sequential registered model calls;32 distinct PVT package/alias combinations blocked before CLI, with source/package preservation assertions intact. |
| [System HEIF](86e810d-system-observation.json) | Synthetic HDR roundtrip1/1 and three real Tone -1/0/+1 render rows,168960 components each. |
| [Full Tone](86e810d-tone-observation.json) | 88 successful nonempty renders: five reference and three product samples x11; all three product sweeps have finite endpoint comparisons and monotonic luminance. |
| [Relocated arm64 package](86e810d-package-observation.json) | Exact source package and system-only environment/outside-checkout conversion steps passed. Full relocated artifact not independently materialized locally. |
| [PVT](86e810d-pvt-native-observation.json) | FAILED/blocked at installation-service inventory; zero installation attempts, no Photos authorization/baseline/sharing/readback. |

## PVT: actual coordinator reached, service state unknown

The [full native receipt](86e810d-pvt-native-receipt.json) and
[original log](86e810d-pvt-job119062245441.log) preserve the actual first error.
Cold SDK5.143s, reader1.036s and signing succeeded. Boot4.996s and
bootstatus88.877s exited0, but the latter reported only already booted.
The new coordinator actually ran: three read-only `simctl listapps` calls to
the exact owned UUID exceeded20s ceilings, observed25.751/20.962/21.981s.
The owned simulator's15s service-log query also timed out with empty output.
No native installation or Apple media rejection code was obtained.

`install_attempts=0`; no install was repeated or even started. Photos grant,
baseline, three whole-package representations and independent single Live
asset/original resources/pairing ID/cover/hash readback were never reached.
The late-install reconciliation branch remains behaviorally tested, not
native-accepted. Source/package hashes stayed unchanged. Shutdown/delete of
the exact owned UUID both succeeded. No decomposed PhotoKit pair fallback,
daemon reset, permission workaround, user-library write, new rerun or timeout
increase was used. The earlier uncertain af223 installation cannot be
retroactively resolved after that different simulator was deleted.

The small [artifact12325600964](https://github.com/21Z121Z1/XDRemux/actions/runs/39551734230/artifacts/12325600964)
is32102bytes; GitHub reports SHA256
`0c3757b660baecef5fff137f63dd5e744544bc9f6a6b0a17954d959e431836de`.
It retains raw runtime/device-type and query outputs. The connector returned
a reusable file reference, but this environment's supported signed-URL read
was blocked by proxy403. Bytes were not independently verified locally; no
access restriction was bypassed. Selected iOS27/iPhone11 is recorded, not
declared incompatible from its name alone. Any device-selection change must
first use the retained actual runtime compatibility facts. Whole-package
Photos admission, macOS Photos, physical device and wallpaper remain unaccepted.

## Performance: all attempts stay visible

[Current observation](86e810d-performance-observation.json),
[complete log](86e810d-performance-job119062245748.log), attempt1 only.
Native release equivalence and absolute budgets are explicitly recorded there.

- apple-styles median_wall_seconds: observed=9.152s reference=8.691s budget=11.732s
- apple-styles p95_wall_seconds: observed=9.707s reference=11.412s budget=15.406s
- apple-styles max_peak_rss_mib: observed=1083.688MiB reference=1089.094MiB budget=1361.367MiB

Same-runner diagnostic, not an alternative acceptance budget:

```json
{
  "baseline": {
    "median_wall_seconds": 9.659553125000002,
    "median_user_seconds": 9.81,
    "median_sys_seconds": 0.79,
    "max_peak_rss_bytes": 1141129216
  },
  "candidate": {
    "median_wall_seconds": 10.919372874999908,
    "median_user_seconds": 10.79,
    "median_sys_seconds": 0.96,
    "max_peak_rss_bytes": 1136328704
  },
  "candidate_over_baseline_median": 1.1304221565632628
}
```

Earlier failed attempts and the one reasoned7a retry remain in inherited
appendices. The preceding392 absolute pass9.196/9.472s coexists with
main7.920269/candidate10.543739s (+33.1235%); af223 was+17.8593%.
No threshold or baseline was relaxed; no complete ARM64/E2E speedup or full
relative-regression attribution is claimed. The bit-exact Linux lookup
prototype and actual native release equivalence are separate from E2E cost.

## PR disposition and recoverability: execution remains disabled

| PR | Exact tip | Actual state | Ancestor of candidate |
| --- | --- | --- | --- |
| #58 | `0688ea7d6270df52cc5968e2c36dd016f13bd31b` | open; draft=True; merged=False | True |
| #65 | `bc4d54753f18905dbe73f595a06a6eaf21aaa014` | open; draft=True; merged=False | True |
| #68 | `1f29e3cdc1b8fd49d56718ceecf96b8de80ec817` | open; draft=False; merged=False | True |
| #69 | `7952d0b19c6b47738cc0722acae57465a243186f` | open; draft=False; merged=False | True |
| #71 | `86e810dfb52cbe2516d816bc0ac11b94de84d0e7` | open; draft=True; merged=False | True |

Original four tips are preserved by ordinary merges plus reviewed semantic
corrections, not claimed independently green. The earlier per-feature
dispositions and architecture/source-to-check mapping remain in inherited
review appendices. [Local scoped source verification](../86e810d/README.en.md)
records4/4,308Python,46 focused and real pair/PVT/no-clobber proof; it is not
substituted for any native result above.

The [37-ref manifest](all-37-backup-manifest.json),
[full independent restoration log](all-37-backup-restore.log), and
[fresh read-only equality/fsck proof](86e810d-recovery-refresh.json) bind all9
exact remote heads, every backup commit/tree, a non-shallow independent
restore, and full strict fsck0. Main remains89d806c8. No source ref changed
while collecting these results. Protection remains strict/never-bypass,
CodeQL high/errors and thread resolution required; PR71 has no review threads.

[Convergence plan is disabled](86e810d-convergence-plan.json): parent review
of final gates/recovery must precede ordinary merge71, inspection of actual
indirect merged/automatic deletion states, durable restoration of resulting
main, then fresh exact cleanup-set/guard review. No merge/delete, force push,
release, upstream write, paid/billing, protection or global security change
occurred. No private portrait or media/executable payload is in this dossier.
