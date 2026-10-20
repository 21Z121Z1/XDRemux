//! Policy for a new image-derived Texture edit. Capture provenance stays in EXIF.
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::apple_photographic_styles::{serialize_binary_plist, PlistValue};
use crate::AppleStyleDataError;

pub const APPLE_TEXTURE_ANALYSIS_MAX_DIMENSION: u32 = 1024;

/// Select native edit tuning; these values are not source camera facts.
pub fn apple_texture_tuning_profile() -> Value {
    json!({"hardware_model":"iPhone19,2", "port_type":"PortTypeBack",
        "capture_mode":"Still", "capture_type":"LF", "preset":"Studio"})
}

fn value(input: &Value) -> Result<PlistValue, AppleStyleDataError> {
    Ok(match input {
        Value::Bool(v) => PlistValue::Bool(*v),
        Value::Number(v) => {
            if let Some(v) = v.as_u64() {
                PlistValue::Integer(v)
            } else {
                PlistValue::Real(
                    v.as_f64()
                        .filter(|v| v.is_finite())
                        .ok_or(AppleStyleDataError::InvalidPropertyListInput)?,
                )
            }
        }
        Value::String(v) => PlistValue::String(v.clone()),
        Value::Array(v) => PlistValue::Array(v.iter().map(value).collect::<Result<_, _>>()?),
        Value::Object(v) => PlistValue::Dictionary(
            v.iter()
                .map(|(k, v)| Ok((k.clone(), value(v)?)))
                .collect::<Result<_, _>>()?,
        ),
        Value::Null => return Err(AppleStyleDataError::InvalidPropertyListInput),
    })
}

/// Preserve native normalized geometry and measured groups. Empty observations
/// produce an empty people array, never a template face or guessed statistics.
pub fn apple_texture_property_list(people: &[Value]) -> Result<Vec<u8>, AppleStyleDataError> {
    let mut keys = BTreeSet::new();
    for person in people {
        let key = person["instanceMaskReferenceKey"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or(AppleStyleDataError::InvalidPropertyListInput)?;
        let stats = person["imageStats"]
            .as_object()
            .ok_or(AppleStyleDataError::InvalidPropertyListInput)?;
        if !keys.insert(key)
            || !stats
                .get("Mattify")
                .is_some_and(|v| v.as_object().is_some_and(|v| !v.is_empty()))
        {
            return Err(AppleStyleDataError::InvalidPropertyListInput);
        }
        for angle in ["facePitch", "faceRoll", "faceYaw"] {
            if !person[angle]
                .as_f64()
                .is_some_and(|v| v.is_finite() && v.abs() <= 360.0)
            {
                return Err(AppleStyleDataError::InvalidPropertyListInput);
            }
        }
    }
    let root = json!({
        "HardwareModel":"iPhone19,2", "PortType":"PortTypeBack",
        "CaptureMode":"Still", "CaptureType":"LF", "Preset":"Standard",
        "TextureStylePeopleDataVersion":3, "FilmGrainSeed":0,
        "TextureStylePostProcessedPeopleData":people,
    });
    let PlistValue::Dictionary(entries) = value(&root)? else {
        unreachable!()
    };
    serialize_binary_plist(PlistValue::Dictionary(BTreeMap::from_iter(entries)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person() -> Value {
        json!({"instanceMaskReferenceKey":"DerivedVisionInstance1", "faceID":1,
            "facePitch":-47.5,"faceRoll":-0.75,"faceYaw":19.25,
            "imageStats":{"Mattify":{"AvgFaceColor":[0.1,0.2,0.3],"SkipPerson":false}}})
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
    fn duplicate_instances_null_statistics_and_unsigned_angle_corruption_fail() {
        assert!(apple_texture_property_list(&[person(), person()]).is_err());
        let mut p = person();
        p["facePitch"] = json!(4294967296.0);
        assert!(apple_texture_property_list(&[p]).is_err());
        let mut p = person();
        p["imageStats"]["Mattify"] = Value::Null;
        assert!(apple_texture_property_list(&[p]).is_err());
    }
}
