use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::{Builder as TempFileBuilder, NamedTempFile};
use xdremux_classification::{
    classification_contract, classify_user_comment_with_context, detect_capabilities,
    plan_photo_assets, resolve_photo_asset, CategorizationDestinationRoot,
    PhotoAssetCategorizationDisposition, PhotoAssetCategorizationItem, PhotoAssetPlanningInput,
    PhotoAssetType, PhotoClassificationContract, PhotoResourceRole, ResourceFingerprint,
};
use xdremux_motion_photo::{
    parse_oppo_motion_photo, read_apple_content_identifier, read_live_photo_content_identifier,
    read_live_photo_still_time, validate_live_photo_movie,
};

use crate::{PortableRuntime, Result, RuntimeError};

const COPY_BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategorizeDisposition {
    Copied,
    Duplicate,
    DryRun,
    Failed,
}

impl CategorizeDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Copied => "copied",
            Self::Duplicate => "duplicate",
            Self::DryRun => "dry-run",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorizeItemReceipt {
    pub asset_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub role: String,
    pub disposition: CategorizeDisposition,
    pub classification: PhotoClassificationContract,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CategorizeReceipt {
    pub items: Vec<CategorizeItemReceipt>,
}

impl CategorizeReceipt {
    pub fn processed(&self) -> usize {
        self.items.len()
    }

    pub fn copied(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.disposition == CategorizeDisposition::Copied)
            .count()
    }

    pub fn duplicates(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.disposition == CategorizeDisposition::Duplicate)
            .count()
    }

    pub fn dry_run(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.disposition == CategorizeDisposition::DryRun)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.disposition == CategorizeDisposition::Failed)
            .count()
    }

    pub fn is_success(&self) -> bool {
        self.failed() == 0
    }
}

#[derive(Debug)]
struct PreparedCategorization {
    inputs: Vec<PhotoAssetPlanningInput>,
    contracts: BTreeMap<String, PhotoClassificationContract>,
    source_fingerprints: BTreeMap<PathBuf, ResourceFingerprint>,
}

#[derive(Debug)]
struct StagedCopy<'a> {
    item: &'a PhotoAssetCategorizationItem,
    temporary: NamedTempFile,
}

fn role_name(role: PhotoResourceRole) -> &'static str {
    match role {
        PhotoResourceRole::PrimaryImage => "primary-image",
        PhotoResourceRole::PairedVideo => "paired-video",
        PhotoResourceRole::Sidecar => "sidecar",
    }
}

fn eq_ascii_case(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn prefixed_user_comment(data: &[u8]) -> Option<String> {
    for offset in 0..data.len() {
        for prefix in [b"oplus_".as_slice(), b"oppo_".as_slice()] {
            let end = offset.checked_add(prefix.len())?;
            let candidate = data.get(offset..end)?;
            if !eq_ascii_case(candidate, prefix) {
                continue;
            }
            let digits = data[end..]
                .iter()
                .copied()
                .take_while(u8::is_ascii_digit)
                .collect::<Vec<_>>();
            if digits.is_empty() {
                continue;
            }
            let digits = std::str::from_utf8(&digits).ok()?;
            if digits.parse::<u64>().is_err() {
                continue;
            }
            let canonical = if prefix[1] == b'p' {
                "Oplus_"
            } else {
                "Oppo_"
            };
            return Some(format!("{canonical}{digits}"));
        }
    }
    None
}

fn json_user_comment(data: &[u8]) -> Option<String> {
    const KEY: &[u8] = b"oplustag";
    for offset in 0..data.len() {
        let end = offset.checked_add(KEY.len())?;
        let candidate = data.get(offset..end)?;
        if !eq_ascii_case(candidate, KEY) {
            continue;
        }
        let mut cursor = end;
        while data
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'"')
        {
            cursor += 1;
        }
        if data.get(cursor) != Some(&b':') {
            continue;
        }
        cursor += 1;
        while data
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            cursor += 1;
        }
        if data.get(cursor) == Some(&b'"') {
            cursor += 1;
        }
        let start = cursor;
        while data.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == start {
            continue;
        }
        let digits = std::str::from_utf8(data.get(start..cursor)?).ok()?;
        if digits.parse::<u64>().is_err() {
            continue;
        }
        return Some(format!(r#"{{"oplustag":"{digits}"}}"#));
    }
    None
}

fn extract_user_comment(data: &[u8]) -> Option<String> {
    prefixed_user_comment(data).or_else(|| json_user_comment(data))
}

fn companion_video(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let stem = path.file_stem()?;
    let mut candidates = fs::read_dir(parent)
        .ok()?
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() {
                return None;
            }
            let candidate = entry.path();
            let extension = candidate.extension()?.to_str()?;
            if !extension.eq_ignore_ascii_case("mov") || candidate.file_stem()? != stem {
                return None;
            }
            Some(candidate)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.into_iter().next()
}

fn valid_live_photo_pair(image: &[u8], video_path: &Path) -> bool {
    let Ok(Some(image_id)) = read_apple_content_identifier(image) else {
        return false;
    };
    let Ok(video) = fs::read(video_path) else {
        return false;
    };
    let Ok(Some(video_id)) = read_live_photo_content_identifier(&video) else {
        return false;
    };
    if image_id != video_id {
        return false;
    }
    let Ok(Some(still_time)) = read_live_photo_still_time(&video) else {
        return false;
    };
    validate_live_photo_movie(&video, &video_id, still_time).is_ok()
}

fn inferred_asset_type(data: &[u8]) -> PhotoAssetType {
    if matches!(parse_oppo_motion_photo(data), Ok(Some(_))) {
        PhotoAssetType::LivePhoto
    } else {
        PhotoAssetType::StaticPhoto
    }
}

fn fingerprint_path(path: &Path) -> Result<ResourceFingerprint> {
    let mut file = File::open(path)
        .map_err(|error| RuntimeError::external("categorization fingerprint open", error))?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| RuntimeError::external("categorization fingerprint read", error))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(ResourceFingerprint::new(digest.finalize().to_vec()))
}

