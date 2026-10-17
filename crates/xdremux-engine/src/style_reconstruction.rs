//! Policy for the separate, source-preserving Styles reconstruction outcome.
pub const STYLE_RECONSTRUCTION_METADATA_VERSIONS: &[u32] = &[8, 9, 15, 16];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleReconstructionPlan {
    pub max_edge: u32,
    pub preserve_sdr: bool,
}

pub fn plan_style_reconstruction(max_edge: u32) -> Result<StyleReconstructionPlan, &'static str> {
    if max_edge != 0 && !(256..=16384).contains(&max_edge) {
        return Err("max-edge must be zero (native size) or 256 through 16384");
    }
    Ok(StyleReconstructionPlan {
        max_edge,
        preserve_sdr: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_reconstruction_plan_preserves_the_source_sdr() {
        for edge in [0, 256, 1024, 16384] {
            assert!(plan_style_reconstruction(edge).unwrap().preserve_sdr);
        }
        assert!(plan_style_reconstruction(1).is_err());
        assert!(plan_style_reconstruction(16385).is_err());
        assert!(!STYLE_RECONSTRUCTION_METADATA_VERSIONS.contains(&17));
    }
}
