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

/// A separate display outcome, with method coefficients rather than editor axes.
#[derive(Debug, Clone, Copy)]
pub struct StyleDisplayPlan {
    pub headroom: u32,
    pub methods: [f32; 8],
}

pub fn plan_style_display(headroom: u32) -> Result<StyleDisplayPlan, &'static str> {
    if ![2, 4, 8].contains(&headroom) {
        return Err("display headroom must be 2, 4 or 8");
    }
    // mids background/foreground, SDR chroma anchor background/skin,
    // skin brightness/highlight, chroma gain, foreground protection.
    Ok(StyleDisplayPlan {
        headroom,
        methods: [0.08, 0.04, 0.75, 0.95, -0.03, 0., 0.98, 0.85],
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
    #[test]
    fn display_outcome_has_an_explicit_supported_headroom() {
        for h in [2, 4, 8] {
            assert_eq!(plan_style_display(h).unwrap().headroom, h);
        }
        for h in [0, 1, 3, 16] {
            assert!(plan_style_display(h).is_err());
        }
    }
}
