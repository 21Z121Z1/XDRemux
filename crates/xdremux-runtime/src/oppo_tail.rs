use xdremux_container::{
    complete_oppo_camera_tail, is_oppo_compact_tail_entry, is_oppo_portrait_editing_tail_entry,
    is_oppo_private_hdr_tail_entry, is_oppo_private_uhdr_tail_entry, is_oppo_watermark_tail_entry,
    neutralize_oppo_camera_tail_entries, pack_filtered_oppo_camera_tail, ExtractedLhdr,
};
use xdremux_engine::OppoCameraTail;

use crate::{Result, RuntimeError};

pub(crate) fn build_tail(
    source: &[u8],
    extracted: &ExtractedLhdr,
    mode: OppoCameraTail,
) -> Result<Vec<u8>> {
    let filtered = |preserve: &mut dyn FnMut(&xdremux_container::ManifestEntry) -> bool| {
        pack_filtered_oppo_camera_tail(
            source,
            &extracted.manifest_info,
            extracted.data_base,
            preserve,
        )
        .map_err(|error| RuntimeError::external("OPPO camera tail", error))
    };

    match mode {
        OppoCameraTail::Off => Ok(Vec::new()),
        OppoCameraTail::Watermark => {
            let mut preserve = |entry: &xdremux_container::ManifestEntry| {
                is_oppo_watermark_tail_entry(&entry.name)
            };
            filtered(&mut preserve)
        }
        OppoCameraTail::Compact => {
            let mut preserve = |entry: &xdremux_container::ManifestEntry| {
                is_oppo_compact_tail_entry(&entry.name)
            };
            filtered(&mut preserve)
        }
        OppoCameraTail::Preserve => complete_oppo_camera_tail(source, &extracted.manifest_info)
            .map_err(|error| RuntimeError::external("OPPO camera tail", error)),
        OppoCameraTail::PreserveWithoutPortrait => {
            let mut preserve = |entry: &xdremux_container::ManifestEntry| {
                !is_oppo_portrait_editing_tail_entry(&entry.name)
            };
            filtered(&mut preserve)
        }
        OppoCameraTail::PreserveWithoutPortraitOrPrivateHdr => {
            let mut preserve = |entry: &xdremux_container::ManifestEntry| {
                !is_oppo_portrait_editing_tail_entry(&entry.name)
                    && !is_oppo_private_hdr_tail_entry(&entry.name)
            };
            filtered(&mut preserve)
        }
        OppoCameraTail::PreserveWithoutPrivateUhdr => {
            let mut preserve = |entry: &xdremux_container::ManifestEntry| {
                !is_oppo_private_uhdr_tail_entry(&entry.name)
            };
            filtered(&mut preserve)
        }
        OppoCameraTail::PreserveWithoutPrivateHdr => {
            let mut preserve = |entry: &xdremux_container::ManifestEntry| {
                !is_oppo_private_hdr_tail_entry(&entry.name)
            };
            filtered(&mut preserve)
        }
        OppoCameraTail::PreserveNoUhdr => neutralize_oppo_camera_tail_entries(
            source,
            &extracted.manifest_info,
            |entry| is_oppo_private_uhdr_tail_entry(&entry.name),
        )
        .map_err(|error| RuntimeError::external("OPPO camera tail", error)),
        OppoCameraTail::PreserveNoHdr => neutralize_oppo_camera_tail_entries(
            source,
            &extracted.manifest_info,
            |entry| is_oppo_private_hdr_tail_entry(&entry.name),
        )
        .map_err(|error| RuntimeError::external("OPPO camera tail", error)),
    }
}
