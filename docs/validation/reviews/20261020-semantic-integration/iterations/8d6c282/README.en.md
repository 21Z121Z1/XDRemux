# Observe the host before changing simulator or product behavior

Source **8d6c2823f015f06ca28d35ef80e4b3e7e2a4971a**, tree
**24128c312feb507a95672c168c43bb1b0c07c805**. Only the probe, four portable
regressions and the existing verification recipe changed. Product Rust/Swift/App,
Cargo, workflows, device selection, all budgets and media/admission assertions
are byte-identical to86e810d. The current-head PVT job has executed and remains blocked; other native checks
are pending. Old-head passes are not relabeled current acceptance.

## Actual failure and corrected diagnostic dependency

[The prior exact native dossier](../86e810d-native/README.en.md) preserves all20
86checks:19success and one PVT failure. Cold SDK5.143s/reader1.036s/signing and
bootstatus succeeded, but three real owned-device inventory requests exceeded
20s ceilings, observed25.751/20.962/21.981s. Installation attempts0; authorization,
baseline, sharing and Live asset readback never ran. Original/package hashes
stayed unchanged and owned cleanup succeeded. This is not a Photos rejection.

The15s in-device `simctl spawn log` diagnostic also hung. That deciding gap
justifies changing the diagnostic source, not deleting the real readiness
check, reinstalling or extending it. After failure the new collector reads:

- Canonical-home/owned-UUID device plist and64KiB device-log tail using
  no-follow directory descriptors and bounded regular-file reads.
- A5s host process query with only selected CoreSimulator service names,
  lifecycle, CPU and RSS; no argv/environment. Its PIDs are not claimed to
  belong to this one device merely because their service names match.
- Host load and a5s VM page-count query; a single snapshot does not prove a cause.
- A5s host logd query restricted to the exact owned UUID. Its client runs in
  an owned process group with bounded termination, independently of device spawn.

Missing/denied/timed-out diagnostics remain visible and retain the original
installation-service failure. Host metadata/process success is not application
readiness, installation success or Photos admission. No global daemon, reset,
permission/security change or other device mutation is used. The three20s
inventory requests, at most one120s install, exact installed-state reconciliation,
whole-package callbacks, one Live asset/original resources/ID/cover/hash and
cleanup checks are unchanged. Existing command budgets include separately
recorded bounded client termination; they are not promises of exact wall time.

## Compatibility evidence and checks

[Parent-verified raw artifact facts](86e810d-parent-runtime-facts.json) explicitly
include iPhone11 in iOS27.0 build24A434's62 supported device types, arm64.
The raw runtime/device-type JSON identities are retained there. These are
attributed parent observations from the existing32102-byte artifact, not local
materialization. The execution environment's403 download restriction was not
retried or bypassed. Existing metadata does not support incompatibility; device
selection remains unchanged. New receipts expose selected capability facts.

The [before check](pvt-host-diagnostics-before.log) includes one actual wrong
external diagnostic-route failure and three missing-new-capability errors;
it is not a portable reproduction of an Apple service hang. All30 affected
checks pass after the change, including owned bounded file/process/VM facts,
foreign identity/alias refusal and preservation of the original error when
host queries fail. They are behavioral models, not Apple execution.

The [clean committed-head gate](8d6c282-portable-receipt.json) passes4/4:
all312 Python checks, format, current CLI build and real pair/PVT roundtrip,
source byte preservation and no-clobber. Unchanged full Rust/App/native suites
were not manually repeated locally. [The initial CI snapshot](8d6c282-ci-initial.json)
is timestamped, incomplete and not a final acceptance claim. Existing PR CI
automatically tests the new exact source; no old failed run was replayed.

## First current-head native host observation

[Current receipt](8d6c282-pvt-native-receipt.json), [observation](8d6c282-pvt-native-observation.json)
and [full log](8d6c282-pvt-job119082111612.log) bind
run39557962421/job119082111612 to this exact source. It failed at the same
actual inventory prerequisite, not at installation or Photos admission.
SDK5.917s/reader1.202s/signing succeeded; boot12.842s/bootstatus85.300s exited0.
The three20s inventory budgets elapsed22.841/21.892/22.038s; installation0.

The independent host collector actually executed. Its device plist matches the
owned UUID, runtime and type, state3. Its owned system log is393bytes, only
boot/syslog startup lines. Host load was432.994/168.835/69.361. VM query returned
page size16384 and free5301, active115022, inactive114087, speculative569,
wired136909, compressor86170 pages. These are raw counters, not a measured
memory-pressure conclusion. The host ps query itself exceeded5s, so no PID/CPU
state was obtained; host logd also exceeded5s, with its own client stopped
(exit-15,5.259s) and empty output. No particular daemon or causal mechanism is
identified. These observations establish this invocation could not provide
responsive service/native admission evidence; they do not diagnose the product,
prove media rejection or justify another blind replay.

The selected runtime facts independently retain its supported device list;
no device-selection change followed these observations. Source/package hashes
are unchanged. Own shutdown33.904s/delete17.711s both exited0. Grant, baseline,
whole-package URL/provider sharing, one new Live asset/resources/ID/cover/hash
readback did not execute. No install retry, deadline increase or service reset
is proposed. The33895-byte artifact12326109422 is identified in the observation
with GitHub's reported digest; no new download/materialization was attempted.
All relevant facts above are read directly from the complete console receipt.

## Recovery and remaining acceptance

[38-ref manifest](all-38-backup-manifest.json), [full restore log](all-38-backup-restore.log)
and the source push receipt bind all9 exact heads, every remote commit/tree,
non-shallow independent restore and full strict fsck0. The new tag was restored
before the normal source fast-forward; the other8 heads stayed unchanged.
Original four PR tips remain ancestors. Main stays89d806c8.

The [convergence plan](8d6c282-convergence-plan.json) is disabled, pending final
gates/recovery and parent execution review. No merge/delete, force push, release,
upstream write, paid/billing, protection or global security change occurred.

Prior86performance median9.152s/p959.707s passes the original absolute budget,
but same-runner main9.659553/candidate10.919373s remains+13.0422%; prior392's
+33.1235% and all other attempts/reasons remain archived. No cross-runner
improvement or complete relative-cost attribution is claimed. The observed host facts do not identify a supported lifecycle repair. Whole-PVT Photos, macOS Photos,
physical device and wallpaper remain unaccepted. If this hosted simulator
cannot answer its services, the next feasible validation is this exact probe
on an available arm64/macOS27/Xcode27 host with its own fresh simulator/library;
the same complete-package and independent readback assertions still apply.
