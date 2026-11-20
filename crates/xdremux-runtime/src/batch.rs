use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use xdremux_engine::{
    ConversionRequest, ConversionTarget, LivePhotoOutput, MotionOutputTarget,
    MotionVideoCompatibility, TargetAssetKind,
};
use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions};
use xdremux_source::{probe_bytes, SourceAsset};

use crate::android_motion_photo::resolve_motion_output_target;
use crate::batch_checkpoint::{
    canonical_or_absolute, source_signature, source_signature_path, validate_checkpoint_path,
    MotionPhotoCheckpoint, SourceSignature,
};
use crate::batch_transaction::{
    journal_path, BatchJournal, MotionLayout, Publication, Stage, TransactionIndex, TransactionKey,
};
use crate::categorize::classification_relative_directory;
use crate::vendor_motion_metadata::VendorMetadataCarry;
use crate::{
    is_harmony_motion_photo, ApplePortraitSemanticProfile, PortableRuntime, Result, RuntimeError,
};

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
    pub photographic_styles: bool,
    pub live_photo_output: LivePhotoOutput,
    /// Android targets use one MP-suffixed output, not an Apple pair.
    pub motion_target: Option<MotionOutputTarget>,
    pub motion_video_compatibility: MotionVideoCompatibility,
    /// Included in the Android checkpoint key so vendor metadata policies do not reuse each other.
    pub target: Option<ConversionTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchExecutionOptions {
    pub checkpoint_path: Option<PathBuf>,
    pub reuse_existing: bool,
    pub jobs: usize,
    /// Resolved Apple capability helper for Rust-owned Apple feature intents.
    /// The public CLI supplies this only for a packaged macOS deployment.
    pub apple_adapter_executable: Option<PathBuf>,
    pub motion_target: Option<MotionOutputTarget>,
    /// Destination profile, resolved per asset rather than across a mixed batch.
    pub target: Option<ConversionTarget>,
    pub motion_video_compatibility: MotionVideoCompatibility,
}

