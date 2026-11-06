//! Absolute luminance calibration for an explicit offline display target.
//!
//! Relative linear reconstruction has no absolute meaning by itself. This
//! module applies a caller-declared reference white and peak, records the
//! clipping it performed, and encodes PQ codes for that absolute luminance.
//!
//! It does not query a display, select a tone curve, apply the HLG OOTF, or
//! execute dynamic metadata. A PQ code from this module describes a luminance,
//! not a display result.
use crate::dual_layer::encode_pq;
use crate::error::{HdrError, Result};

/// Largest luminance that PQ can represent, in cd/m².
pub const PQ_PEAK_NITS: f64 = 10_000.0;

fn invalid(message: impl Into<String>) -> HdrError {
    HdrError::invalid("display target", message)
}

/// Explicit target parameters. No value is inferred from a device.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbsoluteDisplayTarget {
    /// Luminance that relative linear `1.0` denotes.
    pub reference_white_nits: f64,
    /// Highest luminance the target can show.
    pub peak_nits: f64,
    /// Lowest luminance the target can show.
    pub black_nits: f64,
}

impl AbsoluteDisplayTarget {
    pub fn validate(&self) -> Result<()> {
        if !self.reference_white_nits.is_finite() || self.reference_white_nits <= 0.0 {
            return Err(invalid("the reference white must be finite and positive"));
        }
        if !self.peak_nits.is_finite() || !self.black_nits.is_finite() {
            return Err(invalid("target luminance bounds must be finite"));
        }
        if self.black_nits < 0.0 {
            return Err(invalid("the target black level cannot be negative"));
        }
        if self.black_nits >= self.peak_nits {
            return Err(invalid("the target black level must stay below its peak"));
        }
        if self.peak_nits > PQ_PEAK_NITS {
            return Err(invalid(
                "the target peak exceeds the PQ representable maximum",
            ));
        }
        Ok(())
    }
}

/// What the calibration did to the samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationReport {
    pub sample_count: usize,
    /// Components above the target peak, clamped to it.
    pub clipped_above_peak: usize,
    /// Components below the target black, clamped to it.
    pub clipped_below_black: usize,
}

/// Calibrate relative linear components to absolute luminance.
///
/// A component above the target peak or below its black level is clamped and
/// counted. Nothing else changes the value.
pub fn calibrate_relative_linear(
    samples: &[f32],
    target: &AbsoluteDisplayTarget,
) -> Result<(Vec<f64>, CalibrationReport)> {
    target.validate()?;
    let mut nits = Vec::with_capacity(samples.len());
    let mut clipped_above_peak = 0;
    let mut clipped_below_black = 0;
    for value in samples {
        if !value.is_finite() {
            return Err(invalid("relative linear input must be finite"));
        }
        let absolute = f64::from(*value) * target.reference_white_nits;
        if absolute > target.peak_nits {
            clipped_above_peak += 1;
            nits.push(target.peak_nits);
        } else if absolute < target.black_nits {
            clipped_below_black += 1;
            nits.push(target.black_nits);
        } else {
            nits.push(absolute);
        }
    }
    Ok((
        nits,
        CalibrationReport {
            sample_count: samples.len(),
            clipped_above_peak,
            clipped_below_black,
        },
    ))
}

/// Encode absolute luminance as PQ codes normalized to 10,000 cd/m².
pub fn encode_pq_nits(nits: &[f64]) -> Result<Vec<f64>> {
    let mut codes = Vec::with_capacity(nits.len());
    for value in nits {
        if !value.is_finite() || *value < 0.0 || *value > PQ_PEAK_NITS {
            return Err(invalid("absolute luminance is outside the PQ range"));
        }
        codes.push(encode_pq(value / PQ_PEAK_NITS)?);
    }
    Ok(codes)
}

/// Calibrate relative linear components and encode them as PQ codes.
pub fn relative_linear_to_pq(
    samples: &[f32],
    target: &AbsoluteDisplayTarget,
) -> Result<(Vec<f64>, CalibrationReport)> {
    let (nits, report) = calibrate_relative_linear(samples, target)?;
    Ok((encode_pq_nits(&nits)?, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> AbsoluteDisplayTarget {
        AbsoluteDisplayTarget {
            reference_white_nits: 203.0,
            peak_nits: 1000.0,
            black_nits: 0.0,
        }
    }

    #[test]
    fn a_declared_reference_white_defines_absolute_luminance() {
        let target = AbsoluteDisplayTarget {
            peak_nits: PQ_PEAK_NITS,
            ..target()
        };
        let (nits, report) = calibrate_relative_linear(&[0.0, 1.0, 2.0], &target).unwrap();
        assert_eq!(nits, vec![0.0, 203.0, 406.0]);
        assert_eq!(report.clipped_above_peak, 0);
        assert_eq!(report.clipped_below_black, 0);
        assert_eq!(report.sample_count, 3);
    }

    #[test]
    fn pq_codes_match_fixed_reference_values() {
        // Fixed vectors for absolute luminance, independent of this module.
        for (nits, expected) in [
            (100.0, 0.508_078_421_517_394_9),
            (203.0, 0.580_688_881_041_607_8),
            (1000.0, 0.751_827_096_247_041_8),
            (PQ_PEAK_NITS, 1.0),
        ] {
            let code = encode_pq_nits(&[nits]).unwrap()[0];
            assert!(
                (code - expected).abs() < 1e-12,
                "PQ({nits}) = {code}, expected {expected}"
            );
        }
        assert!(encode_pq_nits(&[PQ_PEAK_NITS + 1.0]).is_err());
        assert!(encode_pq_nits(&[-1.0]).is_err());
    }

    #[test]
    fn clipping_is_recorded_instead_of_hidden() {
        let (nits, report) = calibrate_relative_linear(&[0.0, 1.0, 10.0, -1.0], &target()).unwrap();
        assert_eq!(nits, vec![0.0, 203.0, 1000.0, 0.0]);
        assert_eq!(report.clipped_above_peak, 1);
        assert_eq!(report.clipped_below_black, 1);
        assert_eq!(report.sample_count, 4);
    }

    #[test]
    fn relative_linear_to_pq_combines_both_steps() {
        let (codes, report) = relative_linear_to_pq(&[1.0, 0.0], &target()).unwrap();
        // 203 nits at the fixed reference value, and PQ black for zero.
        assert!((codes[0] - 0.580_688_881_041_607_8).abs() < 1e-12);
        assert!(codes[1] > 0.0 && codes[1] < 1e-6);
        assert_eq!(report.sample_count, 2);
    }

    #[test]
    fn invalid_targets_and_inputs_are_rejected() {
        for target in [
            AbsoluteDisplayTarget {
                reference_white_nits: 0.0,
                ..target()
            },
            AbsoluteDisplayTarget {
                peak_nits: 0.0,
                ..target()
            },
            AbsoluteDisplayTarget {
                peak_nits: PQ_PEAK_NITS + 1.0,
                ..target()
            },
            AbsoluteDisplayTarget {
                black_nits: 1000.0,
                peak_nits: 1000.0,
                ..target()
            },
            AbsoluteDisplayTarget {
                black_nits: -1.0,
                ..target()
            },
        ] {
            assert!(target.validate().is_err(), "{target:?}");
        }
        assert!(calibrate_relative_linear(&[f32::NAN], &target()).is_err());
        assert!(calibrate_relative_linear(&[f32::INFINITY], &target()).is_err());
    }
}
