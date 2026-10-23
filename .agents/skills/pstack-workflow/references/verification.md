# Verification checklist

Read [testing policy](../../../../docs/quality/testing.en.md) and the [validation runbook](../../../../docs/validation/README.en.md). Select evidence for the behavior that changed.

1. Record the intended behavior and the artifact, command, or consumer that can observe it.
2. For a bug, retain the actual failing-before observation when available. Confirm the failure reason. If it cannot be obtained, state why; do not label the result red/green.
3. Run the targeted regression and affected functional path. Check real output bytes, decoded facts, exit status, errors, and publication effects as applicable. Inspect delegated artifacts rather than trusting a summary.
4. Keep evidence classes distinct. A Rust parser can prove structure. An Apple framework can prove the readback it returned on that host. A device edit/save/reopen claim requires that device interaction. A failed or unavailable native operation does not become a pass because static checks succeeded.
5. Check failure preservation when the change can replace a file or a paired asset. Missing required source data, native capability, or validation must fail before damaging input or existing output.
6. Inspect the final diff for stale callers, documentation, fixtures, generated configuration, and unsupported claims. Read current English canonical documentation before changing the equivalent Chinese scope.
7. Rerun affected checks after a repair. Reuse unchanged evidence only within its actual source/environment scope. Follow the repository's committed-HEAD receipt contract when completion requires it; a later tracked edit invalidates that receipt. This skill does not authorize a commit.

For performance, preserve the measured workload, host, repeat-run method, baseline, and existing thresholds. Explain what the number measures before acting on it. Do not change a budget, runner class, or acceptance threshold to make a run pass.

Report the commands actually run, outcomes, evidence paths, and remaining host/device limits. Do not call a skipped check, an unavailable dependency, or a reviewer opinion functional evidence.
