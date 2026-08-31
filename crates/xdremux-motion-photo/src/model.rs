use std::collections::BTreeMap;

use crate::error::{MotionPhotoError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub lower_bound: u64,
    pub upper_bound: u64,
}

impl ByteRange {
    pub fn new(lower_bound: u64, upper_bound: u64) -> Result<Self> {
        if upper_bound < lower_bound {
            return Err(MotionPhotoError::InvalidByteRange);
        }
        Ok(Self {
            lower_bound,
            upper_bound,
        })
    }

    pub fn length(self) -> u64 {
        self.upper_bound - self.lower_bound
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionPhotoItem {
    pub mime: String,
    pub semantic: String,
    pub length: u64,
    pub padding: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OppoMetadata {
    pub cover_frame_pts_us: Option<i64>,
    pub version: i64,
    pub matrix_count: i64,
    pub photo_crop_matrix: Option<[f64; 9]>,
    pub photo_eis_matrix: Option<[f64; 9]>,
    pub matrices: BTreeMap<String, [f64; 9]>,
    pub video_width: Option<i64>,
    pub video_height: Option<i64>,
    pub origin_photo_width: Option<i64>,
    pub origin_photo_height: Option<i64>,
    pub photo_eis_crop_factor: Option<Vec<f64>>,
    pub eis_crop_factor: Option<Vec<f64>>,
    pub photo_crop_factor: Option<f64>,
    pub stream_count: usize,
}

impl Default for OppoMetadata {
    fn default() -> Self {
        Self {
            cover_frame_pts_us: None,
            version: 0,
            matrix_count: 0,
            photo_crop_matrix: None,
            photo_eis_matrix: None,
            matrices: BTreeMap::new(),
            video_width: None,
            video_height: None,
            origin_photo_width: None,
            origin_photo_height: None,
            photo_eis_crop_factor: None,
            eis_crop_factor: None,
            photo_crop_factor: None,
            stream_count: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoStreamRole {
    Primary,
    AuxiliaryGeometry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoStream {
    pub index: usize,
    pub role: VideoStreamRole,
    pub range: ByteRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStreamLayout {
    pub primary: VideoStream,
    pub auxiliary_geometry: Vec<VideoStream>,
}
