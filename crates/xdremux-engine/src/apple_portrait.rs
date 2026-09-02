use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

const REND_HEADER_BYTES: usize = 16;
const REND_RECORD_BYTES: usize = 8;
const REND_MAGIC: [u8; 4] = *b"REND";

/// Dynamic REND record identifiers controlled by the recovered XHLRB logic.
pub const APPLE_XHLRB_DYNAMIC_RECORD_IDS: [u16; 14] = [
    0x0190, 0x0191, 0x0192, 0x0193, 0x0194, 0x0195, 0x0196, 0x0197, 0x0198, 0x0199, 0x01c2, 0x01c3,
    0x01c4, 0x01c5,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleRendError {
    InvalidHeader,
    InvalidLength,
    UnsupportedRecordType { identifier: u16, value_type: u16 },
    DuplicateIdentifier(u16),
    NonFiniteControlInput(&'static str),
}

impl fmt::Display for AppleRendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => formatter.write_str("REND is missing its 16-byte header"),
            Self::InvalidLength => {
                formatter.write_str("REND declared length or record alignment is invalid")
            }
            Self::UnsupportedRecordType {
                identifier,
                value_type,
            } => write!(
                formatter,
                "REND record 0x{identifier:04x} uses unsupported type {value_type}"
            ),
            Self::DuplicateIdentifier(identifier) => {
                write!(
                    formatter,
                    "REND contains duplicate record 0x{identifier:04x}"
                )
            }
            Self::NonFiniteControlInput(name) => {
                write!(formatter, "XHLRB control input {name} must be finite")
            }
        }
    }
}

impl Error for AppleRendError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleRendRecord {
    pub identifier: u16,
    pub value_type: u16,
    pub raw_value: u32,
}

impl AppleRendRecord {
    pub const fn new(identifier: u16, value_type: u16, raw_value: u32) -> Self {
        Self {
            identifier,
            value_type,
            raw_value,
        }
    }

    pub const fn integer(identifier: u16, value: i32) -> Self {
        Self::new(identifier, 2, value as u32)
    }

    pub fn float(identifier: u16, value: f32) -> Self {
        Self::new(identifier, 1, value.to_bits())
    }

    pub fn float_value(self) -> Option<f32> {
        (self.value_type == 1).then(|| f32::from_bits(self.raw_value))
    }