impl Default for BatchExecutionOptions {
    fn default() -> Self {
        Self {
            checkpoint_path: None,
            reuse_existing: false,
            jobs: 1,
            apple_adapter_executable: None,
            motion_target: None,
            target: None,
            motion_video_compatibility: MotionVideoCompatibility::Preserve,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CinematicFocalLength {
    Automatic,
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
            focal_length: CinematicFocalLength::Automatic,
            aperture: 2.8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchAssetKind {
    ProXdr,
    LivePhoto,
    AndroidMotionPhoto,
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
    pub published_outputs: Vec<PathBuf>,
    pub published_unrecorded: bool,
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
    android: Option<MotionOutputTarget>,
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
    if android.is_some() {
        name.push("MP");
    }
    name.push(if cinematic {
        ".mov"
    } else if pvt {
        ".pvt"
    } else if android == Some(MotionOutputTarget::GoogleJpeg) {
        ".jpg"
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
    let Some(prior) =
        checkpoint.reusable_item(input, &signature, format == LivePhotoOutput::Pvt)?
    else {
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

fn google_target(target: Option<MotionOutputTarget>) -> Option<MotionOutputTarget> {
    match target {
        Some(
            target @ (MotionOutputTarget::GoogleAuto
            | MotionOutputTarget::GoogleJpeg
            | MotionOutputTarget::GoogleHeic),
        ) => Some(target),
        _ => None,
    }
}

fn google_target_name(target: MotionOutputTarget) -> &'static str {
    match target {
        MotionOutputTarget::GoogleJpeg => "google-jpeg",
        MotionOutputTarget::GoogleHeic => "google-heic",
        _ => unreachable!("Android checkpoint requires a Google target"),
    }
}

fn destination_name(target: Option<ConversionTarget>) -> Option<&'static str> {
    target.map(ConversionTarget::name)
}

fn video_compatibility_name(policy: MotionVideoCompatibility) -> &'static str {
    match policy {
        MotionVideoCompatibility::Preserve => "preserve",
    }
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
    if let Some(path) = options.checkpoint_path.as_deref() {
        validate_checkpoint_path(path, inputs.iter().map(PathBuf::as_path))?;
        validate_checkpoint_path(&journal_path(path), inputs.iter().map(PathBuf::as_path))?;
    }
    let checkpoint = if options.reuse_existing {
        match options.checkpoint_path.as_deref() {
            Some(path) => Some(MotionPhotoCheckpoint::load(path)?),
            None => None,
        }
    } else {
        None
    };
    let transactions = options
        .checkpoint_path
        .as_deref()
        .map(TransactionIndex::load)
        .transpose()?
        .unwrap_or_default();
    let mut reserved = inputs.iter().cloned().collect::<BTreeSet<_>>();
    let mut items = Vec::with_capacity(inputs.len());

    for input in inputs {
        let file_bytes = fs::read(input).ok();
        let harmony = options.motion_target == Some(MotionOutputTarget::GoogleAuto)
            && options.target == Some(ConversionTarget::Huawei)
            && file_bytes
                .as_ref()
                .is_some_and(|bytes| is_harmony_motion_photo(bytes));
        let analysis = (google_target(options.motion_target).is_some()
            || options.live_photo_output == LivePhotoOutput::Pvt
            || options.reuse_existing)
            .then_some(file_bytes)
            .flatten()
            .and_then(|source| probe_bytes(&source).ok().map(|asset| (source, asset)));
        let motion = harmony
            || analysis
                .as_ref()
                .is_some_and(|(_, asset)| matches!(asset, SourceAsset::MotionPhoto { .. }));
        let android = if harmony {
            Some(MotionOutputTarget::GoogleHeic)
        } else if motion {
            google_target(options.motion_target).and_then(|target| {
                if target == MotionOutputTarget::GoogleAuto {
                    let (source, asset) = analysis.as_ref()?;
                    resolve_motion_output_target(target, source, asset).ok()
                } else {
                    Some(target)
                }
            })
        } else {
            None
        };
        let pvt = android.is_none() && options.live_photo_output == LivePhotoOutput::Pvt && motion;
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

        let transaction = if options.reuse_existing && motion {
            let signature = source_signature_path(input)?;
            let route = match android {
                Some(target) => {
                    android_route(target, options.motion_video_compatibility, options.target)
                }
                None => apple_route(format, options.photographic_styles),
            };
            let key = TransactionKey::new(input, &signature, route, &parent)?;
            transactions.get(&key).map(|entry| entry.output.clone())
        } else {
            None
        };
        let prior = if transaction.is_some() {
            transaction
        } else {
            match (checkpoint.as_ref(), android) {
                (Some(state), Some(target)) => {
                    let signature = source_signature_path(input)?;
                    state
                        .reusable_android_item(
                            input,
                            &signature,
                            google_target_name(target),
                            video_compatibility_name(options.motion_video_compatibility),
                            destination_name(options.target),
                        )?
                        .map(|item| item.output)
                        .filter(|output| {
                            output.parent().is_some_and(|p| {
                                canonical_or_absolute(&parent).ok().as_deref() == Some(p)
                            })
                        })
                }
                (Some(state), None) if !options.photographic_styles => {
                    reusable_planned_output(state, input, &parent, format)?
                }
                _ => None,
            }
        };
        if let Some(prior) = prior {
            let companion = if pvt || android.is_some() {
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
            let candidate = output_candidate(
                input,
                &parent,
                suffix,
                options.cinematic_output,
                pvt,
                android,
            )?;
            let companion = if pvt || android.is_some() {
                candidate.clone()
            } else {
                candidate.with_extension("mov")
            };
            if candidate != *input
                && !reserved.contains(&candidate)
                && !reserved.contains(&companion)
                && fs::symlink_metadata(&candidate).is_err()
                && fs::symlink_metadata(&companion).is_err()
                && !transactions.reserves(&crate::batch_transaction::destination_path(&candidate)?)
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

fn matches_android_output(
    output: &Path,
    expected_sha256: &str,
    target: MotionOutputTarget,
) -> bool {
    let Ok(signature) = source_signature_path(output) else {
        return false;
    };
    if signature.sha256 != expected_sha256 {
        return false;
    }
    let Ok(bytes) = fs::read(output) else {
        return false;
    };
    let Ok(photo) = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict()) else {
        return false;
    };
    if photo.validate().is_err() {
        return false;
    }
    let expected_mime = match target {
        MotionOutputTarget::GoogleJpeg => "image/jpeg",
        MotionOutputTarget::GoogleHeic => "image/heic",
        _ => return false,
    };
    let matching_primary = photo.resources().any(|resource| {
        resource.role == xdremux_motion_photo::ResourceRole::PrimaryStill
            && resource.mime == expected_mime
    });
    matching_primary
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

fn apple_route(format: LivePhotoOutput, photographic_styles: bool) -> String {
    let container = match format {
        LivePhotoOutput::Pair => "apple-pair",
        LivePhotoOutput::Pvt => "apple-pvt",
    };
    if photographic_styles {
        format!("{container}:styles")
    } else {
        container.to_owned()
    }
}

fn android_route(
    target: MotionOutputTarget,
    compatibility: MotionVideoCompatibility,
    destination: Option<ConversionTarget>,
) -> String {
    format!(
        "{}:{}:{}",
        google_target_name(target),
        video_compatibility_name(compatibility),
        destination_name(destination).unwrap_or("legacy")
    )
}

#[derive(Default, Debug)]
struct PublicationProgress {
    outputs: Vec<PathBuf>,
    complete: bool,
}

struct MotionWork<'a> {
    item: &'a BatchItem,
    kind: BatchAssetKind,
    layout: MotionLayout,
    transaction: Option<(&'a BatchJournal, TransactionKey)>,
    prior: Option<Publication>,
}

impl<'a> MotionWork<'a> {
    fn new(
        item: &'a BatchItem,
        signature: &SourceSignature,
        layout: MotionLayout,
        route: String,
        journal: Option<&'a BatchJournal>,
    ) -> Result<Self> {
        let mut prior = None;
        let transaction = if let Some(journal) = journal {
            let key = TransactionKey::new(
                &item.input,
                signature,
                route,
                crate::publication_parent(&item.output),
            )?;
            prior = journal.get(&key)?;
            Some((journal, key))
        } else {
            None
        };
        let kind = if layout == MotionLayout::File {
            BatchAssetKind::AndroidMotionPhoto
        } else {
            BatchAssetKind::LivePhoto
        };
        Ok(Self {
            item,
            kind,
            layout,
            transaction,
            prior,
        })
    }

    fn output_paths(&self) -> Vec<PathBuf> {
        if self.layout == MotionLayout::Pair {
            vec![
                self.item.output.clone(),
                self.item.output.with_extension("mov"),
            ]
        } else {
            vec![self.item.output.clone()]
        }
    }

    fn receipt(
        &self,
        disposition: BatchSuccessDisposition,
        omitted_resource_count: Option<usize>,
    ) -> BatchSuccess {
        BatchSuccess {
            input: self.item.input.clone(),
            outputs: self.output_paths(),
            kind: self.kind,
            disposition,
            omitted_resource_count,
        }
    }

    fn restore(
        &self,
        allow_reuse: bool,
        progress: &mut PublicationProgress,
    ) -> Result<Option<BatchSuccess>> {
        let Some((journal, key)) = &self.transaction else {
            return Ok(None);
        };
        let Some(prior) = &self.prior else {
            return Ok(None);
        };
        if !allow_reuse {
            if !prior.is_committed() {
                return Err(RuntimeError::new(
                    "Motion Photo transaction",
                    "an incomplete publication exists for this request; run with --resume",
                ));
            }
            return Ok(None);
        }
        if prior.output != crate::batch_transaction::destination_path(&self.item.output)? {
            return Err(RuntimeError::new(
                "Motion Photo transaction",
                "checkpoint changed after batch planning; retry --resume",
            ));
        }
        if !prior.publish(self.layout, &mut progress.outputs)? {
            return Ok(None);
        }
        if !prior.is_committed() {
            progress.complete = true;
            journal.append(key, &prior.clone().committed())?;
        }
        prior.remove_staging();
        Ok(Some(
            self.receipt(BatchSuccessDisposition::SkippedExisting, None),
        ))
    }

    fn convert(
        &self,
        progress: &mut PublicationProgress,
        build: impl FnOnce(&Path) -> Result<usize>,
    ) -> Result<BatchSuccess> {
        let omitted = if let Some((journal, key)) = &self.transaction {
            let stage = Stage::new(&self.item.output)?;
            let omitted = build(&stage.output)?;
            let publication = Publication::prepared(&self.item.output, self.layout, &stage)?;
            journal.append(key, &publication)?;
            let commit = (|| {
                if !publication.publish(self.layout, &mut progress.outputs)? {
                    return Err(RuntimeError::new(
                        "Motion Photo transaction",
                        "prepared stage is missing",
                    ));
                }
                progress.complete = true;
                journal.append(key, &publication.committed())
            })();
            if let Err(error) = commit {
                stage.retain();
                return Err(error);
            }
            omitted
        } else {
            build(&self.item.output)?
        };
        Ok(self.receipt(BatchSuccessDisposition::Converted, Some(omitted)))
    }
}

#[derive(Debug)]
struct BatchWorkResult {
    item: BatchItem,
    result: Result<BatchSuccess>,
    publication: PublicationProgress,
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
    journal: Option<&BatchJournal>,
    options: &BatchExecutionOptions,
    apple_options: &BatchAppleOptions,
) -> BatchWorkResult {
    #[cfg(not(target_os = "macos"))]
    let _ = (apple_options.portrait_profile, &apple_options.cinematic);
    let mut publication = PublicationProgress::default();
    let reuse_existing = options.reuse_existing;
    let apple_adapter_executable = options.apple_adapter_executable.as_deref();
    let motion_target = options.motion_target;
    let motion_video_compatibility = options.motion_video_compatibility;
    let result: Result<BatchSuccess> = (|| {
        request
            .validate_hdr_profile_intents()
            .map_err(|e| RuntimeError::external("HDR profile batch intent", e))?;
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
        if motion_target == Some(MotionOutputTarget::GoogleAuto)
            && options.target == Some(ConversionTarget::Huawei)
            && is_harmony_motion_photo(&source)
        {
            let signature = source_signature(&item.input, &source)?;
            let target = MotionOutputTarget::GoogleHeic;
            let work = MotionWork::new(
                &item,
                &signature,
                MotionLayout::File,
                android_route(target, motion_video_compatibility, options.target),
                journal,
            )?;
            if let Some(reused) = work.restore(reuse_existing, &mut publication)? {
                return Ok(reused);
            }
            if reuse_existing {
                let previous = checkpoint.reusable_android_item(
                    &item.input,
                    &signature,
                    google_target_name(target),
                    video_compatibility_name(motion_video_compatibility),
                    destination_name(options.target),
                )?;
                if let Some(prior) = previous {
                    let current = fs::read(&item.output).ok();
                    if prior.output == canonical_or_absolute(&item.output)?
                        && current.as_deref() == Some(source.as_slice())
                        && signature.sha256 == prior.sha256
                    {
                        return Ok(work.receipt(BatchSuccessDisposition::SkippedExisting, Some(0)));
                    }
                }
            }
            if fs::symlink_metadata(&item.output).is_ok() {
                return Err(RuntimeError::new(
                    "batch resume provenance",
                    "existing Harmony output does not match the preserved source; refusing to reuse or overwrite",
                ));
            }
            return work.convert(&mut publication, |destination| {
                runtime
                    .convert_huawei_harmony_file(
                        &source,
                        destination,
                        motion_video_compatibility,
                        VendorMetadataCarry::from_target(ConversionTarget::Huawei),
                    )
                    .map(|receipt| receipt.omitted_resource_count)
            });
        }
        let asset = probe_bytes(&source)
            .map_err(|error| RuntimeError::external("batch source probe", error))?;
        let request = if let Some(target) = options.target {
            let kind = match &asset {
                SourceAsset::MotionPhoto { .. } => TargetAssetKind::MotionPhoto,
                SourceAsset::ProXdr { .. } => TargetAssetKind::ProXdr,
            };
            target.apply_request(kind, request).map_err(|reason| {
                RuntimeError::new(
                    "batch destination target",
                    format!("{}: {reason}", target.name()),
                )
            })?
        } else {
            request
        };
        let resolved_android = if matches!(asset, SourceAsset::MotionPhoto { .. }) {
            google_target(motion_target)
                .map(|target| resolve_motion_output_target(target, &source, &asset))
                .transpose()?
        } else {
            None
        };
        if request.hdr_profile != xdremux_engine::HdrExportProfile::Compatible
            && !matches!(asset, SourceAsset::ProXdr { .. })
        {
            return Err(RuntimeError::new(
                "HDR profile batch intent",
                "explicit HDR profiles apply only to ProXDR still images",
            ));
        }

        match asset {
            SourceAsset::MotionPhoto { .. } => {
                let signature = source_signature(&item.input, &source)?;
                if let Some(target) = resolved_android {
                    runtime.validate_android_motion_request(request)?;
                    let work = MotionWork::new(
                        &item,
                        &signature,
                        MotionLayout::File,
                        android_route(target, motion_video_compatibility, options.target),
                        journal,
                    )?;
                    if let Some(reused) = work.restore(reuse_existing, &mut publication)? {
                        return Ok(reused);
                    }
                    if reuse_existing {
                        let previous = checkpoint.reusable_android_item(
                            &item.input,
                            &signature,
                            google_target_name(target),
                            video_compatibility_name(motion_video_compatibility),
                            destination_name(options.target),
                        )?;
                        if let Some(prior) = previous {
                            if prior.output == canonical_or_absolute(&item.output)?
                                && matches_android_output(&item.output, &prior.sha256, target)
                            {
                                return Ok(
                                    work.receipt(BatchSuccessDisposition::SkippedExisting, None)
                                );
                            }
                        }
                    }
                    if fs::symlink_metadata(&item.output).is_ok() {
                        return Err(RuntimeError::new(
                            "batch resume provenance",
                            "existing Google Motion Photo has no matching source, target, video policy and output digest; refusing to reuse or overwrite",
                        ));
                    }
                    let carry = options
                        .target
                        .map(|destination| {
                            VendorMetadataCarry::from_target(
                                destination.android_motion_destination(
                                    target == MotionOutputTarget::GoogleHeic,
                                ),
                            )
                        })
                        .unwrap_or_else(VendorMetadataCarry::legacy);
                    return work.convert(&mut publication, |output| {
                        runtime
                            .convert_android_motion_photo_file_with_carry(
                                &source,
                                &item.input,
                                output,
                                target,
                                request,
                                motion_video_compatibility,
                                carry,
                            )
                            .map(|receipt| receipt.omitted_resource_count)
                    });
                }

                let styles = request.apple_features.photographic_styles;
                runtime.validate_live_photo_request(request, styles)?;
                if styles {
                    #[cfg(not(target_os = "macos"))]
                    return Err(RuntimeError::new(
                        "Apple capability",
                        "Live Photo Styles requires the macOS Apple adapter",
                    ));
                    #[cfg(target_os = "macos")]
                    if apple_adapter_executable.is_none() {
                        return Err(RuntimeError::new(
                            "Apple capability",
                            "Live Photo Styles requires a resolved Apple adapter",
                        ));
                    }
                }

                let pvt = request.live_photo_output == LivePhotoOutput::Pvt;
                let layout = if pvt {
                    MotionLayout::Package
                } else {
                    MotionLayout::Pair
                };
                let work = MotionWork::new(
                    &item,
                    &signature,
                    layout,
                    apple_route(request.live_photo_output, styles),
                    journal,
                )?;
                if let Some(reused) = work.restore(reuse_existing, &mut publication)? {
                    return Ok(reused);
                }
                let (output_image, output_video) = if pvt {
                    let members = crate::live_photo_package::planned_members(&item.output)?;
                    (members.image, members.video)
                } else {
                    (item.output.clone(), item.output.with_extension("mov"))
                };
                if reuse_existing && !styles {
                    if let Some(prior) = checkpoint.reusable_item(&item.input, &signature, pvt)? {
                        if prior.image == canonical_or_absolute(&output_image)?
                            && prior.video == canonical_or_absolute(&output_video)?
                            && prior.package
                                == pvt
                                    .then(|| canonical_or_absolute(&item.output))
                                    .transpose()?
                            && (!pvt
                                || crate::live_photo_package::read_members(&item.output).is_ok_and(
                                    |members| {
                                        members.image == output_image
                                            && members.video == output_video
                                    },
                                ))
                            && pair_matches_identifier(
                                &output_image,
                                &output_video,
                                &prior.asset_identifier,
                            )
                        {
                            return Ok(work.receipt(BatchSuccessDisposition::SkippedExisting, None));
                        }
                    }
                }
                if work
                    .output_paths()
                    .iter()
                    .any(|path| fs::symlink_metadata(path).is_ok())
                {
                    return Err(RuntimeError::new(
                        "batch resume provenance",
                        "existing Live Photo output has no matching source, container and intent; refusing to reuse or overwrite",
                    ));
                }
                work.convert(&mut publication, |output| {
                    #[cfg(target_os = "macos")]
                    if styles {
                        let adapter = apple_adapter_executable.expect("validated adapter");
                        return runtime
                            .convert_apple_live_styles_file(
                                adapter,
                                &source,
                                &item.input,
                                output,
                                request,
                            )
                            .map(|receipt| receipt.omitted_resource_count());
                    }
                    runtime
                        .convert_motion_photo_file_with_request(
                            &source,
                            &item.input,
                            output,
                            request,
                        )
                        .map(|receipt| receipt.omitted_resource_count())
                })
            }
            SourceAsset::ProXdr { .. } => {
                if google_target(motion_target).is_some() {
                    return Err(RuntimeError::new(
                        "batch Motion Photo target",
                        "Google Motion Photo targets apply only to Motion Photo inputs, not ProXDR stills",
                    ));
                }
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
        publication,
    }
}

fn finalize_work_result(work: BatchWorkResult) -> OrderedBatchOutcome {
    match work.result {
        Ok(success) => OrderedBatchOutcome::Success(success),
        Err(error) => OrderedBatchOutcome::Failure(BatchFailure {
            input: work.item.input,
            output: work.item.output,
            error: error.to_string(),
            published_outputs: work.publication.outputs,
            published_unrecorded: work.publication.complete,
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
                    published_outputs: Vec::new(),
                    published_unrecorded: false,
                }],
            };
        }
        let items = items.into_iter().collect::<Vec<_>>();
        if items.is_empty() {
            return BatchReceipt::default();
        }
        let checkpoint: Result<(MotionPhotoCheckpoint, Option<BatchJournal>)> = (|| {
            let Some(path) = options.checkpoint_path.as_deref() else {
                return Ok((MotionPhotoCheckpoint::default(), None));
            };
            let protected = items
                .iter()
                .flat_map(|item| {
                    [
                        item.input.clone(),
                        item.output.clone(),
                        item.output.with_extension("mov"),
                    ]
                })
                .collect::<Vec<_>>();
            let journal = BatchJournal::new(path, protected)?;
            let legacy = if options.reuse_existing {
                MotionPhotoCheckpoint::load(path)?
            } else {
                MotionPhotoCheckpoint::default()
            };
            Ok((legacy, Some(journal)))
        })();
        let (checkpoint, journal) = match checkpoint {
            Ok(value) => value,
            Err(error) => {
                return BatchReceipt {
                    successes: Vec::new(),
                    failures: vec![BatchFailure {
                        input: PathBuf::new(),
                        output: options.checkpoint_path.clone().unwrap_or_default(),
                        error: error.to_string(),
                        published_outputs: Vec::new(),
                        published_unrecorded: false,
                    }],
                };
            }
        };
        let jobs = options.jobs.max(1).min(items.len());
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
                    journal.as_ref(),
                    options,
                    &apple_options,
                );
                outcomes[index] = Some(finalize_work_result(work));
            }
            return receipt_from_ordered(outcomes);
        }

        let next = AtomicUsize::new(0);
        let (sender, receiver) = mpsc::sync_channel::<(usize, BatchWorkResult)>(jobs);
        thread::scope(|scope| {
            for _ in 0..jobs {
                let sender = sender.clone();
                let items = &items;
                let checkpoint = &checkpoint;
                let journal = journal.as_ref();
                let next = &next;
                let apple_options = &apple_options;
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
                            journal,
                            options,
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
                outcomes[index] = Some(finalize_work_result(work));
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
                published_outputs: Vec::new(),
                published_unrecorded: false,
            }],
        };
        assert_eq!(receipt.processed(), 2);
        assert_eq!(receipt.succeeded(), 1);
        assert_eq!(receipt.skipped_existing(), 0);
        assert_eq!(receipt.failed(), 1);
        assert!(!receipt.is_success());
    }
}
