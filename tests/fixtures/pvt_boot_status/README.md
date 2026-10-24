# Boot monitor replay fixture

Use this fixture to test boot-status reporting and rejection of failed or unknown output. It does not establish simulator readiness or Photos admission.

## Recorded failure

[`data-migration-failed.stdout`](data-migration-failed.stdout) contains stdout from an owned synthetic control simulator. The source is [job 120304459458](https://github.com/21Z121Z1/XDRemux/actions/runs/39920551693/job/120304459458).

Only trailing blank lines were removed. The untrimmed output was recorded in the job artifact. No media is included.

The process returned exit code `0`. Its final record reports terminal status `3` and `Data Migration Failed`. A zero exit must not make this case pass.

## Synthetic success controls

Success tests use synthetic output. The observed success format contains status `4294967295` and `Finished`. The original fixture record cites [Flutter's simulator execution log](https://github.com/flutter/flutter/issues/64018) for that format.

These records describe observed CLI text. They do not define a guaranteed Apple public API. Unknown output must remain blocked.
