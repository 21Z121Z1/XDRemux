#![forbid(unsafe_code)]

mod error;
pub mod iso21496;
pub mod oppo;
mod oppo_heif;
mod product_tmap;
pub mod ultrahdr_jpeg;

pub use error::{MetadataError, Result};
pub use iso21496::{make_hdrgm_xmp, make_imageio_native_tmap_payload, make_strict_tmap_payload};
pub use oppo::{
    adjusted_extent_for_oppo_user_comment_patch, adjusted_oppo_user_comment,
    adjusted_oppo_user_comment_in_heif, apply_oppo_user_comment_patch, find_oppo_tag_flag,
    target_oppo_tag_flags, OppoCompatibility, OppoTagFlag, OppoUserCommentPatch,
    ISO_ULTRA_HDR_FLAG, LOCAL_HDR_FLAG, OPPO_ULTRA_HDR_FLAG,
};
pub use oppo_heif::{oppo_tag_flags_in_heif, patch_oppo_user_comment_in_heif};
pub use product_tmap::make_apple_tmap_payload;
pub use ultrahdr_jpeg::{parse_ultrahdr_gain_map_metadata, UltraHdrGainMapMetadata};
