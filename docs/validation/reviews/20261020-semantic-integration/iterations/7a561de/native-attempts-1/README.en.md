# Native attempt1 checkpoint at source7a561de

Source **7a561de9b2c6573caabae80642ba01d99e07e05f** remains unchanged. This
checkpoint preserves first-attempt results before latest-check views can hide
the failed performance attempt behind a requested retry. It is not final native
acceptance or merge/delete approval. The32-ref recovery and final portable
receipt remain in the [preceding appendix](../README.en.md).

[System HEIF job119014935686](https://github.com/21Z121Z1/XDRemux/actions/runs/39536719321/job/119014935686)
passed24 consumer regressions on the actual native host, including the repaired
case-version test, then generated three actual168960-component tone renders.
The [observation](7a561de-system-observation.json) and
[complete log](7a561de-system-job119014935686.log) retain the asymmetric response.
This is separate from full Tone11, App and PVT sharing, still pending at capture.

[Performance attempt1 job119014935484](https://github.com/21Z121Z1/XDRemux/actions/runs/39536719296/job/119014935484)
**FAILED** unchanged absolute Styles limits: median13.616s >11.732s,
p9516.015s >15.406s. Its same-runner main/candidate medians are
9.895716667/11.538215750s, ratio1.165980812, user9.89/11.39s. Memory1084.047MiB
and structural helper-launch budgets passed. [Full failed log](7a561de-performance-job119014935484.log)
and [source/binary/fixture comparison](7a561de-performance-attempt1-review.json)
remain failed evidence irrespective of any later result.

Against source4275's passing job, candidate CLI SHA256
`d9bc92d92c1070789b581f8f649def35d170e9da70ad7fec4196e98dd70cd579`,
native adapter SHA256
`d78ee2d5ed7ece698e1b0497edbcf31d787da6d740e1689cd98f2989ecabe4da`,
input fixture and validated profile output are byte-identical. Main CLI also
matches, but baseline adapter byte hashes differ; that distinction is retained.
Both logs identify the same OS/SDK/Swift/M1Virtual3-core/7GiB configuration.
Main and candidate both slow across those runs. Starting loads differ, but
load alone is not a causal proof. The persistent main/candidate relative cost
is not fixed or dismissed as host noise. Candidate-only nested stage spans
are not baseline deltas, and the earlier failed optimization is not adopted.

These concrete identities and the shared slowdown justify exactly one bounded
repeat of **only** the failed performance job at the same immutable source,
unchanged input, workflow, standard free runner and original budgets. The
[confirmed request](7a561de-performance-single-rerun-request.json) records that
action; it confirms request acceptance, not test success. Attempt2 is pending.
There will be no blind retry loop or budget relaxation. Final delivery must
include both attempts and their limits.

The [review/rules/check snapshot](7a561de-review-checks-rules-snapshot.json) is
explicitly from before the retry request:20 current-head checks, no unresolved
PR71 threads, CodeQL success, strict exact-head/protection rules unchanged and
no bypass. Those partial states are not final gating. [PR metadata readback](7a561de-pr-metadata-receipt.json)
confirms draft/unmerged current source. Main and all recovery refs remain intact.
