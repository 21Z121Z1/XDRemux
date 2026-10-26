# Scoped playbooks

Select the playbook that matches the requested result. Each sequence applies within the user's current scope. No sequence automatically authorizes a commit, PR, or external write.

## Investigation

Trace the responsible source, callers, transformations, and resulting behavior. Read tests as contracts. Distinguish a declaration from a reachable producer and consumer.

Use current Git state and available read-only evidence for current-state claims. Provide a source-backed explanation or recommendation with concrete tradeoffs. Do not turn an investigation into implementation without task authorization.

## Bug fix

1. Reproduce the symptom through the affected interface. Prefer an existing command or fixture. If direct reproduction is unavailable, record the missing environment or input. Use the closest check that can distinguish the proposed cause.
2. Find the first wrong decision. Compare competing causes with runtime observations, bounded instrumentation, or a focused test. Revert speculative changes when evidence disproves their premise.
3. Apply [TDD](../../pstack-tdd/SKILL.md) when a cheap local regression is available. Before editing production code, confirm that the observed failure demonstrates the defect.
4. Make the smallest fix in the responsible module. Check another occurrence only when the same demonstrated fault can reach it.
5. Rerun the original reproduction and required nearby checks. A result from another interface or host does not prove the original defect is fixed.
6. Review the final diff. Report the original failure, result after the fix, root cause, and remaining gaps. Do not fabricate a failing baseline after writing the fix.

## Refactor

Before changing structure, record observable behavior with existing tests, representative output, or an equivalence check. State the current and target module responsibilities. Identify the maintained state, branches, or indirection that will disappear.

Before removing an API, trace every caller and dynamic use. Preserve intentional public compatibility and source provenance.

Move one small unit at a time. Keep its behavior check passing and inspect the actual output after each move. Narrow or revert a refactor that only moves the same complexity.

If the work changes behavior, identify that change separately. Use the bug-fix or feature sequence for it instead of describing the whole change as behavior-preserving.

## Feature

Define the user result, input and output data, responsible module, supported source and platform combinations, and failure behavior. Read existing callers before designing an API or option.

Compare materially different designs only when evidence is needed to settle a tradeoff. Choose the smallest complete sequence from the request to the published result.

Implement through the existing product stack. Keep policy in the Rust engine and execution and publication in the Rust runtime. Keep Apple framework operations in the adapter and presentation in the app.

Verify success and required failures through the actual public entry point. Include applicable source-dependent, filesystem, adapter, and native checks for each changed boundary. Keep experimental observations separate from supported product behavior.
