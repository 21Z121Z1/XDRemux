# Exact e107 terminal evidence and a bounded attribution prototype

Source/harness **e1072a90e31b12861dfe8b9cf25e07ce75c17c44**, tree
**209f1b5abc6faed76b65ebf8fcd093f9973465fa**. All20 current-head checks are
terminal: **18 success /2 failure**. This supersedes the v23 initial running
snapshot and pending plan, which remain historical records.

[The terminal check list](e1072a9-ci-terminal.json) records exact job URLs and
completion times. Current raw exact-head, product, Tone, System HEIF and arm64
relocation logs are retained here. The exact gate actually passed14 checks;
App model execution reached its final success marker and32 package-boundary
cases; Tone includes real reference/product renders, and System HEIF completed
its fixture sweep. These successes do not promote the failed PVT/performance
checks. Product delivery remains arm64/macOS27+, with portable Rust retained.

## PVT stopped before the product probe

[Original job119111773529 log](e1072a9-pvt-job119111773529.log) and the
[JSON extracted from its printed receipt](e1072a9-pvt-log-receipt.json) show:

* Harness e107 and pinned product/probe8d6 are exact clean checkouts; all7 product
  Git trees and original probe hashes match.
* One owned control device was created using the actual supported runtime/type.
  Bootstatus returned0 after152.202s with only “Device already booted”.
* The sole20s app query timed out at20.157s. Shutdown60.215s, delete60.507s and
  cleanup verification21.286s also timed out. **cleanup_status=uncertain**;
  successful client-process-group termination does not prove device deletion.
* **product_probe_invocations=0**. No product compilation, synthetic-media
  generation, install, authorization, Photos admission or independent asset
  readback occurred. There are no newly generated media hashes to claim here.

This is an environment/service blocker, not evidence that Photos accepts or
rejects PVT. No control or full-probe repeat was started during this turn. A new
experiment requires new healthy isolated-environment/service-recovery evidence.
The complete-package import and independent one Live asset/resource hashes,
pair ID and cover-time assertions remain outstanding. Decomposed PhotoKit pair
creation, macOS Photos, physical device and wallpaper remain separate claims.

Artifact12330064787 is11302bytes, API digest
`557147ca06b688a9a331bfed6b07a122e282754abaad2dbd2d0cf47fce98f2e1`,
expires2026-10-22T08:26:46Z. The connector returned a reusable download reference,
but this execution environment's one download attempt returned proxy Tunnel403.
[Acquisition receipt](e1072a9-pvt-artifact-acquisition.json) and
[artifact API metadata](e1072a9-research-artifacts.json) are retained. No ZIP bytes
were acquired or verified here; the API digest is not a local checksum claim.
No signed URL, access bypass, global service reset or host permission change is
included. The full printed receipt and original job log are already durable.

## Performance remains failed, with relative CPU cost

[Original job119111773150 log](e1072a9-performance-job119111773150.log) and
[parsed terminal facts](e1072a9-performance-terminal.json) retain attempt1:

| Measurement | main | candidate | Result |
| --- | ---: | ---: | --- |
| Absolute Styles median | — |14.458s | limit11.732s, failed |
| Absolute Styles p95 | — |16.156s | limit15.406s, failed |
| Same-host ABBA median wall |12.434200333s |14.704374375s |+18.2574993% |
| Same-host median user CPU |11.70s |13.29s |+1.59s |
| Same-host median system CPU |1.83s |1.99s |+0.16s |

RSS remained within the existing limit. Both exact build arms, adapter/CLI
hashes and feature graphs are in [the build receipt](e1072a9-performance-build-arms.json).
The12 interleaved rows include2 warmups. This turn did not rerun the failed
job, increase a threshold, discard attempts, or infer a cause from another
runner. Earlier failures,7a's one documented retry and relative costs including
33.1235%,13.0422% and12.3091% remain in inherited dossiers.

The one additional candidate profile measured base conversion3.628s,
semantic production3.226s, scene derivation1.337s and Texture4.003s. Its
scene HDR0.961s and area-reduction0.176s spans are nested in scene derivation;
they must not be added again. These candidate-only observations prioritize
investigation but cannot identify a main/candidate stage delta. In particular,
the full-resolution HDR-before-reduction contract cannot be removed to recover
the legacy speed. No optimization or complete attribution is claimed.