    pub const fn integer_value(self) -> Option<i32> {
        if self.value_type == 2 {
            Some(self.raw_value as i32)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleRendDocument {
    version: u32,
    section_version: u32,
    records: Vec<AppleRendRecord>,
}

impl AppleRendDocument {
    pub fn new(
        version: u32,
        section_version: u32,
        records: Vec<AppleRendRecord>,
    ) -> Result<Self, AppleRendError> {
        validate_records(&records)?;
        Ok(Self {
            version,
            section_version,
            records,
        })
    }

    pub fn parse(data: &[u8]) -> Result<Self, AppleRendError> {
        if data.len() < REND_HEADER_BYTES || data.get(..4) != Some(&REND_MAGIC) {
            return Err(AppleRendError::InvalidHeader);
        }

        let declared_length =
            usize::try_from(read_u32_le(data, 8)?).map_err(|_| AppleRendError::InvalidLength)?;
        if declared_length != data.len()
            || declared_length < REND_HEADER_BYTES
            || !(declared_length - REND_HEADER_BYTES).is_multiple_of(REND_RECORD_BYTES)
        {
            return Err(AppleRendError::InvalidLength);
        }

        let version = read_u32_le(data, 4)?;
        let section_version = read_u32_le(data, 12)?;
        let mut records = Vec::with_capacity((declared_length - REND_HEADER_BYTES) / 8);
        let mut cursor = REND_HEADER_BYTES;
        while cursor < declared_length {
            records.push(AppleRendRecord::new(
                read_u16_le(data, cursor)?,
                read_u16_le(data, cursor + 2)?,
                read_u32_le(data, cursor + 4)?,
            ));
            cursor += REND_RECORD_BYTES;
        }
        Self::new(version, section_version, records)
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn section_version(&self) -> u32 {
        self.section_version
    }

    pub fn records(&self) -> &[AppleRendRecord] {
        &self.records
    }

    /// Serialize the producer format exactly.
    ///
    /// `signed_identifier_order` matches the established Swift producer bridge,
    /// which sorts record identifiers through their signed Int16 bit pattern.
    pub fn serialized(&self, signed_identifier_order: bool) -> Result<Vec<u8>, AppleRendError> {
        validate_records(&self.records)?;
        let mut records = self.records.clone();
        if signed_identifier_order {
            records.sort_by_key(|record| record.identifier as i16);
        }
        let payload_bytes = records
            .len()
            .checked_mul(REND_RECORD_BYTES)
            .and_then(|bytes| bytes.checked_add(REND_HEADER_BYTES))
            .ok_or(AppleRendError::InvalidLength)?;
        let declared_length =
            u32::try_from(payload_bytes).map_err(|_| AppleRendError::InvalidLength)?;

        let mut output = Vec::with_capacity(payload_bytes);
        output.extend_from_slice(&REND_MAGIC);
        output.extend_from_slice(&self.version.to_le_bytes());
        output.extend_from_slice(&declared_length.to_le_bytes());
        output.extend_from_slice(&self.section_version.to_le_bytes());
        for record in records {
            output.extend_from_slice(&record.identifier.to_le_bytes());
            output.extend_from_slice(&record.value_type.to_le_bytes());
            output.extend_from_slice(&record.raw_value.to_le_bytes());
        }
        Ok(output)
    }

    /// Replace matching record identifiers while preserving producer order.
    /// New identifiers are appended in caller order, matching the old Swift
    /// dictionary-backed builder once the final signed-ID serialization sort is
    /// applied.
    pub fn replacing(
        &self,
        replacements: impl IntoIterator<Item = AppleRendRecord>,
    ) -> Result<Self, AppleRendError> {
        let replacements = replacements.into_iter().collect::<Vec<_>>();
        validate_records(&replacements)?;
        let by_identifier = replacements
            .iter()
            .copied()
            .map(|record| (record.identifier, record))
            .collect::<BTreeMap<_, _>>();

        let mut emitted = BTreeSet::new();
        let mut records = Vec::with_capacity(self.records.len() + replacements.len());
        for record in &self.records {
            if let Some(replacement) = by_identifier.get(&record.identifier) {
                records.push(*replacement);
                emitted.insert(record.identifier);
            } else {
                records.push(*record);
            }
        }
        records.extend(
            replacements
                .into_iter()
                .filter(|record| !emitted.contains(&record.identifier)),
        );
        Self::new(self.version, self.section_version, records)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleXhlrbControlOutput {
    records: [AppleRendRecord; 14],
}

impl AppleXhlrbControlOutput {
    pub fn make(
        profile_is_one_x: bool,
        scene_activation: f64,
        gain_map_headroom: f64,
    ) -> Result<Self, AppleRendError> {
        if !scene_activation.is_finite() {
            return Err(AppleRendError::NonFiniteControlInput("scene_activation"));
        }
        if !gain_map_headroom.is_finite() {
            return Err(AppleRendError::NonFiniteControlInput("gain_map_headroom"));
        }

        let activation = scene_activation.clamp(0.0, 1.0);
        let headroom = gain_map_headroom.max(0.0);
        let headroom_factor = headroom.min(4.0) / 4.0;
        let maximum_intensity_gain = if profile_is_one_x { 0.25 } else { 0.10 };
        let maximum_obscene_weight_gain = if profile_is_one_x { 20.0 } else { 23.0 };
        let maximum_obscene_intensity_gain = if profile_is_one_x { 0.60 } else { 0.70 };
        let secondary_activation = activation * headroom_factor;

        let float = |identifier, value: f64| AppleRendRecord::float(identifier, value as f32);
        Ok(Self {
            records: [
                AppleRendRecord::integer(0x0190, (50.0 * activation).round() as i32),
                float(0x0191, 0.25 * activation),
                float(0x0192, 12.0 * activation),
                float(0x0193, maximum_intensity_gain * activation),
                float(0x0194, 0.0),
                float(0x0195, 0.0),
                float(0x0196, 0.0),
                float(0x0197, 0.0),
                float(0x0198, 0.0),
                float(0x0199, 0.0),
                float(0x01c2, 8.0 * secondary_activation),
                float(0x01c3, maximum_obscene_weight_gain * secondary_activation),
                float(
                    0x01c4,
                    maximum_obscene_intensity_gain * secondary_activation,
                ),
                float(0x01c5, headroom),
            ],
        })
    }

    pub const fn records(&self) -> &[AppleRendRecord; 14] {
        &self.records
    }
}

fn validate_records(records: &[AppleRendRecord]) -> Result<(), AppleRendError> {
    let mut identifiers = BTreeSet::new();
    for record in records {
        if !(1..=4).contains(&record.value_type) {
            return Err(AppleRendError::UnsupportedRecordType {
                identifier: record.identifier,
                value_type: record.value_type,
            });
        }
        if !identifiers.insert(record.identifier) {
            return Err(AppleRendError::DuplicateIdentifier(record.identifier));
        }
    }
    Ok(())
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, AppleRendError> {
    data.get(offset..offset.saturating_add(2))
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or(AppleRendError::InvalidLength)
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, AppleRendError> {
    data.get(offset..offset.saturating_add(4))
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or(AppleRendError::InvalidLength)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rend_round_trips_byte_identically() {
        let document = AppleRendDocument::new(
            1,
            2,
            vec![
                AppleRendRecord::float(0x0191, 0.25),
                AppleRendRecord::integer(0x0190, 50),
            ],
        )
        .expect("valid REND document");
        let encoded = document.serialized(false).expect("serialize REND");
        let decoded = AppleRendDocument::parse(&encoded).expect("parse REND");
        assert_eq!(decoded, document);
        assert_eq!(decoded.serialized(false).unwrap(), encoded);
    }

    #[test]
    fn signed_identifier_sort_matches_swift_int16_order() {
        let document = AppleRendDocument::new(
            1,
            1,
            vec![
                AppleRendRecord::integer(0x0001, 1),
                AppleRendRecord::integer(0xffff, 2),
                AppleRendRecord::integer(0x8000, 3),
                AppleRendRecord::integer(0x7fff, 4),
            ],
        )
        .unwrap();
        let encoded = document.serialized(true).unwrap();
        let sorted = AppleRendDocument::parse(&encoded).unwrap();
        assert_eq!(
            sorted
                .records()
                .iter()
                .map(|record| record.identifier)
                .collect::<Vec<_>>(),
            vec![0x8000, 0xffff, 0x0001, 0x7fff]
        );
    }

    #[test]
    fn rend_rejects_duplicate_identifiers_and_unknown_types() {
        assert_eq!(
            AppleRendDocument::new(
                1,
                1,
                vec![
                    AppleRendRecord::integer(7, 1),
                    AppleRendRecord::float(7, 1.0),
                ],
            ),
            Err(AppleRendError::DuplicateIdentifier(7))
        );
        assert_eq!(
            AppleRendDocument::new(1, 1, vec![AppleRendRecord::new(7, 5, 0)]),
            Err(AppleRendError::UnsupportedRecordType {
                identifier: 7,
                value_type: 5,
            })
        );
    }

    #[test]
    fn xhlrb_dynamic_records_match_recovered_swift_formulas() {
        let headroom = 3.466_976_881_027_221_7;
        let output = AppleXhlrbControlOutput::make(false, 1.0, headroom).unwrap();
        let records = output.records();
        assert_eq!(
            records
                .iter()
                .map(|record| record.identifier)
                .collect::<Vec<_>>(),
            APPLE_XHLRB_DYNAMIC_RECORD_IDS.to_vec()
        );
        assert_eq!(records[0].integer_value(), Some(50));
        assert_eq!(records[1].float_value(), Some(0.25));
        assert_eq!(records[2].float_value(), Some(12.0));
        assert_eq!(records[3].float_value(), Some(0.10_f32));
        assert_eq!(records[10].float_value(), Some((2.0 * headroom) as f32));
        assert_eq!(
            records[11].float_value(),
            Some((23.0 * headroom / 4.0) as f32)
        );
        assert_eq!(
            records[12].float_value(),
            Some((0.70 * headroom / 4.0) as f32)
        );
        assert_eq!(records[13].float_value(), Some(headroom as f32));
    }

    #[test]
    fn xhlrb_clamps_finite_inputs_and_rejects_non_finite_inputs() {
        let output = AppleXhlrbControlOutput::make(true, 2.0, -4.0).unwrap();
        assert_eq!(output.records()[0].integer_value(), Some(50));
        assert_eq!(output.records()[10].float_value(), Some(0.0));
        assert_eq!(output.records()[13].float_value(), Some(0.0));
        assert_eq!(
            AppleXhlrbControlOutput::make(true, f64::NAN, 1.0),
            Err(AppleRendError::NonFiniteControlInput("scene_activation"))
        );
    }

    #[test]
    fn xhlrb_records_replace_static_profile_without_duplicate_scene_ids() {
        let static_profile = AppleRendDocument::new(
            1,
            1,
            vec![
                AppleRendRecord::float(0x0100, 2.0),
                AppleRendRecord::float(0x0191, 99.0),
            ],
        )
        .unwrap();
        let dynamic = AppleXhlrbControlOutput::make(false, 0.5, 2.0).unwrap();
        let combined = static_profile
            .replacing(dynamic.records().iter().copied())
            .unwrap();
        let identifiers = combined
            .records()
            .iter()
            .map(|record| record.identifier)
            .collect::<BTreeSet<_>>();
        assert_eq!(identifiers.len(), combined.records().len());
        assert_eq!(
            combined
                .records()
                .iter()
                .find(|record| record.identifier == 0x0191)
                .and_then(|record| record.float_value()),
            Some(0.125)
        );
    }
}
