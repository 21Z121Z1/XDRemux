use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use xdremux_engine::ConversionRequest;
use xdremux_motion_photo::{
    read_apple_content_identifier, read_live_photo_content_identifier, read_live_photo_still_time,
    validate_live_photo_movie,
};
use xdremux_source::{probe_bytes, SourceAsset};

use crate::batch_checkpoint::{
    canonical_or_absolute, source_signature, CheckpointOutcome, MotionPhotoCheckpoint,
    MotionPhotoCheckpointWriter, SourceSignature,
};
use crate::categorize::classification_relative_directory;
use crate::{PortableRuntime, Result, RuntimeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItem {
    pub input: PathBuf,
    pub output: PathBuf,
}

impl BatchItem {
    pub fn new(input: impl Into<PathBuf>, output: impl Into<PathBuf>) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BatchPlanOptions {
    pub output_dir: Option<PathBuf>,
    pub checkpoint_path: Option<PathBuf>,
    pub reuse_existing: bool,
    pub categorize_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchAssetKind {
    ProXdr,
    LivePhoto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchSuccessDisposition {
    Converted,
    SkippedExisting,
}

impl BatchSuccessDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Converted => "converted",
            Self::SkippedExisting => "skipped-existing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchSuccess {
    pub input: PathBuf,
    pub outputs: Vec<PathBuf>,
    pub kind: BatchAssetKind,
    pub disposition: BatchSuccessDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchFailure {
    pub input: PathBuf,
    pub output: PathBuf,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BatchReceipt {
    pub successes: Vec<BatchSuccess>,
    pub failures: Vec<BatchFailure>,
}

impl BatchReceipt {
    pub fn processed(&self) -> usize {
        self.successes.len() + self.failures.len()
    }

    pub fn succeeded(&self) -> usize {
        self.successes.len()
    }

    pub fn skipped_existing(&self) -> usize {
        self.successes
            .iter()
            .filter(|item| item.disposition == BatchSuccessDisposition::SkippedExisting)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.failures.len()
    }

    pub fn is_success(&self) -> bool {
        self.failures.is_empty()
    }
}

fn create_output_parent(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("batch output directory", error))
}

fn output_candidate(input: &Path, parent: &Path, suffix: u32) -> Result<PathBuf> {
    let stem = input.file_stem().ok_or_else(|| {
        RuntimeError::new(
            "batch planning",
            format!("input has no file stem: {}", input.display()),
        )
    })?;
    let mut name = stem.to_os_string();
    name.push(".xdremux");
    if suffix > 1 {
        name.push(format!(" ({suffix})"));
    }
    name.push(".heic");
    Ok(parent.join(name))
}

fn reusable_planned_output(
    checkpoint: &MotionPhotoCheckpoint,
    input: &Path,
    parent: &Path,
) -> Result<Option<PathBuf>> {
    let bytes = fs::read(input)
        .map_err(|error| RuntimeError::external("batch resume source read", error))?;
    let signature = source_signature(input, &bytes)?;
    let Some(prior) = checkpoint.reusable_item(input, &signature)? else {
        return Ok(None);
    };
    if prior.video != prior.image.with_extension("mov") {
        return Ok(None);
    }
    let desired_parent = canonical_or_absolute(parent)?;
    if prior.image.parent() != Some(desired_parent.as_path()) {
        return Ok(None);
    }
    Ok(Some(prior.image))
}

/// Plan all batch outputs before the first conversion write.
///
/// When provenance reuse is enabled, a prior output path is reclaimed only if the
/// source size+SHA-256 still matches the durable checkpoint. Actual HEIC/MOV
/// identifier validation remains an execution-time requirement before reuse.
pub fn plan_batch_items(inputs: &[PathBuf], options: &BatchPlanOptions) -> Result<Vec<BatchItem>> {
    let checkpoint = if options.reuse_existing {
        match options.checkpoint_path.as_deref() {
            Some(path) => Some(MotionPhotoCheckpoint::load(path)?),
            None => None,
        }
    } else {
        None
    };
    let mut reserved = inputs.iter().cloned().collect::<BTreeSet<_>>();
    let mut items = Vec::with_capacity(inputs.len());

    for input in inputs {
        let mut parent = match options.output_dir.as_deref() {
            Some(directory) => directory.to_path_buf(),
            None => input
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
        };
        if options.categorize_output {
            parent.push(classification_relative_directory(input)?);
        }

        if let Some(prior) = checkpoint
            .as_ref()
            .map(|state| reusable_planned_output(state, input, &parent))
            .transpose()?
            .flatten()
        {
            let companion = prior.with_extension("mov");
            if prior == *input || reserved.contains(&prior) || reserved.contains(&companion) {
                return Err(RuntimeError::new(
                    "batch resume planning",
                    format!(
                        "checkpoint output for {} collides with another planned path: {}",
                        input.display(),
                        prior.display()
                    ),
                ));
            }
            reserved.insert(prior.clone());
            reserved.insert(companion);
            items.push(BatchItem::new(input.clone(), prior));
            continue;
        }

        let mut suffix = 1_u32;
        let output = loop {
            let candidate = output_candidate(input, &parent, suffix)?;
            let companion = candidate.with_extension("mov");
            if candidate != *input
                && !reserved.contains(&candidate)
                && !reserved.contains(&companion)
                && !candidate.exists()
                && !companion.exists()
            {
                reserved.insert(candidate.clone());
                reserved.insert(companion);
                break candidate;
            }
            suffix = suffix.checked_add(1).ok_or_else(|| {
                RuntimeError::new(
                    "batch planning",
                    format!("exhausted output suffixes for {}", input.display()),
                )
            })?;
        };
        items.push(BatchItem::new(input.clone(), output));
    }
    Ok(items)
}

fn pair_matches_identifier(image: &Path, video: &Path, expected: &str) -> bool {
    let Ok(image_bytes) = fs::read(image) else {
        return false;
    };
    let Ok(video_bytes) = fs::read(video) else {
        return false;
    };
    let Ok(Some(image_id)) = read_apple_content_identifier(&image_bytes) else {
        return false;
    };
    let Ok(Some(video_id)) = read_live_photo_content_identifier(&video_bytes) else {
        return false;
    };
    if image_id != expected || video_id != expected {
        return false;
    }
    let Ok(Some(still_time)) = read_live_photo_still_time(&video_bytes) else {
        return false;
    };
    validate_live_photo_movie(&video_bytes, expected, still_time).is_ok()
}

#[derive(Debug)]
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
            OwnedCheckpointOutcome::Success(identifier) => CheckpointOutcome::Success(identifier),
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

                match runtime.convert_motion_photo_file_with_request(
                    &source,
                    &item.input,
                    &item.output,
                    request,
                ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_counts_are_derived_from_outcomes() {
        let receipt = BatchReceipt {
            successes: vec![BatchSuccess {
                input: PathBuf::from("a.heic"),
                outputs: vec![PathBuf::from("out/a.heic")],
                kind: BatchAssetKind::ProXdr,
                disposition: BatchSuccessDisposition::Converted,
            }],
            failures: vec![BatchFailure {
                input: PathBuf::from("b.heic"),
                output: PathBuf::from("out/b.heic"),
                error: "invalid".to_owned(),
            }],
        };
        assert_eq!(receipt.processed(), 2);
        assert_eq!(receipt.succeeded(), 1);
        assert_eq!(receipt.skipped_existing(), 0);
        assert_eq!(receipt.failed(), 1);
        assert!(!receipt.is_success());
    }
}
