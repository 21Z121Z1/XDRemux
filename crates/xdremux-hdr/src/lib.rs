#![forbid(unsafe_code)]

mod edr;
mod error;
mod gainmap;
mod info;

pub use edr::{
    edr_scale_calculator, get_knee_point, get_knee_point_result, resolve, ExtractionMode,
    KneePointResult, ResolvedScale,
};
pub use error::{HdrError, Result};
pub use gainmap::{
    gain_map_parameters, reconstruct_gain_map, Family, GainMapParams, GainMapRaster,
};
pub use info::make_private_gain_map_info_floats;
