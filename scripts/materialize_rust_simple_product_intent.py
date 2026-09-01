#!/usr/bin/env python3
from pathlib import Path

engine = Path("crates/xdremux-engine/src/lib.rs")
text = engine.read_text()

anchor = '''impl Default for ConversionRequest {
    fn default() -> Self {
        Self {
            oppo_compatibility: OppoCompatibility::Off,
            input_processing_branch: InputProcessingBranch::Hybrid,
            oppo_camera_tail: OppoCameraTail::PreserveWithoutPrivateHdr,
            tmap_format: TmapFormat::ImageIo,
            apple_features: AppleFeatureRequest::default(),
        }
    }
}
'''
addition = anchor + '''
impl ConversionRequest {
    /// Product intent for output that remains recognizable by OPPO Gallery.
    ///
    /// The public product surface deliberately exposes one intent rather than
    /// the internal routing/tail knobs. Engine policy owns the exact mapping.
    pub fn oppo_gallery_compatible() -> Self {
        Self {
            oppo_compatibility: OppoCompatibility::Auto,
            oppo_camera_tail: OppoCameraTail::Preserve,
            ..Self::default()
        }
    }

    pub const fn requests_oppo_gallery_compatibility(self) -> bool {
        !matches!(self.oppo_compatibility, OppoCompatibility::Off)
    }
}
'''
if "pub fn oppo_gallery_compatible() -> Self" not in text:
    if anchor not in text:
        raise SystemExit("ConversionRequest default anchor not found")
    text = text.replace(anchor, addition, 1)

# Add focused policy tests rather than making CLI tests inspect engine internals.
test_anchor = '''    #[test]
    fn default_request_uses_auto_family_and_canonical_policy() {
'''
# The test was renamed when family disappeared, so accept either nearby marker.
if test_anchor not in text:
    test_anchor = '''    #[test]
    fn default_request_uses_canonical_policy() {
'''
product_test = '''    #[test]
    fn oppo_gallery_intent_owns_internal_compatibility_policy() {
        let request = ConversionRequest::oppo_gallery_compatible();
        assert!(request.requests_oppo_gallery_compatibility());
        assert_eq!(request.oppo_compatibility, OppoCompatibility::Auto);
        assert_eq!(request.oppo_camera_tail, OppoCameraTail::Preserve);
        assert_eq!(request.input_processing_branch, InputProcessingBranch::Hybrid);
        assert_eq!(request.tmap_format, TmapFormat::ImageIo);
        assert_eq!(request.apple_features, AppleFeatureRequest::default());
        assert!(!ConversionRequest::default().requests_oppo_gallery_compatibility());
    }

'''
if "fn oppo_gallery_intent_owns_internal_compatibility_policy()" not in text:
    marker = "#[cfg(test)]\nmod tests {\n    use super::*;\n\n"
    if marker not in text:
        raise SystemExit("engine test module anchor not found")
    text = text.replace(marker, marker + product_test, 1)

for required in (
    "pub fn oppo_gallery_compatible() -> Self",
    "OppoCompatibility::Auto",
    "oppo_camera_tail: OppoCameraTail::Preserve",
    "requests_oppo_gallery_compatibility",
    "fn oppo_gallery_intent_owns_internal_compatibility_policy()",
):
    if required not in text:
        raise SystemExit(f"missing product-intent marker: {required}")

engine.write_text(text)
