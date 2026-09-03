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
        return Err(AppleStyleDataError::NonFiniteCoefficient { index: 0 });
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
        let bound = if index / APPLE_STYLE_CHANNEL_COUNT >= 4 {
            QUADRATIC_BOUND
        } else {
            LINEAR_BOUND
        };
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
}
