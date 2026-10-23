# Scoped playbooks

Use the playbook that matches the requested outcome. These sequences preserve pstack's engineering cadence within the current user's scope. None ends with an automatic commit, PR, or external write.

## Investigation

Trace the canonical source, callers, transformations, and resulting behavior. Read tests as contracts, then distinguish declarations from a reachable producer and consumer. Use current Git and available read-only evidence for current-state claims. Produce a source-backed explanation or a recommendation with concrete tradeoffs. Do not turn an investigation into implementation without task authorization.

## Bug fix

1. Reproduce the reported symptom through the affected interface. Prefer an existing local command or fixture. When direct reproduction is unavailable, record the exact missing environment or input and use the closest check that can discriminate the hypothesis.
2. Trace the first wrong decision, not just the point of failure. Form competing causes and rule them out with runtime observations, bounded instrumentation, or a focused test. Revert speculative changes whose premise is disproved.
3. Apply [TDD](../../pstack-tdd/SKILL.md) when the regression has a cheap local target. Confirm that the failure demonstrates the defect before editing production code.
4. Make the smallest fix in the owning layer. Check sibling occurrences only when the same demonstrated fault can reach them.
5. Rerun the original reproduction and the required nearby checks. A result from a different interface or host does not establish that the original defect is fixed.
6. Review the final diff. Report the failure before, result after, root cause, and any remaining gap. Do not manufacture a failing baseline after the fix has already been written.

## Refactor

Pin the observable behavior with existing tests, representative output, or an equivalence check before moving structure. State the current and target ownership and what maintained state, branches, or indirection will disappear. Trace every caller and dynamic path before removing an API. Preserve intentional public compatibility and source provenance.

Move in small units while holding the behavior check green. Test the real output after the move. Revert or narrow a refactor that only relocates complexity. If the work reveals a behavior change, identify it separately and apply the bug-fix or feature sequence rather than calling it behavior-preserving.

## Feature

Name the user outcome, input and result data, owner, supported source/platform combinations, and failure behavior. Read existing callers before designing a new API or option. Compare materially different designs only when the tradeoff needs evidence. Choose the smallest complete effect path from request to published result.

Implement through the canonical stack. Keep Rust engine policy, Rust runtime execution/publication, Apple framework operations, and app presentation in their existing owners. Verify success and required failures through the actual public entry point. Include source-dependent, filesystem, adapter, and native checks when those boundaries change. Separate experimental observations from promoted product support.