fn prepare_categorization(inputs: &[PathBuf]) -> Result<PreparedCategorization> {
    let mut ordered = inputs.to_vec();
    ordered.sort();
    ordered.dedup();
    if ordered.is_empty() {
        return Err(RuntimeError::new(
            "categorization planning",
            "at least one input image is required",
        ));
    }

    let mut planning_inputs = Vec::with_capacity(ordered.len());
    let mut contracts = BTreeMap::new();
    let mut source_fingerprints = BTreeMap::new();

    for primary in ordered {
        let bytes = fs::read(&primary)
            .map_err(|error| RuntimeError::external("categorization input read", error))?;
        let companion = companion_video(&primary);
        let inferred = inferred_asset_type(&bytes);
        let asset = resolve_photo_asset(primary.clone(), companion, inferred, |_, video| {
            valid_live_photo_pair(&bytes, video)
        });
        let classification = classify_user_comment_with_context(
            extract_user_comment(&bytes).as_deref(),
            asset.asset_type,
            detect_capabilities(&bytes),
        );
        let mut fingerprints = BTreeMap::new();
        for resource in &asset.resources {
            let fingerprint = fingerprint_path(&resource.path)?;
            source_fingerprints.insert(resource.path.clone(), fingerprint.clone());
            fingerprints.insert(resource.path.clone(), fingerprint);
        }
        contracts.insert(asset.id.clone(), classification_contract(&classification));
        planning_inputs.push(PhotoAssetPlanningInput {
            asset,
            classification,
            fingerprints,
        });
    }

    Ok(PreparedCategorization {
        inputs: planning_inputs,
        contracts,
        source_fingerprints,
    })
}

fn collect_existing_destinations(root: &Path) -> Result<BTreeMap<PathBuf, ResourceFingerprint>> {
    if !root.exists() {
        return Ok(BTreeMap::new());
    }
    if !root.is_dir() {
        return Err(RuntimeError::new(
            "categorization output",
            format!("output root is not a directory: {}", root.display()),
        ));
    }

    let mut pending = vec![root.to_path_buf()];
    let mut existing = BTreeMap::new();
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| RuntimeError::external("categorization output scan", error))?;
        let mut entries = entries
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| RuntimeError::external("categorization output scan", error))?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let file_type = entry
                .file_type()
                .map_err(|error| RuntimeError::external("categorization output scan", error))?;
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".xdremux-categorize-"))
            {
                continue;
            }
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() {
                existing.insert(path.clone(), fingerprint_path(&path)?);
            }
        }
    }
    Ok(existing)
}

fn stage_copy<'a>(item: &'a PhotoAssetCategorizationItem) -> Result<StagedCopy<'a>> {
    let parent = item.destination.parent().ok_or_else(|| {
        RuntimeError::new(
            "categorization destination",
            format!("destination has no parent: {}", item.destination.display()),
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("categorization output directory", error))?;

    let mut source = File::open(&item.source)
        .map_err(|error| RuntimeError::external("categorization source open", error))?;
    let source_permissions = source
        .metadata()
        .map_err(|error| RuntimeError::external("categorization source metadata", error))?
        .permissions();
    let mut temporary = TempFileBuilder::new()
        .prefix(".xdremux-categorize-")
        .tempfile_in(parent)
        .map_err(|error| RuntimeError::external("categorization staging file", error))?;
    io::copy(&mut source, temporary.as_file_mut())
        .map_err(|error| RuntimeError::external("categorization staging copy", error))?;
    temporary
        .as_file()
        .set_permissions(source_permissions)
        .map_err(|error| RuntimeError::external("categorization staging permissions", error))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| RuntimeError::external("categorization staging sync", error))?;
    Ok(StagedCopy { item, temporary })
}

