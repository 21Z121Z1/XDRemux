#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: str, old: str, new: str, marker: str) -> None:
    target = Path(path)
    text = target.read_text()
    if marker in text:
        return
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old!r}")
    target.write_text(text.replace(old, new, 1))


# Runtime module/export wiring.
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "mod batch;\nmod categorize;\nmod live_photo;\n",
    "mod batch;\nmod batch_checkpoint;\nmod categorize;\nmod live_photo;\n",
    "mod batch_checkpoint;",
)
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "pub use batch::{BatchAssetKind, BatchFailure, BatchItem, BatchReceipt, BatchSuccess};\n",
    "pub use batch::{\n    plan_batch_items, BatchAssetKind, BatchExecutionOptions, BatchFailure, BatchItem,\n    BatchPlanOptions, BatchReceipt, BatchSuccess, BatchSuccessDisposition,\n};\npub use batch_checkpoint::{\n    motion_photo_checkpoint_path, DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME,\n    MOTION_PHOTO_CHECKPOINT_SCHEMA_VERSION,\n};\n",
    "pub use batch_checkpoint::{",
)

# Keep checkpoint append semantics type-safe instead of suppressing clippy::too_many_arguments.
checkpoint = Path("crates/xdremux-runtime/src/batch_checkpoint.rs")
checkpoint_text = checkpoint.read_text()
if "enum CheckpointOutcome<'a>" not in checkpoint_text:
    anchor = "pub(crate) struct MotionPhotoCheckpointWriter {\n    file: File,\n}\n"
    insertion = """pub(crate) enum CheckpointOutcome<'a> {
    Success(&'a str),
    SkippedExisting(&'a str),
    Failure(&'a str),
}

impl CheckpointOutcome<'_> {
    fn status(&self) -> &'static str {
        match self {
            Self::Success(_) => "success",
            Self::SkippedExisting(_) => "skipped_existing",
            Self::Failure(_) => "failure",
        }
    }

    fn asset_identifier(&self) -> Option<&str> {
        match self {
            Self::Success(identifier) | Self::SkippedExisting(identifier) => Some(identifier),
            Self::Failure(_) => None,
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            Self::Failure(error) => Some(error),
            Self::Success(_) | Self::SkippedExisting(_) => None,
        }
    }
}

pub(crate) struct MotionPhotoCheckpointWriter {
    file: File,
}
"""
    if anchor not in checkpoint_text:
        raise SystemExit("checkpoint writer anchor not found")
    checkpoint_text = checkpoint_text.replace(anchor, insertion, 1)

old_signature = """    pub(crate) fn append_item(
        &mut self,
        source: &Path,
        image: &Path,
        video: &Path,
        status: &str,
        signature: &SourceSignature,
        asset_identifier: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
"""
new_signature = """    pub(crate) fn append_item(
        &mut self,
        source: &Path,
        image: &Path,
        video: &Path,
        signature: &SourceSignature,
        outcome: CheckpointOutcome<'_>,
    ) -> Result<()> {
"""
if old_signature in checkpoint_text:
    checkpoint_text = checkpoint_text.replace(old_signature, new_signature, 1)
checkpoint_text = checkpoint_text.replace(
    "            status: status.to_owned(),\n",
    "            status: outcome.status().to_owned(),\n",
)
checkpoint_text = checkpoint_text.replace(
    "            asset_identifier: asset_identifier.map(ToOwned::to_owned),\n            error: error.map(ToOwned::to_owned),\n",
    "            asset_identifier: outcome.asset_identifier().map(ToOwned::to_owned),\n            error: outcome.error().map(ToOwned::to_owned),\n",
)
checkpoint.write_text(checkpoint_text)

