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
}