fn rollback_published(paths: &[PathBuf]) -> Option<String> {
    let mut failures = Vec::new();
    for path in paths.iter().rev() {
        if let Err(error) = fs::remove_file(path) {
            failures.push(format!("{}: {error}", path.display()));
        }
    }
    if failures.is_empty() {
        None
    } else {
        Some(format!("rollback failed for {}", failures.join(", ")))
    }
}

fn failed_item(
    item: &PhotoAssetCategorizationItem,
    contract: &PhotoClassificationContract,
    detail: &str,
) -> CategorizeItemReceipt {
    CategorizeItemReceipt {
        asset_id: item.asset_id.clone(),
        source: item.source.clone(),
        destination: item.destination.clone(),
        role: role_name(item.role).to_owned(),
        disposition: CategorizeDisposition::Failed,
        classification: contract.clone(),
        error: Some(detail.to_owned()),
    }
}

fn completed_item(
    item: &PhotoAssetCategorizationItem,
    contract: &PhotoClassificationContract,
    disposition: CategorizeDisposition,
) -> CategorizeItemReceipt {
    CategorizeItemReceipt {
        asset_id: item.asset_id.clone(),
        source: item.source.clone(),
        destination: item.destination.clone(),
        role: role_name(item.role).to_owned(),
        disposition,
        classification: contract.clone(),
        error: None,
    }
}

fn execute_asset(
    items: &[&PhotoAssetCategorizationItem],
    contract: &PhotoClassificationContract,
    source_fingerprints: &BTreeMap<PathBuf, ResourceFingerprint>,
    dry_run: bool,
) -> Vec<CategorizeItemReceipt> {
    if dry_run {
        return items
            .iter()
            .map(|item| {
                let disposition = match item.disposition {
                    PhotoAssetCategorizationDisposition::Duplicate => {
                        CategorizeDisposition::Duplicate
                    }
                    PhotoAssetCategorizationDisposition::Copy => CategorizeDisposition::DryRun,
                };
                completed_item(item, contract, disposition)
            })
            .collect();
    }

    let copy_items = items
        .iter()
        .copied()
        .filter(|item| item.disposition == PhotoAssetCategorizationDisposition::Copy)
        .collect::<Vec<_>>();
    let mut staged = Vec::with_capacity(copy_items.len());
    for item in &copy_items {
        match stage_copy(item) {
            Ok(value) => staged.push(value),
            Err(error) => {
                let detail = format!("asset staging aborted before publication: {error}");
                return items
                    .iter()
                    .map(|candidate| match candidate.disposition {
                        PhotoAssetCategorizationDisposition::Duplicate => {
                            completed_item(candidate, contract, CategorizeDisposition::Duplicate)
                        }
                        PhotoAssetCategorizationDisposition::Copy => {
                            failed_item(candidate, contract, &detail)
                        }
                    })
                    .collect();
            }
        }
    }

    let mut published = Vec::new();
    let mut raced_duplicates = BTreeSet::new();
    for staged_copy in staged {
        let destination = staged_copy.item.destination.clone();
        match staged_copy.temporary.persist_noclobber(&destination) {
            Ok(file) => {
                if let Err(error) = file.sync_all() {
                    let mut detail = format!(
                        "asset publication sync failed for {}: {error}",
                        destination.display()
                    );
                    if let Some(rollback) = rollback_published(&published) {
                        detail.push_str("; ");
                        detail.push_str(&rollback);
                    }
                    let _ = fs::remove_file(&destination);
                    return items
                        .iter()
                        .map(|candidate| match candidate.disposition {
                            PhotoAssetCategorizationDisposition::Duplicate => completed_item(
                                candidate,
                                contract,
                                CategorizeDisposition::Duplicate,
                            ),
                            PhotoAssetCategorizationDisposition::Copy => {
                                failed_item(candidate, contract, &detail)
                            }
                        })
                        .collect();
                }
                published.push(destination);
            }
            Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
                let source_fingerprint = source_fingerprints.get(&staged_copy.item.source);
                let destination_fingerprint = fingerprint_path(&destination).ok();
                if source_fingerprint.is_some()
                    && destination_fingerprint.as_ref() == source_fingerprint
                {
                    raced_duplicates.insert(destination);
                    continue;
                }
                let mut detail = format!(
                    "destination appeared during no-clobber publication: {}",
                    destination.display()
                );
                if let Some(rollback) = rollback_published(&published) {
                    detail.push_str("; ");
                    detail.push_str(&rollback);
                }
                return items
                    .iter()
                    .map(|candidate| match candidate.disposition {
                        PhotoAssetCategorizationDisposition::Duplicate => {
                            completed_item(candidate, contract, CategorizeDisposition::Duplicate)
                        }
                        PhotoAssetCategorizationDisposition::Copy => {
                            failed_item(candidate, contract, &detail)
                        }
                    })
                    .collect();
            }
            Err(error) => {
                let mut detail = format!(
                    "no-clobber publication failed for {}: {}",
                    destination.display(),
                    error.error
                );
                if let Some(rollback) = rollback_published(&published) {
                    detail.push_str("; ");
                    detail.push_str(&rollback);
                }
                return items
                    .iter()
                    .map(|candidate| match candidate.disposition {
                        PhotoAssetCategorizationDisposition::Duplicate => {
                            completed_item(candidate, contract, CategorizeDisposition::Duplicate)
                        }
                        PhotoAssetCategorizationDisposition::Copy => {
                            failed_item(candidate, contract, &detail)
                        }
                    })
                    .collect();
            }
        }
    }

    items
        .iter()
        .map(|item| match item.disposition {
            PhotoAssetCategorizationDisposition::Duplicate => {
                completed_item(item, contract, CategorizeDisposition::Duplicate)
            }
            PhotoAssetCategorizationDisposition::Copy => {
                let disposition = if raced_duplicates.contains(&item.destination) {
                    CategorizeDisposition::Duplicate
                } else {
                    CategorizeDisposition::Copied
                };
                completed_item(item, contract, disposition)
            }
        })
        .collect()
}

