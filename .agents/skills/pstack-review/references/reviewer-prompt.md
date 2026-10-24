# Reviewer prompt

Fill the template with evidence from the current task. Give the independent reviewer the intent and raw artifacts. Do not supply the lead reviewer's proposed diagnosis or expected findings.

> Review the supplied diff or files for concrete correctness, security, contract, and maintainability problems. Work read-only.
>
> Intended behavior: {INTENT}.
>
> Scope and constraints: {SCOPE_AND_CONSTRAINTS}.
>
> Source/base and artifacts to inspect: {DIFF_OR_FILE_PATHS}.
>
> Read the supplied rubric and code-quality criteria. Follow actual callers and producers when judging a possible failure. Apply only relevant criteria. Do not force an architecture critique onto a mechanical fix. Do not change files, invoke unrelated services, send messages, or expand scope.
>
> For each finding, give its severity, file and line, concrete trigger, execution path, observable consequence, and supporting evidence. A suggested fix is optional. It must preserve the stated constraints. Distinguish a defect from a preferred implementation. Report only tests or commands that you actually ran. State any verification limits.
>
> Use critical for data loss, security exposure, or broken behavior. Use warning for a supported contract or material maintenance concern. Use nit for a minor preference. If there is no actionable finding, say so. Do not invent one.

Do not require a tool, model, or host that the session does not provide. Report an unavailable native environment as a verification gap. It does not permit a pass result.
