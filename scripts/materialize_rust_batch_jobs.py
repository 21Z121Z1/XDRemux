#!/usr/bin/env python3
from pathlib import Path


batch_path = Path("crates/xdremux-runtime/src/batch.rs")
batch = batch_path.read_text()

if "use std::sync::atomic::{AtomicUsize, Ordering};" not in batch:
    batch = batch.replace(
        "use std::path::{Path, PathBuf};\n",
        "use std::path::{Path, PathBuf};\nuse std::sync::atomic::{AtomicUsize, Ordering};\nuse std::sync::mpsc;\nuse std::thread;\n",
        1,
    )

old_options = """#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BatchExecutionOptions {
    pub checkpoint_path: Option<PathBuf>,
    pub reuse_existing: bool,
}
"""
new_options = """#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchExecutionOptions {
    pub checkpoint_path: Option<PathBuf>,
    pub reuse_existing: bool,
    pub jobs: usize,
}

impl Default for BatchExecutionOptions {
    fn default() -> Self {
        Self {
            checkpoint_path: None,
            reuse_existing: false,
            jobs: 1,
        }
    }
}
"""
if old_options in batch:
    batch = batch.replace(old_options, new_options, 1)
elif "pub jobs: usize," not in batch:
    raise SystemExit("BatchExecutionOptions anchor not found")

start = batch.find("fn append_checkpoint_failure(")
end = batch.find("#[cfg(test)]")
if start == -1 or end == -1 or end <= start:
    if "struct BatchCheckpointEvent" not in batch:
        raise SystemExit("batch execution replacement anchors not found")
