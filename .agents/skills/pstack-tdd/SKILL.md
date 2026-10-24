---
name: pstack-tdd
description: Fix an XDRemux bug with a focused failing-before, passing-after regression when the user requests TDD or the affected behavior has an obvious cheap local test. Skip new-test ceremony when reproduction is unclear, integration-heavy, or expensive, and report the closest useful executable evidence.
---

# Focused TDD bug fix

If a clear, cheap test path exists, make the broken behavior executable before changing production code. Prefer an existing Rust test, repository regression, or fixture command. Do not create a broad test framework only to follow this sequence.

## Reproduce and fix the defect

1. Identify the intended behavior, current behavior, affected caller, and smallest observable reproduction.
2. Choose the narrowest existing executable check that reaches the contract. Derive its expected result from the requirement, not from the implementation under test.
3. Write the smallest test that would have caught the defect. Assert return values, errors, readback state, or external effects that the caller can observe.
4. Run the test before the fix. Confirm that it fails for the intended defect. Repair unrelated test or setup failures before using the result as evidence. A passing test does not reproduce the defect.
5. Fix the first wrong decision in the owning product layer. Preserve nearby contracts and failure-safe publication.
6. Rerun the regression. Then run the smallest additional checks required by [testing policy](../../../docs/quality/testing.en.md).

## Choose useful evidence

If a new failing test is impractical, use the closest useful executable check. State its limits. Evidence can include a focused reproduction command, real fixture conversion, native readback, or recorded device action. Do not infer native results from a portable proxy.

Do not add a brittle mock, timing dependency, implementation mirror, or unrelated fixture changes to produce a new test. In those cases, retain the useful evidence without a new test.

Do not weaken assertions to match an incorrect implementation. Keep exact bytes, messages, schema, order, and native lifecycle assertions when they define a public, wire, durability, or framework contract. Internal helper names and local variables usually do not define that contract.

## Handle code that has already changed

If production code has already changed, state that failing-before evidence was not captured. When practical, use a safe, isolated comparison with the original implementation to establish that evidence. Do not overwrite another worker's changes or claim an unexecuted baseline. This skill does not authorize staging or ordering commits.

## Report the result

Report the exact failing-before check and its failure. Report the passing-after run, nearby validation, and any missing native or device evidence.

If the failure involves boundaries or publication, read [engineering principles](../pstack-engineering-principles/SKILL.md).

Adapted from pstack at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`, `pstack/skills/tdd/SKILL.md`. See [source hashes](sources.json) and [LICENSE](LICENSE).