batch = Path("crates/xdremux-runtime/src/batch.rs")
batch_text = batch.read_text()
batch_text = batch_text.replace(
    "    source_signature, MotionPhotoCheckpoint, MotionPhotoCheckpointWriter, SourceSignature,\n",
    "    source_signature, CheckpointOutcome, MotionPhotoCheckpoint, MotionPhotoCheckpointWriter,\n    SourceSignature,\n",
)
batch_text = batch_text.replace(
    """    writer.append_item(
        source,
        output,
        &output.with_extension("mov"),
        "failure",
        signature,
        None,
        Some(error),
    )
""",
    """    writer.append_item(
        source,
        output,
        &output.with_extension("mov"),
        signature,
        CheckpointOutcome::Failure(error),
    )
""",
)
batch_text = batch_text.replace(
    """                                            writer.append_item(
                                                &item.input,
                                                &item.output,
                                                &output_video,
                                                "skipped_existing",
                                                &signature,
                                                Some(&prior.asset_identifier),
                                                None,
                                            )?;
""",
    """                                            writer.append_item(
                                                &item.input,
                                                &item.output,
                                                &output_video,
                                                &signature,
                                                CheckpointOutcome::SkippedExisting(
                                                    &prior.asset_identifier,
                                                ),
                                            )?;
""",
)
batch_text = batch_text.replace(
    """                                    writer.append_item(
                                        &item.input,
                                        &converted.image,
                                        &converted.video,
                                        "success",
                                        &signature,
                                        Some(&converted.content_identifier),
                                        None,
                                    )?;
""",
    """                                    writer.append_item(
                                        &item.input,
                                        &converted.image,
                                        &converted.video,
                                        &signature,
                                        CheckpointOutcome::Success(&converted.content_identifier),
                                    )?;
""",
)
batch.write_text(batch_text)

# Update checkpoint unit-test call sites.
checkpoint_text = checkpoint.read_text()
checkpoint_text = checkpoint_text.replace(
    """            .append_item(
                &source,
                &image,
                &video,
                "success",
                &signature,
                Some("ASSET-ID"),
                None,
            )
""",
    """            .append_item(
                &source,
                &image,
                &video,
                &signature,
                CheckpointOutcome::Success("ASSET-ID"),
            )
""",
)
checkpoint_text = checkpoint_text.replace(
    """            .append_item(
                &source,
                &image,
                &video,
                "success",
                &original,
                Some("ASSET-ID"),
                None,
            )
""",
    """            .append_item(
                &source,
                &image,
                &video,
                &original,
                CheckpointOutcome::Success("ASSET-ID"),
            )
""",
)
checkpoint.write_text(checkpoint_text)

cli = Path("crates/xdremux-cli/src/lib.rs")
text = cli.read_text()
text = text.replace(
    "use xdremux_runtime::{BatchAssetKind, BatchItem, PortableRuntime};",
    "use xdremux_runtime::{\n    motion_photo_checkpoint_path, plan_batch_items, BatchAssetKind, BatchExecutionOptions,\n    BatchPlanOptions, BatchSuccessDisposition, PortableRuntime,\n};",
)

# Add durable batch flags once.
needle = "    /// Emit one machine-readable JSON receipt instead of human progress.\n    #[arg(long)]\n    json: bool,\n}"
if "skip_existing: bool" not in text:
    replacement = "    /// Reuse a completed Live Photo pair only when durable source provenance matches.\n    #[arg(long)]\n    skip_existing: bool,\n    /// Resume completed Live Photo work from the durable checkpoint and retry remaining items.\n    #[arg(long)]\n    resume: bool,\n    /// Shared Swift/Python/Rust Motion Photo checkpoint base path.\n    ///\n    /// The compatibility state is stored at this path with `.motion-photo` appended.\n    #[arg(long, value_name = \"FILE\")]\n    checkpoint: Option<PathBuf>,\n    /// Emit one machine-readable JSON receipt instead of human progress.\n    #[arg(long)]\n    json: bool,\n}"
    if needle not in text:
        raise SystemExit("BatchArgs JSON anchor not found")
    text = text.replace(needle, replacement, 1)

# Remove the old CLI-owned output planner; the runtime owns naming/provenance policy now.
start = text.find("fn batch_output_candidate(")
end = text.find("fn batch_kind_name(")
if start != -1:
    if end == -1 or end <= start:
        raise SystemExit("could not locate end of legacy CLI batch planner")
    text = text[:start] + text[end:]

# Add a status helper for stable structured output.
if "fn batch_disposition_name(" not in text:
    anchor = "fn batch_kind_name(kind: BatchAssetKind) -> &'static str {\n    match kind {\n        BatchAssetKind::ProXdr => \"pro-xdr\",\n        BatchAssetKind::LivePhoto => \"live-photo\",\n    }\n}\n"
    replacement = anchor + "\nfn batch_disposition_name(disposition: BatchSuccessDisposition) -> &'static str {\n    disposition.as_str()\n}\n"
    if anchor not in text:
        raise SystemExit("batch_kind_name anchor not found")
    text = text.replace(anchor, replacement, 1)