else:
    replacement = r'''#[derive(Debug)]
enum OwnedCheckpointOutcome {
    Success(String),
    SkippedExisting(String),
    Failure(String),
}

#[derive(Debug)]
struct BatchCheckpointEvent {
    source: PathBuf,
    image: PathBuf,
    video: PathBuf,
    signature: SourceSignature,
    outcome: OwnedCheckpointOutcome,
}

impl BatchCheckpointEvent {
    fn append(
        self,
        writer: &mut Option<MotionPhotoCheckpointWriter>,
        checkpoint_path: Option<&Path>,
    ) -> Result<()> {
        if writer.is_none() {
            if let Some(path) = checkpoint_path {
                *writer = Some(MotionPhotoCheckpointWriter::open(path)?);
            }
        }
        let Some(writer) = writer.as_mut() else {
            return Ok(());
        };
        let outcome = match &self.outcome {
            OwnedCheckpointOutcome::Success(identifier) => {
                CheckpointOutcome::Success(identifier)
            }
            OwnedCheckpointOutcome::SkippedExisting(identifier) => {
                CheckpointOutcome::SkippedExisting(identifier)
            }
            OwnedCheckpointOutcome::Failure(error) => CheckpointOutcome::Failure(error),
        };
        writer.append_item(
            &self.source,
            &self.image,
            &self.video,
            &self.signature,
            outcome,
        )
    }
}

#[derive(Debug)]
struct BatchWorkResult {
    item: BatchItem,
    result: Result<BatchSuccess>,
    checkpoint: Option<BatchCheckpointEvent>,
}

#[derive(Debug)]
enum OrderedBatchOutcome {
    Success(BatchSuccess),
    Failure(BatchFailure),
}

fn process_batch_item_with_options(
    runtime: &PortableRuntime,
    item: BatchItem,
    request: ConversionRequest,
    checkpoint: &MotionPhotoCheckpoint,
    reuse_existing: bool,
) -> BatchWorkResult {
    let mut checkpoint_event = None;
    let result: Result<BatchSuccess> = (|| {
        if item.input == item.output {
            return Err(RuntimeError::new(
                "batch output",
                "batch conversion never overwrites its source",
            ));
        }
        create_output_parent(&item.output)?;
        let source = fs::read(&item.input)
            .map_err(|error| RuntimeError::external("batch input read", error))?;
        let asset = probe_bytes(&source)
            .map_err(|error| RuntimeError::external("batch source probe", error))?;

        match asset {
            SourceAsset::MotionPhoto { .. } => {
                let signature = source_signature(&item.input, &source)?;
                let output_video = item.output.with_extension("mov");

                if reuse_existing {
                    if let Some(prior) = checkpoint.reusable_item(&item.input, &signature)? {
                        if prior.image == canonical_or_absolute(&item.output)?
                            && prior.video == canonical_or_absolute(&output_video)?
                        {
                            if pair_matches_identifier(
                                &item.output,
                                &output_video,
                                &prior.asset_identifier,
                            ) {
                                checkpoint_event = Some(BatchCheckpointEvent {
                                    source: item.input.clone(),
                                    image: item.output.clone(),
                                    video: output_video.clone(),
                                    signature,
                                    outcome: OwnedCheckpointOutcome::SkippedExisting(
                                        prior.asset_identifier,
                                    ),
                                });
                                return Ok(BatchSuccess {
                                    input: item.input.clone(),
                                    outputs: vec![item.output.clone(), output_video],
                                    kind: BatchAssetKind::LivePhoto,
                                    disposition: BatchSuccessDisposition::SkippedExisting,
                                });
                            }
                            if item.output.exists() || output_video.exists() {
                                return Err(RuntimeError::new(
                                    "batch resume provenance",
                                    "checkpoint matches the source, but the existing HEIC/MOV pair no longer matches its recorded asset identifier; refusing to overwrite",
                                ));
                            }
                        }
                    }
                    if item.output.exists() || output_video.exists() {
                        return Err(RuntimeError::new(
                            "batch resume provenance",
                            "existing Live Photo output has no matching source provenance; refusing to reuse or overwrite",
                        ));
                    }
                } else if item.output.exists() || output_video.exists() {
                    return Err(RuntimeError::new(
                        "batch output",
                        "output HEIC/MOV pair already exists; refusing to overwrite",
                    ));
                }

                match runtime.convert_motion_photo_file(&source, &item.input, &item.output) {
                    Ok(converted) => {
                        checkpoint_event = Some(BatchCheckpointEvent {
                            source: item.input.clone(),
                            image: converted.image.clone(),
                            video: converted.video.clone(),
                            signature,
                            outcome: OwnedCheckpointOutcome::Success(
                                converted.content_identifier.clone(),
                            ),
                        });
                        Ok(BatchSuccess {
                            input: item.input.clone(),
                            outputs: vec![converted.image, converted.video],
                            kind: BatchAssetKind::LivePhoto,
                            disposition: BatchSuccessDisposition::Converted,
                        })
                    }
                    Err(error) => {
                        checkpoint_event = Some(BatchCheckpointEvent {
                            source: item.input.clone(),
                            image: item.output.clone(),
                            video: output_video,
                            signature,
                            outcome: OwnedCheckpointOutcome::Failure(error.to_string()),
                        });
                        Err(error)
                    }
                }
            }
            SourceAsset::ProXdr { .. } => {
                if item.output.exists() {
                    return Err(RuntimeError::new(
                        "batch output",
                        "output already exists; refusing to overwrite",
                    ));
                }
                let converted =
                    runtime.convert_proxdr_file(&source, &item.output, request, |_| {})?;
                Ok(BatchSuccess {
                    input: item.input.clone(),
                    outputs: vec![converted.output],
                    kind: BatchAssetKind::ProXdr,
                    disposition: BatchSuccessDisposition::Converted,
                })
            }
        }
    })();

    BatchWorkResult {
        item,
        result,
        checkpoint: checkpoint_event,
    }
}

fn finalize_work_result(
    work: BatchWorkResult,
    writer: &mut Option<MotionPhotoCheckpointWriter>,
    checkpoint_path: Option<&Path>,
) -> OrderedBatchOutcome {
    let BatchWorkResult {
        item,
        mut result,
        checkpoint,
    } = work;
    if let Some(event) = checkpoint {
        if let Err(error) = event.append(writer, checkpoint_path) {
            result = Err(error);
        }
    }
    match result {
        Ok(success) => OrderedBatchOutcome::Success(success),
        Err(error) => OrderedBatchOutcome::Failure(BatchFailure {
            input: item.input,
            output: item.output,
            error: error.to_string(),
        }),
    }
}

fn receipt_from_ordered(outcomes: Vec<Option<OrderedBatchOutcome>>) -> BatchReceipt {
    let mut receipt = BatchReceipt::default();
    for outcome in outcomes.into_iter().flatten() {
        match outcome {
            OrderedBatchOutcome::Success(success) => receipt.successes.push(success),
            OrderedBatchOutcome::Failure(failure) => receipt.failures.push(failure),
        }
    }
    receipt
}

impl PortableRuntime {
    fn convert_batch_item(
        &self,
        item: &BatchItem,
        request: ConversionRequest,
    ) -> Result<BatchSuccess> {
        if item.input == item.output {
            return Err(RuntimeError::new(
                "batch output",
                "batch conversion never overwrites its source",
            ));
        }
        if item.output.exists() {
            return Err(RuntimeError::new(
                "batch output",
                "output already exists; refusing to overwrite",
            ));
        }
        create_output_parent(&item.output)?;
        let source = fs::read(&item.input)
            .map_err(|error| RuntimeError::external("batch input read", error))?;
        let asset = probe_bytes(&source)
            .map_err(|error| RuntimeError::external("batch source probe", error))?;

        match asset {
            SourceAsset::MotionPhoto { .. } => {
                let receipt = self.convert_motion_photo_file(&source, &item.input, &item.output)?;
                Ok(BatchSuccess {
                    input: item.input.clone(),
                    outputs: vec![receipt.image, receipt.video],
                    kind: BatchAssetKind::LivePhoto,
                    disposition: BatchSuccessDisposition::Converted,
                })
            }
            SourceAsset::ProXdr { .. } => {
                let receipt = self.convert_proxdr_file(&source, &item.output, request, |_| {})?;
                Ok(BatchSuccess {
                    input: item.input.clone(),
                    outputs: vec![receipt.output],
                    kind: BatchAssetKind::ProXdr,
                    disposition: BatchSuccessDisposition::Converted,
                })
            }
        }
    }

    pub fn convert_batch<I>(&self, items: I, request: ConversionRequest) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
        let mut receipt = BatchReceipt::default();
        for item in items {
            match self.convert_batch_item(&item, request) {
                Ok(success) => receipt.successes.push(success),
                Err(error) => receipt.failures.push(BatchFailure {
                    input: item.input,
                    output: item.output,
                    error: error.to_string(),
                }),
            }
        }
        receipt
    }

    pub fn convert_batch_with_options<I>(
        &self,
        items: I,
        request: ConversionRequest,
        options: &BatchExecutionOptions,
    ) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
        let checkpoint = options
            .checkpoint_path
            .as_deref()
            .map(MotionPhotoCheckpoint::load)
            .transpose();
        let checkpoint = match checkpoint {
            Ok(value) => value.unwrap_or_default(),
            Err(error) => {
                return BatchReceipt {
                    successes: Vec::new(),
                    failures: vec![BatchFailure {
                        input: PathBuf::new(),
                        output: options.checkpoint_path.clone().unwrap_or_default(),
                        error: error.to_string(),
                    }],
                };
            }
        };

        let items = items.into_iter().collect::<Vec<_>>();
        if items.is_empty() {
            return BatchReceipt::default();
        }
        let jobs = options.jobs.max(1).min(items.len());
        let mut writer: Option<MotionPhotoCheckpointWriter> = None;
        let mut outcomes = (0..items.len()).map(|_| None).collect::<Vec<_>>();

        if jobs == 1 {
            for (index, item) in items.iter().cloned().enumerate() {
                let work = process_batch_item_with_options(
                    self,
                    item,
                    request,
                    &checkpoint,
                    options.reuse_existing,
                );
                outcomes[index] = Some(finalize_work_result(
                    work,
                    &mut writer,
                    options.checkpoint_path.as_deref(),
                ));
            }
            return receipt_from_ordered(outcomes);
        }

        let next = AtomicUsize::new(0);
        let (sender, receiver) = mpsc::sync_channel::<(usize, BatchWorkResult)>(jobs);
        let reuse_existing = options.reuse_existing;
        thread::scope(|scope| {
            for _ in 0..jobs {
                let sender = sender.clone();
                let items = &items;
                let checkpoint = &checkpoint;
                let next = &next;
                scope.spawn(move || {
                    let runtime = PortableRuntime::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index).cloned() else {
                            break;
                        };
                        let work = process_batch_item_with_options(
                            &runtime,
                            item,
                            request,
                            checkpoint,
                            reuse_existing,
                        );
                        if sender.send((index, work)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(sender);
            for _ in 0..items.len() {
                let (index, work) = receiver
                    .recv()
                    .expect("batch workers must return exactly one result per planned item");
                outcomes[index] = Some(finalize_work_result(
                    work,
                    &mut writer,
                    options.checkpoint_path.as_deref(),
                ));
            }
        });
        receipt_from_ordered(outcomes)
    }
}

'''
    batch = batch[:start] + replacement + batch[end:]

