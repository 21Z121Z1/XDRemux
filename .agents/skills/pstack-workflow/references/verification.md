# Verification checklist

Read [testing policy](../../../../docs/quality/testing.en.md) and the [validation runbook](../../../../docs/validation/README.en.md). Select evidence that observes the changed behavior.

1. Record the intended behavior. Identify the artifact, command, or consumer that can observe it.
2. For a bug, retain the actual failure before the fix when available. Confirm its cause. If this observation is unavailable, explain why. Do not describe the result as a failing-then-passing regression.
3. Run the targeted regression and affected functional path. Check output bytes, decoded facts, exit status, errors, and publication effects as applicable. Inspect delegated artifacts directly.
4. Keep evidence classes distinct. A Rust parser can prove structure. An Apple framework can prove the readback observed on that host. Editing, saving, and reopening on a device require those device actions. Static checks do not turn a failed or unavailable native operation into a pass.
5. If the change can replace a file or pair, verify preservation on failure. Missing required source data, native capability, or validation must fail before damaging input or existing output.
6. Inspect the final diff for stale callers, documentation, fixtures, generated configuration, and unsupported claims. Read the current English document before changing its Chinese counterpart.
7. After a repair, rerun affected checks. Reuse unchanged evidence only within its actual source and environment scope. When completion requires a committed-HEAD receipt, follow that contract. A later tracked edit invalidates the receipt. This skill does not authorize a commit.

## Performance evidence

Preserve the measured workload, host, repeat-run method, baseline, and existing thresholds. Explain what a measurement represents before acting on it. Do not change a budget, runner class, or acceptance threshold to make a run pass.

## Report the result

Report the commands actually run, their outcomes, evidence paths, and remaining host or device limits. A skipped check, unavailable dependency, or reviewer opinion is not functional evidence.
