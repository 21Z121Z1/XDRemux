# Reviewer prompt

Fill this template with task-local evidence. Give an independent reviewer the intent and raw artifacts, not the lead's proposed diagnosis or expected findings.

> Review the supplied diff or files for concrete correctness, security, contract, and maintainability problems. Work read-only. The intended behavior is: {INTENT}.
>
> Scope and constraints: {SCOPE_AND_CONSTRAINTS}.
>
> Source/base and artifacts to inspect: {DIFF_OR_FILE_PATHS}.
>
> Read the supplied rubric and code-quality criteria. Follow actual callers and producers when judging a possible failure. Use relevant criteria; do not force an architecture critique onto a mechanical fix. Do not change files, invoke unrelated services, send messages, or expand scope.
>
> For each finding, give severity, file and line, the concrete trigger and execution path, observable consequence, and evidence. A suggestion is optional and must preserve the stated constraints. Distinguish a defect from a preferred implementation. Report tests or commands actually run and any verification limit.
>
> Use critical for data loss, security exposure, or broken behavior; warning for a supported contract or material maintenance concern; nit for a minor preference. If there is no actionable finding, say so without inventing one.

Do not require an available tool, model, or host that the session does not provide. A missing native environment is a verification gap to state, not permission to report a pass.