for marker in (
    "pub jobs: usize,",
    "struct BatchCheckpointEvent",
    "mpsc::sync_channel::<(usize, BatchWorkResult)>(jobs)",
    "thread::scope(|scope|",
    "outcomes[index] = Some(finalize_work_result",
):
    if marker not in batch:
        raise SystemExit(f"batch jobs wiring missing marker: {marker}")
batch_path.write_text(batch)

cli_path = Path("crates/xdremux-cli/src/lib.rs")
cli = cli_path.read_text()

if "fn default_batch_jobs() -> usize" not in cli:
    anchor = "#[derive(Debug, Args)]\nstruct BatchArgs {\n"
    helper = """fn default_batch_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(4)
}

"""
    if anchor not in cli:
        raise SystemExit("BatchArgs anchor not found")
    cli = cli.replace(anchor, helper + anchor, 1)

jobs_field = """    /// Maximum number of concurrent conversions. Zero is treated as one.
    #[arg(long, default_value_t = default_batch_jobs(), value_name = "N")]
    jobs: usize,
"""
if "jobs: usize," not in cli:
    anchor = """    /// Emit one machine-readable JSON receipt instead of human progress.
    #[arg(long)]
    json: bool,
"""
    if anchor not in cli:
        raise SystemExit("BatchArgs JSON field anchor not found")
    cli = cli.replace(anchor, jobs_field + anchor, 1)

