use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use half::f16;
use sha2::{Digest, Sha256};

pub const APPLE_STYLE_POLYNOMIAL_COUNT: usize = 10;
pub const APPLE_STYLE_CHANNEL_COUNT: usize = 3;
pub const APPLE_STYLE_BLOCK_VALUE_COUNT: usize =
    APPLE_STYLE_POLYNOMIAL_COUNT * APPLE_STYLE_CHANNEL_COUNT;
pub const APPLE_STYLE_GRID_WIDTH: usize = 12;
pub const APPLE_STYLE_GRID_HEIGHT: usize = 9;
pub const APPLE_STYLE_PLANE_COUNT: usize = 8;
pub const APPLE_STYLE_TILE_COUNT: usize =
    APPLE_STYLE_GRID_WIDTH * APPLE_STYLE_GRID_HEIGHT * APPLE_STYLE_PLANE_COUNT;
pub const APPLE_STYLE_VALUE_BYTE_COUNT: usize = 2;
pub const APPLE_STYLE_BYTE_COUNT: usize =
    APPLE_STYLE_BLOCK_VALUE_COUNT * APPLE_STYLE_TILE_COUNT * APPLE_STYLE_VALUE_BYTE_COUNT;
pub const APPLE_STYLE_IDENTITY_SHA256: &str =
    "43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe";
pub const APPLE_STYLE_REFINEMENT_PARAMETER_COUNT: usize = 12;
pub const APPLE_STYLE_REFINEMENT_MAX_PIXELS: usize = 50_000;
pub const APPLE_STYLE_REFINEMENT_EPSILON: f64 = 1.0 / 32.0;
pub const APPLE_STYLE_LIGHT_MAP_SIDE: usize = 32;
pub const APPLE_STYLE_LIGHT_MAP_BYTE_COUNT: usize =
    APPLE_STYLE_LIGHT_MAP_SIDE * APPLE_STYLE_LIGHT_MAP_SIDE * 2;

const IDENTITY_INDICES: [usize; 3] = [3, 7, 11];
const DIRECT_PARAMETER_INDICES: [usize; 6] = [0, 1, 2, 3, 7, 11];
const LINEAR_BOUND: f64 = 1.0 / 8.0;
const QUADRATIC_BOUND: f64 = 1.0 / 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct AppleStyleDataFacts {
    pub value_count: usize,
    pub minimum: f32,
    pub maximum: f32,
    pub identity_residual_rmse: f64,
    pub identity_residual_maximum_absolute: f32,
    pub complete_identity: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppleStyleDataError {
    InvalidLength {
        actual: usize,
        expected: usize,
    },
    InvalidCoefficientCount {
        actual: usize,
        expected: usize,
    },
    InvalidRasterPair,
    InvalidBasisInput,
    NonFiniteCoefficient {
        index: usize,
    },
    CoefficientOutOfBounds {
        index: usize,
        value: f64,
        bound: f64,
    },
    NonFiniteValue {
        index: usize,
    },
    InvalidRgbRaster {
        actual: usize,
    },
    InvalidRefinementParameter {
        actual: usize,
        expected: usize,
    },
    InvalidRefinementStep,
    InvalidScalarRow {
        index: usize,
    },
    InvalidJacobianSampleCount {
        actual: usize,
        expected: usize,
    },
    InvalidSceneScore {
        class: AppleStyleSceneClass,
        value: f64,
    },
    InvalidLightMapInput,
    InvalidLightMapSample {
        index: usize,
    },
    InvalidOrientation(u8),
    NonFiniteLightMapValue {
        index: usize,
    },
    InvalidPropertyListInput,
    PropertyListOverflow,
    SingularSystem,
}

impl fmt::Display for AppleStyleDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles data has {actual} bytes; expected {expected}"
            ),
            Self::InvalidCoefficientCount { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles coefficient vector has {actual} values; expected {expected}"
            ),
            Self::InvalidRasterPair => formatter.write_str(
                "Apple Photographic Styles polynomial fit requires matching finite RGB rasters",
            ),
            Self::InvalidBasisInput => {
                formatter.write_str("Apple Photographic Styles polynomial basis input is not finite")
            }
            Self::NonFiniteCoefficient { index } => write!(
                formatter,
                "Apple Photographic Styles coefficient {index} is not finite"
            ),
            Self::CoefficientOutOfBounds { index, value, bound } => write!(
                formatter,
                "Apple Photographic Styles coefficient {index} value {value} exceeds bound {bound}"
            ),
            Self::NonFiniteValue { index } => write!(
                formatter,
                "Apple Photographic Styles Float16 value {index} is not finite"
            ),
            Self::InvalidRgbRaster { actual } => write!(
                formatter,
                "Apple Photographic Styles RGB raster has {actual} values; expected a non-empty multiple of 3"
            ),
            Self::InvalidRefinementParameter { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles refinement parameter {actual} is outside 0..{expected}"
            ),
            Self::InvalidRefinementStep => formatter.write_str(
                "Apple Photographic Styles refinement Jacobian step must be finite and non-zero",
            ),
            Self::InvalidScalarRow { index } => write!(
                formatter,
                "Apple Photographic Styles scalar constraint row {index} is invalid"
            ),
            Self::InvalidJacobianSampleCount { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles sampled Jacobian has {actual} values; expected {expected}"
            ),
            Self::InvalidSceneScore { class, value } => write!(
                formatter,
                "Apple Photographic Styles {class:?} scene score {value} is outside 0 through 1"
            ),
            Self::InvalidLightMapInput => formatter.write_str(
                "Apple Photographic Styles light-map input dimensions or numeric bounds are invalid",
            ),
            Self::InvalidLightMapSample { index } => write!(
                formatter,
                "Apple Photographic Styles light-map sample {index} is not finite"
            ),
            Self::InvalidOrientation(value) => {
                write!(formatter, "Apple Photographic Styles orientation {value} is invalid")
            }
            Self::NonFiniteLightMapValue { index } => write!(
                formatter,
                "Apple Photographic Styles serialized light-map value {index} is not finite"
            ),
            Self::InvalidPropertyListInput => {
                formatter.write_str("Apple Photographic Styles property-list input is invalid")
            }
            Self::PropertyListOverflow => {
                formatter.write_str("Apple Photographic Styles property-list size overflows")
            }
            Self::SingularSystem => {
                formatter.write_str("Apple Photographic Styles polynomial system is singular")
            }
        }
    }
}

impl Error for AppleStyleDataError {}

/// Build the verified identity key-1 payload used by CMImaging.
pub fn apple_style_identity_data() -> Vec<u8> {
    let mut block =
        Vec::with_capacity(APPLE_STYLE_BLOCK_VALUE_COUNT * APPLE_STYLE_VALUE_BYTE_COUNT);
    for index in 0..APPLE_STYLE_BLOCK_VALUE_COUNT {
        let value = if IDENTITY_INDICES.contains(&index) {
            1.0
        } else {
            0.0
        };
        block.extend_from_slice(&f16::from_f32(value).to_le_bytes());
    }

    let mut result = Vec::with_capacity(APPLE_STYLE_BYTE_COUNT);
    for _ in 0..APPLE_STYLE_TILE_COUNT {
        result.extend_from_slice(&block);
    }
    result
}

