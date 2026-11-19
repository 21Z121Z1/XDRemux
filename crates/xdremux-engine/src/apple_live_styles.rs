//! Style resources for a rendered motion frame. The input is an observation of
//! the final video and does not describe an Apple camera capture.
use std::collections::BTreeMap;

use crate::apple_photographic_styles::{serialize_binary_plist, PlistValue};
use crate::{
    apple_style_linear_thumbnail_codes, apple_style_monotonic_global_tone_curve,
    apple_texture_tuning_profile, AppleStyleDataError,
};

pub struct AppleLiveFrameStyle {
    pub apple_log_codes: Vec<u16>,
    pub metadata: Vec<u8>,
}

/// Use one storage exposure for the clip, so the reference does not change its
/// brightness scale when a highlight briefly enters the frame.
pub fn apple_live_frame_style(
    linear_rec2020: &[f32],
    storage_exposure: f32,
) -> Result<AppleLiveFrameStyle, AppleStyleDataError> {
    let apple_log_codes = apple_style_linear_thumbnail_codes(linear_rec2020, storage_exposure)?;
    let reference: Vec<_> = linear_rec2020
        .iter()
        .map(|channel| (channel / storage_exposure).clamp(0.0, 1.0))
        .collect();
    let output: Vec<_> = linear_rec2020
        .iter()
        .map(|channel| channel.clamp(0.0, 1.0))
        .collect();
    let curve = apple_style_monotonic_global_tone_curve(&reference, &output)?;
    let metadata = serialize_binary_plist(PlistValue::Dictionary(BTreeMap::from([
        ("0".into(), PlistValue::Integer(16)),
        ("3".into(), PlistValue::Data(curve.data)),
        ("4".into(), PlistValue::Real(f64::from(storage_exposure))),
        ("5".into(), PlistValue::Integer(2)),
        ("h".into(), PlistValue::Real(1.0)),
    ])))?;
    Ok(AppleLiveFrameStyle {
        apple_log_codes,
        metadata,
    })
}

/// Describe the neutral Texture edit state. The tuning model selects the
/// Apple renderer's profile. Source camera identity remains in the container.
pub fn apple_live_texture_metadata() -> Result<Vec<u8>, AppleStyleDataError> {
    let tuning = apple_texture_tuning_profile();
    let text = |s: &str| PlistValue::String(s.to_string());
    serialize_binary_plist(PlistValue::Dictionary(BTreeMap::from([
        ("HardwareModel".into(), text(tuning.hardware_model)),
        ("PortType".into(), text(tuning.port_type)),
        ("CaptureMode".into(), text("LivePhoto")),
        ("Preset".into(), text("Standard")),
        (
            "TextureStylePeopleDataVersion".into(),
            PlistValue::Integer(3),
        ),
        (
            "TextureStyleFaceAttitudeMetadata".into(),
            PlistValue::Array(vec![]),
        ),
    ])))
}