## Attribution prototype, not a native result

[Executable prototype](e107-stage-attribution-prototype.py) reuses the existing
comparison owner's process, reference-build, fixture and CPU helpers. It pins
main89/e107 and adds only13 reversible timer calls plus the same timer helper
to an owned disposable main worktree. The checked
[patch](e107-stage-baseline-timers.patch) removes back to exact original bytes.
Candidate code and both real adapters remain unchanged. Each build has its own
target; only the owned worktree can be removed. It refuses changed revisions,
unexpected mutations, confounding system-gain-map opt-in, different supplied
binary hashes, missing/duplicate stages, unsuccessful native readback or output
hash mutation. One warmup plus5 alternating samples per arm follows the existing
order, with a90minute overall ceiling and bounded subprocess groups. Native
validation occurs after timing. Only13 common non-overlapping spans are
compared; candidate's nested details remain descriptive. Per-stage medians are
not added to assert E2E attribution. The formal budget/previous ABBA is untouched.

[Five portable checks passed](e107-stage-prototype-tests.log), including real
fixed-source insertion/removal and an actual Linux CLI refusal before builds.
No failing-before product defect was reproduced. A local build setup attempt
found no cargo executable in this restored container and cleaned its owned
worktree; [receipt](e107-stage-prototype-compile-receipt.json). This module is
macOS-cfg-only, so a Linux cargo check would not prove its compilation anyway;
[host prerequisite](e107-stage-prototype-host-prerequisite.json). **The prototype
is uncompiled/unexecuted on macOS and supplies no stage comparison result.**

No workflow-dispatch connector is exposed. A harmless `gh api repos/...` read
returned Forbidden, so no dispatch POST or bypass was attempted. Pushing a new
PR head would also retrigger the currently blocked PVT control; no such push was
made. The next native step is one performance-only allocation, with the fixed
prototype/source hashes, fresh build receipts and unchanged absolute gate. It
must not replay PVT without its new environment prerequisite. Example after
building exact e107 on the existing free standard arm64/macOS27 runner:

```sh
python3 e107-stage-attribution-prototype.py --source /path/to/clean/e107 \
  --cli /path/to/e107/target/release/xdremux \
  --apple-adapter /path/to/e107/xdremux-apple-adapter \
  --cli-sha256 <hash-from-that-build-receipt> \
  --adapter-sha256 <hash-from-that-build-receipt> \
  --output-dir /fresh/owned/stage-evidence
```

This is an archived diagnostic prototype, not a second product implementation,
verification framework or CI admission rule. Its [author review](e107-stage-prototype-author-review.json)
names limits and the native compile prerequisite. No generic abstraction or
feature/validation deletion was introduced for simplicity.

## Fresh recovery and disabled convergence

[39ref/9head manifest](e1072a9-terminal-39-ref-manifest.json) and
[fresh independent restore log](e1072a9-terminal-restore.log) verify all remote
commit/tree mappings, a non-shallow repository and full strict fsck0. Existing
heads are unchanged. The [local inventory](e1072a9-terminal-local-inventory.json)
and [dirty reconstruction](e1072a9-terminal-local-dirty-recovery.json) confirm
the one preexisting dirty file/patch against the durable tag without applying
anything. The prototype's temporary worktree was removed successfully.

[PR/default settings/rules snapshot](e1072a9-terminal-pr-rules-snapshot.json)
shows5 open PRs, main89, PR71draft/e107, no review threads, strict ruleset21932863
with no bypass, and automatic source deletion on merge still enabled. An
independent running-workflow query returned none. These are timestamped
observations, not a lock against later writers. No protection setting changed.

[The corrected terminal plan](e1072a9-terminal-convergence-plan.json) has actual
same-directory evidence targets, names18/2 and both blockers, and keeps
**operation_enabled=false**. Merge/delete still require fresh final gates,
head/writer/protection/backup checks and parent execution review. Four original
PR tips remain ancestors, not independently relabeled green or merged. Main is
unchanged; no merge, branch deletion, force push, release or upstream write
occurred. This dossier's review tag is an additive recovery/evidence publication.