/// Return a stable lowercase SHA-256 digest for a style payload.
pub fn apple_style_data_sha256(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Validate the fixed Float16 key-1 layout and report identity residuals.
pub fn validate_apple_style_data(data: &[u8]) -> Result<AppleStyleDataFacts, AppleStyleDataError> {
    if data.len() != APPLE_STYLE_BYTE_COUNT {
        return Err(AppleStyleDataError::InvalidLength {
            actual: data.len(),
            expected: APPLE_STYLE_BYTE_COUNT,
        });
    }

    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    let mut identity_squared_error = 0.0_f64;
    let mut identity_maximum_absolute = 0.0_f32;
    for value_index in 0..(data.len() / APPLE_STYLE_VALUE_BYTE_COUNT) {
        let offset = value_index * APPLE_STYLE_VALUE_BYTE_COUNT;
        let value = f16::from_le_bytes([data[offset], data[offset + 1]]).to_f32();
        if !value.is_finite() {
            return Err(AppleStyleDataError::NonFiniteValue { index: value_index });
        }
        minimum = minimum.min(value);
        maximum = maximum.max(value);
        let expected = if IDENTITY_INDICES.contains(&(value_index % APPLE_STYLE_BLOCK_VALUE_COUNT))
        {
            1.0_f32
        } else {
            0.0_f32
        };
        let error = value - expected;
        identity_squared_error += f64::from(error) * f64::from(error);
        identity_maximum_absolute = identity_maximum_absolute.max(error.abs());
    }

    Ok(AppleStyleDataFacts {
        value_count: data.len() / APPLE_STYLE_VALUE_BYTE_COUNT,
        minimum,
        maximum,
        identity_residual_rmse: (identity_squared_error
            / (data.len() / APPLE_STYLE_VALUE_BYTE_COUNT) as f64)
            .sqrt(),
        identity_residual_maximum_absolute: identity_maximum_absolute,
        complete_identity: data == apple_style_identity_data(),
    })
}

/// Build the polynomial basis used by the portable style solver.
pub fn apple_style_polynomial_basis(
    red: f32,
    green: f32,
    blue: f32,
) -> Result<[f32; APPLE_STYLE_POLYNOMIAL_COUNT], AppleStyleDataError> {
    if !red.is_finite() || !green.is_finite() || !blue.is_finite() {
        return Err(AppleStyleDataError::InvalidBasisInput);
    }
    Ok([
        1.0,
        red,
        green,
        blue,
        red * red,
        red * green,
        red * blue,
        green * green,
        green * blue,
        blue * blue,
    ])
}

/// Fit the bounded global quadratic style transform used to initialize the
/// constrained solver.
///
/// The renderer remains a platform primitive, but this policy is pure numeric
/// work: sample the RGB pair deterministically, run three Huber-weighted IRLS
/// passes, add the same trace-scaled ridge, and clamp each coefficient to the
/// verified key-1 bounds. Keeping it here means a future Apple adapter can
/// provide observations without owning a second solver.
pub fn apple_style_fit_global_polynomial(
    source_rgb8: &[f32],
    target_rgb8: &[f32],
) -> Result<Vec<f64>, AppleStyleDataError> {
    if source_rgb8.len() != target_rgb8.len()
        || source_rgb8.len() < APPLE_STYLE_CHANNEL_COUNT
        || !source_rgb8.len().is_multiple_of(APPLE_STYLE_CHANNEL_COUNT)
        || !source_rgb8.iter().all(|value| value.is_finite())
        || !target_rgb8.iter().all(|value| value.is_finite())
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }

    let pixel_count = source_rgb8.len() / APPLE_STYLE_CHANNEL_COUNT;
    let pixel_stride = (pixel_count / 100_000).max(1);
    let sampled_pixel_count = (pixel_count - 1) / pixel_stride + 1;
    let basis_values = (0..sampled_pixel_count)
        .map(|sampled_pixel| {
            let source_offset = sampled_pixel * pixel_stride * APPLE_STYLE_CHANNEL_COUNT;
            let red = f64::from(source_rgb8[source_offset]) / 255.0;
            let green = f64::from(source_rgb8[source_offset + 1]) / 255.0;
            let blue = f64::from(source_rgb8[source_offset + 2]) / 255.0;
            [
                1.0,
                red,
                green,
                blue,
                red * red,
                red * green,
                red * blue,
                green * green,
                green * blue,
                blue * blue,
            ]
        })
        .collect::<Vec<_>>();

    let mut coefficients = vec![0.0_f64; APPLE_STYLE_BLOCK_VALUE_COUNT];
    for _ in 0..3 {
        for output in 0..APPLE_STYLE_CHANNEL_COUNT {
            let mut normal =
                vec![0.0_f64; APPLE_STYLE_POLYNOMIAL_COUNT * APPLE_STYLE_POLYNOMIAL_COUNT];
            let mut right_hand_side = vec![0.0_f64; APPLE_STYLE_POLYNOMIAL_COUNT];
            for (sampled_pixel, basis) in basis_values.iter().enumerate() {
                let source_offset = sampled_pixel * pixel_stride * APPLE_STYLE_CHANNEL_COUNT;
                let source_value = source_rgb8[source_offset + output];
                let target_value = target_rgb8[source_offset + output];
                let observed = f64::from(target_value - source_value) / 255.0;
                let predicted = basis
                    .iter()
                    .enumerate()
                    .map(|(term, value)| {
                        value * coefficients[term * APPLE_STYLE_CHANNEL_COUNT + output]
                    })
                    .sum::<f64>();
                let residual = observed - predicted;
                let huber_threshold = 4.0_f64 / 255.0;
                let mut weight = (huber_threshold / huber_threshold.max(residual.abs())).min(1.0);
                if source_value <= 2.0 || source_value >= 253.0 {
                    weight *= 0.25;
                }
                for row in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                    let row_value = basis[row];
                    right_hand_side[row] += weight * row_value * observed;
                    for column in row..APPLE_STYLE_POLYNOMIAL_COUNT {
                        normal[row * APPLE_STYLE_POLYNOMIAL_COUNT + column] +=
                            weight * row_value * basis[column];
                    }
                }
            }

            for row in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                for column in 0..row {
                    normal[row * APPLE_STYLE_POLYNOMIAL_COUNT + column] =
                        normal[column * APPLE_STYLE_POLYNOMIAL_COUNT + row];
                }
            }
            let trace = (0..APPLE_STYLE_POLYNOMIAL_COUNT)
                .map(|term| normal[term * APPLE_STYLE_POLYNOMIAL_COUNT + term])
                .sum::<f64>();
            let ridge = (trace / APPLE_STYLE_POLYNOMIAL_COUNT as f64 * 1e-5).max(1e-9);
            for term in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                normal[term * APPLE_STYLE_POLYNOMIAL_COUNT + term] +=
                    ridge * f64::from((term >= 4) as u8 * 9 + 1);
            }
            let solution = solve_linear_system(&normal, &right_hand_side)?;
            for (term, value) in solution.into_iter().enumerate() {
                let coefficient_index = term * APPLE_STYLE_CHANNEL_COUNT + output;
                let bound = coefficient_bound(coefficient_index);
                coefficients[coefficient_index] = value.clamp(-bound, bound);
            }
        }
    }
    Ok(coefficients)
}

/// Parameter-major finite-difference observations consumed by the constrained
/// style refinement solve.
///
/// The Apple renderer is allowed to produce the perturbed rasters, but it does
/// not own the sampling or solve policy. Keeping the sampled storage here also
/// preserves the Swift implementation's bounded-memory contract: at most
/// `APPLE_STYLE_REFINEMENT_MAX_PIXELS` pixels are retained by the Jacobian.
#[derive(Debug, Clone, PartialEq)]
pub struct AppleStyleSampledJacobian {
    rgb_value_count: usize,
    pub pixel_stride: usize,
    pub sample_count: usize,
    values: Vec<f32>,
}

impl AppleStyleSampledJacobian {
    /// Allocate the exact sample grid used by the refinement solver.
    pub fn new(rgb_value_count: usize) -> Result<Self, AppleStyleDataError> {
        if rgb_value_count < 3 || !rgb_value_count.is_multiple_of(3) {
            return Err(AppleStyleDataError::InvalidRgbRaster {
                actual: rgb_value_count,
            });
        }
        let pixel_count = rgb_value_count / 3;
        let pixel_stride = (pixel_count / APPLE_STYLE_REFINEMENT_MAX_PIXELS).max(1);
        let sampled_pixel_count = (pixel_count - 1) / pixel_stride + 1;
        let sample_count = sampled_pixel_count * 3;
        Ok(Self {
            rgb_value_count,
            pixel_stride,
            sample_count,
            values: vec![0.0; sample_count * APPLE_STYLE_REFINEMENT_PARAMETER_COUNT],
        })
    }

    /// Populate one parameter column from a renderer perturbation and return
    /// its sampled derivative RMS in the renderer's native 8-bit domain.
    pub fn populate_parameter(
        &mut self,
        parameter: usize,
        rendered_rgb: &[f32],
        current_rgb: &[f32],
        step: f64,
    ) -> Result<f64, AppleStyleDataError> {
        if parameter >= APPLE_STYLE_REFINEMENT_PARAMETER_COUNT {
            return Err(AppleStyleDataError::InvalidRefinementParameter {
                actual: parameter,
                expected: APPLE_STYLE_REFINEMENT_PARAMETER_COUNT,
            });
        }
        if rendered_rgb.len() != current_rgb.len()
            || rendered_rgb.len() != self.rgb_value_count
            || rendered_rgb.len() < 3
            || !rendered_rgb.len().is_multiple_of(3)
            || !rendered_rgb.iter().all(|value| value.is_finite())
            || !current_rgb.iter().all(|value| value.is_finite())
        {
            return Err(AppleStyleDataError::InvalidRasterPair);
        }
        if !step.is_finite() || step == 0.0 {
            return Err(AppleStyleDataError::InvalidRefinementStep);
        }
        let float_step = step as f32;
        if !float_step.is_finite() || float_step == 0.0 {
            return Err(AppleStyleDataError::InvalidRefinementStep);
        }

        let pixel_count = current_rgb.len() / 3;
        let parameter_base = parameter * self.sample_count;
        let mut sampled = 0;
        let mut squared = 0.0_f64;
        for pixel in (0..pixel_count).step_by(self.pixel_stride) {
            let base = pixel * 3;
            for channel in 0..3 {
                let derivative =
                    (rendered_rgb[base + channel] - current_rgb[base + channel]) / float_step;
                self.values[parameter_base + sampled] = derivative;
                squared += f64::from(derivative) * f64::from(derivative);
                sampled += 1;
            }
        }
        if sampled != self.sample_count {
            return Err(AppleStyleDataError::InvalidJacobianSampleCount {
                actual: sampled,
                expected: self.sample_count,
            });
        }
        Ok((squared / self.sample_count.max(1) as f64).sqrt())
    }

