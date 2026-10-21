# Tone case names across actual interpreter rules

Source `3cac5e934a68a26200f77607603cbf4a84b20da3`, tree
`9177e3dd07caa36a7995ff8a83dc8e7a7d78448a`. Only the owned Tone launcher and
its portable regression changed after4275. No product media algorithm, App,
PVT/native-probe source, upstream research code or numerical acceptance changed.

The actual4275 System job118989380311 and Tone job118989380338 each failed the
same two assertions before native sweep execution. The unconditional test
expected `..heic` and `...heic` never to launch a child. The native dependency
log identifies cp315; no full sys.version was printed by that old receipt.
Python3.12 returns `.`/`..` as those stems. Newer pathlib preserves the complete
basenames because the prefix before the final dot has no non-dot character.
Both wrapper and fixed upstream use photo.stem. Therefore a launch marker alone
is not evidence of directory escape on the newer interpreter. This was a
cross-version test expectation error, not proof of a reachable product escape.

Primary reference: CPython3.15 `PurePath.stem`,
https://github.com/python/cpython/blob/3.15/Lib/pathlib/__init__.py#L408 .
The same non-dot-prefix rule is visible in3.14 source:
https://github.com/python/cpython/blob/3.14/Lib/pathlib/__init__.py#L405 .
The local interpreter is3.12.14; modeled path fixtures are explicitly not actual
execution of Python3.14/3.15 or Apple frameworks.

The maintained rule is the actual consumer case component: empty/current/parent
components, owned diagnostic/compatibility/receipt namespaces and duplicates
reject before preparing or launching the consumer. A resolved case must also
have the exact owned output directory as its parent. Receipts now record the
actual Python executable/version. Legitimate leading-dot basenames remain
permitted when that interpreter actually uses them as safe child components;
source paths and file bytes are unchanged.

The regression runs31 cases: ten names through the actual host plus explicitly
modeled legacy/modern stem rules, and one adversarial parent-component fixture.
Accepted cases must return complete finite RGB data, valid trace and comparison
metrics with an actual case directory inside the owned root; a launch marker is
not sufficient. Rejected cases retain failed receipt, no returncode/compatibility
root and no launched child assertions. The adversarial custom Path fixture is a
boundary invariant test, not a claim that ordinary Path.stem emits a separator.
Running the committed4275 launcher against this new regression gives exactly
one expected failure, that modeled parent component. Other namespace/positive
cases pass. The corrected final launcher passes all24 focused tests, including
the two complete original pinned-extractor integrations.

`tone-name-containment-old-source-red.log` is the verified old-source negative
receipt; `tone-name-actual-case-green.log` is the final focused run. An initial
separate local invocation omitted the session's OMP/MKL limits and produced two
20-second original-extractor subprocess timeouts. Its full failed log remains
`tone-name-version-unbounded-env-failure.log`. With the established session
OMP_NUM_THREADS=2/MKL_NUM_THREADS=2 and owned TMPDIR, the complete checks pass
without increasing timeouts or weakening assertions. That observation does not
prove a sole causal explanation of host scheduling. An initial audit runner's
post-result assertion inspected the wrong unittest object representation;
`tone-name-containment-red-runner-assertion-error.log` preserves it, and the
verified rerun checks the actual subtest parameters. Neither audit error is
reported as a successful final-source run.

Final committed-head seven-check verification is separate. Final macOS/native
System/Tone and other configured gates still must run against the pushed final
source. Old4275 failures remain failed; source91's real native passes do not
replace these final gates.
