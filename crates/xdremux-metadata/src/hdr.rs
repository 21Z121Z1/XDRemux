//! Checked HDR property payloads. Values are supplied by the source or caller;
//! this module does not infer mastering conditions or content light levels.
use crate::{MetadataError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdrTransfer {
    Pq,
    Hlg,
}

impl HdrTransfer {
    pub const fn cicp(self) -> u16 {
        match self {
            Self::Pq => 16,
            Self::Hlg => 18,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasteringDisplay {
    /// CIE xy, in units of 0.00002. The source's array order is retained.
    pub primaries: [[u16; 2]; 3],
    pub white_point: [u16; 2],
    /// cd/m2 in units of 0.0001.
    pub maximum_luminance: u32,
    pub minimum_luminance: u32,
}

fn invalid(message: &str) -> MetadataError {
    MetadataError::invalid("HDR metadata", message)
}

impl MasteringDisplay {
    pub fn validate(&self) -> Result<()> {
        for [x, y] in self.primaries.iter().chain([&self.white_point]) {
            if *y == 0 || u32::from(*x) + u32::from(*y) > 50_000 {
                return Err(invalid("mastering chromaticity is outside CIE xy"));
            }
        }
        if self.maximum_luminance == 0 || self.minimum_luminance >= self.maximum_luminance {
            return Err(invalid("mastering luminance bounds are empty or reversed"));
        }
        Ok(())
    }

    pub fn to_mdcv_payload(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut bytes = Vec::with_capacity(24);
        for value in self
            .primaries
            .iter()
            .flatten()
            .chain(self.white_point.iter())
        {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        bytes.extend_from_slice(&self.maximum_luminance.to_be_bytes());
        bytes.extend_from_slice(&self.minimum_luminance.to_be_bytes());
        Ok(bytes)
    }

    pub fn from_mdcv_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 24 {
            return Err(invalid("mdcv payload must contain exactly 24 bytes"));
        }
        let word = |i| u16::from_be_bytes([bytes[i], bytes[i + 1]]);
        let result = Self {
            primaries: [[word(0), word(2)], [word(4), word(6)], [word(8), word(10)]],
            white_point: [word(12), word(14)],
            maximum_luminance: u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            minimum_luminance: u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        };
        result.validate()?;
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentLightLevel {
    /// cd/m2; zero denotes an unknown measurement.
    pub maximum_content_light_level: u16,
    pub maximum_frame_average_light_level: u16,
}

impl ContentLightLevel {
    pub fn validate(self) -> Result<()> {
        if self.maximum_content_light_level != 0
            && self.maximum_frame_average_light_level > self.maximum_content_light_level
        {
            return Err(invalid("MaxFALL exceeds known MaxCLL"));
        }
        Ok(())
    }

    pub fn to_clli_payload(self) -> Result<[u8; 4]> {
        self.validate()?;
        Ok([
            self.maximum_content_light_level.to_be_bytes(),
            self.maximum_frame_average_light_level.to_be_bytes(),
        ]
        .concat()
        .try_into()
        .unwrap())
    }

    pub fn from_clli_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 4 {
            return Err(invalid("clli payload must contain exactly four bytes"));
        }
        let result = Self {
            maximum_content_light_level: u16::from_be_bytes([bytes[0], bytes[1]]),
            maximum_frame_average_light_level: u16::from_be_bytes([bytes[2], bytes[3]]),
        };
        result.validate()?;
        Ok(result)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HdrStaticMetadata {
    pub mastering_display: Option<MasteringDisplay>,
    pub content_light_level: Option<ContentLightLevel>,
}

impl HdrStaticMetadata {
    pub fn validate(&self) -> Result<()> {
        if let Some(mastering) = &self.mastering_display {
            mastering.validate()?;
        }
        if let Some(light) = self.content_light_level {
            light.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_payloads_preserve_units_and_reject_malformed_values() {
        let mastering = MasteringDisplay {
            primaries: [[8500, 39850], [6550, 2300], [35400, 14600]],
            white_point: [15635, 16450],
            maximum_luminance: 10_000_000,
            minimum_luminance: 50,
        };
        let bytes = mastering.to_mdcv_payload().unwrap();
        assert_eq!(bytes.len(), 24);
        assert_eq!(&bytes[16..20], &10_000_000_u32.to_be_bytes());
        assert_eq!(
            MasteringDisplay::from_mdcv_payload(&bytes).unwrap(),
            mastering
        );
        assert!(MasteringDisplay::from_mdcv_payload(&bytes[..23]).is_err());
        assert!(MasteringDisplay {
            minimum_luminance: 10_000_000,
            ..mastering
        }
        .validate()
        .is_err());
        let light = ContentLightLevel {
            maximum_content_light_level: 1000,
            maximum_frame_average_light_level: 400,
        };
        assert_eq!(
            ContentLightLevel::from_clli_payload(&light.to_clli_payload().unwrap()).unwrap(),
            light
        );
        assert!(ContentLightLevel {
            maximum_content_light_level: 100,
            ..light
        }
        .validate()
        .is_err());
        assert!(ContentLightLevel::from_clli_payload(&[0; 5]).is_err());
    }
}
