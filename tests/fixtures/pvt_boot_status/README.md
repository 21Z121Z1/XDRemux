# Boot monitor replay

`data-migration-failed.stdout` is stdout from the owned synthetic
control simulator in [job 120304459458](https://github.com/21Z121Z1/XDRemux/actions/runs/39920551693/job/120304459458).
Only trailing blank lines were removed; the original output remains in the job artifact.
The process exited zero, but its last record reports terminal status 3 and
`Data Migration Failed`. This fixture tests reporting and fail-closed behavior,
not simulator or Photos acceptance. No media is included.

Success tests are synthetic. The status 4294967295 / `Finished` format is also
visible in [Flutter's simulator execution log](https://github.com/flutter/flutter/issues/64018).
This is observed CLI output, not a guaranteed Apple public API. Unknown output
must remain blocked.
