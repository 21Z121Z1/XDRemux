# Batch and categorize

Exercise the batch and classification interfaces separately from single-file conversion, including receipts, provenance, paired assets, and output reservations when affected.

## Sub-features

Check input discovery, converted/failed/skipped outcomes, collision handling, checkpoint identity, resource-omission reporting, and categorization dry-run effects as applicable to the change.

## How to get to it (user POV)

Use `xdremux batch --input INPUT --output-dir NEW_DIRECTORY --jobs 1 --json`. Use `xdremux categorize --input INPUT --output-dir NEW_DIRECTORY --dry-run --json` for a classification plan. `--input-dir`, `--recursive`, `--resume`, and `--skip-existing` add distinct behaviors; select those that changed.

## Driving it with the Rust CLI

Preconditions: create isolated output/checkpoint roots and record fixture identity. Use the public Samsung HEIF Motion Photo to avoid a large JPEG encode for a small batch check. Inspect the receipt and independently validate the resulting pair. Check both members stay together and the source hash remains unchanged.

Run `bash scripts/validation/check_rust_cli_smoke.sh` for its existing CLI/receipt/categorization journey. For related changes, use `bash scripts/validation/verify_batch_categorize_idempotence.sh` and the owning runtime batch/provenance tests. Observe the filesystem before and after a dry-run instead of assuming its name proves no writes. Retain transcripts and output identities outside scripts' disposable work directories.

## Gotchas

Single convert success does not prove batch receipts, omitted-resource reporting, resume identity, or parallel path reservations. An input hash match alone does not establish all output identity/provenance requirements. Do not reuse a shared checkpoint or output directory from another worker.
