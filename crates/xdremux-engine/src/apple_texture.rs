//! Policy for a new image-derived Texture edit. Capture provenance stays in EXIF.
use std::collections::{BTreeMap, BTreeSet};

use crate::apple_photographic_styles::serialize_binary_plist;
pub use crate::apple_photographic_styles::PlistValue as AppleTextureValue;
use crate::AppleStyleDataError;
use AppleTextureValue as Value;

pub const APPLE_TEXTURE_ANALYSIS_MAX_DIMENSION: u32 = 1024;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct AppleTextureTuningProfile {
    pub hardware_model: &'static str,
    pub port_type: &'static str,
    pub capture_mode: &'static str,
    pub capture_type: &'static str,
    pub preset: &'static str,
}

/// Select native edit tuning; these values are not source camera facts.
pub fn apple_texture_tuning_profile() -> AppleTextureTuningProfile {
    AppleTextureTuningProfile {
        hardware_model: "iPhone19,2",
        port_type: "PortTypeBack",
        capture_mode: "Still",
        capture_type: "LF",
        preset: "Studio",
    }
}

fn valid(value: &Value) -> bool {
    match value {
        Value::Real(v) => v.is_finite(),
        Value::Array(v) => v.iter().all(valid),
        Value::Dictionary(v) => v.values().all(valid),
        _ => true,
    }
}

/// Preserve native normalized geometry and measured groups. Empty observations
/// produce an empty people array, never a template face or guessed statistics.
/// The runtime converts its transport values into this independent data model.
pub fn apple_texture_property_list(people: &[Value]) -> Result<Vec<u8>, AppleStyleDataError> {
    let invalid = || AppleStyleDataError::InvalidPropertyListInput;
    let mut keys = BTreeSet::new();
    for person in people {
        if !valid(person) {
            return Err(invalid());
        }
        let Value::Dictionary(fields) = person else {
            return Err(invalid());
        };
        let Some(Value::String(key)) = fields.get("instanceMaskReferenceKey") else {
            return Err(invalid());
        };
        let Some(Value::Dictionary(stats)) = fields.get("imageStats") else {
            return Err(invalid());
        };
        if key.is_empty()
            || !keys.insert(key)
            || !matches!(stats.get("Mattify"), Some(Value::Dictionary(v)) if !v.is_empty())
        {
            return Err(invalid());
        }
        for angle in ["facePitch", "faceRoll", "faceYaw"] {
            let value = match fields.get(angle) {
                Some(Value::Real(v)) => *v,
                Some(Value::Integer(v)) => *v as f64,
                _ => return Err(invalid()),
            };
            if !value.is_finite() || value.abs() > 360.0 {
                return Err(invalid());
            }
        }
    }
    let text = |v: &str| Value::String(v.to_owned());
    let root = Value::Dictionary(BTreeMap::from([
        ("HardwareModel".to_owned(), text("iPhone19,2")),
        ("PortType".to_owned(), text("PortTypeBack")),
        ("CaptureMode".to_owned(), text("Still")),
        ("CaptureType".to_owned(), text("LF")),
        ("Preset".to_owned(), text("Standard")),
        (
            "TextureStylePeopleDataVersion".to_owned(),
            Value::Integer(3),
        ),
        ("FilmGrainSeed".to_owned(), Value::Integer(0)),
        (
            "TextureStylePostProcessedPeopleData".to_owned(),
            Value::Array(people.to_vec()),
        ),
    ]));
    serialize_binary_plist(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn person() -> Value {
        Value::Dictionary(BTreeMap::from([
            (
                "instanceMaskReferenceKey".to_owned(),
                Value::String("DerivedVisionInstance1".to_owned()),
            ),
            ("facePitch".to_owned(), Value::Real(-47.5)),
            ("faceRoll".to_owned(), Value::Real(-0.75)),
            ("faceYaw".to_owned(), Value::Real(19.25)),
            (
                "imageStats".to_owned(),
                Value::Dictionary(BTreeMap::from([(
                    "Mattify".to_owned(),
                    Value::Dictionary(BTreeMap::from([
                        (
                            "AverageFaceColor".to_owned(),
                            Value::Array(vec![
                                Value::Real(0.1),
                                Value::Real(0.2),
                                Value::Real(0.3),
                            ]),
                        ),
                        ("SkipPerson".to_owned(), Value::Bool(false)),
                    ])),
                )])),
            ),
        ]))
    }
    #[test]
    fn texture_contains_measured_people_signed_angles_and_empty_context() {
        let data = apple_texture_property_list(&[person()]).unwrap();
        assert!(data.starts_with(b"bplist00"));
        assert!(data.windows(7).any(|v| v == b"Mattify"));
        assert!(data.windows(8).any(|v| v == (-47.5_f64).to_be_bytes()));
        assert!(apple_texture_property_list(&[]).is_ok());
    }
    #[test]
    fn duplicate_instances_empty_statistics_and_corrupt_angles_fail() {
        assert!(apple_texture_property_list(&[person(), person()]).is_err());
        for bad in [4294967296.0, f64::NAN] {
            let Value::Dictionary(mut fields) = person() else {
                unreachable!()
            };
            fields.insert("facePitch".to_owned(), Value::Real(bad));
            assert!(apple_texture_property_list(&[Value::Dictionary(fields)]).is_err());
        }
        let Value::Dictionary(mut fields) = person() else {
            unreachable!()
        };
        fields.insert("imageStats".to_owned(), Value::Dictionary(BTreeMap::new()));
        assert!(apple_texture_property_list(&[Value::Dictionary(fields)]).is_err());
    }
}
