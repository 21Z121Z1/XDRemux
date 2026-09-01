#![forbid(unsafe_code)]

mod error;
mod extractor;
mod oppo_tail;

pub use error::{ContainerError, Result};
pub use extractor::{
    extract, portrait_blocks, ExtractedLhdr, ExtractionMode, LocalHdrInfo, ManifestEntry,
    ManifestInfo,
};
pub use oppo_tail::{
    complete_oppo_camera_tail, is_oppo_compact_tail_entry, is_oppo_portrait_editing_tail_entry,
    is_oppo_private_hdr_tail_entry, is_oppo_private_uhdr_tail_entry, is_oppo_watermark_tail_entry,
    neutralize_oppo_camera_tail_entries, pack_filtered_oppo_camera_tail,
    OPPO_CAMERA_PORTRAIT_EDITING_ENTRY_NAMES, OPPO_CAMERA_WATERMARK_AUXILIARY_ENTRY_NAMES,
    OPPO_PRIVATE_UHDR_TAIL_ENTRY_NAMES,
};
