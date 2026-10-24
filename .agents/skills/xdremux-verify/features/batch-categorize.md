# Batch and categorize

Exercise batch and categorization separately from single-file conversion. If affected, check receipts, provenance, paired assets, and output reservations.

## Select the affected behavior

Check the behavior changed by the task:

- Input discovery and converted, failed, or skipped outcomes.
- Collision handling and checkpoint identity.
- Resource-omission reporting and categorization dry-run effects.

## Use the public entry points

For batch conversion, use `xdremux batch --input INPUT --output-dir NEW_DIRECTORY --jobs 1 --json`.

For a classification plan, use `xdremux categorize --input INPUT --output-dir NEW_DIRECTORY --dry-run --json`.

The options `--input-dir`, `--recursive`, `--resume`, and `--skip-existing` select distinct behavior. Exercise the options affected by the change.

## Run the checks

Before starting, create separate output and checkpoint roots. Record the fixture identity. For a small batch check, use the public Samsung HEIF Motion Photo. This avoids encoding a large JPEG.

1. Inspect the receipt and independently validate the output pair.
2. Check that both pair members stay together and the source hash is unchanged.
3. Run `bash scripts/validation/check_rust_cli_smoke.sh` for its existing CLI, receipt, and categorization journey.
4. For related changes, run `bash scripts/validation/verify_batch_categorize_idempotence.sh` and the owning runtime batch and provenance tests.
5. Observe the filesystem before and after a dry-run. The option name alone does not prove that no writes occurred.
6. Keep transcripts and output identities outside the scripts' disposable work directories.

## Interpret the evidence

Single-file conversion does not establish batch receipts, omitted-resource reporting, resume identity, or parallel path reservations. A matching input hash does not establish every output identity or provenance requirement.

Do not reuse another worker's shared checkpoint or output directory.
