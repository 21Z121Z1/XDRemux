# Raw recovery attachment for the final clone-object correction

The corrected code is fixed at **1a1698e47e008b688cf5c3fc492dfc307b04ae1a**,
tree **ab487dafe6f6567726b9a2166172052cf8fe347f**, ref
`refs/tags/review/semantic-integration-20261020-evidence-v26`.
[Correction and22+22 portable checks](../e1072a9-clone-object-independence/README.en.md)
are unchanged. This attachment adds no code, native case or test run.

[Original v26 fetch/restore/fsck transcript](evidence-v26-restore.log),
[publication receipt](evidence-v26-publication.json) and
[push output](evidence-v26-push.log) are public here. The transcript's `<restore>`
label denotes the independent bare repository at
`/workspace/xdremux-audit/restore-test.git`.

[Full current raw-command transcript](v26-publication-raw/commands.log) and
[structured result](v26-publication-raw/report.json) additionally retain actual
argv/cwd, stdout/stderr file paths, exit codes, deadlines and process results.
Individual stream files are preserved. The advertised remote ref, restored
commit/tree, no alternate metadata, non-shallow state and full strict fsck exit0
match exactly. The source store is independent of the product checkout. No
branch/main write or deletion was part of these operations. This attachment
avoids pretending a commit can contain a recovery log naming its own unknown SHA.

The prior v25 successful raw restoration/current local fsck and the **separate
unsuccessful empty-store fetch** remain in the correction dossier. That120s fetch
is not replayed or relabeled: its ref was absent, fsck unexecuted and cleanup
uncertain; only zombie descendants were subsequently observed. All stores and
owned fixture clones are retained.

[Resolved native-input requirements](resolved-native-input-requirements.json)
now pin the exact reviewed harness commit1a1698e47e008b688cf5c3fc492dfc307b04ae1a
and product e1072a90e31b12861dfe8b9cf25e07ce75c17c44. The six prototype CLI arguments
are source, fresh CLI/adapter paths, their build hashes, and a new text-output
path. Current `.github/workflows/performance.yml` at integration/e107 has no
dispatch inputs and does not invoke this corrected prototype. The proposed
explicit performance-only mode/product SHA/harness SHA inputs are **missing,
not implemented**. No workflow or trigger was changed. The selected environment
is Linux without native Xcode/Swift; no dispatch tool is exposed, and no dispatch
was attempted. There is no evidence of an Actions-write/server rejection. A
parent-reviewed macOS27 arm64/free-standard execution path is still required.
No product-head push, old-run rerun, PVT or alternate trigger may substitute.

Rust/Swift native compilation, macOS execution, stage-cost results and the
previous failed product gates remain outstanding. Portable22+22 checks are not
native acceptance. No runtime resource/permission/budget/security setting changed.