impl PortableRuntime {
    pub fn categorize_files(
        &self,
        inputs: &[PathBuf],
        output_root: impl AsRef<Path>,
        dry_run: bool,
    ) -> Result<CategorizeReceipt> {
        let output_root = output_root.as_ref();
        let prepared = prepare_categorization(inputs)?;
        let existing = collect_existing_destinations(output_root)?;
        let plan = plan_photo_assets(
            &prepared.inputs,
            CategorizationDestinationRoot::Explicit(output_root.to_path_buf()),
            &existing,
        )
        .map_err(|error| RuntimeError::external("categorization planning", error))?;

        let mut grouped = BTreeMap::<String, Vec<&PhotoAssetCategorizationItem>>::new();
        for item in &plan.items {
            grouped.entry(item.asset_id.clone()).or_default().push(item);
        }

        let mut receipt = CategorizeReceipt::default();
        for (asset_id, items) in grouped {
            let contract = prepared.contracts.get(&asset_id).ok_or_else(|| {
                RuntimeError::new(
                    "categorization execution",
                    format!("missing classification contract for asset {asset_id}"),
                )
            })?;
            receipt.items.extend(execute_asset(
                &items,
                contract,
                &prepared.source_fingerprints,
                dry_run,
            ));
        }
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "xdremux-runtime-categorize-{label}-{}-{stamp}",
            std::process::id()
        ))
    }

    #[test]
    fn categorization_is_idempotent_and_never_clobbers_conflicts() {
        let root = unique_dir("copy");
        let left = root.join("left");
        let right = root.join("right");
        let output = root.join("out");
        fs::create_dir_all(&left).unwrap();
        fs::create_dir_all(&right).unwrap();
        let first = left.join("portrait.heic");
        let second = right.join("portrait.heic");
        fs::write(&first, b"first Oplus_16 payload").unwrap();
        fs::write(&second, b"second Oplus_16 payload").unwrap();

        let runtime = PortableRuntime::new();
        let receipt = runtime
            .categorize_files(&[first.clone(), second.clone()], &output, false)
            .unwrap();
        assert_eq!(receipt.copied(), 2);
        assert_eq!(receipt.failed(), 0);
        let directory = output.join("静态照片").join("人像");
        assert_eq!(fs::read(directory.join("portrait.heic")).unwrap(), fs::read(&first).unwrap());
        assert_eq!(
            fs::read(directory.join("portrait (2).heic")).unwrap(),
            fs::read(&second).unwrap()
        );

        let rerun = runtime.categorize_files(&[first], &output, false).unwrap();
        assert_eq!(rerun.duplicates(), 1);
        assert_eq!(rerun.copied(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dry_run_plans_without_creating_output() {
        let root = unique_dir("dry-run");
        fs::create_dir_all(&root).unwrap();
        let input = root.join("master.heic");
        let output = root.join("out");
        fs::write(&input, b"capture Oplus_4294967296 payload").unwrap();
        let receipt = PortableRuntime::new()
            .categorize_files(&[input], &output, true)
            .unwrap();
        assert_eq!(receipt.dry_run(), 1);
        assert_eq!(receipt.failed(), 0);
        assert!(!output.exists());
        fs::remove_dir_all(root).unwrap();
    }
}