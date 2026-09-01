## Intent

<!-- Describe the product or architecture outcome, not only the files changed. -->

## System boundary

- Target capability / layer:
- Intended base:
- Current owner:
- Invariant that must remain true:
- Execution plan, if this work spans sessions/PRs:

## Evidence

- Oracle / standard / fixture / model baseline:
- Required or promotion evidence:
- Targeted regression / conformance check:
- Functional / integration / device evidence, if required:
- Diagnostic probes used for discovery only:

<!--
A diagnostic probe is not acceptance evidence merely because it is green.
If a probe used temporary instrumentation or an in-workflow source patch,
record the finding here and encode it in a stable implementation/test contract
before counting it toward completion or promotion.
-->

## Verification

- Exact committed HEAD:
- `scripts/agent_completion_gate.py` receipt, when applicable:
- Required GitHub Actions / CodeQL checks:

## Residual gaps

<!-- State what is not proven and what evidence would close the gap. -->

## Migration / research promotion

<!-- Delete this section when the change is unrelated to migration or research. -->

- Normalized contract:
- Rust owner / adapter boundary:
- Promotion evidence reached:
- Promotion evidence still missing:

## Knowledge promotion

<!--
If this PR discovered a stable repository-wide rule, say where that rule now
lives (architecture, validation, model card, test, etc.). Do not leave stable
knowledge only in this PR description or a completed execution plan.
-->
