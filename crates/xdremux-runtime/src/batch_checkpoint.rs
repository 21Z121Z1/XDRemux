use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Result, RuntimeError};

pub const MOTION_PHOTO_CHECKPOINT_SCHEMA_VERSION: u32 = 2;
pub const DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME: &str = ".xdremux-motion-photo-checkpoint.jsonl";

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
    #[serde(rename = "outputVideoPath", alias = "output_video_path")]
    output_video_path: String,
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
    fn reusable(&self, signature: &SourceSignature) -> bool {
        matches!(self.status.as_str(), "success" | "skipped_existing")
            && self
                .asset_identifier
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && self.input_size == Some(signature.size)
            && self.input_sha256.as_deref() == Some(signature.sha256.as_str())
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MotionPhotoCheckpoint {
    items: BTreeMap<String, CheckpointItem>,
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
                || item.output_video_path.is_empty()
                || item.input_sha256.as_deref().is_none_or(str::is_empty)
                || item.asset_identifier.as_deref().is_none_or(str::is_empty)
            {
                continue;
            }
            items.insert(item.input_path.clone(), item);
        }
        Ok(Self { items })
    }

    pub(crate) fn reusable_item(
        &self,
        source: &Path,
        signature: &SourceSignature,
    ) -> Result<Option<ReusableMotionPhoto>> {
        let source = canonical_existing(source)?;
        let key = path_string(&source);
        let Some(item) = self.items.get(&key) else {
            return Ok(None);
        };
        if !item.reusable(signature) {
            return Ok(None);
        }
        let Some(asset_identifier) = item.asset_identifier.clone() else {
            return Ok(None);
        };
        Ok(Some(ReusableMotionPhoto {
            image: PathBuf::from(&item.output_image_path),
            video: PathBuf::from(&item.output_video_path),
            asset_identifier,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReusableMotionPhoto {
    pub image: PathBuf,
    pub video: PathBuf,
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

impl MotionPhotoCheckpointWriter {
    pub(crate) fn open(path: &Path) -> Result<Self> {
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
            output_video_path: path_string(&output_video),
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
    let mtime_ns = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_nanos()).ok());
    let sha256 = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(SourceSignature {
        size,
        mtime_ns,
        sha256,
    })
}

fn canonical_existing(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|error| RuntimeError::external("batch provenance path", error))
}

fn canonical_or_absolute(path: &Path) -> Result<PathBuf> {
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "xdremux-batch-checkpoint-{}-{stamp}",
            std::process::id()
        ))
    }

    #[test]
    fn truncated_tail_record_never_grants_reuse() {
        let root = unique_dir();
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.jpg");
        let image = root.join("source.heic");
        let video = root.join("source.mov");
        fs::write(&source, b"source").unwrap();
        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        let bytes = fs::read(&source).unwrap();
        let signature = source_signature(&source, &bytes).unwrap();
        let checkpoint = root.join("state.jsonl");
        let mut writer = MotionPhotoCheckpointWriter::open(&checkpoint).unwrap();
        writer
            .append_item(
                &source,
                &image,
                &video,
                &signature,
                CheckpointOutcome::Success("ASSET-ID"),
            )
            .unwrap();
        drop(writer);
        let mut file = OpenOptions::new().append(true).open(&checkpoint).unwrap();
        file.write_all(br#"{"kind":"item","inputPath":"broken""#)
            .unwrap();
        file.sync_all().unwrap();

        let loaded = MotionPhotoCheckpoint::load(&checkpoint).unwrap();
        let prior = loaded.reusable_item(&source, &signature).unwrap().unwrap();
        assert_eq!(prior.asset_identifier, "ASSET-ID");
        assert_eq!(prior.image, fs::canonicalize(image).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn changed_source_digest_invalidates_reuse_even_when_mtime_is_irrelevant() {
        let root = unique_dir();
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.jpg");
        let image = root.join("source.heic");
        let video = root.join("source.mov");
        fs::write(&source, b"source-a").unwrap();
        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        let original_bytes = fs::read(&source).unwrap();
        let original = source_signature(&source, &original_bytes).unwrap();
        let checkpoint = root.join("state.jsonl");
        let mut writer = MotionPhotoCheckpointWriter::open(&checkpoint).unwrap();
        writer
            .append_item(
                &source,
                &image,
                &video,
                &original,
                CheckpointOutcome::Success("ASSET-ID"),
            )
            .unwrap();
        drop(writer);

        fs::write(&source, b"source-b").unwrap();
        let changed_bytes = fs::read(&source).unwrap();
        let changed = source_signature(&source, &changed_bytes).unwrap();
        let loaded = MotionPhotoCheckpoint::load(&checkpoint).unwrap();
        assert!(loaded.reusable_item(&source, &changed).unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
