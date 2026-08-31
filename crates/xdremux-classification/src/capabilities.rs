use std::collections::BTreeSet;

use crate::model::PhotoCapability;

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// Detects only capabilities backed by container evidence that the existing
/// Swift and Python classification contracts both understand.
pub fn detect_capabilities(data: &[u8]) -> BTreeSet<PhotoCapability> {
    let mut capabilities = BTreeSet::new();
    let has_private_gain_map = contains_bytes(data, br#""local.uhdr.gainmap.data""#)
        || contains_bytes(data, br#""local.uhdr.gainmap.info""#);
    if has_private_gain_map {
        capabilities.extend([
            PhotoCapability::ProXdr,
            PhotoCapability::GainMap,
            PhotoCapability::Hdr,
        ]);
    }
    if contains_bytes(data, br#""rear.depth""#) && contains_bytes(data, br#""rear.depth.config""#) {
        capabilities.insert(PhotoCapability::Depth);
    }
    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_complete_depth_manifest_evidence() {
        let complete = br#"oplus_18 {"name":"local.uhdr.gainmap.data"} {"name":"rear.depth"} {"name":"rear.depth.config"}"#;
        assert_eq!(
            detect_capabilities(complete),
            BTreeSet::from([
                PhotoCapability::ProXdr,
                PhotoCapability::GainMap,
                PhotoCapability::Hdr,
                PhotoCapability::Depth,
            ])
        );

        let config_only = br#"oplus_18 {"name":"rear.depth.config"}"#;
        assert!(!detect_capabilities(config_only).contains(&PhotoCapability::Depth));
    }
}