# Replace run_batch planning/execution prefix.
old = """fn run_batch(arguments: BatchArgs, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let inputs = match discover_batch_inputs(&arguments) {
        Ok(inputs) => inputs,
        Err(error) => {
            let _ = writeln!(stderr, \"error: {error}\");
            return 2;
        }
    };
    let items = match plan_batch_items(&inputs, arguments.output_dir.as_deref()) {
        Ok(items) => items,
        Err(error) => {
            let _ = writeln!(stderr, \"error: {error}\");
            return 2;
        }
    };

    let runtime = PortableRuntime::new();
    let receipt = runtime.convert_batch(items, ConversionRequest::default());
"""
new = """fn run_batch(arguments: BatchArgs, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let inputs = match discover_batch_inputs(&arguments) {
        Ok(inputs) => inputs,
        Err(error) => {
            let _ = writeln!(stderr, \"error: {error}\");
            return 2;
        }
    };
    let reuse_existing = arguments.skip_existing || arguments.resume;
    let checkpoint_path = motion_photo_checkpoint_path(
        arguments.output_dir.as_deref(),
        arguments.checkpoint.as_deref(),
    );
    if reuse_existing && checkpoint_path.is_none() {
        let _ = writeln!(
            stderr,
            \"error: --skip-existing/--resume requires --output-dir or --checkpoint for durable provenance\"
        );
        return 2;
    }
    let plan_options = BatchPlanOptions {
        output_dir: arguments.output_dir.clone(),
        checkpoint_path: checkpoint_path.clone(),
        reuse_existing,
    };
    let items = match plan_batch_items(&inputs, &plan_options) {
        Ok(items) => items,
        Err(error) => {
            let _ = writeln!(stderr, \"error: {error}\");
            return 2;
        }
    };

    let runtime = PortableRuntime::new();
    let execution_options = BatchExecutionOptions {
        checkpoint_path,
        reuse_existing,
    };
    let receipt = runtime.convert_batch_with_options(
        items,
        ConversionRequest::default(),
        &execution_options,
    );
"""
if old in text:
    text = text.replace(old, new, 1)
elif "let execution_options = BatchExecutionOptions" not in text:
    raise SystemExit("run_batch legacy prefix not found")

# Add status and skipped count to JSON.
text = text.replace(
    '                    "kind": batch_kind_name(success.kind),\n                    "outputs":',
    '                    "kind": batch_kind_name(success.kind),\n                    "status": batch_disposition_name(success.disposition),\n                    "outputs":',
)
text = text.replace(
    '            "succeeded": receipt.succeeded(),\n            "failed": receipt.failed(),',
    '            "succeeded": receipt.succeeded(),\n            "skipped_existing": receipt.skipped_existing(),\n            "failed": receipt.failed(),',
)

# Existing unit-test BatchArgs literals need the new fields.
text = text.replace(
    "            output_dir: None,\n            json: false,",
    "            output_dir: None,\n            skip_existing: false,\n            resume: false,\n            checkpoint: None,\n            json: false,",
)
if "assert!(!arguments.skip_existing);" not in text:
    text = text.replace(
        "        assert_eq!(arguments.output_dir, Some(PathBuf::from(\"out\")));\n",
        "        assert_eq!(arguments.output_dir, Some(PathBuf::from(\"out\")));\n        assert!(!arguments.skip_existing);\n        assert!(!arguments.resume);\n        assert_eq!(arguments.checkpoint, None);\n",
        1,
    )

# Replace the planner unit test call with the runtime planner API.
text = text.replace(
    "        let plan = plan_batch_items(&[a.clone(), b.clone()], Some(&root)).unwrap();",
    "        let plan = plan_batch_items(\n            &[a.clone(), b.clone()],\n            &BatchPlanOptions {\n                output_dir: Some(root.clone()),\n                ..BatchPlanOptions::default()\n            },\n        )\n        .unwrap();",
)

cli.write_text(text)
