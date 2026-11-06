//! Bounded decoding of the optional registered matrix and 3D lookup payloads.
//!
//! The wire record stores one scalar sample per LUT position, and does not
//! declare a colour domain or a channel-vector interpretation. Preserve the
//! source numbers and their exact encoding; rendering policy belongs to the
//! caller and must not invent those missing semantics.

use crate::{MetadataError, Result};

const MAX_LUT_EDGE: usize = 65;

fn invalid(message: impl Into<String>) -> MetadataError {
    MetadataError::invalid("registered mapping", message)
}

/// One optional matrix/LUT record, including its exact source bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredFloatLut {
    precision_bits: u8,
    matrix: [[f32; 3]; 3],
    edge: usize,
    samples: Vec<f32>,
    encoded: Vec<u8>,
}

// Construction is private and parsing rejects every nonfinite value, so the
// floating-point fields are reflexive under PartialEq.
impl Eq for RegisteredFloatLut {}

impl RegisteredFloatLut {
    pub const fn precision_bits(&self) -> u8 {
        self.precision_bits
    }

    pub const fn matrix(&self) -> &[[f32; 3]; 3] {
        &self.matrix
    }

    pub const fn edge(&self) -> usize {
        self.edge
    }

    /// Scalar source values in their original wire order. A colour mapping
    /// must not infer RGB output vectors or a LUT domain from this slice.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn encoded(&self) -> &[u8] {
        &self.encoded
    }
}

/// A zero flag is retained distinctly from an absent section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisteredMapping {
    Disabled,
    Lut(RegisteredFloatLut),
}

impl RegisteredMapping {
    fn parse(section: &[u8]) -> Result<Self> {
        let Some(&flag) = section.first() else {
            return Err(invalid("mapping section is empty"));
        };
        if flag == 0 {
            if section.len() != 1 {
                return Err(invalid("disabled mapping contains unexpected payload"));
            }
            return Ok(Self::Disabled);
        }
        let width = match flag {
            16 => 2,
            32 => 4,
            _ => return Err(invalid("mapping precision must be 16 or 32 bits")),
        };
        let float_bytes = section
            .len()
            .checked_sub(1)
            .ok_or_else(|| invalid("mapping section is empty"))?;
        if float_bytes % width != 0 || float_bytes / width < 10 {
            return Err(invalid("mapping matrix and LUT have an invalid size"));
        }
        let count = float_bytes / width - 9;
        let edge = (1..=MAX_LUT_EDGE)
            .find(|&edge| edge * edge * edge == count)
            .ok_or_else(|| invalid("mapping LUT sample count is not a bounded cube"))?;

        let mut values = Vec::with_capacity(count + 9);
        for chunk in section[1..].chunks_exact(width) {
            let value = if width == 2 {
                let bits = u16::from_be_bytes([chunk[0], chunk[1]]);
                let sign = bits & 0x8000 != 0;
                let exponent = (bits >> 10) & 31;
                let fraction = bits & 0x03ff;
                if exponent == 31 {
                    return Err(invalid("mapping contains nonfinite half precision data"));
                }
                if exponent == 0 {
                    let value = f32::from(fraction) * 2.0_f32.powi(-24);
                    if sign { -value } else { value }
                } else {
                    let expanded = (u32::from(bits & 0x8000) << 16)
                        | (u32::from(exponent + 112) << 23)
                        | (u32::from(fraction) << 13);
                    f32::from_bits(expanded)
                }
            } else {
                f32::from_bits(u32::from_be_bytes(chunk.try_into().unwrap()))
            };
            if !value.is_finite() {
                return Err(invalid("mapping contains nonfinite float data"));
            }
            values.push(value);
        }
        let mut matrix = [[0.0_f32; 3]; 3];
        for (row, values) in matrix.iter_mut().zip(values[..9].chunks_exact(3)) {
            row.copy_from_slice(values);
        }
        Ok(Self::Lut(RegisteredFloatLut {
            precision_bits: flag,
            matrix,
            edge,
            samples: values[9..].to_vec(),
            encoded: section.to_vec(),
        }))
    }

    fn encoded(&self) -> &[u8] {
        match self {
            Self::Disabled => &[0],
            Self::Lut(lut) => lut.encoded(),
        }
    }
}

/// The two length-delimited optional mapping sections in a registered carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredMappingExtension {
    pub base: Option<RegisteredMapping>,
    pub combined: Option<RegisteredMapping>,
}

