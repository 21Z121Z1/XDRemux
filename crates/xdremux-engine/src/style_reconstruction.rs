//! Policy for the separate, source-preserving Styles reconstruction outcome.
pub const STYLE_RECONSTRUCTION_METADATA_VERSIONS: &[u32] = &[8, 9, 15, 16];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StyleReconstructionPlan {
    pub max_edge: u32,
    pub preserve_sdr: bool,
    pub guidance_analysis_edge: usize,
    pub guidance_primary_span_ev: f32,
    pub guidance_minimum_region_pixels: usize,
    pub guidance_feather_radius: usize,
    pub guidance_merge_radius: usize,
    pub guidance_detail_radius: usize,
    pub guidance_detail_threshold_ev: f32,
    pub guidance_detail_limit_ev: f32,
    pub guidance_minimum_gain_support: f32,
    pub guidance_contrast_limit: f32,
    pub guidance_minimum_detail_correlation: f32,
    pub guidance_maximum_detail_fit_error_ev: f32,
    pub guidance_minimum_structure_correlation: f32,
    pub guidance_edge_ev: f32,
    pub guidance_flat_ev: f32,
    pub guidance_limit_ev: f32,
}

pub fn plan_style_reconstruction(max_edge: u32) -> Result<StyleReconstructionPlan, &'static str> {
    if max_edge != 0 && !(256..=16384).contains(&max_edge) {
        return Err("max-edge must be zero (native size) or 256 through 16384");
    }
    Ok(StyleReconstructionPlan {
        max_edge,
        preserve_sdr: true,
        guidance_analysis_edge: 512,
        guidance_primary_span_ev: 0.4,
        guidance_minimum_region_pixels: 64,
        guidance_feather_radius: 4,
        guidance_merge_radius: 16,
        guidance_detail_radius: 3,
        guidance_detail_threshold_ev: 0.1,
        guidance_detail_limit_ev: 1.5,
        guidance_minimum_gain_support: 0.9,
        guidance_contrast_limit: 4.,
        guidance_minimum_detail_correlation: 0.5,
        guidance_maximum_detail_fit_error_ev: 0.1,
        guidance_minimum_structure_correlation: 0.9,
        guidance_edge_ev: 0.1,
        guidance_flat_ev: 0.075,
        guidance_limit_ev: 0.25,
    })
}

/// A separate display outcome, with method coefficients rather than editor axes.
#[derive(Debug, Clone, Copy)]
pub struct StyleDisplayPlan {
    pub headroom: u32,
    pub methods: [f32; 8],
    pub analysis_edge: usize,
    pub local_limit_ev: f32,
}

pub fn plan_style_display(headroom: u32) -> Result<StyleDisplayPlan, &'static str> {
    if ![2, 4, 8].contains(&headroom) {
        return Err("display headroom must be 2, 4 or 8");
    }
    // Creative chroma and skin protection stay separate from range-dependent
    // spatial compensation. Independent tonal leaves remain available to
    // research callers, but are not added to this family's range budget.
    Ok(StyleDisplayPlan {
        headroom,
        methods: [0., 0., 0.75, 0.95, 0., 0., 0.98, 0.85],
        analysis_edge: 512,
        local_limit_ev: 1.5,
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
            let plan = plan_style_display(h).unwrap();
            assert_eq!(plan.headroom, h);
            assert_eq!(
                [
                    plan.methods[0],
                    plan.methods[1],
                    plan.methods[4],
                    plan.methods[5]
                ],
                [0.; 4]
            );
            assert_eq!(plan.analysis_edge, 512);
            assert!(plan.local_limit_ev > 0.);
        }
        for h in [0, 1, 3, 16] {
            assert!(plan_style_display(h).is_err());
        }
    }
}