    fn derivative(&self, parameter: usize, sample: usize) -> f32 {
        self.values[parameter * self.sample_count + sample]
    }
}

/// One optional scalar response constraint added to the sampled RGB normal
/// equations. The fixed-size derivative prevents a caller from supplying a
/// second, differently-sized solver parameter vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleScalarRow {
    pub derivative: [f64; APPLE_STYLE_REFINEMENT_PARAMETER_COUNT],
    pub residual: f64,
    pub weight: f64,
}

/// Solve one bounded constrained-polynomial refinement update.
///
/// This is the policy half of the Swift `solveUpdate` implementation. A
/// platform adapter may render the finite-difference candidates and report
/// scalar observations, while Rust owns deterministic sampling, Huber weights,
/// normalization, ridge regularization, linear solving, and update bounds.
pub fn apple_style_solve_refinement_update(
    current_rgb: &[f32],
    target_rgb: &[f32],
    jacobian: &AppleStyleSampledJacobian,
    scalar_rows: &[AppleStyleScalarRow],
) -> Result<Vec<f64>, AppleStyleDataError> {
    if current_rgb.len() != target_rgb.len()
        || current_rgb.len() < 3
        || !current_rgb.len().is_multiple_of(3)
        || !current_rgb.iter().all(|value| value.is_finite())
        || !target_rgb.iter().all(|value| value.is_finite())
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    if jacobian.rgb_value_count != current_rgb.len() {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    let sampled_pixel_count = (current_rgb.len() / 3 - 1) / jacobian.pixel_stride + 1;
    let expected_sample_count = sampled_pixel_count * 3;
    if jacobian.sample_count != expected_sample_count {
        return Err(AppleStyleDataError::InvalidJacobianSampleCount {
            actual: jacobian.sample_count,
            expected: expected_sample_count,
        });
    }
    for (index, row) in scalar_rows.iter().enumerate() {
        if !row.derivative.iter().all(|value| value.is_finite())
            || !row.residual.is_finite()
            || !row.weight.is_finite()
            || row.weight < 0.0
        {
            return Err(AppleStyleDataError::InvalidScalarRow { index });
        }
    }

    let count = APPLE_STYLE_REFINEMENT_PARAMETER_COUNT;
    let mut normal = vec![0.0_f64; count * count];
    let mut gradient = vec![0.0_f64; count];
    let mut sample_count = 0_usize;
    let mut sample_ordinal = 0_usize;
    for pixel in (0..current_rgb.len() / 3).step_by(jacobian.pixel_stride) {
        for channel in 0..3 {
            let sample = pixel * 3 + channel;
            let residual = f64::from(target_rgb[sample] - current_rgb[sample]);
            let huber_weight = (12.0 / 12.0_f64.max(residual.abs())).min(1.0);
            sample_count += 1;
            for row in 0..count {
                let row_value = f64::from(jacobian.derivative(row, sample_ordinal));
                gradient[row] += huber_weight * row_value * residual;
                for column in row..count {
                    normal[row * count + column] += huber_weight
                        * row_value
                        * f64::from(jacobian.derivative(column, sample_ordinal));
                }
            }
            sample_ordinal += 1;
        }
    }
    if sample_ordinal != jacobian.sample_count {
        return Err(AppleStyleDataError::InvalidJacobianSampleCount {
            actual: sample_ordinal,
            expected: jacobian.sample_count,
        });
    }

    // Mean-scale the pixel block so scalar-response rows have
    // sample-count-independent weights. The pure pixel solution is unchanged
    // because the linear system is scale invariant.
    if sample_count > 0 {
        let normalization = 1.0 / sample_count as f64;
        for row in 0..count {
            gradient[row] *= normalization;
            for column in row..count {
                normal[row * count + column] *= normalization;
            }
        }
    }
    for scalar in scalar_rows {
        for row in 0..count {
            gradient[row] += scalar.weight * scalar.derivative[row] * scalar.residual;
            for column in row..count {
                normal[row * count + column] +=
                    scalar.weight * scalar.derivative[row] * scalar.derivative[column];
            }
        }
    }
    for row in 0..count {
        for column in 0..row {
            normal[row * count + column] = normal[column * count + row];
        }
    }
    let trace = (0..count)
        .map(|index| normal[index * count + index])
        .sum::<f64>();
    let ridge = (trace / count as f64 * 1e-6).max(1e-9);
    for index in 0..count {
        normal[index * count + index] += ridge;
    }
    let mut solution = solve_linear_system(&normal, &gradient)?;
    for value in &mut solution {
        *value = value.clamp(
            -APPLE_STYLE_REFINEMENT_EPSILON,
            APPLE_STYLE_REFINEMENT_EPSILON,
        );
    }
    Ok(solution)
}

/// The four public Vision score buckets consumed by Apple's scene-type policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleStyleSceneClass {
    Food,
    Sunset,
    Indoor,
    Outdoor,
}

/// Vision observations passed from a platform primitive to the Rust scene
/// policy. Scores are confidence values and therefore must already be in 0..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleSceneScores {
    pub food: f64,
    pub sunset: f64,
    pub indoor: f64,
    pub outdoor: f64,
}

/// The product-level scene choice used in the style property-list contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleStyleSceneDecision {
    pub scene_type: u8,
    pub selected_class: Option<AppleStyleSceneClass>,
    pub native_default_applied: bool,
}

/// Resolve the current iPhone scene-type priority in Rust.
///
/// Vision classification is a platform observation. The thresholds and
/// priority are product policy and remain portable here: food, sunset, indoor,
/// outdoor, then the native default scene type 0.
pub fn resolve_apple_style_scene_type(
    scores: AppleStyleSceneScores,
) -> Result<AppleStyleSceneDecision, AppleStyleDataError> {
    for (class, value) in [
        (AppleStyleSceneClass::Food, scores.food),
        (AppleStyleSceneClass::Sunset, scores.sunset),
        (AppleStyleSceneClass::Indoor, scores.indoor),
        (AppleStyleSceneClass::Outdoor, scores.outdoor),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(AppleStyleDataError::InvalidSceneScore { class, value });
        }
    }

    let (scene_type, selected_class) = if scores.food >= 0.08 {
        (1, Some(AppleStyleSceneClass::Food))
    } else if scores.sunset >= 0.08 {
        (3, Some(AppleStyleSceneClass::Sunset))
    } else if scores.indoor >= 0.15 {
        (0, Some(AppleStyleSceneClass::Indoor))
    } else if scores.outdoor >= 0.15 {
        (2, Some(AppleStyleSceneClass::Outdoor))
    } else {
        (0, None)
    };
    Ok(AppleStyleSceneDecision {
        scene_type,
        selected_class,
        native_default_applied: selected_class.is_none(),
    })
}

/// Derive the source-dependent face exposure boost used by the style plist.
/// Missing or non-positive medians deliberately resolve to the neutral value;
/// the caller still has to provide a credible person observation.
pub fn apple_style_face_exposure_boost(
    global_median: f64,
    person_median: f64,
    has_credible_person: bool,
) -> Result<f64, AppleStyleDataError> {
    if !global_median.is_finite()
        || !person_median.is_finite()
        || global_median < 0.0
        || person_median < 0.0
    {
        return Err(AppleStyleDataError::InvalidLightMapInput);
    }
    if has_credible_person && global_median > 0.0 && person_median > 0.0 {
        Ok((global_median / person_median).sqrt().clamp(1.0, 2.5))
    } else {
        Ok(1.0)
    }
}

/// Typed source and bounds for one source-derived style light map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleLightMapRequest<'a> {
    pub luma: &'a [f32],
    pub width: usize,
    pub height: usize,
    pub value_scale: f32,
    pub value_offset: f32,
    pub output_minimum: f32,
    pub output_maximum: f32,
    pub storage_orientation: u8,
}

