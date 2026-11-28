use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{Result, RuntimeError};

pub const DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME: &str = ".xdremux-motion-photo-checkpoint.jsonl";
const SOURCE_HASH_BUFFER_BYTES: usize = 64 * 1024;

/// The journal is a writable control file. It must be disjoint from every
/// source, planned output and companion, even when two names alias one inode.
pub(crate) fn validate_checkpoint_path<'a>(
    path: &Path,
    protected_paths: impl IntoIterator<Item = &'a Path>,
) -> Result<()> {
    crate::live_photo_package::validate_output_parent(path)?;
    let path_key = canonical_or_absolute(path)?;
    let handle = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            Some(same_file::Handle::from_path(path).map_err(|error| {
                RuntimeError::external("Motion Photo checkpoint identity", error)
            })?)
        }
        Ok(_) => {
            return Err(RuntimeError::new(
                "Motion Photo checkpoint path",
                "checkpoint must be a regular file, not a directory or symbolic link",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(RuntimeError::external(
                "Motion Photo checkpoint path",
                error,
            ))
        }
    };

    for protected in protected_paths {
        let same_path = canonical_or_absolute(protected)? == path_key;
        let same_file = handle.as_ref().is_some_and(|handle| {
            protected.is_file()
                && same_file::Handle::from_path(protected).is_ok_and(|other| *handle == other)
        });
        if same_path || same_file {
            return Err(RuntimeError::new(
                "Motion Photo checkpoint path",
                format!(
                    "checkpoint aliases a source or output: {}",
                    protected.display()
                ),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSignature {
    pub size: u64,
    pub mtime_ns: Option<u64>,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CheckpointStatus {
    Success,
    SkippedExisting,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct CheckpointItem {
    #[serde(rename = "inputPath", alias = "input_path")]
    input_path: String,
    #[serde(rename = "outputImagePath", alias = "output_image_path")]
    output_image_path: String,
    #[serde(
        rename = "outputVideoPath",
        alias = "output_video_path",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    output_video_path: Option<String>,
    #[serde(
        rename = "outputPackagePath",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    output_package_path: Option<String>,
    #[serde(
        rename = "outputTarget",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    output_target: Option<String>,
    #[serde(
        rename = "videoCompatibility",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    video_compatibility: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    #[serde(
        rename = "outputSHA256",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    output_sha256: Option<String>,
    status: CheckpointStatus,
    #[serde(rename = "inputSize", alias = "input_size", default)]
    input_size: Option<u64>,
    #[serde(rename = "inputSHA256", alias = "input_sha256", default)]
    input_sha256: Option<String>,
    #[serde(rename = "assetIdentifier", alias = "asset_identifier", default)]
    asset_identifier: Option<String>,
}

impl CheckpointItem {
    fn route_key(&self) -> String {
        match &self.output_target {
            Some(target) => {
                let compatibility = self.video_compatibility.as_deref().unwrap_or("preserve");
                match &self.destination {
                    Some(destination) => format!("{target}:{compatibility}:{destination}"),
                    None => format!("{target}:{compatibility}"),
                }
            }
            None if self.output_package_path.is_some() => "apple-pvt".to_owned(),
            None => "apple-pair".to_owned(),
        }
    }

    fn reusable(&self, signature: &SourceSignature) -> bool {
        matches!(
            self.status,
            CheckpointStatus::Success | CheckpointStatus::SkippedExisting
        ) && self.input_size == Some(signature.size)
            && self.input_sha256.as_deref() == Some(signature.sha256.as_str())
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MotionPhotoCheckpoint {
    items: BTreeMap<(String, String), CheckpointItem>,
}

impl MotionPhotoCheckpoint {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => {
                return Err(RuntimeError::external(
                    "Motion Photo checkpoint open",
                    error,
                ));
            }
        };
        let mut items = BTreeMap::new();
        for line in BufReader::new(file).lines() {
            let line = match line {
                Ok(line) => line,
                Err(_) => continue,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value = match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if value.get("kind").and_then(serde_json::Value::as_str) != Some("item") {
                continue;
            }
            let item = match serde_json::from_value::<CheckpointItem>(value) {
                Ok(item) => item,
                Err(_) => continue,
            };
            if item.input_path.is_empty()
                || item.output_image_path.is_empty()
                || item.input_sha256.as_deref().is_none_or(str::is_empty)
            {
                continue;
            }
            let valid = if item.output_target.is_some() {
                matches!(
                    item.output_target.as_deref(),
                    Some("google-jpeg" | "google-heic")
                ) && item
                    .output_sha256
                    .as_deref()
                    .is_some_and(|sha| sha.len() == 64)
                    && item.output_video_path.is_none()
                    && item.output_package_path.is_none()
            } else {
                item.output_video_path
                    .as_deref()
                    .is_some_and(|v| !v.is_empty())
                    && item
                        .asset_identifier
                        .as_deref()
                        .is_some_and(|id| !id.is_empty())
            };
            // Failed records do not carry output identities, but must still
            // supersede older successful entries for the same source/route.
            if !valid && item.status != CheckpointStatus::Failure {
                continue;
            }
            let key = (item.input_path.clone(), item.route_key());
            items.insert(key, item);
        }
        Ok(Self { items })
    }

    pub(crate) fn reusable_item(
        &self,
        source: &Path,
        signature: &SourceSignature,
        pvt: bool,
    ) -> Result<Option<ReusableMotionPhoto>> {
        let source = canonical_existing(source)?;
        let key = (
            path_string(&source),
            if pvt { "apple-pvt" } else { "apple-pair" }.to_owned(),
        );
        let Some(item) = self.items.get(&key) else {
            return Ok(None);
        };
        if !item.reusable(signature) {
            return Ok(None);
        }
        let Some(video) = item.output_video_path.as_ref() else {
            return Ok(None);
        };
        let Some(asset_identifier) = item.asset_identifier.clone() else {
            return Ok(None);
        };
        Ok(Some(ReusableMotionPhoto {
            image: PathBuf::from(&item.output_image_path),
            video: PathBuf::from(video),
            package: item.output_package_path.as_ref().map(PathBuf::from),
            asset_identifier,
        }))
    }

    pub(crate) fn reusable_android_item(
        &self,
        source: &Path,
        signature: &SourceSignature,
        target: &str,
        compatibility: &str,
        destination: Option<&str>,
    ) -> Result<Option<ReusableAndroidMotionPhoto>> {
        let source = canonical_existing(source)?;
        let key = (
            path_string(&source),
            match destination {
                Some(destination) => format!("{target}:{compatibility}:{destination}"),
                None => format!("{target}:{compatibility}"),
            },
        );
        let Some(item) = self.items.get(&key) else {
            return Ok(None);
        };
        if !item.reusable(signature) {
            return Ok(None);
        }
        let Some(sha256) = item.output_sha256.clone() else {
            return Ok(None);
        };
        Ok(Some(ReusableAndroidMotionPhoto {
            output: PathBuf::from(&item.output_image_path),
            sha256,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReusableAndroidMotionPhoto {
    pub output: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReusableMotionPhoto {
    pub image: PathBuf,
    pub video: PathBuf,
    pub package: Option<PathBuf>,
    pub asset_identifier: String,
}

pub fn motion_photo_checkpoint_path(
    output_dir: Option<&Path>,
    requested: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(requested) = requested {
        let mut name = requested.as_os_str().to_os_string();
        name.push(".motion-photo");
        return Some(PathBuf::from(name));
    }
    output_dir.map(|directory| directory.join(DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME))
}

fn mtime_ns(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_nanos()).ok())
}

fn digest_hex(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn source_signature(path: &Path, bytes: &[u8]) -> Result<SourceSignature> {
    let metadata = fs::metadata(path)
        .map_err(|error| RuntimeError::external("batch source metadata", error))?;
    let size = metadata.len();
    if size != u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
        return Err(RuntimeError::new(
            "batch source provenance",
            "source size changed while it was being read",
        ));
    }
    Ok(SourceSignature {
        size,
        mtime_ns: mtime_ns(&metadata),
        sha256: digest_hex(Sha256::digest(bytes)),
    })
}

/// Compute durable source provenance without allocating a buffer proportional
/// to the media file. Planning only needs size, mtime and SHA-256, so hashing
/// directly from the file keeps memory bounded even for Motion Photos with a
/// large video tail.
pub(crate) fn source_signature_path(path: &Path) -> Result<SourceSignature> {
    let mut file = File::open(path)
        .map_err(|error| RuntimeError::external("batch source provenance open", error))?;
    let before = file
        .metadata()
        .map_err(|error| RuntimeError::external("batch source provenance metadata", error))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; SOURCE_HASH_BUFFER_BYTES];
    let mut observed_size = 0_u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| RuntimeError::external("batch source provenance read", error))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        observed_size = observed_size
            .checked_add(u64::try_from(count).map_err(|_| {
                RuntimeError::new("batch source provenance", "read size exceeds u64")
            })?)
            .ok_or_else(|| {
                RuntimeError::new("batch source provenance", "source size overflows u64")
            })?;
    }
    let after = file
        .metadata()
        .map_err(|error| RuntimeError::external("batch source provenance metadata", error))?;
    if before.len() != observed_size
        || after.len() != observed_size
        || mtime_ns(&before) != mtime_ns(&after)
    {
        return Err(RuntimeError::new(
            "batch source provenance",
            "source changed while its signature was being streamed",
        ));
    }
    Ok(SourceSignature {
        size: observed_size,
        mtime_ns: mtime_ns(&after),
        sha256: digest_hex(hasher.finalize()),
    })
}

fn canonical_existing(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|error| RuntimeError::external("batch provenance path", error))
}

pub(crate) fn canonical_or_absolute(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)
        .map_err(|error| RuntimeError::external("batch provenance absolute path", error))?;
    for ancestor in absolute.ancestors() {
        match fs::canonicalize(ancestor) {
            Ok(parent) => {
                let relative = absolute
                    .strip_prefix(ancestor)
                    .expect("ancestor of this path");
                return Ok(if relative.as_os_str().is_empty() {
                    parent
                } else {
                    parent.join(relative)
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(RuntimeError::external("batch provenance path", error)),
        }
    }
    Ok(absolute)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streamed_signature_matches_in_memory_signature_for_large_input() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.bin");
        let payload = (0..SOURCE_HASH_BUFFER_BYTES * 3 + 137)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        fs::write(&source, &payload).unwrap();

        let in_memory = source_signature(&source, &payload).unwrap();
        let streamed = source_signature_path(&source).unwrap();
        assert_eq!(streamed, in_memory);
        assert_eq!(streamed.size, u64::try_from(payload.len()).unwrap());
    }

    fn write_legacy_pair(
        checkpoint: &Path,
        source: &Path,
        image: &Path,
        video: &Path,
        sig: &SourceSignature,
    ) {
        let header = serde_json::json!({"kind":"header", "schemaVersion":3});
        let item = serde_json::json!({
            "kind":"item", "inputPath":canonical_existing(source).unwrap(),
            "outputImagePath":canonical_existing(image).unwrap(),
            "outputVideoPath":canonical_existing(video).unwrap(), "status":"success",
            "inputSize":sig.size, "inputSHA256":sig.sha256, "assetIdentifier":"ASSET-ID",
        });
        fs::write(checkpoint, format!("{header}\n{item}\n")).unwrap();
    }

    #[test]
    fn truncated_tail_record_never_grants_reuse() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.jpg");
        let image = root.path().join("source.heic");
        let video = root.path().join("source.mov");
        fs::write(&source, b"source").unwrap();
        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        let bytes = fs::read(&source).unwrap();
        let signature = source_signature(&source, &bytes).unwrap();
        let checkpoint = root.path().join("state.jsonl");
        write_legacy_pair(&checkpoint, &source, &image, &video, &signature);
        let mut torn = fs::read(&checkpoint).unwrap();
        torn.extend_from_slice(br#"{"kind":"item","inputPath":"broken""#);
        fs::write(&checkpoint, torn).unwrap();

        let loaded = MotionPhotoCheckpoint::load(&checkpoint).unwrap();
        let prior = loaded
            .reusable_item(&source, &signature, false)
            .unwrap()
            .unwrap();
        assert_eq!(prior.asset_identifier, "ASSET-ID");
        assert_eq!(prior.image, fs::canonicalize(image).unwrap());
    }

    #[test]
    fn changed_source_digest_invalidates_reuse_even_when_mtime_is_irrelevant() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.jpg");
        let image = root.path().join("source.heic");
        let video = root.path().join("source.mov");
        fs::write(&source, b"source-a").unwrap();
        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        let original_bytes = fs::read(&source).unwrap();
        let original = source_signature(&source, &original_bytes).unwrap();
        let checkpoint = root.path().join("state.jsonl");
        write_legacy_pair(&checkpoint, &source, &image, &video, &original);

        fs::write(&source, b"source-b").unwrap();
        let changed_bytes = fs::read(&source).unwrap();
        let changed = source_signature(&source, &changed_bytes).unwrap();
        let loaded = MotionPhotoCheckpoint::load(&checkpoint).unwrap();
        assert!(loaded
            .reusable_item(&source, &changed, false)
            .unwrap()
            .is_none());
    }

    #[test]
    fn version_two_apple_records_remain_reusable() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.jpg");
        let image = root.path().join("legacy.heic");
        let video = root.path().join("legacy.mov");
        fs::write(&source, b"source").unwrap();
        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        let signature = source_signature_path(&source).unwrap();
        let checkpoint = root.path().join("legacy.jsonl");
        let header = serde_json::json!({"kind":"header", "schemaVersion":2});
        let item = serde_json::json!({
            "kind":"item",
            "inputPath":fs::canonicalize(&source).unwrap().display().to_string(),
            "outputImagePath":fs::canonicalize(&image).unwrap().display().to_string(),
            "outputVideoPath":fs::canonicalize(&video).unwrap().display().to_string(),
            "status":"success",
            "inputSize":signature.size,
            "inputSHA256":signature.sha256,
            "assetIdentifier":"LEGACY-ID",
        });
        fs::write(&checkpoint, format!("{header}\n{item}\n")).unwrap();
        let loaded = MotionPhotoCheckpoint::load(&checkpoint).unwrap();
        assert_eq!(
            loaded
                .reusable_item(&source, &signature, false)
                .unwrap()
                .unwrap()
                .asset_identifier,
            "LEGACY-ID",
        );
        assert!(loaded
            .reusable_item(&source, &signature, true)
            .unwrap()
            .is_none());
    }

    #[test]
    fn unknown_checkpoint_status_is_rejected_during_load() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.jpg");
        let image = root.path().join("source.heic");
        let video = root.path().join("source.mov");
        fs::write(&source, b"source").unwrap();
        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        let signature = source_signature_path(&source).unwrap();
        let checkpoint = root.path().join("unknown-status.jsonl");
        let header = serde_json::json!({"kind":"header", "schemaVersion":3});
        let item = serde_json::json!({
            "kind":"item",
            "inputPath": canonical_existing(&source).unwrap(),
            "outputImagePath": canonical_existing(&image).unwrap(),
            "outputVideoPath": canonical_existing(&video).unwrap(),
            "status": "unknown_state",
            "inputSize": signature.size,
            "inputSHA256": signature.sha256,
            "assetIdentifier": "ASSET-ID",
        });
        fs::write(&checkpoint, format!("{header}\n{item}\n")).unwrap();

        let loaded = MotionPhotoCheckpoint::load(&checkpoint).unwrap();
        assert!(loaded.items.is_empty());
    }
}
