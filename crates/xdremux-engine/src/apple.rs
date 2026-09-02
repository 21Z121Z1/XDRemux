#[path = "apple_portrait.rs"]
mod portrait;
pub(crate) use portrait::{AppleRendDocument, AppleRendError, AppleRendRecord, AppleXhlrbControlOutput};

use xdremux_format::FourCC;

/// Apple consumer facts reported by a platform capability adapter.
///
/// These are observations, not platform policy. The adapter reports what
/// ImageIO exposes; the Rust engine decides which facts a product feature
/// requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppleImageAuxiliaryFacts {
    pub iso_gain_map: bool,
    pub disparity: bool,
    pub portrait_effects_matte: bool,
    pub skin_matte: bool,
    pub hair_matte: bool,
    pub teeth_matte: bool,
    pub glasses_matte: bool,
}

impl AppleImageAuxiliaryFacts {
    /// Resource contract required for Apple Photos portrait editing output.
    ///
    /// Keep this policy in Rust rather than teaching the platform adapter to
    /// answer a business-level "valid portrait" question.
    pub const fn satisfies_portrait_editing(self) -> bool {
        self.iso_gain_map
            && self.disparity
            && self.portrait_effects_matte
            && self.skin_matte
            && self.hair_matte
            && self.teeth_matte
            && self.glasses_matte
    }
}

/// Semantic image resources used by Apple photo features.
///
/// Product code deals in these roles. Framework class names, selectors, and
/// whether a role currently requires public API or SPI stay behind the Apple
/// adapter and are not part of Rust product policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppleSemanticRole {
    Person,
    Skin,
    Hair,
    Teeth,
    Glasses,
    Sky,
}

/// Semantic resources required by the current Apple Photos Portrait contract.
///
/// This is product policy. The adapter is given these roles explicitly rather
/// than choosing a feature profile itself.
pub const APPLE_PORTRAIT_SEMANTIC_ROLES: [AppleSemanticRole; 5] = [
    AppleSemanticRole::Person,
    AppleSemanticRole::Skin,
    AppleSemanticRole::Hair,
    AppleSemanticRole::Teeth,
    AppleSemanticRole::Glasses,
];

/// ImageIO observations about one ISO Gain Map auxiliary image.
///
/// The platform adapter reports the raw FourCC and geometry. Product policy
/// such as which pixel formats are accepted for Portrait remains in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleGainMapFacts {
    pub pixel_format: FourCC,
    pub width: u32,
    pub height: u32,
}

impl AppleGainMapFacts {
    pub fn supports_portrait_source(self) -> bool {
        matches!(self.pixel_format.as_bytes(), b"444f" | b"L008")
    }

    pub const fn has_geometry(self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_policy_requires_the_complete_auxiliary_resource_set() {
        let complete = AppleImageAuxiliaryFacts {
            iso_gain_map: true,
            disparity: true,
            portrait_effects_matte: true,
            skin_matte: true,
            hair_matte: true,
            teeth_matte: true,
            glasses_matte: true,
        };
        assert!(complete.satisfies_portrait_editing());

        let missing_glasses = AppleImageAuxiliaryFacts {
            glasses_matte: false,
            ..complete
        };
        assert!(!missing_glasses.satisfies_portrait_editing());
    }

    #[test]
    fn portrait_semantic_contract_is_explicit_and_stable() {
        assert_eq!(
            APPLE_PORTRAIT_SEMANTIC_ROLES,
            [
                AppleSemanticRole::Person,
                AppleSemanticRole::Skin,
                AppleSemanticRole::Hair,
                AppleSemanticRole::Teeth,
                AppleSemanticRole::Glasses,
            ]
        );
    }

    #[test]
    fn portrait_source_gain_map_policy_accepts_only_observed_apple_formats() {
        for pixel_format in [FourCC::new(*b"444f"), FourCC::new(*b"L008")] {
            let facts = AppleGainMapFacts {
                pixel_format,
                width: 1024,
                height: 768,
            };
            assert!(facts.supports_portrait_source());
            assert!(facts.has_geometry(1024, 768));
        }

        let unsupported = AppleGainMapFacts {
            pixel_format: FourCC::new(*b"420f"),
            width: 1024,
            height: 768,
        };
        assert!(!unsupported.supports_portrait_source());
    }
}
