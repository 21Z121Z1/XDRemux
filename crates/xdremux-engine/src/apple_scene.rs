use crate::apple_photographic_styles::{
    AppleStyleDataError, AppleStyleSceneClass, AppleStyleSceneScores,
};

/// One raw observation emitted by Apple's `VNClassifyImageRequest`.
///
/// The Apple adapter owns only framework execution. Mapping Vision identifiers
/// into XDRemux product concepts remains deterministic Rust policy here.
#[derive(Debug, Clone, PartialEq)]
pub struct AppleVisionClassificationObservation {
    pub identifier: String,
    pub confidence: f64,
}

impl AppleVisionClassificationObservation {
    pub fn new(identifier: impl Into<String>, confidence: f64) -> Self {
        Self {
            identifier: identifier.into(),
            confidence,
        }
    }
}

const FOOD_IDENTIFIERS: &[&str] = &["food", "meal", "dish"];
const SUNSET_IDENTIFIERS: &[&str] = &["sunset", "sunrise", "dusk"];
const INDOOR_IDENTIFIERS: &[&str] = &["indoor", "interior", "room"];
const OUTDOOR_IDENTIFIERS: &[&str] = &["outdoor"];

fn scene_class(identifier: &str) -> Option<AppleStyleSceneClass> {
    if FOOD_IDENTIFIERS.contains(&identifier) {
        Some(AppleStyleSceneClass::Food)
    } else if SUNSET_IDENTIFIERS.contains(&identifier) {
        Some(AppleStyleSceneClass::Sunset)
    } else if INDOOR_IDENTIFIERS.contains(&identifier) {
        Some(AppleStyleSceneClass::Indoor)
    } else if OUTDOOR_IDENTIFIERS.contains(&identifier) {
        Some(AppleStyleSceneClass::Outdoor)
    } else {
        None
    }
}

/// Convert raw Vision classifications into the four confidence buckets used by
/// the Rust-owned Photographic Styles scene policy.
///
/// Unknown Vision identifiers are intentionally ignored. Known identifiers are
/// validated before aggregation so malformed platform facts fail closed rather
/// than silently influencing scene selection.
pub fn apple_style_scene_scores_from_vision_observations(
    observations: &[AppleVisionClassificationObservation],
) -> Result<AppleStyleSceneScores, AppleStyleDataError> {
    let mut scores = AppleStyleSceneScores {
        food: 0.0,
        sunset: 0.0,
        indoor: 0.0,
        outdoor: 0.0,
    };

    for observation in observations {
        let Some(class) = scene_class(&observation.identifier) else {
            continue;
        };
        if !observation.confidence.is_finite() || !(0.0..=1.0).contains(&observation.confidence) {
            return Err(AppleStyleDataError::InvalidSceneScore {
                class,
                value: observation.confidence,
            });
        }
        let slot = match class {
            AppleStyleSceneClass::Food => &mut scores.food,
            AppleStyleSceneClass::Sunset => &mut scores.sunset,
            AppleStyleSceneClass::Indoor => &mut scores.indoor,
            AppleStyleSceneClass::Outdoor => &mut scores.outdoor,
        };
        *slot = slot.max(observation.confidence);
    }

    Ok(scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_are_rust_policy_and_max_confidence_wins() {
        let scores = apple_style_scene_scores_from_vision_observations(&[
            AppleVisionClassificationObservation::new("meal", 0.31),
            AppleVisionClassificationObservation::new("dish", 0.74),
            AppleVisionClassificationObservation::new("sunrise", 0.42),
            AppleVisionClassificationObservation::new("room", 0.55),
            AppleVisionClassificationObservation::new("outdoor", 0.63),
        ])
        .unwrap();

        assert_eq!(scores.food, 0.74);
        assert_eq!(scores.sunset, 0.42);
        assert_eq!(scores.indoor, 0.55);
        assert_eq!(scores.outdoor, 0.63);
    }

    #[test]
    fn unknown_vision_identifiers_do_not_change_product_scores() {
        let scores = apple_style_scene_scores_from_vision_observations(&[
            AppleVisionClassificationObservation::new("cat", 0.99),
            AppleVisionClassificationObservation::new("landscape", 0.88),
        ])
        .unwrap();

        assert_eq!(
            scores,
            AppleStyleSceneScores {
                food: 0.0,
                sunset: 0.0,
                indoor: 0.0,
                outdoor: 0.0,
            }
        );
    }

    #[test]
    fn invalid_known_confidence_fails_closed() {
        let error = apple_style_scene_scores_from_vision_observations(&[
            AppleVisionClassificationObservation::new("food", f64::NAN),
        ])
        .unwrap_err();

        assert!(matches!(
            error,
            AppleStyleDataError::InvalidSceneScore {
                class: AppleStyleSceneClass::Food,
                value
            } if value.is_nan()
        ));
    }
}
