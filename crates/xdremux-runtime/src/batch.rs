use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use xdremux_engine::{ConversionRequest, LivePhotoOutput};
use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions};
use xdremux_source::{probe_bytes, SourceAsset};

use crate::batch_checkpoint::{
    canonical_or_absolute, source_signature, source_signature_path, CheckpointOutcome,
    MotionPhotoCheckpoint, MotionPhotoCheckpointWriter, SourceSignature,
};
use crate::categorize::classification_relative_directory;
use crate::{ApplePortraitSemanticProfile, PortableRuntime, Result, RuntimeError};

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
    pub cinematic_output: bool,
    pub live_photo_output: LivePhotoOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchExecutionOptions {
    pub checkpoint_path: Option<PathBuf>,
    pub reuse_existing: bool,
    pub jobs: usize,
    /// Resolved Apple capability helper for Rust-owned Apple feature intents.
    /// The public CLI supplies this only for a packaged macOS deployment.
    pub apple_adapter_executable: Option<PathBuf>,
}

impl Default for BatchExecutionOptions {
    fn default() -> Self {
        Self {
            checkpoint_path: None,
            reuse_existing: false,
            jobs: 1,
            apple_adapter_executable: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CinematicFocalLength {
    Assumed(f32),
    CameraEstimate(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CinematicEnvelopeOptions {
    pub focal_length: CinematicFocalLength,
    pub aperture: f32,
}
impl Default for CinematicEnvelopeOptions {
    fn default() -> Self {
        Self {
            focal_length: CinematicFocalLength::Assumed(24.0),
            aperture: 2.8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchAssetKind {
    ProXdr,
    LivePhoto,
    Cinematic,
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
    /// Known source resources omitted by this conversion. Reused pairs have
    /// no new conversion report, so their count remains unknown.
    pub omitted_resource_count: Option<usize>,
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
    crate::live_photo_package::validate_output_parent(path)?;
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("batch output directory", error))
}

fn publish_new_batch_output(
    output: &Path,
    convert: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let staged = tempfile::Builder::new()
        .prefix(".xdremux-batch-")
        .suffix(".heic")
        .tempfile_in(parent)
        .map_err(|error| RuntimeError::external("batch staging", error))?
        .into_temp_path();
    // Converters atomically replace this private path. Close the original
    // handle first, then publish the resulting inode without replacing a
    // destination that may have appeared since planning or the preflight.
    convert(staged.as_ref())?;
    staged
        .persist_noclobber(output)
        .map_err(|error| RuntimeError::external("batch no-clobber publication", error.error))
}

fn output_candidate(
    input: &Path,
    parent: &Path,
    suffix: u32,
    cinematic: bool,
    pvt: bool,
) -> Result<PathBuf> {
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
    name.push(if cinematic {
        ".mov"
    } else if pvt {
        ".pvt"
    } else {
        ".heic"
    });
    Ok(parent.join(name))
}

fn reusable_planned_output(
    checkpoint: &MotionPhotoCheckpoint,
    input: &Path,
    parent: &Path,
    format: LivePhotoOutput,
) -> Result<Option<PathBuf>> {
    let signature = source_signature_path(input)?;
    let Some(prior) = checkpoint.reusable_item(input, &signature)? else {
        return Ok(None);
    };
    if prior.video != prior.image.with_extension("mov") {
        return Ok(None);
    }
    if prior.package.is_some() != (format == LivePhotoOutput::Pvt) {
        return Ok(None);
    }
    let output = prior.package.as_deref().unwrap_or(&prior.image);
    let desired_parent = canonical_or_absolute(parent)?;
    if output.parent() != Some(desired_parent.as_path()) {
        return Ok(None);
    }
    Ok(Some(output.to_path_buf()))
}

/// Plan all batch outputs before the first conversion write.
///
/// When provenance reuse is enabled, a prior output path is reclaimed only if the
/// source size+SHA-256 still matches the durable checkpoint. Actual HEIC/MOV
/// identifier validation remains an execution-time requirement before reuse.
pub fn plan_batch_items(inputs: &[PathBuf], options: &BatchPlanOptions) -> Result<Vec<BatchItem>> {
    if options.cinematic_output
        && (options.reuse_existing
            || options.checkpoint_path.is_some()
            || options.categorize_output)
    {
        return Err(RuntimeError::new(
            "Cinematic batch planning",
            "Cinematic video does not support photo categorization or Live Photo checkpoints",
        ));
    }
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
        let pvt = options.live_photo_output == LivePhotoOutput::Pvt
            && xdremux_source::inspect_path(input)
                .is_ok_and(|value| matches!(value.asset, SourceAsset::MotionPhoto { .. }));
        let format = if pvt {
            LivePhotoOutput::Pvt
        } else {
            LivePhotoOutput::Pair
        };
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
        crate::live_photo_package::validate_output_parent(&parent.join("planned-output"))?;

        if let Some(prior) = checkpoint
            .as_ref()
            .map(|state| reusable_planned_output(state, input, &parent, format))
            .transpose()?
            .flatten()
        {
            let companion = if pvt {
                prior.clone()
            } else {
                prior.with_extension("mov")
            };
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
            let candidate =
                output_candidate(input, &parent, suffix, options.cinematic_output, pvt)?;
            let companion = if pvt {
                candidate.clone()
            } else {
                candidate.with_extension("mov")
            };
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
    MotionPhoto::parse(
        Input::ApplePair {
            still: &image_bytes,
            movie: &video_bytes,
        },
        ParseOptions::strict(),
    )
    .is_ok_and(|photo| photo.asset().pairing.identifier.as_deref() == Some(expected))
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
    package: Option<PathBuf>,
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
            self.package.as_deref(),
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

struct BatchAppleOptions {
    portrait_profile: ApplePortraitSemanticProfile,
    cinematic: CinematicEnvelopeOptions,
}

fn process_batch_item_with_options(
    runtime: &PortableRuntime,
    item: BatchItem,
    request: ConversionRequest,
    checkpoint: &MotionPhotoCheckpoint,
    reuse_existing: bool,
    apple_adapter_executable: Option<&Path>,
    apple_options: &BatchAppleOptions,
) -> BatchWorkResult {
    #[cfg(not(target_os = "macos"))]
    let _ = (apple_options.portrait_profile, &apple_options.cinematic);
    let mut checkpoint_event = None;
    let result: Result<BatchSuccess> = (|| {
        if item.input == item.output {
            return Err(RuntimeError::new(
                "batch output",
                "batch conversion never overwrites its source",
            ));
        }
        if request.apple_features.cinematic {
            if request.apple_features.portrait
                || request.apple_features.photographic_styles
                || request.requests_oppo_gallery_compatibility()
                || reuse_existing
            {
                return Err(RuntimeError::new(
                    "Cinematic batch intent",
                    "Cinematic video intent is exclusive and cannot reuse Live Photo checkpoints",
                ));
            }
            #[cfg(target_os = "macos")]
            {
                let adapter = apple_adapter_executable.ok_or_else(|| {
                    RuntimeError::new(
                        "Apple capability",
                        "Cinematic intent requires a resolved Apple adapter",
                    )
                })?;
                create_output_parent(&item.output)?;
                let receipt = runtime.generate_cinematic_envelope_file(
                    adapter,
                    &item.input,
                    &item.output,
                    &apple_options.cinematic,
                )?;
                return Ok(BatchSuccess {
                    input: item.input.clone(),
                    outputs: vec![receipt.output],
                    kind: BatchAssetKind::Cinematic,
                    disposition: BatchSuccessDisposition::Converted,
                    omitted_resource_count: None,
                });
            }
            #[cfg(not(target_os = "macos"))]
            return Err(RuntimeError::new(
                "Apple capability",
                "Cinematic intent requires macOS Cinematic frameworks",
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
                let package = (request.live_photo_output == LivePhotoOutput::Pvt)
                    .then(|| item.output.clone());
                let (output_image, output_video) = if package.is_some() {
                    let members = crate::live_photo_package::planned_members(&item.output)?;
                    (members.image, members.video)
                } else {
                    (item.output.clone(), item.output.with_extension("mov"))
                };
                let output_paths = || match &package {
                    Some(path) => vec![path.clone()],
                    None => vec![output_image.clone(), output_video.clone()],
                };
                let exists = fs::symlink_metadata(&item.output).is_ok()
                    || (package.is_none() && fs::symlink_metadata(&output_video).is_ok());
                if reuse_existing {
                    if let Some(prior) = checkpoint.reusable_item(&item.input, &signature)? {
                        if prior.image == canonical_or_absolute(&output_image)?
                            && prior.video == canonical_or_absolute(&output_video)?
                            && prior.package
                                == package.as_deref().map(canonical_or_absolute).transpose()?
                        {
                            let layout_matches = package.is_none()
                                || crate::live_photo_package::read_members(&item.output).is_ok_and(
                                    |members| {
                                        members.image == output_image
                                            && members.video == output_video
                                    },
                                );
                            if layout_matches
                                && pair_matches_identifier(
                                    &output_image,
                                    &output_video,
                                    &prior.asset_identifier,
                                )
                            {
                                checkpoint_event = Some(BatchCheckpointEvent {
                                    source: item.input.clone(),
                                    image: output_image.clone(),
                                    video: output_video.clone(),
                                    package: package.clone(),
                                    signature,
                                    outcome: OwnedCheckpointOutcome::SkippedExisting(
                                        prior.asset_identifier,
                                    ),
                                });
                                return Ok(BatchSuccess {
                                    input: item.input.clone(),
                                    outputs: output_paths(),
                                    kind: BatchAssetKind::LivePhoto,
                                    disposition: BatchSuccessDisposition::SkippedExisting,
                                    omitted_resource_count: None,
                                });
                            }
                            if exists {
                                return Err(RuntimeError::new("batch resume provenance",
                                    "checkpoint matches the source, but the existing Live Photo output no longer matches its recorded container and asset identifier; refusing to overwrite"));
                            }
                        }
                    }
                    if exists {
                        return Err(RuntimeError::new("batch resume provenance",
                            "existing Live Photo output has no matching source provenance; refusing to reuse or overwrite"));
                    }
                } else if exists {
                    return Err(RuntimeError::new(
                        "batch output",
                        "Live Photo output already exists; refusing to overwrite",
                    ));
                }

                match runtime.convert_motion_photo_file_with_request(
                    &source,
                    &item.input,
                    &item.output,
                    request,
                ) {
                    Ok(converted) => {
                        let omitted_resource_count = Some(converted.omitted_resource_count());
                        checkpoint_event = Some(BatchCheckpointEvent {
                            source: item.input.clone(),
                            image: converted.image.clone(),
                            video: converted.video.clone(),
                            package: converted.package.clone(),
                            signature,
                            outcome: OwnedCheckpointOutcome::Success(
                                converted.content_identifier.clone(),
                            ),
                        });
                        Ok(BatchSuccess {
                            input: item.input.clone(),
                            outputs: converted.output_paths(),
                            kind: BatchAssetKind::LivePhoto,
                            disposition: BatchSuccessDisposition::Converted,
                            omitted_resource_count,
                        })
                    }
                    Err(error) => {
                        checkpoint_event = Some(BatchCheckpointEvent {
                            source: item.input.clone(),
                            image: output_image,
                            video: output_video,
                            package,
                            signature,
                            outcome: OwnedCheckpointOutcome::Failure(error.to_string()),
                        });
                        Err(error)
                    }
                }
            }
            SourceAsset::ProXdr { .. } => {
                crate::validate_still_output_path(&item.output)?;
                if item.output.exists() {
                    return Err(RuntimeError::new(
                        "batch output",
                        "output already exists; refusing to overwrite",
                    ));
                }
                publish_new_batch_output(&item.output, |staged| {
                    if request.apple_features.portrait {
                        #[cfg(target_os = "macos")]
                        {
                            let adapter = apple_adapter_executable.ok_or_else(|| {
                                RuntimeError::new(
                                    "Apple Portrait conversion",
                                    "the macOS Apple adapter executable was not resolved",
                                )
                            })?;
                            runtime
                                .convert_apple_portrait_file_with_profile(
                                    adapter,
                                    &source,
                                    staged,
                                    apple_options.portrait_profile,
                                )
                                .map(|_| ())
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = apple_adapter_executable;
                            Err(RuntimeError::new(
                                "Apple Portrait conversion",
                                "Portrait conversion requires macOS ImageIO/Core Image; the complete profile also requires Vision semantics",
                            ))
                        }
                    } else if request.apple_features.photographic_styles {
                        #[cfg(target_os = "macos")]
                        {
                            let adapter = apple_adapter_executable.ok_or_else(|| {
                                RuntimeError::new(
                                    "Photographic Styles conversion",
                                    "the macOS Apple adapter executable was not resolved",
                                )
                            })?;
                            runtime
                                .convert_apple_photographic_styles_file(adapter, &source, staged)
                                .map(|_| ())
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = apple_adapter_executable;
                            Err(RuntimeError::new(
                                "Photographic Styles conversion",
                                "Photographic Styles requires macOS ImageIO/Vision capabilities",
                            ))
                        }
                    } else {
                        runtime
                            .convert_proxdr_file(&source, staged, request, |_| {})
                            .map(|_| ())
                    }
                })?;
                Ok(BatchSuccess {
                    input: item.input.clone(),
                    outputs: vec![item.output.clone()],
                    kind: BatchAssetKind::ProXdr,
                    disposition: BatchSuccessDisposition::Converted,
                    omitted_resource_count: None,
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
        self.convert_batch_with_options_and_portrait_profile(
            items,
            request,
            options,
            ApplePortraitSemanticProfile::Complete,
        )
    }

    pub fn convert_batch_with_options_and_portrait_profile<I>(
        &self,
        items: I,
        request: ConversionRequest,
        options: &BatchExecutionOptions,
        apple_portrait_profile: ApplePortraitSemanticProfile,
    ) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
        self.convert_batch_with_options_and_apple_profiles(
            items,
            request,
            options,
            apple_portrait_profile,
            CinematicEnvelopeOptions::default(),
        )
    }

    pub fn convert_batch_with_options_and_apple_profiles<I>(
        &self,
        items: I,
        request: ConversionRequest,
        options: &BatchExecutionOptions,
        apple_portrait_profile: ApplePortraitSemanticProfile,
        cinematic_options: CinematicEnvelopeOptions,
    ) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
        if request.apple_features.cinematic
            && (options.reuse_existing || options.checkpoint_path.is_some())
        {
            return BatchReceipt {
                successes: Vec::new(),
                failures: vec![BatchFailure {
                    input: PathBuf::new(),
                    output: options.checkpoint_path.clone().unwrap_or_default(),
                    error: "Cinematic video does not support Live Photo checkpoint/resume options"
                        .to_owned(),
                }],
            };
        }
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
        let apple_options = BatchAppleOptions {
            portrait_profile: apple_portrait_profile,
            cinematic: cinematic_options,
        };

        if jobs == 1 {
            for (index, item) in items.iter().cloned().enumerate() {
                let work = process_batch_item_with_options(
                    self,
                    item,
                    request,
                    &checkpoint,
                    options.reuse_existing,
                    options.apple_adapter_executable.as_deref(),
                    &apple_options,
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
        let apple_adapter_executable = options.apple_adapter_executable.clone();
        thread::scope(|scope| {
            for _ in 0..jobs {
                let sender = sender.clone();
                let items = &items;
                let checkpoint = &checkpoint;
                let next = &next;
                let apple_options = &apple_options;
                let apple_adapter_executable = apple_adapter_executable.clone();
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
                            apple_adapter_executable.as_deref(),
                            apple_options,
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
    fn new_batch_publication_preserves_a_destination_created_during_conversion() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("output.heic");
        let error = publish_new_batch_output(&output, |staged| {
            assert!(!output.exists());
            crate::AtomicFilePublisher::new(staged.to_path_buf())
                .publish_bytes(b"converted candidate".to_vec())?;
            fs::write(&output, b"independent output appeared").unwrap();
            Ok(())
        })
        .unwrap_err();
        assert!(error.to_string().contains("batch no-clobber publication"));
        assert_eq!(fs::read(&output).unwrap(), b"independent output appeared");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn new_batch_publication_uses_the_converted_file_and_cleans_private_paths() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("output.heic");
        publish_new_batch_output(&output, |staged| {
            crate::AtomicFilePublisher::new(staged.to_path_buf())
                .publish_bytes(b"converted candidate".to_vec())?;
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"converted candidate");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_batch_conversion_never_publishes_and_cleans_private_paths() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("output.heic");
        let error = publish_new_batch_output(&output, |staged| {
            fs::write(staged, b"partial candidate").unwrap();
            Err(RuntimeError::new("conversion", "synthetic failure"))
        })
        .unwrap_err();
        assert!(error.to_string().contains("synthetic failure"));
        assert!(!output.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn receipt_counts_are_derived_from_outcomes() {
        let receipt = BatchReceipt {
            successes: vec![BatchSuccess {
                input: PathBuf::from("a.heic"),
                outputs: vec![PathBuf::from("out/a.heic")],
                kind: BatchAssetKind::ProXdr,
                disposition: BatchSuccessDisposition::Converted,
                omitted_resource_count: None,
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
