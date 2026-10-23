#![forbid(unsafe_code)]

mod display_family;
mod display_methods;
mod edr;
mod error;
mod gainmap;
mod info;
mod scene_guidance;
mod scene_reconstruction;
pub use display_family::{
    DisplayFamily, DisplayFamilyPolicy, DisplayFamilyTarget, DisplaySchedule,
    DISPLAY_FAMILY_HEADROOM, DISPLAY_FAMILY_OFFSET,
};
pub use display_methods::{
    render_display_pixel, render_styles_display, DisplayMethods, DisplayTarget,
};
pub use scene_guidance::{SceneGuidanceMetrics, SceneGuidancePolicy};
pub use scene_reconstruction::{
    decode_styles_thumbnail, display_p3_luminance, make_scene_gain_map, reconstruct_relative_scene,
    styles_thumbnail_to_linear, LinearPixel, LinearThumbnailEncoding, SceneGainMap,
    SceneReconstruction,
};
mod scene_linear;

pub use edr::{
    edr_scale_calculator, get_knee_point, get_knee_point_result, resolve, ExtractionMode,
    KneePointResult, ResolvedScale,
};
pub use error::{HdrError, Result};
pub use gainmap::{gain_map_parameters, reconstruct_gain_map, GainMapParams, GainMapRaster};
pub use info::make_private_gain_map_info_floats;
pub use scene_linear::{
    apply_iso_gain_sample, area_resize_linear_rgb, fitted_even_dimensions,
    iso21496_application_weight, iso_gain_application_for_alternate, reconstruct_scene_linear_hdr,
    GainMapCodes, IsoGainApplication, IsoGainChannel, SceneLinearRgb,
};
