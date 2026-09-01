#![forbid(unsafe_code)]

mod android;
mod error;
mod heif;
mod live_photo_mov;
mod lpex;
mod model;
mod motion_video;
mod oppo;
mod payload;
mod publication;
mod scanner;
mod topology;

pub use android::parse_android_motion_photo;
pub use error::{MotionPhotoError, Result};
pub use heif::{is_heif_mime, resolve_heif_motion_photo_ranges};
pub use live_photo_mov::{
    media_mdat_payloads, oppo_live_photo_transform, read_live_photo_content_identifier,
    read_live_photo_still_time, resolve_live_photo_still_time, validate_live_photo_movie,
    write_live_photo_movie, LivePhotoMovieError, LivePhotoMovieResult,
};
pub use lpex::parse_first_lpex_object;
pub use model::{
    ByteRange, MotionPhotoAsset, MotionPhotoItem, MotionPhotoSourceKind, OppoMetadata,
    PresentationSource, VideoStream, VideoStreamLayout, VideoStreamRole,
};
pub use motion_video::{normalize_embedded_video, standalone_bmff_length, NormalizedVideo};
pub use oppo::{enrich_oppo_asset, parse_oppo_fallback, parse_oppo_motion_photo};
pub use payload::{
    copy_payload_range, copy_payload_range_with_options, CopyResult, MotionPhotoCopyError,
    DEFAULT_COPY_BUFFER_SIZE, DEFAULT_MAX_PAYLOAD_BYTES,
};
pub use publication::{
    companion_video_path, publish_live_photo_pair, reconcile_live_photo_pair,
    PairPublicationError, PairPublicationResult,
};
pub use scanner::{ftyp_box_offsets, is_ftyp_box_start};
pub use topology::{enrich_oppo_video_range, resolve_video_stream_layout};