/// Build one source-derived 32x32 Float16 light map in primary-item storage
/// order. The aggregation grid and all eight EXIF orientation mappings match
/// the existing producer contract; non-finite source samples fail closed.
pub fn apple_style_light_map(
    request: &AppleStyleLightMapRequest<'_>,
) -> Result<Vec<u8>, AppleStyleDataError> {
    let expected_count = request
        .width
        .checked_mul(request.height)
        .ok_or(AppleStyleDataError::InvalidLightMapInput)?;
    if request.width == 0
        || request.height == 0
        || request.luma.len() != expected_count
        || !request.value_scale.is_finite()
        || !request.value_offset.is_finite()
        || !request.output_minimum.is_finite()
        || !request.output_maximum.is_finite()
        || request.output_minimum > request.output_maximum
        || !(1..=8).contains(&request.storage_orientation)
    {
        if !(1..=8).contains(&request.storage_orientation) {
            return Err(AppleStyleDataError::InvalidOrientation(
                request.storage_orientation,
            ));
        }
        return Err(AppleStyleDataError::InvalidLightMapInput);
    }
    if let Some(index) = request.luma.iter().position(|value| !value.is_finite()) {
        return Err(AppleStyleDataError::InvalidLightMapSample { index });
    }

    let side = APPLE_STYLE_LIGHT_MAP_SIDE;
    let mut presentation = vec![f16::from_f32(0.0); side * side];
    for target_y in 0..side {
        let y0 = target_y * request.height / side;
        let y1 = (target_y + 1)
            .checked_mul(request.height)
            .ok_or(AppleStyleDataError::InvalidLightMapInput)?
            / side;
        let y1 = y1.max(y0 + 1).min(request.height);
        for target_x in 0..side {
            let x0 = target_x * request.width / side;
            let x1 = (target_x + 1)
                .checked_mul(request.width)
                .ok_or(AppleStyleDataError::InvalidLightMapInput)?
                / side;
            let x1 = x1.max(x0 + 1).min(request.width);
            let mut sum = 0.0_f64;
            let mut count = 0_usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    sum += f64::from(request.luma[y * request.width + x]);
                    count += 1;
                }
            }
            let average = (sum / count as f64) as f32;
            let scaled = (average * request.value_scale + request.value_offset)
                .clamp(request.output_minimum, request.output_maximum);
            let encoded = f16::from_f32(scaled);
            if !encoded.to_f32().is_finite() {
                return Err(AppleStyleDataError::NonFiniteLightMapValue {
                    index: target_y * side + target_x,
                });
            }
            presentation[target_y * side + target_x] = encoded;
        }
    }

    let mut output = Vec::with_capacity(APPLE_STYLE_LIGHT_MAP_BYTE_COUNT);
    for storage_y in 0..side {
        for storage_x in 0..side {
            let (display_x, display_y) = match request.storage_orientation {
                1 => (storage_x, storage_y),
                2 => (side - 1 - storage_x, storage_y),
                3 => (side - 1 - storage_x, side - 1 - storage_y),
                4 => (storage_x, side - 1 - storage_y),
                5 => (storage_y, storage_x),
                6 => (side - 1 - storage_y, storage_x),
                7 => (side - 1 - storage_y, side - 1 - storage_x),
                8 => (storage_y, side - 1 - storage_x),
                _ => unreachable!("orientation was validated above"),
            };
            output.extend_from_slice(&presentation[display_y * side + display_x].to_le_bytes());
        }
    }
    Ok(output)
}

/// The nine-number distribution stored for each style statistics channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleDistribution {
    pub black_point: f64,
    pub high_key: f64,
    pub p02: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p98: f64,
    pub white_point: f64,
}

/// The fixed statistics dictionary consumed by the Apple style property-list
/// contract. An explicit field for every producer channel makes omissions
/// impossible at the call site and keeps the metadata schema out of Swift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleStatistics {
    pub linear_gtc_image: AppleStyleDistribution,
    pub linear_image: AppleStyleDistribution,
    pub linear_image_person_segment_based: AppleStyleDistribution,
    pub linear_image_skin_based: AppleStyleDistribution,
    pub tone_mapped_image: AppleStyleDistribution,
    pub tone_mapped_image_blue_channel_skin_based: AppleStyleDistribution,
    pub tone_mapped_image_green_channel_skin_based: AppleStyleDistribution,
    pub tone_mapped_image_person_segment_based: AppleStyleDistribution,
    pub tone_mapped_image_red_channel_skin_based: AppleStyleDistribution,
    pub tone_mapped_image_skin_based: AppleStyleDistribution,
}

/// Typed input for the version-15 Apple Photographic Styles metadata object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStylePropertyListRequest<'a> {
    pub style_data: &'a [u8],
    pub global_tone_curve: &'a [u8],
    pub baseline_exposure: f64,
    pub scene_type: u8,
    pub statistics: &'a AppleStyleStatistics,
    pub people_ratio: f64,
    pub person_masks_valid_hint: f64,
    pub skin_ratio: f64,
    pub tone_light_map: &'a [u8],
    pub linear_light_map: &'a [u8],
    pub base_gain: f64,
    pub linear_gain: f64,
    pub original_range_min: f64,
    pub original_range_max: f64,
    pub face_exposure_boost: f64,
}

/// Synthesize the binary property list carried by the Styles MIME item.
///
/// `PropertyListSerialization` is a platform primitive for consuming this
/// object, not a product-policy boundary. Rust validates every fixed-size
/// resource, constructs the complete key schema, and emits a deterministic
/// binary plist without delegating metadata synthesis to Swift.
pub fn apple_style_property_list(
    request: &AppleStylePropertyListRequest<'_>,
) -> Result<Vec<u8>, AppleStyleDataError> {
    validate_apple_style_data(request.style_data)?;
    if request.global_tone_curve.len() != 516
        || request.tone_light_map.len() != APPLE_STYLE_LIGHT_MAP_BYTE_COUNT
        || request.linear_light_map.len() != APPLE_STYLE_LIGHT_MAP_BYTE_COUNT
        || request.scene_type > 3
        || !(0.0..=1.0).contains(&request.people_ratio)
        || !(-1.0..=1.0).contains(&request.person_masks_valid_hint)
        || !(0.0..=1.0).contains(&request.skin_ratio)
        || !request.baseline_exposure.is_finite()
        || !request.base_gain.is_finite()
        || !request.linear_gain.is_finite()
        || !request.original_range_min.is_finite()
        || !request.original_range_max.is_finite()
        || request.original_range_min > request.original_range_max
        || !request.face_exposure_boost.is_finite()
    {
        return Err(AppleStyleDataError::InvalidPropertyListInput);
    }
    validate_style_statistics(request.statistics)?;

    let statistics = style_statistics_value(request.statistics);
    let root = plist_dictionary([
        ("0", PlistValue::Integer(15)),
        ("1", PlistValue::Data(request.style_data.to_vec())),
        ("2", PlistValue::Bool(true)),
        ("3", PlistValue::Data(request.global_tone_curve.to_vec())),
        ("4", PlistValue::Real(request.baseline_exposure)),
        ("5", PlistValue::Integer(u64::from(request.scene_type))),
        ("6", statistics),
        (
            "7",
            plist_dictionary([
                ("PeopleRatio", PlistValue::Real(request.people_ratio)),
                (
                    "PersonMasksValidHint",
                    PlistValue::Real(request.person_masks_valid_hint),
                ),
                ("SkinRatio", PlistValue::Real(request.skin_ratio)),
            ]),
        ),
        ("c", PlistValue::Data(request.tone_light_map.to_vec())),
        ("d", PlistValue::Data(request.linear_light_map.to_vec())),
        ("e", PlistValue::Integer(32)),
        ("f", PlistValue::Integer(32)),
        ("g", PlistValue::Integer(0x4C30_3068)),
        ("h", PlistValue::Real(request.base_gain)),
        (
            "i",
            plist_dictionary([
                ("Gain", PlistValue::Real(request.linear_gain)),
                (
                    "OriginalRangeMin",
                    PlistValue::Real(request.original_range_min),
                ),
                (
                    "OriginalRangeMax",
                    PlistValue::Real(request.original_range_max),
                ),
            ]),
        ),
        ("j", PlistValue::Real(request.face_exposure_boost)),
        ("k", PlistValue::Bool(false)),
    ]);
    serialize_binary_plist(root)
}

