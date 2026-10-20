#![forbid(unsafe_code)]
//! XDRemux consumer boundary. Portable format code belongs to LibLivePhoto.
mod geometry;
mod payload;
mod publication;

pub use liblivephoto::*;
// Temporary source compatibility for existing XDRemux callers. These vendor
// facts are not part of the stable LibLivePhoto facade.
pub use geometry::{oppo_live_photo_transform, presentation_geometry, write_live_photo_movie};
pub use liblivephoto::compat::*;
pub use payload::{
    copy_payload_range, copy_payload_range_with_options, CopyResult, MotionPhotoCopyError,
    DEFAULT_COPY_BUFFER_SIZE, DEFAULT_MAX_PAYLOAD_BYTES,
};
pub use publication::{
    companion_video_path, publish_live_photo_pair, reconcile_live_photo_pair, PairPublicationError,
    PairPublicationResult,
};

/// Existing product receipt labels, derived from the independent format facts.
pub fn product_source_kind(asset: &MotionAsset) -> &'static str {
    match (asset.dialect, asset.layout, asset.format) {
        (VendorDialect::Oplus, _, _) => "oppoLivePhoto",
        (_, _, AssetFormat::LegacyMicroVideo) => "legacyMicroVideoV1b",
        (_, PhysicalLayout::BmffVideoBox, _) => "androidHeifMotionPhotoV1",
        (VendorDialect::SamsungSef, _, AssetFormat::Unspecified) => "samsungSef",
        _ => "androidMotionPhotoV1",
    }
}