old_execution = """    let execution_options = BatchExecutionOptions {
        checkpoint_path,
        reuse_existing,
    };
"""
new_execution = """    let execution_options = BatchExecutionOptions {
        checkpoint_path,
        reuse_existing,
        jobs: arguments.jobs.max(1),
    };
"""
if old_execution in cli:
    cli = cli.replace(old_execution, new_execution, 1)
elif "jobs: arguments.jobs.max(1)," not in cli:
    raise SystemExit("BatchExecutionOptions CLI anchor not found")

cli = cli.replace(
    "            checkpoint: None,\n            json: false,",
    "            checkpoint: None,\n            jobs: 1,\n            json: false,",
)
if "assert!(arguments.jobs >= 1 && arguments.jobs <= 4);" not in cli:
    anchor = "        assert_eq!(arguments.checkpoint, None);\n"
    if anchor not in cli:
        raise SystemExit("batch argument assertion anchor not found")
    cli = cli.replace(
        anchor,
        anchor + "        assert!(arguments.jobs >= 1 && arguments.jobs <= 4);\n",
        1,
    )

for marker in (
    "fn default_batch_jobs() -> usize",
    "jobs: usize,",
    "jobs: arguments.jobs.max(1),",
):
    if marker not in cli:
        raise SystemExit(f"CLI jobs wiring missing marker: {marker}")
cli_path.write_text(cli)