impl RegisteredMappingExtension {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut position = 0usize;
        fn section<'a>(bytes: &'a [u8], position: &mut usize) -> Result<&'a [u8]> {
            let end = position
                .checked_add(2)
                .ok_or_else(|| invalid("mapping length overflows"))?;
            let word = bytes
                .get(*position..end)
                .ok_or_else(|| invalid("truncated mapping section length"))?;
            let length = usize::from(u16::from_be_bytes([word[0], word[1]]));
            *position = end;
            let end = position
                .checked_add(length)
                .ok_or_else(|| invalid("mapping section length overflows"))?;
            let data = bytes
                .get(*position..end)
                .ok_or_else(|| invalid("truncated mapping section"))?;
            *position = end;
            Ok(data)
        }
        let base = section(bytes, &mut position)?;
        let combined = section(bytes, &mut position)?;
        if position != bytes.len() {
            return Err(invalid("unexpected mapping extension tail"));
        }
        Ok(Self {
            base: (!base.is_empty()).then(|| RegisteredMapping::parse(base)).transpose()?,
            combined: (!combined.is_empty())
                .then(|| RegisteredMapping::parse(combined))
                .transpose()?,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        for section in [&self.base, &self.combined] {
            let raw = section.as_ref().map_or(&[][..], RegisteredMapping::encoded);
            let length = u16::try_from(raw.len())
                .map_err(|_| invalid("mapping section exceeds its u16 length"))?;
            output.extend_from_slice(&length.to_be_bytes());
            output.extend_from_slice(raw);
        }
        Self::parse(&output)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(flag: u8, values: &[u32]) -> Vec<u8> {
        let mut result = vec![flag];
        for value in values {
            match flag {
                16 => result.extend_from_slice(&(*value as u16).to_be_bytes()),
                32 => result.extend_from_slice(&value.to_be_bytes()),
                _ => unreachable!(),
            }
        }
        result
    }

    fn extension(base: &[u8], combined: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&(base.len() as u16).to_be_bytes());
        result.extend_from_slice(base);
        result.extend_from_slice(&(combined.len() as u16).to_be_bytes());
        result.extend_from_slice(combined);
        result
    }

    #[test]
    fn half_precision_cube_and_matrix_roundtrip_without_requantization() {
        let mut numbers = vec![0_u32; 17];
        numbers[0] = 0x3c00;
        numbers[4] = 0x3c00;
        numbers[8] = 0x3c00;
        numbers[9] = 0x0001; // Smallest half subnormal: 2^-24.
        numbers[10] = 0xb800; // -0.5.
        let base = section(16, &numbers);
        let bytes = extension(&base, &[0]);
        let parsed = RegisteredMappingExtension::parse(&bytes).unwrap();
        let RegisteredMapping::Lut(lut) = parsed.base.as_ref().unwrap() else {
            panic!("the base mapping should carry a LUT");
        };
        assert_eq!(lut.precision_bits(), 16);
        assert_eq!(lut.edge(), 2);
        assert_eq!(lut.matrix(), &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        assert_eq!(lut.samples()[0], 2.0_f32.powi(-24));
        assert_eq!(lut.samples()[1], -0.5);
        assert_eq!(parsed.combined, Some(RegisteredMapping::Disabled));
        assert_eq!(parsed.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn float32_mapping_rejects_malformed_and_nonfinite_source() {
        let mut numbers = vec![0_u32; 17];
        numbers[0] = 1.0_f32.to_bits();
        numbers[4] = 1.0_f32.to_bits();
        numbers[8] = 1.0_f32.to_bits();
        let bytes = extension(&[], &section(32, &numbers));
        let parsed = RegisteredMappingExtension::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes().unwrap(), bytes);
        let RegisteredMapping::Lut(lut) = parsed.combined.unwrap() else {
            panic!("the combined mapping should carry a LUT");
        };
        assert_eq!(lut.edge(), 2);
        assert_eq!(lut.matrix()[0][0], 1.0);

        let mut bad = numbers.clone();
        bad[9] = f32::INFINITY.to_bits();
        assert!(RegisteredMappingExtension::parse(&extension(&section(32, &bad), &[])).is_err());
        bad = numbers;
        bad.pop(); // 7 samples is not a cube.
        assert!(RegisteredMappingExtension::parse(&extension(&section(32, &bad), &[])).is_err());
        assert!(RegisteredMappingExtension::parse(&extension(&[0, 1], &[])).is_err());
        assert!(RegisteredMappingExtension::parse(&extension(&[8], &[])).is_err());
        assert!(RegisteredMappingExtension::parse(&bytes[..bytes.len() - 1]).is_err());
        let mut trailing = bytes;
        trailing.push(0);
        assert!(RegisteredMappingExtension::parse(&trailing).is_err());
    }
}
