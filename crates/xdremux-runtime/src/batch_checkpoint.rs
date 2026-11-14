use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Result, RuntimeError};

pub const MOTION_PHOTO_CHECKPOINT_SCHEMA_VERSION: u32 = 3;
pub const DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME: &str = ".xdremux-motion-photo-checkpoint.jsonl";
const SOURCE_HASH_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSignature {
    pub size: u64,
    pub mtime_ns: Option<u64>,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CheckpointItem {
    kind: String,
    #[serde(rename = "inputPath", alias = "input_path")]
    input_path: String,
    #[serde(rename = "sourceRelativePath", alias = "relative_source_path", default)]
    source_relative_path: Option<String>,
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
    status: String,
    #[serde(rename = "inputSize", alias = "input_size", default)]
    input_size: Option<u64>,
    #[serde(rename = "inputMtimeNs", alias = "input_mtime_ns", default)]
    input_mtime_ns: Option<u64>,
    #[serde(rename = "inputSHA256", alias = "input_sha256", default)]
    input_sha256: Option<String>,
    #[serde(rename = "assetIdentifier", alias = "asset_identifier", default)]
    asset_identifier: Option<String>,
    #[serde(default)]
    error: Option<String>,
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
        matches!(self.status.as_str(), "success" | "skipped_existing")
            && self.input_size == Some(signature.size)
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
            if !valid && item.status != "failure" {
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

pub(crate) enum CheckpointOutcome<'a> {
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

pub(crate) struct AndroidCheckpointIdentity<'a> {
    pub target: &'a str,
    pub compatibility: &'a str,
    pub destination: Option<&'a str>,
    pub output_sha256: Option<&'a str>,
}

impl MotionPhotoCheckpointWriter {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        crate::live_photo_package::validate_output_parent(path)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| {
                RuntimeError::external("Motion Photo checkpoint directory", error)
            })?;
        }
        let new_file = !path.exists()
            || fs::metadata(path)
                .map(|value| value.len() == 0)
                .unwrap_or(true);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| RuntimeError::external("Motion Photo checkpoint open", error))?;
        let mut writer = Self { file };
        if new_file {
            writer.append_json(&serde_json::json!({
                "kind": "header",
                "schemaVersion": MOTION_PHOTO_CHECKPOINT_SCHEMA_VERSION,
            }))?;
        }
        Ok(writer)
    }

    fn append_json(&mut self, value: &serde_json::Value) -> Result<()> {
        serde_json::to_writer(&mut self.file, value)
            .map_err(|error| RuntimeError::external("Motion Photo checkpoint encode", error))?;
        self.file
            .write_all(b"\n")
            .map_err(|error| RuntimeError::external("Motion Photo checkpoint append", error))?;
        self.file
            .flush()
            .map_err(|error| RuntimeError::external("Motion Photo checkpoint flush", error))?;
        self.file
            .sync_all()
            .map_err(|error| RuntimeError::external("Motion Photo checkpoint sync", error))
    }

    pub(crate) fn append_item(
        &mut self,
        source: &Path,
        image: &Path,
        video: &Path,
        signature: &SourceSignature,
        package: Option<&Path>,
        outcome: CheckpointOutcome<'_>,
    ) -> Result<()> {
        let input = canonical_existing(source)?;
        let output_image = canonical_or_absolute(image)?;
        let output_video = canonical_or_absolute(video)?;
        let item = CheckpointItem {
            kind: "item".to_owned(),
            input_path: path_string(&input),
            source_relative_path: None,
            output_image_path: path_string(&output_image),
            output_video_path: Some(path_string(&output_video)),
            output_package_path: package
                .map(canonical_or_absolute)
                .transpose()?
                .as_deref()
                .map(path_string),
            output_target: None,
            video_compatibility: None,
            destination: None,
            output_sha256: None,
            status: outcome.status().to_owned(),
            input_size: Some(signature.size),
            input_mtime_ns: signature.mtime_ns,
            input_sha256: Some(signature.sha256.clone()),
            asset_identifier: outcome.asset_identifier().map(ToOwned::to_owned),
            error: outcome.error().map(ToOwned::to_owned),
        };
        let value = serde_json::to_value(item)
            .map_err(|error| RuntimeError::external("Motion Photo checkpoint encode", error))?;
        self.append_json(&value)
    }

    pub(crate) fn append_android_item(
        &mut self,
        source: &Path,
        output: &Path,
        signature: &SourceSignature,
        identity: AndroidCheckpointIdentity<'_>,
        outcome: CheckpointOutcome<'_>,
    ) -> Result<()> {
        let item = CheckpointItem {
            kind: "item".to_owned(),
            input_path: path_string(&canonical_existing(source)?),
            source_relative_path: None,
            output_image_path: path_string(&canonical_or_absolute(output)?),
            output_video_path: None,
            output_package_path: None,
            output_target: Some(identity.target.to_owned()),
            video_compatibility: Some(identity.compatibility.to_owned()),
            destination: identity.destination.map(ToOwned::to_owned),
            output_sha256: identity.output_sha256.map(ToOwned::to_owned),
            status: outcome.status().to_owned(),
            input_size: Some(signature.size),
            input_mtime_ns: signature.mtime_ns,
            input_sha256: Some(signature.sha256.clone()),
            asset_identifier: None,
            error: outcome.error().map(ToOwned::to_owned),
        };
        self.append_json(
            &serde_json::to_value(item)
                .map_err(|error| RuntimeError::external("Motion Photo checkpoint encode", error))?,
        )
    }
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
    if path.exists() {
        return canonical_existing(path);
    }
    std::path::absolute(path)
        .map_err(|error| RuntimeError::external("batch provenance absolute path", error))
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
        let mut writer = MotionPhotoCheckpointWriter::open(&checkpoint).unwrap();
        writer
            .append_item(
                &source,
                &image,
                &video,
                &signature,
                None,
                CheckpointOutcome::Success("ASSET-ID"),
            )
            .unwrap();
        drop(writer);
        let mut file = OpenOptions::new().append(true).open(&checkpoint).unwrap();
        file.write_all(br#"{"kind":"item","inputPath":"broken""#)
            .unwrap();
        file.sync_all().unwrap();

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
        let mut writer = MotionPhotoCheckpointWriter::open(&checkpoint).unwrap();
        writer
            .append_item(
                &source,
                &image,
                &video,
                &original,
                None,
                CheckpointOutcome::Success("ASSET-ID"),
            )
            .unwrap();
        drop(writer);

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
}
