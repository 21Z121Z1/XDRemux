use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const CLASSIFICATION_LAYOUT_VERSION: &str = "asset-type-v1";
pub const UNCLASSIFIED_FOLDER_NAME: &str = "未分类";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhotoAssetType {
    #[serde(rename = "static-photo")]
    StaticPhoto,
    #[serde(rename = "live-photo")]
    LivePhoto,
}

impl PhotoAssetType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StaticPhoto => "static-photo",
            Self::LivePhoto => "live-photo",
        }
    }

    pub const fn folder_name(self) -> &'static str {
        match self {
            Self::StaticPhoto => "静态照片",
            Self::LivePhoto => "实况照片",
        }
    }

    pub fn tag_id(self) -> String {
        format!("asset.{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhotoResourceRole {
    #[serde(rename = "primary-image")]
    PrimaryImage,
    #[serde(rename = "paired-video")]
    PairedVideo,
    #[serde(rename = "sidecar")]
    Sidecar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoResource {
    pub path: PathBuf,
    pub role: PhotoResourceRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoAsset {
    pub id: String,
    pub asset_type: PhotoAssetType,
    pub resources: Vec<PhotoResource>,
}

impl PhotoAsset {
    pub fn static_photo(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            id: path.to_string_lossy().into_owned(),
            asset_type: PhotoAssetType::StaticPhoto,
            resources: vec![PhotoResource {
                path,
                role: PhotoResourceRole::PrimaryImage,
            }],
        }
    }

    pub fn live_photo(
        image: impl Into<PathBuf>,
        video: impl Into<PathBuf>,
        id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            asset_type: PhotoAssetType::LivePhoto,
            resources: vec![
                PhotoResource {
                    path: image.into(),
                    role: PhotoResourceRole::PrimaryImage,
                },
                PhotoResource {
                    path: video.into(),
                    role: PhotoResourceRole::PairedVideo,
                },
            ],
        }
    }

    pub fn primary_image(&self) -> Option<&std::path::Path> {
        self.resources
            .iter()
            .find(|resource| resource.role == PhotoResourceRole::PrimaryImage)
            .map(|resource| resource.path.as_path())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OppoCaptureMode {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "master")]
    Master,
    #[serde(rename = "ricoh-gr")]
    RicohGr,
    #[serde(rename = "professional")]
    Professional,
    #[serde(rename = "portrait")]
    Portrait,
    #[serde(rename = "night")]
    Night,
    #[serde(rename = "panorama")]
    Panorama,
    #[serde(rename = "time-lapse")]
    TimeLapse,
    #[serde(rename = "ultra-high-resolution")]
    UltraHighResolution,
    #[serde(rename = "id-photo")]
    IdPhoto,
    #[serde(rename = "sticker")]
    Sticker,
    #[serde(rename = "enhanced-text")]
    EnhancedText,
    #[serde(rename = "group-photo")]
    GroupPhoto,
    #[serde(rename = "double-exposure")]
    DoubleExposure,
    #[serde(rename = "beauty")]
    Beauty,
}

impl OppoCaptureMode {
    pub const FOLDER_PROJECTION_PRIORITY: [Self; 14] = [
        Self::Master,
        Self::RicohGr,
        Self::Professional,
        Self::Portrait,
        Self::Night,
        Self::Panorama,
        Self::TimeLapse,
        Self::UltraHighResolution,
        Self::IdPhoto,
        Self::Sticker,
        Self::EnhancedText,
        Self::GroupPhoto,
        Self::DoubleExposure,
        Self::Beauty,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Master => "master",
            Self::RicohGr => "ricoh-gr",
            Self::Professional => "professional",
            Self::Portrait => "portrait",
            Self::Night => "night",
            Self::Panorama => "panorama",
            Self::TimeLapse => "time-lapse",
            Self::UltraHighResolution => "ultra-high-resolution",
            Self::IdPhoto => "id-photo",
            Self::Sticker => "sticker",
            Self::EnhancedText => "enhanced-text",
            Self::GroupPhoto => "group-photo",
            Self::DoubleExposure => "double-exposure",
            Self::Beauty => "beauty",
        }
    }

    pub const fn folder_name(self) -> &'static str {
        match self {
            Self::Normal => "普通拍照",
            Self::Master => "大师模式",
            Self::RicohGr => "RICOH GR",
            Self::Professional => "专业模式",
            Self::Portrait => "人像",
            Self::Night => "夜景",
            Self::Panorama => "全景",
            Self::TimeLapse => "延时摄影",
            Self::UltraHighResolution => "超清",
            Self::IdPhoto => "证件照",
            Self::Sticker => "贴纸",
            Self::EnhancedText => "超级文本",
            Self::GroupPhoto => "合影",
            Self::DoubleExposure => "双重曝光",
            Self::Beauty => "美颜",
        }
    }

    pub const fn bit(self) -> u64 {
        match self {
            Self::Normal => 0,
            Self::Master => 0x1_0000_0000,
            Self::RicohGr => 0x8000_0000,
            Self::Professional => 0x100,
            Self::Portrait => 0x10,
            Self::Night => 0x800,
            Self::Panorama => 0x4,
            Self::TimeLapse => 0x8,
            Self::UltraHighResolution => 0x2000,
            Self::IdPhoto => 0x4000,
            Self::Sticker => 0x200,
            Self::EnhancedText => 0x1000,
            Self::GroupPhoto => 0x40_0000,
            Self::DoubleExposure => 0x8000,
            Self::Beauty => 0x2,
        }
    }

    pub fn tag_id(self) -> String {
        format!("capture.{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhotoCapability {
    #[serde(rename = "proxdr")]
    ProXdr,
    #[serde(rename = "gain-map")]
    GainMap,
    #[serde(rename = "hdr")]
    Hdr,
    #[serde(rename = "depth")]
    Depth,
}

impl PhotoCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProXdr => "proxdr",
            Self::GainMap => "gain-map",
            Self::Hdr => "hdr",
            Self::Depth => "depth",
        }
    }

    pub fn tag_id(self) -> String {
        format!("capability.{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CameraVendor {
    #[serde(rename = "oppo")]
    Oppo,
}

impl CameraVendor {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Oppo => "oppo",
        }
    }

    pub fn tag_id(self) -> String {
        format!("vendor.{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhotoMetadataReadStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "missing-user-comment")]
    MissingUserComment,
    #[serde(rename = "malformed-user-comment")]
    MalformedUserComment,
    #[serde(rename = "unreadable-image")]
    UnreadableImage,
}

impl PhotoMetadataReadStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::MissingUserComment => "missing-user-comment",
            Self::MalformedUserComment => "malformed-user-comment",
            Self::UnreadableImage => "unreadable-image",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OppoPhotoClassificationStatus {
    #[serde(rename = "categorized")]
    Categorized,
    #[serde(rename = "missing-user-comment")]
    MissingUserComment,
    #[serde(rename = "malformed-user-comment")]
    MalformedUserComment,
    #[serde(rename = "unknown-flags")]
    UnknownFlags,
    #[serde(rename = "unreadable-image")]
    UnreadableImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OppoFlagEvidence {
    pub raw_flags: u64,
    pub recognized_flags: u64,
    pub known_unmapped_flags: u64,
    pub unknown_flags: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OppoPhotoClassification {
    pub raw_user_comment: Option<String>,
    pub flag_evidence: Option<OppoFlagEvidence>,
    pub capture_modes: BTreeSet<OppoCaptureMode>,
    pub metadata_status: PhotoMetadataReadStatus,
}

impl OppoPhotoClassification {
    pub fn status(&self) -> OppoPhotoClassificationStatus {
        match self.metadata_status {
            PhotoMetadataReadStatus::MissingUserComment => {
                OppoPhotoClassificationStatus::MissingUserComment
            }
            PhotoMetadataReadStatus::MalformedUserComment => {
                OppoPhotoClassificationStatus::MalformedUserComment
            }
            PhotoMetadataReadStatus::UnreadableImage => {
                OppoPhotoClassificationStatus::UnreadableImage
            }
            PhotoMetadataReadStatus::Ok => {
                if !self.capture_modes.is_empty() || self.unknown_flags() == 0 {
                    OppoPhotoClassificationStatus::Categorized
                } else {
                    OppoPhotoClassificationStatus::UnknownFlags
                }
            }
        }
    }

    pub fn raw_flags(&self) -> Option<u64> {
        self.flag_evidence.as_ref().map(|evidence| evidence.raw_flags)
    }

    pub fn recognized_flags(&self) -> u64 {
        self.flag_evidence
            .as_ref()
            .map_or(0, |evidence| evidence.recognized_flags)
    }

    pub fn known_unmapped_flags(&self) -> u64 {
        self.flag_evidence
            .as_ref()
            .map_or(0, |evidence| evidence.known_unmapped_flags)
    }

    pub fn unknown_flags(&self) -> u64 {
        self.flag_evidence
            .as_ref()
            .map_or(0, |evidence| evidence.unknown_flags)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoClassification {
    pub asset_type: PhotoAssetType,
    pub capture_modes: BTreeSet<OppoCaptureMode>,
    pub capabilities: BTreeSet<PhotoCapability>,
    pub vendor: Option<CameraVendor>,
    pub evidence: OppoPhotoClassification,
}

impl PhotoClassification {
    pub fn primary_capture_mode(&self) -> Option<OppoCaptureMode> {
        for candidate in OppoCaptureMode::FOLDER_PROJECTION_PRIORITY {
            if self.capture_modes.contains(&candidate) {
                return Some(candidate);
            }
        }
        if self.evidence.metadata_status == PhotoMetadataReadStatus::Ok
            && self.evidence.raw_flags().is_some()
            && self.evidence.unknown_flags() == 0
        {
            return Some(OppoCaptureMode::Normal);
        }
        None
    }

    pub fn folder_name(&self) -> &'static str {
        self.primary_capture_mode()
            .map_or(UNCLASSIFIED_FOLDER_NAME, OppoCaptureMode::folder_name)
    }

    pub fn relative_directory_components(&self) -> [&'static str; 2] {
        [self.asset_type.folder_name(), self.folder_name()]
    }

    pub fn tags(&self) -> Vec<String> {
        let mut tags = BTreeSet::new();
        tags.insert(self.asset_type.tag_id());
        tags.extend(self.capture_modes.iter().copied().map(OppoCaptureMode::tag_id));
        tags.extend(self.capabilities.iter().copied().map(PhotoCapability::tag_id));
        if let Some(vendor) = self.vendor {
            tags.insert(vendor.tag_id());
        }
        tags.into_iter().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoClassificationContract {
    pub asset_type: String,
    pub capture_modes: Vec<String>,
    pub primary_capture_mode: Option<String>,
    pub folder: String,
    pub metadata_status: String,
    pub recognized_flags: u64,
    pub known_unmapped_flags: u64,
    pub unknown_flags: u64,
    pub tags: Vec<String>,
}
