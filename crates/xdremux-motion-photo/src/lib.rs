#![forbid(unsafe_code)]

mod android;
mod error;
mod heif;
mod lpex;
mod model;
mod scanner;
mod topology;

pub use android::parse_android_motion_photo;
pub use error::{MotionPhotoError, Result};
pub use heif::{is_heif_mime, resolve_heif_motion_photo_ranges};
pub use lpex::parse_first_lpex_object;
pub use model::{
    ByteRange, MotionPhotoAsset, MotionPhotoItem, MotionPhotoSourceKind, OppoMetadata,
    PresentationSource, VideoStream, VideoStreamLayout, VideoStreamRole,
};
pub use scanner::{ftyp_box_offsets, is_ftyp_box_start};
pub use topology::{enrich_oppo_video_range, resolve_video_stream_layout};
