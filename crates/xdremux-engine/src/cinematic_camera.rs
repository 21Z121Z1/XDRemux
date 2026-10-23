use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CinematicCameraEstimator {
    pub name: String,
    pub revision: String,
    pub weights_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CinematicCameraIntrinsics {
    pub image_size: [u32; 2],
    pub focal_length_pixels: [f64; 2],
    pub principal_point_pixels: [f64; 2],
    pub focal_uncertainty_pixels: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CinematicCameraSample {
    pub time_seconds: f64,
    pub intrinsics: CinematicCameraIntrinsics,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CinematicCameraEstimate {
    pub schema_version: u32,
    pub source_sha256: String,
    pub estimator: CinematicCameraEstimator,
    pub samples: Vec<CinematicCameraSample>,
    pub shared_intrinsics: Option<CinematicCameraIntrinsics>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CinematicCameraSummary {
    pub focal_length_35mm: f32,
    pub estimator: String,
    pub sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CinematicCameraError(&'static str);

impl fmt::Display for CinematicCameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for CinematicCameraError {}

fn hex_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl CinematicCameraIntrinsics {
    fn equivalent_mm(&self) -> Result<f64, CinematicCameraError> {
        let [width, height] = self.image_size.map(f64::from);
        let [fx, fy] = self.focal_length_pixels;
        let [cx, cy] = self.principal_point_pixels;
        let uncertainty = self.focal_uncertainty_pixels;
        if !(1.0..=16384.0).contains(&width)
            || !(1.0..=16384.0).contains(&height)
            || !fx.is_finite()
            || !fy.is_finite()
            || fx <= 0.0
            || fy <= 0.0
            || !cx.is_finite()
            || !cy.is_finite()
            || (cx - width / 2.0).abs() > 1.0
            || (cy - height / 2.0).abs() > 1.0
            || (fx / fy - 1.0).abs() > 0.01
            || !uncertainty.is_finite()
            || uncertainty < 0.0
        {
            return Err(CinematicCameraError(
                "invalid sampled pinhole camera geometry",
            ));
        }
        // Match the diagonal field of view to a 36 by 24 mm frame.
        let equivalent = 36.0_f64.hypot(24.0) / (width / fx).hypot(height / fy);
        if !(1.0..=300.0).contains(&equivalent) {
            return Err(CinematicCameraError(
                "estimated 35mm-equivalent focal length is outside 1 to 300 mm",
            ));
        }
        Ok(equivalent)
    }

    fn require_certain_focal(&self) -> Result<(), CinematicCameraError> {
        if self.focal_uncertainty_pixels / self.focal_length_pixels[1] > 0.20 {
            return Err(CinematicCameraError(
                "camera estimate focal uncertainty exceeds 20 percent",
            ));
        }
        Ok(())
    }
}

impl CinematicCameraEstimate {
    pub fn resolve(&self) -> Result<CinematicCameraSummary, CinematicCameraError> {
        if self.schema_version != 1
            || !hex_digest(&self.source_sha256, 64)
            || !hex_digest(&self.estimator.weights_sha256, 64)
            || !hex_digest(&self.estimator.revision, 40)
            || self.estimator.name.is_empty()
            || self.estimator.name.len() > 128
        {
            return Err(CinematicCameraError(
                "unsupported camera estimate schema or missing estimator provenance",
            ));
        }
        if !(3..=128).contains(&self.samples.len()) {
            return Err(CinematicCameraError(
                "camera estimate requires 3 to 128 sampled frames",
            ));
        }
        let mut focal_lengths = Vec::with_capacity(self.samples.len());
        let mut previous_time = -1.0;
        for sample in &self.samples {
            if !sample.time_seconds.is_finite()
                || sample.time_seconds < 0.0
                || sample.time_seconds <= previous_time
            {
                return Err(CinematicCameraError(
                    "invalid camera estimate sampling times",
                ));
            }
            focal_lengths.push(sample.intrinsics.equivalent_mm()?);
            previous_time = sample.time_seconds;
        }
        let selected = if let Some(shared) = &self.shared_intrinsics {
            let focal = shared.equivalent_mm()?;
            shared.require_certain_focal()?;
            focal
        } else {
            for sample in &self.samples {
                sample.intrinsics.require_certain_focal()?;
            }
            focal_lengths.sort_by(f64::total_cmp);
            let middle = focal_lengths.len() / 2;
            if focal_lengths.len().is_multiple_of(2) {
                (focal_lengths[middle - 1] + focal_lengths[middle]) / 2.0
            } else {
                focal_lengths[middle]
            }
        };
        if focal_lengths
            .iter()
            .any(|focal| (focal / selected - 1.0).abs() > 0.20)
        {
            return Err(CinematicCameraError("camera estimates disagree by more than 20 percent; split cuts or zooms into fixed-view clips or supply a manual focal length"));
        }
        Ok(CinematicCameraSummary {
            focal_length_35mm: selected as f32,
            estimator: self.estimator.name.clone(),
            sample_count: self.samples.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intrinsics() -> CinematicCameraIntrinsics {
        CinematicCameraIntrinsics {
            image_size: [3600, 2400],
            focal_length_pixels: [3500.0, 3500.0],
            principal_point_pixels: [1800.0, 1200.0],
            focal_uncertainty_pixels: 35.0,
        }
    }

    fn estimate() -> CinematicCameraEstimate {
        CinematicCameraEstimate {
            schema_version: 1,
            source_sha256: "a".repeat(64),
            estimator: CinematicCameraEstimator {
                name: "test-camera".to_owned(),
                revision: "b".repeat(40),
                weights_sha256: "c".repeat(64),
            },
            samples: (0..3)
                .map(|i| CinematicCameraSample {
                    time_seconds: f64::from(i),
                    intrinsics: intrinsics(),
                })
                .collect(),
            shared_intrinsics: None,
        }
    }

    #[test]
    fn focal_estimate_preserves_diagonal_view_under_resize_and_rotation() {
        let original = estimate();
        let mut resized = original.clone();
        for sample in &mut resized.samples {
            sample.intrinsics = CinematicCameraIntrinsics {
                image_size: [1200, 1800],
                focal_length_pixels: [1750.0, 1750.0],
                principal_point_pixels: [600.0, 900.0],
                focal_uncertainty_pixels: 17.5,
            };
        }
        assert_eq!(original.resolve().unwrap().focal_length_35mm, 35.0);
        assert_eq!(resized.resolve().unwrap().focal_length_35mm, 35.0);
    }

    #[test]
    fn focal_estimate_rejects_uncertain_or_changing_camera() {
        let mut value = estimate();
        value.samples[1].intrinsics.focal_uncertainty_pixels = 701.0;
        assert!(value
            .resolve()
            .unwrap_err()
            .to_string()
            .contains("uncertainty"));
        value.samples[1].intrinsics.focal_uncertainty_pixels = 35.0;
        value.samples[2].intrinsics.focal_length_pixels = [7000.0, 7000.0];
        assert!(value
            .resolve()
            .unwrap_err()
            .to_string()
            .contains("disagree"));
    }

    #[test]
    fn shared_fit_can_resolve_ambiguity_but_cannot_hide_camera_changes() {
        let mut value = estimate();
        for sample in &mut value.samples {
            sample.intrinsics.focal_uncertainty_pixels = 1000.0;
        }
        let mut shared = intrinsics();
        shared.focal_length_pixels = [4000.0, 4000.0];
        value.shared_intrinsics = Some(shared);
        assert_eq!(value.resolve().unwrap().focal_length_35mm, 40.0);
        value.samples[2].intrinsics.focal_length_pixels = [7000.0, 7000.0];
        assert!(value
            .resolve()
            .unwrap_err()
            .to_string()
            .contains("disagree"));
    }

    #[test]
    fn uncertain_shared_fit_cannot_use_confident_independent_frames_as_fallback() {
        let mut value = estimate();
        let mut shared = intrinsics();
        shared.focal_uncertainty_pixels = 701.0;
        value.shared_intrinsics = Some(shared);
        assert!(value
            .resolve()
            .unwrap_err()
            .to_string()
            .contains("uncertainty"));
    }

    #[test]
    fn focal_estimate_rejects_invalid_geometry_times_and_provenance() {
        for field in 0..7 {
            let mut value = estimate();
            match field {
                0 => value.samples[1].time_seconds = 0.0,
                1 => value.samples[0].intrinsics.focal_length_pixels[0] = f64::NAN,
                2 => value.samples[0].intrinsics.principal_point_pixels[0] = 0.0,
                3 => value.samples[0].intrinsics.image_size[1] = 0,
                4 => value.estimator.weights_sha256.clear(),
                5 => value.schema_version = 2,
                _ => value.samples.truncate(2),
            }
            assert!(value.resolve().is_err(), "field {field}");
        }
    }
}