fn validate_style_statistics(statistics: &AppleStyleStatistics) -> Result<(), AppleStyleDataError> {
    let distributions = [
        statistics.linear_gtc_image,
        statistics.linear_image,
        statistics.linear_image_person_segment_based,
        statistics.linear_image_skin_based,
        statistics.tone_mapped_image,
        statistics.tone_mapped_image_blue_channel_skin_based,
        statistics.tone_mapped_image_green_channel_skin_based,
        statistics.tone_mapped_image_person_segment_based,
        statistics.tone_mapped_image_red_channel_skin_based,
        statistics.tone_mapped_image_skin_based,
    ];
    for distribution in distributions {
        let values = [
            distribution.black_point,
            distribution.high_key,
            distribution.p02,
            distribution.p10,
            distribution.p25,
            distribution.p50,
            distribution.p75,
            distribution.p98,
            distribution.white_point,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(AppleStyleDataError::InvalidPropertyListInput);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum PlistValue {
    Bool(bool),
    Integer(u64),
    Real(f64),
    String(String),
    Data(Vec<u8>),
    Dictionary(BTreeMap<String, PlistValue>),
}

fn plist_dictionary<const N: usize>(entries: [(&str, PlistValue); N]) -> PlistValue {
    PlistValue::Dictionary(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn distribution_value(distribution: AppleStyleDistribution) -> PlistValue {
    plist_dictionary([
        ("blackPoint", PlistValue::Real(distribution.black_point)),
        ("highKey", PlistValue::Real(distribution.high_key)),
        ("p02", PlistValue::Real(distribution.p02)),
        ("p10", PlistValue::Real(distribution.p10)),
        ("p25", PlistValue::Real(distribution.p25)),
        ("p50", PlistValue::Real(distribution.p50)),
        ("p75", PlistValue::Real(distribution.p75)),
        ("p98", PlistValue::Real(distribution.p98)),
        ("whitePoint", PlistValue::Real(distribution.white_point)),
    ])
}

fn style_statistics_value(statistics: &AppleStyleStatistics) -> PlistValue {
    plist_dictionary([
        (
            "LinearGTCImage",
            distribution_value(statistics.linear_gtc_image),
        ),
        ("LinearImage", distribution_value(statistics.linear_image)),
        (
            "LinearImagePersonSegmentBased",
            distribution_value(statistics.linear_image_person_segment_based),
        ),
        (
            "LinearImageSkinBased",
            distribution_value(statistics.linear_image_skin_based),
        ),
        (
            "ToneMappedImage",
            distribution_value(statistics.tone_mapped_image),
        ),
        (
            "ToneMappedImageBlueChannelSkinBased",
            distribution_value(statistics.tone_mapped_image_blue_channel_skin_based),
        ),
        (
            "ToneMappedImageGreenChannelSkinBased",
            distribution_value(statistics.tone_mapped_image_green_channel_skin_based),
        ),
        (
            "ToneMappedImagePersonSegmentBased",
            distribution_value(statistics.tone_mapped_image_person_segment_based),
        ),
        (
            "ToneMappedImageRedChannelSkinBased",
            distribution_value(statistics.tone_mapped_image_red_channel_skin_based),
        ),
        (
            "ToneMappedImageSkinBased",
            distribution_value(statistics.tone_mapped_image_skin_based),
        ),
    ])
}

#[derive(Debug, Clone, PartialEq)]
enum PlistObject {
    Bool(bool),
    Integer(u64),
    Real(f64),
    String(String),
    Data(Vec<u8>),
    Dictionary(Vec<(usize, usize)>),
}

fn serialize_binary_plist(root: PlistValue) -> Result<Vec<u8>, AppleStyleDataError> {
    let mut objects = Vec::new();
    let top_object = collect_plist_object(root, &mut objects)?;
    let object_ref_size = integer_width(
        u64::try_from(objects.len().saturating_sub(1))
            .map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
    );
    let mut output = b"bplist00".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for object in &objects {
        offsets.push(
            u64::try_from(output.len()).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
        );
        encode_plist_object(object, object_ref_size, &mut output)?;
    }
    let offset_table_offset =
        u64::try_from(output.len()).map_err(|_| AppleStyleDataError::PropertyListOverflow)?;
    let largest_offset = offset_table_offset.max(offsets.last().copied().unwrap_or(0));
    let offset_int_size = integer_width(largest_offset);
    for offset in offsets {
        write_fixed_width(&mut output, offset, offset_int_size);
    }

    output.extend_from_slice(&[0; 6]);
    output.push(offset_int_size as u8);
    output.push(object_ref_size as u8);
    output.extend_from_slice(
        &u64::try_from(objects.len())
            .map_err(|_| AppleStyleDataError::PropertyListOverflow)?
            .to_be_bytes(),
    );
    output.extend_from_slice(
        &u64::try_from(top_object)
            .map_err(|_| AppleStyleDataError::PropertyListOverflow)?
            .to_be_bytes(),
    );
    output.extend_from_slice(&offset_table_offset.to_be_bytes());
    Ok(output)
}

fn collect_plist_object(
    value: PlistValue,
    objects: &mut Vec<PlistObject>,
) -> Result<usize, AppleStyleDataError> {
    let object = match value {
        PlistValue::Bool(value) => PlistObject::Bool(value),
        PlistValue::Integer(value) => PlistObject::Integer(value),
        PlistValue::Real(value) => PlistObject::Real(value),
        PlistValue::String(value) => PlistObject::String(value),
        PlistValue::Data(value) => PlistObject::Data(value),
        PlistValue::Dictionary(entries) => {
            let mut references = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let key_index = collect_plist_object(PlistValue::String(key), objects)?;
                let value_index = collect_plist_object(value, objects)?;
                references.push((key_index, value_index));
            }
            PlistObject::Dictionary(references)
        }
    };
    let index = objects.len();
    objects.push(object);
    Ok(index)
}

fn encode_plist_object(
    object: &PlistObject,
    object_ref_size: usize,
    output: &mut Vec<u8>,
) -> Result<(), AppleStyleDataError> {
    match object {
        PlistObject::Bool(value) => output.push(if *value { 0x09 } else { 0x08 }),
        PlistObject::Integer(value) => encode_integer_object(*value, output),
        PlistObject::Real(value) => {
            output.push(0x23);
            output.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        PlistObject::String(value) => encode_string_object(value, output)?,
        PlistObject::Data(value) => {
            encode_count_marker(0x40, value.len(), output)?;
            output.extend_from_slice(value);
        }
        PlistObject::Dictionary(references) => {
            encode_count_marker(0xD0, references.len(), output)?;
            for (key, _) in references {
                write_fixed_width(
                    output,
                    u64::try_from(*key).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
                    object_ref_size,
                );
            }
            for (_, value) in references {
                write_fixed_width(
                    output,
                    u64::try_from(*value).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
                    object_ref_size,
                );
            }
        }
    }
    Ok(())
}

fn encode_integer_object(value: u64, output: &mut Vec<u8>) {
    let width = integer_width(value);
    output.push(0x10 | width.trailing_zeros() as u8);
    write_fixed_width(output, value, width);
}

fn encode_string_object(value: &str, output: &mut Vec<u8>) -> Result<(), AppleStyleDataError> {
    if value.is_ascii() {
        encode_count_marker(0x50, value.len(), output)?;
        output.extend_from_slice(value.as_bytes());
    } else {
        let utf16 = value.encode_utf16().collect::<Vec<_>>();
        encode_count_marker(0x60, utf16.len(), output)?;
        for unit in utf16 {
            output.extend_from_slice(&unit.to_be_bytes());
        }
    }
    Ok(())
}

fn encode_count_marker(
    marker: u8,
    count: usize,
    output: &mut Vec<u8>,
) -> Result<(), AppleStyleDataError> {
    if count < 15 {
        output.push(marker | count as u8);
    } else {
        output.push(marker | 0x0f);
        encode_integer_object(
            u64::try_from(count).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
            output,
        );
    }
    Ok(())
}

fn integer_width(value: u64) -> usize {
    if value <= u64::from(u8::MAX) {
        1
    } else if value <= u64::from(u16::MAX) {
        2
    } else if value <= u64::from(u32::MAX) {
        4
    } else {
        8
    }
}

fn write_fixed_width(output: &mut Vec<u8>, value: u64, width: usize) {
    output.extend_from_slice(&value.to_be_bytes()[8 - width..]);
}

/// Serialize bounded coefficient deltas over the verified identity payload.
pub fn apple_style_data_from_coefficient_deltas(
    coefficient_deltas: &[f64],
) -> Result<Vec<u8>, AppleStyleDataError> {
    validate_coefficient_deltas(coefficient_deltas)?;
    let mut block = [0.0_f32; APPLE_STYLE_BLOCK_VALUE_COUNT];
    for index in IDENTITY_INDICES {
        block[index] = 1.0;
    }
    for (index, delta) in coefficient_deltas.iter().copied().enumerate() {
        block[index] += delta as f32;
    }

    let mut result = Vec::with_capacity(APPLE_STYLE_BYTE_COUNT);
    for _ in 0..APPLE_STYLE_TILE_COUNT {
        for value in block {
            result.extend_from_slice(&f16::from_f32(value).to_le_bytes());
        }
    }
    validate_apple_style_data(&result)?;
    Ok(result)
}

/// Serialize the six direct editor parameters into the full coefficient block.
pub fn apple_style_data_from_parameters(
    parameters: &[f64],
) -> Result<Vec<u8>, AppleStyleDataError> {
    if parameters.len() != DIRECT_PARAMETER_INDICES.len() {
        return Err(AppleStyleDataError::InvalidCoefficientCount {
            actual: parameters.len(),
            expected: DIRECT_PARAMETER_INDICES.len(),
        });
    }
    let mut deltas = [0.0_f64; APPLE_STYLE_BLOCK_VALUE_COUNT];
    for (parameter_index, coefficient_index) in DIRECT_PARAMETER_INDICES.iter().copied().enumerate()
    {
        deltas[coefficient_index] = parameters[parameter_index];
    }
    apple_style_data_from_coefficient_deltas(&deltas)
}

/// Add bounded coefficient deltas to an existing Float16 key-1 payload.
pub fn apple_style_data_apply_coefficient_deltas(
    base: &[u8],
    coefficient_deltas: &[f64],
) -> Result<Vec<u8>, AppleStyleDataError> {
    validate_apple_style_data(base)?;
    validate_coefficient_deltas(coefficient_deltas)?;

    let mut result = Vec::with_capacity(base.len());
    for value_index in 0..(base.len() / APPLE_STYLE_VALUE_BYTE_COUNT) {
        let offset = value_index * APPLE_STYLE_VALUE_BYTE_COUNT;
        let base_value = f16::from_le_bytes([base[offset], base[offset + 1]]).to_f32();
        let value =
            base_value + coefficient_deltas[value_index % APPLE_STYLE_BLOCK_VALUE_COUNT] as f32;
        result.extend_from_slice(&f16::from_f32(value).to_le_bytes());
    }
    validate_apple_style_data(&result)?;
    Ok(result)
}

fn validate_coefficient_deltas(values: &[f64]) -> Result<(), AppleStyleDataError> {
    if values.len() != APPLE_STYLE_BLOCK_VALUE_COUNT {
        return Err(AppleStyleDataError::InvalidCoefficientCount {
            actual: values.len(),
            expected: APPLE_STYLE_BLOCK_VALUE_COUNT,
        });
    }
    for (index, value) in values.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(AppleStyleDataError::NonFiniteCoefficient { index });
        }
        let bound = coefficient_bound(index);
        if value.abs() > bound + 1e-9 {
            return Err(AppleStyleDataError::CoefficientOutOfBounds {
                index,
                value,
                bound,
            });
        }
    }
    Ok(())
}

fn coefficient_bound(index: usize) -> f64 {
    if index / APPLE_STYLE_CHANNEL_COUNT >= 4 {
        QUADRATIC_BOUND
    } else {
        LINEAR_BOUND
    }
}

fn solve_linear_system(matrix: &[f64], vector: &[f64]) -> Result<Vec<f64>, AppleStyleDataError> {
    let count = vector.len();
    if matrix.len() != count * count {
        return Err(AppleStyleDataError::SingularSystem);
    }
    let mut augmented = (0..count)
        .map(|row| {
            let mut values = matrix[row * count..(row + 1) * count].to_vec();
            values.push(vector[row]);
            values
        })
        .collect::<Vec<_>>();
    for pivot in 0..count {
        let best = (pivot..count)
            .max_by(|left, right| {
                augmented[*left][pivot]
                    .abs()
                    .total_cmp(&augmented[*right][pivot].abs())
            })
            .ok_or(AppleStyleDataError::SingularSystem)?;
        if augmented[best][pivot].abs() <= 1e-12 {
            return Err(AppleStyleDataError::SingularSystem);
        }
        if best != pivot {
            augmented.swap(best, pivot);
        }
        let divisor = augmented[pivot][pivot];
        for value in augmented[pivot][pivot..=count].iter_mut() {
            *value /= divisor;
        }
        let pivot_values = augmented[pivot].clone();
        let (before, pivot_and_after) = augmented.split_at_mut(pivot);
        let (_, after) = pivot_and_after
            .split_first_mut()
            .ok_or(AppleStyleDataError::SingularSystem)?;
        for row_values in before.iter_mut().chain(after.iter_mut()) {
            let factor = row_values[pivot];
            for (value, pivot_value) in row_values[pivot..=count]
                .iter_mut()
                .zip(pivot_values[pivot..=count].iter())
            {
                *value -= factor * pivot_value;
            }
        }
    }
    Ok(augmented.into_iter().map(|row| row[count]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_layout_matches_verified_dimensions_and_values() {
        let data = apple_style_identity_data();
        assert_eq!(data.len(), APPLE_STYLE_BYTE_COUNT);
        let facts = validate_apple_style_data(&data).unwrap();
        assert_eq!(facts.value_count, APPLE_STYLE_BYTE_COUNT / 2);
        assert_eq!(facts.minimum, 0.0);
        assert_eq!(facts.maximum, 1.0);
        assert_eq!(facts.identity_residual_rmse, 0.0);
        assert_eq!(facts.identity_residual_maximum_absolute, 0.0);
        assert!(facts.complete_identity);
        assert_eq!(apple_style_data_sha256(&data), APPLE_STYLE_IDENTITY_SHA256);
        for tile in data.chunks_exact(APPLE_STYLE_BLOCK_VALUE_COUNT * 2) {
            for index in 0..APPLE_STYLE_BLOCK_VALUE_COUNT {
                let offset = index * 2;
                let value = f16::from_le_bytes([tile[offset], tile[offset + 1]]).to_f32();
                let expected = if IDENTITY_INDICES.contains(&index) {
                    1.0
                } else {
                    0.0
                };
                assert_eq!(value, expected);
            }
        }
    }

    #[test]
    fn polynomial_basis_matches_swift_contract() {
        assert_eq!(
            apple_style_polynomial_basis(2.0, 3.0, 5.0).unwrap(),
            [1.0, 2.0, 3.0, 5.0, 4.0, 6.0, 10.0, 9.0, 15.0, 25.0]
        );
    }

    #[test]
    fn direct_parameters_change_only_the_six_owned_coefficients() {
        let data =
            apple_style_data_from_parameters(&[0.01, -0.02, 0.03, 0.04, -0.05, 0.06]).unwrap();
        let facts = validate_apple_style_data(&data).unwrap();
        assert!(!facts.complete_identity);
        let first = data.chunks_exact(2).take(APPLE_STYLE_BLOCK_VALUE_COUNT);
        let values = first
            .map(|bytes| f16::from_le_bytes([bytes[0], bytes[1]]).to_f32())
            .collect::<Vec<_>>();
        assert_eq!(values[0], f16::from_f32(0.01).to_f32());
        assert_eq!(values[1], f16::from_f32(-0.02).to_f32());
        assert_eq!(values[2], f16::from_f32(0.03).to_f32());
        assert_eq!(values[3], f16::from_f32(1.04).to_f32());
        assert_eq!(values[7], f16::from_f32(0.95).to_f32());
        assert_eq!(values[11], f16::from_f32(1.06).to_f32());
        for index in [4, 5, 6, 8, 9, 10] {
            assert_eq!(values[index], 0.0);
        }
    }

    #[test]
    fn coefficient_bounds_and_malformed_payloads_fail_closed() {
        let mut deltas = vec![0.0; APPLE_STYLE_BLOCK_VALUE_COUNT];
        deltas[0] = 0.125 + 1e-6;
        assert!(matches!(
            apple_style_data_from_coefficient_deltas(&deltas),
            Err(AppleStyleDataError::CoefficientOutOfBounds { index: 0, .. })
        ));
        deltas[0] = f64::NAN;
        assert!(matches!(
            apple_style_data_from_coefficient_deltas(&deltas),
            Err(AppleStyleDataError::NonFiniteCoefficient { index: 0 })
        ));
        assert!(matches!(
            validate_apple_style_data(&[0; APPLE_STYLE_BYTE_COUNT - 2]),
            Err(AppleStyleDataError::InvalidLength { .. })
        ));
        assert!(matches!(
            apple_style_data_apply_coefficient_deltas(&apple_style_identity_data(), &[0.0; 11]),
            Err(AppleStyleDataError::InvalidCoefficientCount { .. })
        ));
        assert!(matches!(
            apple_style_fit_global_polynomial(&[], &[]),
            Err(AppleStyleDataError::InvalidRasterPair)
        ));
        assert!(matches!(
            apple_style_fit_global_polynomial(&[0.0, 0.0, 0.0], &[f32::NAN, 0.0, 0.0]),
            Err(AppleStyleDataError::InvalidRasterPair)
        ));
    }

    #[test]
    fn existing_payload_deltas_are_applied_per_repeated_block() {
        let base = apple_style_identity_data();
        let mut deltas = vec![0.0; APPLE_STYLE_BLOCK_VALUE_COUNT];
        deltas[3] = 0.01;
        let output = apple_style_data_apply_coefficient_deltas(&base, &deltas).unwrap();
        let block_size = APPLE_STYLE_BLOCK_VALUE_COUNT * 2;
        let first = &output[..block_size];
        let last = &output[output.len() - block_size..];
        assert_eq!(first, last);
        let value = f16::from_le_bytes([first[6], first[7]]).to_f32();
        assert_eq!(value, f16::from_f32(1.01).to_f32());
    }

    #[test]
    fn global_polynomial_fit_recovers_a_bounded_known_transform() {
        let mut expected = vec![0.0_f64; APPLE_STYLE_BLOCK_VALUE_COUNT];
        for output in 0..APPLE_STYLE_CHANNEL_COUNT {
            expected[output] = [0.010, -0.008, 0.006][output];
            expected[APPLE_STYLE_CHANNEL_COUNT + output] = [0.008, -0.005, 0.004][output];
            expected[2 * APPLE_STYLE_CHANNEL_COUNT + output] = [-0.006, 0.007, -0.003][output];
            expected[3 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.005, 0.004, -0.007][output];
            expected[4 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.002, -0.001, 0.001][output];
            expected[5 * APPLE_STYLE_CHANNEL_COUNT + output] = [-0.001, 0.002, -0.001][output];
            expected[6 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.001, -0.001, 0.002][output];
            expected[7 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.002, 0.001, -0.002][output];
            expected[8 * APPLE_STYLE_CHANNEL_COUNT + output] = [-0.002, 0.001, 0.001][output];
            expected[9 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.001, -0.002, 0.001][output];
        }

        let mut source = Vec::with_capacity(512 * APPLE_STYLE_CHANNEL_COUNT);
        let mut target = Vec::with_capacity(512 * APPLE_STYLE_CHANNEL_COUNT);
        for pixel in 0..512 {
            let source_pixel = [
                16.0 + (pixel * 37 % 220) as f32,
                17.0 + (pixel * 53 % 219) as f32,
                18.0 + (pixel * 71 % 218) as f32,
            ];
            source.extend_from_slice(&source_pixel);
            let basis = apple_style_polynomial_basis(
                source_pixel[0] / 255.0,
                source_pixel[1] / 255.0,
                source_pixel[2] / 255.0,
            )
            .unwrap();
            for output in 0..APPLE_STYLE_CHANNEL_COUNT {
                let delta = basis
                    .iter()
                    .enumerate()
                    .map(|(term, value)| {
                        f64::from(*value) * expected[term * APPLE_STYLE_CHANNEL_COUNT + output]
                    })
                    .sum::<f64>();
                target.push(source_pixel[output] + (delta * 255.0) as f32);
            }
        }

        let fitted = apple_style_fit_global_polynomial(&source, &target).unwrap();
        assert_eq!(fitted.len(), APPLE_STYLE_BLOCK_VALUE_COUNT);
        for (index, value) in fitted.iter().copied().enumerate() {
            assert!(value.is_finite());
            assert!(value.abs() <= coefficient_bound(index) + 1e-9);
            assert!(
                (value - expected[index]).abs() < 2e-3,
                "coefficient {index}"
            );
        }
        assert_eq!(
            fitted,
            apple_style_fit_global_polynomial(&source, &target).unwrap()
        );
    }

    #[test]
    fn sampled_jacobian_recovers_bounded_refinement_update() {
        let current = vec![128.0_f32; 12];
        let mut jacobian = AppleStyleSampledJacobian::new(current.len()).unwrap();
        let expected = (0..APPLE_STYLE_REFINEMENT_PARAMETER_COUNT)
            .map(|index| 0.001 * (index + 1) as f64)
            .collect::<Vec<_>>();

        for parameter in 0..APPLE_STYLE_REFINEMENT_PARAMETER_COUNT {
            let mut rendered = current.clone();
            rendered[parameter] += 1.0;
            let derivative_rms = jacobian
                .populate_parameter(parameter, &rendered, &current, 1.0)
                .unwrap();
            assert!((derivative_rms - (1.0 / 12.0_f64).sqrt()).abs() < 1e-6);
        }

        let target = current
            .iter()
            .enumerate()
            .map(|(index, value)| *value + expected[index] as f32)
            .collect::<Vec<_>>();
        let fitted =
            apple_style_solve_refinement_update(&[128.0; 12], &target, &jacobian, &[]).unwrap();
        assert_eq!(fitted.len(), APPLE_STYLE_REFINEMENT_PARAMETER_COUNT);
        for (actual, expected) in fitted.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn refinement_scalar_rows_are_typed_and_updates_are_clamped() {
        let current = vec![128.0_f32; 12];
        let mut jacobian = AppleStyleSampledJacobian::new(current.len()).unwrap();
        for parameter in 0..APPLE_STYLE_REFINEMENT_PARAMETER_COUNT {
            let mut rendered = current.clone();
            rendered[parameter] += 1.0;
            jacobian
                .populate_parameter(parameter, &rendered, &current, 1.0)
                .unwrap();
        }
        let mut derivative = [0.0_f64; APPLE_STYLE_REFINEMENT_PARAMETER_COUNT];
        derivative[0] = 1.0;
        let row = AppleStyleScalarRow {
            derivative,
            residual: 1.0,
            weight: 1.0,
        };
        let update =
            apple_style_solve_refinement_update(&current, &current, &jacobian, &[row]).unwrap();
        assert_eq!(update[0], APPLE_STYLE_REFINEMENT_EPSILON);
        assert!(update[1..].iter().all(|value| value.abs() < 1e-6));
    }

    #[test]
    fn refinement_inputs_fail_closed() {
        assert!(matches!(
            AppleStyleSampledJacobian::new(2),
            Err(AppleStyleDataError::InvalidRgbRaster { actual: 2 })
        ));
        let current = vec![128.0_f32; 12];
        let mut jacobian = AppleStyleSampledJacobian::new(current.len()).unwrap();
        assert!(matches!(
            jacobian.populate_parameter(
                APPLE_STYLE_REFINEMENT_PARAMETER_COUNT,
                &current,
                &current,
                1.0,
            ),
            Err(AppleStyleDataError::InvalidRefinementParameter { .. })
        ));
        assert!(matches!(
            jacobian.populate_parameter(0, &current, &current, 0.0),
            Err(AppleStyleDataError::InvalidRefinementStep)
        ));
        let mut derivative = [0.0_f64; APPLE_STYLE_REFINEMENT_PARAMETER_COUNT];
        derivative[0] = f64::NAN;
        let invalid_row = AppleStyleScalarRow {
            derivative,
            residual: 0.0,
            weight: 1.0,
        };
        assert!(matches!(
            apple_style_solve_refinement_update(&current, &current, &jacobian, &[invalid_row]),
            Err(AppleStyleDataError::InvalidScalarRow { index: 0 })
        ));
    }

    #[test]
    fn scene_policy_preserves_priority_and_native_default() {
        let food = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.08,
            sunset: 1.0,
            indoor: 1.0,
            outdoor: 1.0,
        })
        .unwrap();
        assert_eq!(food.scene_type, 1);
        assert_eq!(food.selected_class, Some(AppleStyleSceneClass::Food));
        assert!(!food.native_default_applied);

        let sunset = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.0,
            sunset: 0.08,
            indoor: 1.0,
            outdoor: 1.0,
        })
        .unwrap();
        assert_eq!(sunset.scene_type, 3);
        assert_eq!(sunset.selected_class, Some(AppleStyleSceneClass::Sunset));

        let indoor = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.0,
            sunset: 0.0,
            indoor: 0.15,
            outdoor: 1.0,
        })
        .unwrap();
        assert_eq!(indoor.scene_type, 0);
        assert_eq!(indoor.selected_class, Some(AppleStyleSceneClass::Indoor));
        assert!(!indoor.native_default_applied);

        let outdoor = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.0,
            sunset: 0.0,
            indoor: 0.0,
            outdoor: 0.15,
        })
        .unwrap();
        assert_eq!(outdoor.scene_type, 2);
        assert_eq!(outdoor.selected_class, Some(AppleStyleSceneClass::Outdoor));

        let default_scene = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.079,
            sunset: 0.079,
            indoor: 0.149,
            outdoor: 0.149,
        })
        .unwrap();
        assert_eq!(default_scene.scene_type, 0);
        assert_eq!(default_scene.selected_class, None);
        assert!(default_scene.native_default_applied);
        assert!(matches!(
            resolve_apple_style_scene_type(AppleStyleSceneScores {
                food: f64::NAN,
                sunset: 0.0,
                indoor: 0.0,
                outdoor: 0.0,
            }),
            Err(AppleStyleDataError::InvalidSceneScore {
                class: AppleStyleSceneClass::Food,
                ..
            })
        ));
    }

    #[test]
    fn face_exposure_boost_matches_swift_formula_and_neutral_fallback() {
        assert_eq!(
            apple_style_face_exposure_boost(4.0, 1.0, true).unwrap(),
            2.0
        );
        assert_eq!(
            apple_style_face_exposure_boost(4.0, 1.0, false).unwrap(),
            1.0
        );
        assert_eq!(
            apple_style_face_exposure_boost(0.0, 0.0, true).unwrap(),
            1.0
        );
        assert!(matches!(
            apple_style_face_exposure_boost(-1.0, 1.0, true),
            Err(AppleStyleDataError::InvalidLightMapInput)
        ));
    }

    #[test]
    fn light_map_storage_orientation_matches_all_eight_exif_mappings() {
        let side = APPLE_STYLE_LIGHT_MAP_SIDE;
        let luma = (0..side * side)
            .map(|value| value as f32)
            .collect::<Vec<_>>();
        let expected_corners = [
            [0.0, 31.0, 992.0, 1023.0],
            [31.0, 0.0, 1023.0, 992.0],
            [1023.0, 992.0, 31.0, 0.0],
            [992.0, 1023.0, 0.0, 31.0],
            [0.0, 992.0, 31.0, 1023.0],
            [31.0, 1023.0, 0.0, 992.0],
            [1023.0, 31.0, 992.0, 0.0],
            [992.0, 0.0, 1023.0, 31.0],
        ];
        for orientation in 1..=8 {
            let request = AppleStyleLightMapRequest {
                luma: &luma,
                width: side,
                height: side,
                value_scale: 1.0,
                value_offset: 0.0,
                output_minimum: 0.0,
                output_maximum: 2_000.0,
                storage_orientation: orientation,
            };
            let data = apple_style_light_map(&request).unwrap();
            assert_eq!(data.len(), APPLE_STYLE_LIGHT_MAP_BYTE_COUNT);
            let corners = [(0, 0), (side - 1, 0), (0, side - 1), (side - 1, side - 1)];
            for (corner_index, (x, y)) in corners.into_iter().enumerate() {
                let offset = (y * side + x) * 2;
                let value = f16::from_le_bytes([data[offset], data[offset + 1]]).to_f32();
                assert_eq!(
                    value,
                    expected_corners[usize::from(orientation - 1)][corner_index]
                );
            }
        }
    }

    #[test]
    fn light_map_rejects_invalid_geometry_orientation_and_samples() {
        let empty = AppleStyleLightMapRequest {
            luma: &[],
            width: 0,
            height: 0,
            value_scale: 1.0,
            value_offset: 0.0,
            output_minimum: 0.0,
            output_maximum: 1.0,
            storage_orientation: 1,
        };
        assert!(matches!(
            apple_style_light_map(&empty),
            Err(AppleStyleDataError::InvalidLightMapInput)
        ));
        let invalid_orientation = AppleStyleLightMapRequest {
            luma: &[0.0],
            width: 1,
            height: 1,
            value_scale: 1.0,
            value_offset: 0.0,
            output_minimum: 0.0,
            output_maximum: 1.0,
            storage_orientation: 9,
        };
        assert!(matches!(
            apple_style_light_map(&invalid_orientation),
            Err(AppleStyleDataError::InvalidOrientation(9))
        ));
        let invalid_sample = AppleStyleLightMapRequest {
            luma: &[f32::NAN],
            width: 1,
            height: 1,
            value_scale: 1.0,
            value_offset: 0.0,
            output_minimum: 0.0,
            output_maximum: 1.0,
            storage_orientation: 1,
        };
        assert!(matches!(
            apple_style_light_map(&invalid_sample),
            Err(AppleStyleDataError::InvalidLightMapSample { index: 0 })
        ));
    }

    #[test]
    fn property_list_serialization_emits_complete_deterministic_contract() {
        let distribution = AppleStyleDistribution {
            black_point: 0.01,
            high_key: 0.99,
            p02: 0.02,
            p10: 0.10,
            p25: 0.25,
            p50: 0.50,
            p75: 0.75,
            p98: 0.98,
            white_point: 1.0,
        };
        let statistics = AppleStyleStatistics {
            linear_gtc_image: distribution,
            linear_image: distribution,
            linear_image_person_segment_based: distribution,
            linear_image_skin_based: distribution,
            tone_mapped_image: distribution,
            tone_mapped_image_blue_channel_skin_based: distribution,
            tone_mapped_image_green_channel_skin_based: distribution,
            tone_mapped_image_person_segment_based: distribution,
            tone_mapped_image_red_channel_skin_based: distribution,
            tone_mapped_image_skin_based: distribution,
        };
        let global_tone_curve = vec![0x34_u8; 516];
        let tone_light_map = vec![0x12_u8; APPLE_STYLE_LIGHT_MAP_BYTE_COUNT];
        let linear_light_map = vec![0x56_u8; APPLE_STYLE_LIGHT_MAP_BYTE_COUNT];
        let request = AppleStylePropertyListRequest {
            style_data: &apple_style_identity_data(),
            global_tone_curve: &global_tone_curve,
            baseline_exposure: 4.0,
            scene_type: 2,
            statistics: &statistics,
            people_ratio: 0.25,
            person_masks_valid_hint: -1.0,
            skin_ratio: 0.10,
            tone_light_map: &tone_light_map,
            linear_light_map: &linear_light_map,
            base_gain: 1.25,
            linear_gain: 5.0,
            original_range_min: 0.0,
            original_range_max: 4.0,
            face_exposure_boost: 1.1,
        };
        let first = apple_style_property_list(&request).unwrap();
        let second = apple_style_property_list(&request).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with(b"bplist00"));
        assert!(first
            .windows(APPLE_STYLE_BYTE_COUNT)
            .any(|window| { window == request.style_data }));
        for key in [
            b"LinearGTCImage".as_slice(),
            b"LinearImage".as_slice(),
            b"ToneMappedImage".as_slice(),
            b"PeopleRatio".as_slice(),
            b"OriginalRangeMax".as_slice(),
        ] {
            assert!(first.windows(key.len()).any(|window| window == key));
        }
        let trailer = first.len() - 32;
        let object_count = u64::from_be_bytes(first[trailer + 8..trailer + 16].try_into().unwrap());
        let top_object = u64::from_be_bytes(first[trailer + 16..trailer + 24].try_into().unwrap());
        let offset_table =
            u64::from_be_bytes(first[trailer + 24..trailer + 32].try_into().unwrap());
        assert!(object_count > 1);
        assert_eq!(top_object, object_count - 1);
        assert!(offset_table < trailer as u64);
    }

    #[test]
    fn property_list_rejects_incomplete_resources_and_nonfinite_statistics() {
        let distribution = AppleStyleDistribution {
            black_point: 0.0,
            high_key: 1.0,
            p02: 0.0,
            p10: 0.1,
            p25: 0.25,
            p50: 0.5,
            p75: 0.75,
            p98: 0.98,
            white_point: 1.0,
        };
        let statistics = AppleStyleStatistics {
            linear_gtc_image: distribution,
            linear_image: distribution,
            linear_image_person_segment_based: distribution,
            linear_image_skin_based: distribution,
            tone_mapped_image: distribution,
            tone_mapped_image_blue_channel_skin_based: distribution,
            tone_mapped_image_green_channel_skin_based: distribution,
            tone_mapped_image_person_segment_based: distribution,
            tone_mapped_image_red_channel_skin_based: distribution,
            tone_mapped_image_skin_based: distribution,
        };
        let mut invalid_distribution = distribution;
        invalid_distribution.p50 = f64::NAN;
        let invalid_statistics = AppleStyleStatistics {
            linear_gtc_image: invalid_distribution,
            ..statistics
        };
        let style_data = apple_style_identity_data();
        let global_tone_curve = vec![0_u8; 516];
        let light_map = vec![0_u8; APPLE_STYLE_LIGHT_MAP_BYTE_COUNT];
        let valid = AppleStylePropertyListRequest {
            style_data: &style_data,
            global_tone_curve: &global_tone_curve,
            baseline_exposure: 4.0,
            scene_type: 0,
            statistics: &statistics,
            people_ratio: 0.0,
            person_masks_valid_hint: -1.0,
            skin_ratio: 0.0,
            tone_light_map: &light_map,
            linear_light_map: &light_map,
            base_gain: 1.0,
            linear_gain: 4.0,
            original_range_min: 0.0,
            original_range_max: 1.0,
            face_exposure_boost: 1.0,
        };
        let invalid_curve = AppleStylePropertyListRequest {
            global_tone_curve: &global_tone_curve[..515],
            ..valid
        };
        assert!(matches!(
            apple_style_property_list(&invalid_curve),
            Err(AppleStyleDataError::InvalidPropertyListInput)
        ));
        let invalid_stats = AppleStylePropertyListRequest {
            statistics: &invalid_statistics,
            ..valid
        };
        assert!(matches!(
            apple_style_property_list(&invalid_stats),
            Err(AppleStyleDataError::InvalidPropertyListInput)
        ));
    }
}
