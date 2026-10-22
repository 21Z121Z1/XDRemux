---
name: pstack-tdd
description: Fix an XDRemux bug with a focused failing-before, passing-after regression when the user requests TDD or the affected behavior has an obvious cheap local test. Skip new-test ceremony when reproduction is unclear, integration-heavy, or expensive, and report the closest useful executable evidence.
---

# Focused TDD bug fix

Make broken behavior executable before changing production code when a clear, cheap test path exists. Prefer an existing Rust test, repository regression, or fixture command. Do not build a broad new test framework merely to satisfy this sequence.

1. Identify intended behavior, current behavior, affected caller, and smallest observable reproduction.
2. Choose the narrowest existing executable check that reaches that contract. Derive the expected result from the requirement rather than the implementation under test.
3. Write the smallest test that would have caught the defect. Assert return values, errors, readback state, or external effects observed by the caller.
4. Run it before the fix. Confirm it fails for the intended defect. Repair an unrelated test/setup failure before using it as evidence. A passing test does not establish a reproduced bug.
5. Fix the first wrong decision in the owning product layer. Preserve nearby contracts and failure-safe publication.
6. Rerun the regression, then the smallest additional checks required by [testing policy](../../../docs/quality/testing.en.md).

If a new failing test is impractical, use the closest useful executable check and state the limitation. Acceptable evidence can be a focused reproduction command, real fixture conversion, native readback, or recorded device action. Do not invent native results from a portable proxy. Prefer no new test over a brittle mock, timing dependency, implementation mirror, or unrelated fixture churn.

Do not weaken assertions to match an incorrect implementation. Keep exact bytes, messages, schema, order, and native lifecycle assertions when they are the actual public, wire, durability, or framework contract. Internal helper names and local variables usually are not that contract.

If production code is already changed, say that failing-before evidence was not captured. A safe isolated comparison against the original implementation can establish it when practical; do not overwrite another worker's changes or claim an unexecuted baseline. This skill does not authorize staging or ordering commits.

Report the exact failing-before check and failure, the passing-after run, nearby validation, and any missing native/device evidence. Read [engineering principles](../pstack-engineering-principles/SKILL.md) when the failure involves boundaries or publication.

Adapted from pstack at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`, `pstack/skills/tdd/SKILL.md`. See [source hashes](sources.json) and [LICENSE](LICENSE).
