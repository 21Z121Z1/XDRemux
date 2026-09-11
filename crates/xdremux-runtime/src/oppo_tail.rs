use xdremux_container::{
    complete_oppo_camera_tail, is_oppo_private_hdr_tail_entry, pack_filtered_oppo_camera_tail,
    ExtractedLhdr,
};
use xdremux_engine::OutputIntent;

use crate::{Result, RuntimeError};

/// Build the vendor tail implied by the product output intent.
///
/// Standard ISO output keeps useful vendor resources but removes private HDR
/// payloads that the canonical ISO graph replaces. OPPO Gallery output retains
/// the complete original camera tail so Gallery can continue consuming its
/// vendor resources. Fine-grained legacy tail modes are intentionally not part
/// of the canonical product model.
pub(crate) fn build_tail(
    source: &[u8],
    extracted: &ExtractedLhdr,
    output: OutputIntent,
) -> Result<Vec<u8>> {
    match output {
        OutputIntent::Standard => {
            let mut preserve = |entry: &xdremux_container::ManifestEntry| {
                !is_oppo_private_hdr_tail_entry(&entry.name)
            };
            pack_filtered_oppo_camera_tail(
                source,
                &extracted.manifest_info,
                extracted.data_base,
                &mut preserve,
            )
            .map_err(|error| RuntimeError::external("OPPO camera tail", error))
        }
        OutputIntent::OppoGallery => complete_oppo_camera_tail(source, &extracted.manifest_info)
            .map_err(|error| RuntimeError::external("OPPO camera tail", error)),
    }
}
