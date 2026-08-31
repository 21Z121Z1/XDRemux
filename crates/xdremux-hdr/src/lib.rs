#![forbid(unsafe_code)]

mod edr;
mod error;

pub use edr::{
    edr_scale_calculator, get_knee_point, get_knee_point_result, resolve, ExtractionMode,
    KneePointResult, ResolvedScale,
};
pub use error::{HdrError, Result};
