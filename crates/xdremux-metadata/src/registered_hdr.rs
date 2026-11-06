//! Bounded registered HDR metadata carriers. Dynamic payloads are checked for
//! syntax and retained exactly; applying their display mapping is separate.
use crate::{
    ContentLightLevel, MasteringDisplay, MetadataError, RegisteredMappingExtension, Result,
};

const PREFIX: &[u8] = &[0x26, 0, 4, 0, 7, 2];

fn invalid(message: &str) -> MetadataError {
    MetadataError::invalid("registered HDR metadata", message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicHdrMetadata {
    payload: Vec<u8>,
    statistics: DynamicHdrStatistics,
    tone_mappings: Vec<DynamicHdrToneMapping>,
    saturation_gains: Vec<u8>,
}

/// Source PQ-code statistics. These are encoded codes, not linear luminance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicHdrStatistics {
    pub minimum_maxrgb_pq: u16,
    pub average_maxrgb_pq: u16,
    pub variance_maxrgb_pq: u16,
    pub maximum_maxrgb_pq: u16,
}

/// Encoded base-curve parameters. Decoding these fields does not, by itself,
/// define a display adaptation algorithm or a target-display policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicHdrBaseParameters {
    pub m_p: u16,
    pub m_m: u8,
    pub m_a: u16,
    pub m_n: u8,
    pub m_b: u16,
    pub k1: u8,
    pub k2: u8,
    pub k3: u8,
    pub delta_mode: u8,
    pub delta: u8,
}

/// One encoded spline segment, retained without inferring curve semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicHdrSpline {
    pub threshold_mode: u8,
    pub threshold_mb: Option<u8>,
    pub threshold: u16,
    pub delta1: u16,
    pub delta2: u16,
    pub strength: u8,
}

/// One target-display set of encoded tone-mapping parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicHdrToneMapping {
    pub target_maximum_luminance_pq: u16,
    pub base: Option<DynamicHdrBaseParameters>,
    pub splines: Vec<DynamicHdrSpline>,
}

struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl Bits<'_> {
    fn read(&mut self, n: usize) -> Result<u32> {
        let end = self
            .position
            .checked_add(n)
            .filter(|e| *e <= self.bytes.len() * 8)
            .ok_or_else(|| invalid("truncated dynamic metadata"))?;
        let mut value = 0;
        while self.position < end {
            value = (value << 1)
                | u32::from((self.bytes[self.position / 8] >> (7 - self.position % 8)) & 1);
            self.position += 1;
        }
        Ok(value)
    }
}

impl DynamicHdrMetadata {
    /// Construct packed metadata from source- or caller-supplied encoded
    /// fields. No measurements, curve parameters, or display target are inferred.
    pub fn from_parts(
        statistics: DynamicHdrStatistics,
        tone_mappings: Vec<DynamicHdrToneMapping>,
        saturation_gains: Vec<u8>,
    ) -> Result<Self> {
        if tone_mappings.len() > 2 || saturation_gains.len() > 7 {
            return Err(invalid("too many dynamic target sets or saturation gains"));
        }
        let mut packed = PackedBits::default();
        packed.write(1, 8)?;
        for value in [
            statistics.minimum_maxrgb_pq,
            statistics.average_maxrgb_pq,
            statistics.variance_maxrgb_pq,
            statistics.maximum_maxrgb_pq,
        ] {
            packed.write(u32::from(value), 12)?;
        }
        packed.write(u32::from(!tone_mappings.is_empty()), 1)?;
        if !tone_mappings.is_empty() {
            packed.write((tone_mappings.len() - 1) as u32, 1)?;
        }
        for target in &tone_mappings {
            packed.write(u32::from(target.target_maximum_luminance_pq), 12)?;
            packed.write(u32::from(target.base.is_some()), 1)?;
            if let Some(base) = target.base {
                for (value, width) in [
                    (u32::from(base.m_p), 14),
                    (u32::from(base.m_m), 6),
                    (u32::from(base.m_a), 10),
                    (u32::from(base.m_n), 6),
                    (u32::from(base.m_b), 10),
                    (u32::from(base.k1), 2),
                    (u32::from(base.k2), 2),
                    (u32::from(base.k3), 4),
                    (u32::from(base.delta_mode), 3),
                    (u32::from(base.delta), 7),
                ] {
                    packed.write(value, width)?;
                }
            }
            if target.splines.len() > 2 {
                return Err(invalid("too many dynamic spline pieces"));
            }
            packed.write(u32::from(!target.splines.is_empty()), 1)?;
            if !target.splines.is_empty() {
                packed.write((target.splines.len() - 1) as u32, 1)?;
            }
            for spline in &target.splines {
                packed.write(u32::from(spline.threshold_mode), 2)?;
                let needs_mb = matches!(spline.threshold_mode, 0 | 2);
                if needs_mb != spline.threshold_mb.is_some() {
                    return Err(invalid("spline threshold MB disagrees with its mode"));
                }
                if let Some(mb) = spline.threshold_mb {
                    packed.write(u32::from(mb), 8)?;
                }
                for (value, width) in [
                    (u32::from(spline.threshold), 12),
                    (u32::from(spline.delta1), 10),
                    (u32::from(spline.delta2), 10),
                    (u32::from(spline.strength), 8),
                ] {
                    packed.write(value, width)?;
                }
            }
        }
        packed.write(u32::from(!saturation_gains.is_empty()), 1)?;
        if !saturation_gains.is_empty() {
            packed.write(saturation_gains.len() as u32, 3)?;
            for gain in &saturation_gains {
                packed.write(u32::from(*gain), 8)?;
            }
        }
        Self::parse(&packed.bytes)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > u16::MAX as usize {
            return Err(invalid("invalid dynamic metadata length"));
        }
        let mut bits = Bits { bytes, position: 0 };
        if bits.read(8)? != 1 {
            return Err(invalid("unsupported dynamic metadata version"));
        }
        let statistics = DynamicHdrStatistics {
            minimum_maxrgb_pq: bits.read(12)? as u16,
            average_maxrgb_pq: bits.read(12)? as u16,
            variance_maxrgb_pq: bits.read(12)? as u16,
            maximum_maxrgb_pq: bits.read(12)? as u16,
        };
        if statistics.minimum_maxrgb_pq > statistics.average_maxrgb_pq
            || statistics.average_maxrgb_pq > statistics.maximum_maxrgb_pq
        {
            return Err(invalid("dynamic luminance statistics are reversed"));
        }
        let mut tone_mappings = Vec::new();
        if bits.read(1)? != 0 {
            let count = bits.read(1)? + 1;
            for _ in 0..count {
                let target_maximum_luminance_pq = bits.read(12)? as u16;
                let base = if bits.read(1)? != 0 {
                    Some(DynamicHdrBaseParameters {
                        m_p: bits.read(14)? as u16,
                        m_m: bits.read(6)? as u8,
                        m_a: bits.read(10)? as u16,
                        m_n: bits.read(6)? as u8,
                        m_b: bits.read(10)? as u16,
                        k1: bits.read(2)? as u8,
                        k2: bits.read(2)? as u8,
                        k3: bits.read(4)? as u8,
                        delta_mode: bits.read(3)? as u8,
                        delta: bits.read(7)? as u8,
                    })
                } else {
                    None
                };
                let mut splines = Vec::new();
                if bits.read(1)? != 0 {
                    let spline_count = bits.read(1)? + 1;
                    for _ in 0..spline_count {
                        let threshold_mode = bits.read(2)? as u8;
                        let threshold_mb = if threshold_mode == 0 || threshold_mode == 2 {
                            Some(bits.read(8)? as u8)
                        } else {
                            None
                        };
                        splines.push(DynamicHdrSpline {
                            threshold_mode,
                            threshold_mb,
                            threshold: bits.read(12)? as u16,
                            delta1: bits.read(10)? as u16,
                            delta2: bits.read(10)? as u16,
                            strength: bits.read(8)? as u8,
                        });
                    }
                }
                tone_mappings.push(DynamicHdrToneMapping {
                    target_maximum_luminance_pq,
                    base,
                    splines,
                });
            }
        }
        let mut saturation_gains = Vec::new();
        if bits.read(1)? != 0 {
            let count = bits.read(3)?;
            for _ in 0..count {
                saturation_gains.push(bits.read(8)? as u8);
            }
        }
        let remaining = bytes.len() * 8 - bits.position;
        if remaining > 7 || bits.read(remaining)? != 0 {
            return Err(invalid(
                "unexpected dynamic metadata tail or nonzero padding",
            ));
        }
        Ok(Self {
            payload: bytes.to_vec(),
            statistics,
            tone_mappings,
            saturation_gains,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.payload
    }

    pub const fn statistics(&self) -> DynamicHdrStatistics {
        self.statistics
    }

    pub fn tone_mappings(&self) -> &[DynamicHdrToneMapping] {
        &self.tone_mappings
    }

    pub fn saturation_gains(&self) -> &[u8] {
        &self.saturation_gains
    }
}

#[derive(Default)]
struct PackedBits {
    bytes: Vec<u8>,
    length: usize,
}

impl PackedBits {
    fn write(&mut self, value: u32, width: usize) -> Result<()> {
        if value >= 1 << width {
            return Err(invalid("dynamic field exceeds its packed bit width"));
        }
        for shift in (0..width).rev() {
            if self.length.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let index = self.length / 8;
            self.bytes[index] |= (((value >> shift) & 1) as u8) << (7 - self.length % 8);
            self.length += 1;
        }
        Ok(())
    }
}

/// Registered JPEG base-image declaration.
///
/// The base image declares the image count and, for each additional image, two
/// offsets to the enhanced image. The first offset is measured from the
/// declaration's APP8 marker in the file. The second is measured in a view
/// without APP segments other than the JFIF one.
///
/// The second offset's coordinate rule is implemented as documented here and
/// is not yet confirmed by an independent producer sample. A caller that needs
/// external interoperability must treat it as unverified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredJpegBaseDeclaration {
    pub image_count: u8,
    /// `(raw offset, offset without non-JFIF APP segments)` for each additional
    /// image, in image order.
    pub offsets: Vec<(u32, u32)>,
}

/// `ITUT35` identity followed by the T.35 codes and the base image type.
const APP8_BASE_HEADER: [u8; 12] = [b'I', b'T', b'U', b'T', b'3', b'5', 0x26, 0, 4, 0, 7, 1];
const APP8_BASE_TYPE: u32 = 0x0100_0000;

/// Payload length of a base declaration for `image_count` images.
pub const fn registered_jpeg_base_payload_len(image_count: u8) -> usize {
    // Identity, country code, two provider codes, image count, image type and
    // one offset pair for every additional image.
    12 + 4 + 8 * image_count.saturating_sub(1) as usize
}

impl RegisteredJpegBaseDeclaration {
    pub fn validate(&self) -> Result<()> {
        if !(2..=8).contains(&self.image_count) {
            return Err(invalid("a base declaration describes two to eight images"));
        }
        if self.offsets.len() != usize::from(self.image_count) - 1 {
            return Err(invalid(
                "base declaration offsets disagree with the image count",
            ));
        }
        for (raw, virtual_offset) in &self.offsets {
            if *raw == 0 || *virtual_offset == 0 {
                return Err(invalid("base declaration offsets must be non-zero"));
            }
            if virtual_offset > raw {
                return Err(invalid(
                    "the reduced-view offset cannot exceed the raw offset",
                ));
            }
        }
        Ok(())
    }

    /// APP8 payload, excluding the JPEG marker and length word.
    pub fn to_app8_payload(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut payload = APP8_BASE_HEADER.to_vec();
        payload[11] = self.image_count;
        payload.extend_from_slice(&APP8_BASE_TYPE.to_be_bytes());
        for (raw, virtual_offset) in &self.offsets {
            payload.extend_from_slice(&raw.to_be_bytes());
            payload.extend_from_slice(&virtual_offset.to_be_bytes());
        }
        Self::parse_app8_payload(&payload)?;
        Ok(payload)
    }

    /// Read a base declaration and reject an enhanced or extended payload.
    pub fn parse_app8_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 16 || bytes.get(..6) != Some(b"ITUT35") {
            return Err(invalid("missing registered APP8 identity"));
        }
        let image_count = bytes[11];
        if bytes[..11] != APP8_BASE_HEADER[..11] || image_count < 2 {
            return Err(invalid("unsupported registered APP8 base identity"));
        }
        let kind = u32::from_be_bytes(bytes[12..16].try_into().unwrap());
        if kind != APP8_BASE_TYPE {
            return Err(invalid(
                "registered APP8 payload is not a base image declaration",
            ));
        }
        let expected = registered_jpeg_base_payload_len(image_count);
        if bytes.len() != expected {
            return Err(invalid(
                "registered APP8 base declaration length disagrees with its image count",
            ));
        }
        let mut offsets = Vec::with_capacity(usize::from(image_count) - 1);
        for pair in bytes[16..].as_chunks::<8>().0 {
            offsets.push((
                u32::from_be_bytes(pair[..4].try_into().unwrap()),
                u32::from_be_bytes(pair[4..].try_into().unwrap()),
            ));
        }
        let declaration = Self {
            image_count,
            offsets,
        };
        declaration.validate()?;
        Ok(declaration)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredHdrMetadata {
    pub static_metadata: Option<(MasteringDisplay, ContentLightLevel)>,
    pub dynamic_metadata: Option<DynamicHdrMetadata>,
    pub mapping_extension: Option<RegisteredMappingExtension>,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(n)
            .filter(|e| *e <= self.bytes.len())
            .ok_or_else(|| invalid("truncated registered metadata"))?;
        let result = &self.bytes[self.position..end];
        self.position = end;
        Ok(result)
    }
    fn word(&mut self) -> Result<usize> {
        Ok(usize::from(u16::from_be_bytes(
            self.take(2)?.try_into().unwrap(),
        )))
    }
}

impl RegisteredHdrMetadata {
    pub fn has_enabled_mapping(&self) -> bool {
        self.mapping_extension.as_ref().is_some_and(|extension| {
            [&extension.base, &extension.combined]
                .into_iter()
                .any(|mapping| matches!(mapping, Some(crate::RegisteredMapping::Lut(_))))
        })
    }

    pub fn parse_it35(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor { bytes, position: 0 };
        if cursor.take(6)? != PREFIX {
            return Err(invalid("unsupported T.35 identity or image count"));
        }
        let kind = u32::from_be_bytes(cursor.take(4)?.try_into().unwrap());
        if kind >> 24 != 2 || (kind >> 16) & 255 != 1 || kind & 255 != 0 || (kind >> 8) & 255 > 1 {
            return Err(invalid("unsupported registered HDR image type or flags"));
        }
        let size = cursor.word()?;
        let section = cursor.take(size)?;
        let mut data = Cursor {
            bytes: section,
            position: 0,
        };
        let static_size = data.word()?;
        let static_metadata = match static_size {
            0 => None,
            28 => {
                let raw = data.take(28)?;
                Some((
                    MasteringDisplay::from_mdcv_payload(&raw[..24])?,
                    ContentLightLevel::from_clli_payload(&raw[24..])?,
                ))
            }
            _ => return Err(invalid("registered static metadata must contain 28 bytes")),
        };
        let dynamic_size = data.word()?;
        let dynamic_metadata = if dynamic_size == 0 {
            None
        } else {
            Some(DynamicHdrMetadata::parse(data.take(dynamic_size)?)?)
        };
        if data.position != section.len() {
            return Err(invalid("metadata_size disagrees with nested sections"));
        }
        let mapping_extension = if (kind >> 8) & 255 != 0 {
            let size = cursor.word()?;
            Some(RegisteredMappingExtension::parse(cursor.take(size)?)?)
        } else {
            None
        };
        if cursor.position != bytes.len() {
            return Err(invalid("unexpected registered metadata tail"));
        }
        if static_metadata.is_none() && dynamic_metadata.is_none() && mapping_extension.is_none() {
            return Err(invalid(
                "registered HDR metadata contains no source information",
            ));
        }
        Ok(Self {
            static_metadata,
            dynamic_metadata,
            mapping_extension,
        })
    }

    pub fn to_it35_payload(&self) -> Result<Vec<u8>> {
        let mut section = Vec::new();
        if let Some((mastering, light)) = &self.static_metadata {
            section.extend_from_slice(&28_u16.to_be_bytes());
            section.extend_from_slice(&mastering.to_mdcv_payload()?);
            section.extend_from_slice(&light.to_clli_payload()?);
        } else {
            section.extend_from_slice(&0_u16.to_be_bytes());
        }
        let dynamic = self
            .dynamic_metadata
            .as_ref()
            .map_or(&[][..], |d| d.as_bytes());
        section.extend_from_slice(
            &u16::try_from(dynamic.len())
                .map_err(|_| invalid("dynamic metadata exceeds carrier length"))?
                .to_be_bytes(),
        );
        section.extend_from_slice(dynamic);
        let size = u16::try_from(section.len())
            .map_err(|_| invalid("registered metadata exceeds carrier length"))?;
        let mut result = PREFIX.to_vec();
        let kind = if self.mapping_extension.is_some() {
            0x02010100_u32
        } else {
            0x02010000_u32
        };
        result.extend_from_slice(&kind.to_be_bytes());
        result.extend_from_slice(&size.to_be_bytes());
        result.extend(section);
        if let Some(mapping) = &self.mapping_extension {
            let encoded = mapping.to_bytes()?;
            let length = u16::try_from(encoded.len())
                .map_err(|_| invalid("registered mapping exceeds carrier length"))?;
            result.extend_from_slice(&length.to_be_bytes());
            result.extend_from_slice(&encoded);
        }
        Self::parse_it35(&result)?;
        Ok(result)
    }

    /// APP8 segment payload, excluding its JPEG marker and length word.
    pub fn to_app8_payload(&self) -> Result<Vec<u8>> {
        let mut result = b"ITUT35".to_vec();
        result.extend(self.to_it35_payload()?);
        if result.len() > 65533 {
            return Err(invalid("registered APP8 payload exceeds JPEG segment size"));
        }
        Ok(result)
    }

    pub fn parse_app8_payload(bytes: &[u8]) -> Result<Self> {
        if bytes.get(..6) != Some(b"ITUT35") {
            return Err(invalid("missing registered APP8 identity"));
        }
        if bytes.len() > 65533 {
            return Err(invalid("registered APP8 payload exceeds JPEG segment size"));
        }
        Self::parse_it35(&bytes[6..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn append_bits(target: &mut Vec<u8>, bit_len: &mut usize, value: u32, width: usize) {
        assert!(width < 32 && value < (1 << width));
        for shift in (0..width).rev() {
            if (*bit_len).is_multiple_of(8) {
                target.push(0);
            }
            let last = target.last_mut().unwrap();
            *last |= (((value >> shift) & 1) as u8) << (7 - *bit_len % 8);
            *bit_len += 1;
        }
    }

    #[test]
    fn dynamic_parameters_are_decoded_without_losing_packed_source() {
        let mut packed = Vec::new();
        let mut pos = 0;
        for (value, width) in [(1, 8), (10, 12), (20, 12), (4, 12), (30, 12)] {
            append_bits(&mut packed, &mut pos, value, width);
        }
        append_bits(&mut packed, &mut pos, 1, 1); // Tone mapping enabled.
        append_bits(&mut packed, &mut pos, 1, 1); // Two target parameters.
        append_bits(&mut packed, &mut pos, 1024, 12);
        append_bits(&mut packed, &mut pos, 1, 1); // Base curve enabled.
        for (value, width) in [
            (8192, 14),
            (15, 6),
            (500, 10),
            (4, 6),
            (200, 10),
            (2, 2),
            (1, 2),
            (9, 4),
            (3, 3),
            (64, 7),
        ] {
            append_bits(&mut packed, &mut pos, value, width);
        }
        append_bits(&mut packed, &mut pos, 1, 1); // Splines enabled.
        append_bits(&mut packed, &mut pos, 0, 1); // One spline.
        for (value, width) in [(2, 2), (7, 8), (100, 12), (45, 10), (50, 10), (125, 8)] {
            append_bits(&mut packed, &mut pos, value, width);
        }
        append_bits(&mut packed, &mut pos, 2048, 12);
        append_bits(&mut packed, &mut pos, 0, 1); // No base curve.
        append_bits(&mut packed, &mut pos, 0, 1); // No splines.
        append_bits(&mut packed, &mut pos, 1, 1); // Saturation enabled.
        append_bits(&mut packed, &mut pos, 2, 3);
        append_bits(&mut packed, &mut pos, 20, 8);
        append_bits(&mut packed, &mut pos, 30, 8);

        let dynamic = DynamicHdrMetadata::parse(&packed).unwrap();
        assert_eq!(dynamic.as_bytes(), packed);
        assert_eq!(
            DynamicHdrMetadata::from_parts(
                dynamic.statistics(),
                dynamic.tone_mappings().to_vec(),
                dynamic.saturation_gains().to_vec()
            )
            .unwrap()
            .as_bytes(),
            packed
        );
        assert_eq!(
            dynamic.statistics(),
            DynamicHdrStatistics {
                minimum_maxrgb_pq: 10,
                average_maxrgb_pq: 20,
                variance_maxrgb_pq: 4,
                maximum_maxrgb_pq: 30,
            }
        );
        assert_eq!(dynamic.tone_mappings().len(), 2);
        assert_eq!(dynamic.tone_mappings()[0].target_maximum_luminance_pq, 1024);
        assert_eq!(dynamic.tone_mappings()[0].base.unwrap().m_p, 8192);
        assert_eq!(dynamic.tone_mappings()[0].base.unwrap().delta, 64);
        assert_eq!(
            dynamic.tone_mappings()[0].splines,
            vec![DynamicHdrSpline {
                threshold_mode: 2,
                threshold_mb: Some(7),
                threshold: 100,
                delta1: 45,
                delta2: 50,
                strength: 125,
            }]
        );
        assert_eq!(dynamic.tone_mappings()[1].target_maximum_luminance_pq, 2048);
        assert!(dynamic.tone_mappings()[1].base.is_none());
        assert!(dynamic.tone_mappings()[1].splines.is_empty());
        assert_eq!(dynamic.saturation_gains(), &[20, 30]);

        let mut extra = packed.clone();
        extra.push(0);
        assert!(DynamicHdrMetadata::parse(&extra).is_err());
        let mut bad_padding = packed;
        *bad_padding.last_mut().unwrap() |= 1;
        if pos % 8 != 0 {
            assert!(DynamicHdrMetadata::parse(&bad_padding).is_err());
        }
    }

    #[test]
    fn dynamic_construction_covers_optional_modes_and_refuses_overflow() {
        let stats = DynamicHdrStatistics {
            minimum_maxrgb_pq: 1,
            average_maxrgb_pq: 2048,
            variance_maxrgb_pq: 1023,
            maximum_maxrgb_pq: 4095,
        };
        let base = DynamicHdrBaseParameters {
            m_p: 16383,
            m_m: 63,
            m_a: 1023,
            m_n: 63,
            m_b: 1023,
            k1: 3,
            k2: 3,
            k3: 15,
            delta_mode: 7,
            delta: 127,
        };
        for mode in 0..4 {
            let spline = DynamicHdrSpline {
                threshold_mode: mode,
                threshold_mb: if mode % 2 == 0 { Some(255) } else { None },
                threshold: 4095,
                delta1: 1023,
                delta2: 1023,
                strength: 255,
            };
            for with_base in [false, true] {
                for count in 0..=2 {
                    let target = DynamicHdrToneMapping {
                        target_maximum_luminance_pq: 4095,
                        base: with_base.then_some(base),
                        splines: vec![spline; count],
                    };
                    let targets = vec![target; 2];
                    let parsed = DynamicHdrMetadata::from_parts(
                        stats,
                        targets.clone(),
                        vec![0, 1, 64, 128, 200, 254, 255],
                    )
                    .unwrap();
                    assert_eq!(parsed.tone_mappings(), targets);
                    assert_eq!(parsed.statistics(), stats);
                    for end in 0..parsed.as_bytes().len() {
                        assert!(DynamicHdrMetadata::parse(&parsed.as_bytes()[..end]).is_err());
                    }
                }
            }
        }
        let minimal = DynamicHdrMetadata::from_parts(
            DynamicHdrStatistics {
                minimum_maxrgb_pq: 0,
                average_maxrgb_pq: 0,
                variance_maxrgb_pq: 0,
                maximum_maxrgb_pq: 0,
            },
            vec![],
            vec![],
        )
        .unwrap();
        assert_eq!(minimal.as_bytes(), &[1, 0, 0, 0, 0, 0, 0, 0]);
        assert!(DynamicHdrMetadata::from_parts(
            DynamicHdrStatistics {
                maximum_maxrgb_pq: 4096,
                ..stats
            },
            vec![],
            vec![]
        )
        .is_err());
        assert!(DynamicHdrMetadata::from_parts(stats, vec![], vec![0; 8]).is_err());
        let invalid = DynamicHdrToneMapping {
            target_maximum_luminance_pq: 4096,
            base: None,
            splines: vec![],
        };
        assert!(DynamicHdrMetadata::from_parts(stats, vec![invalid], vec![]).is_err());
        let invalid = DynamicHdrToneMapping {
            target_maximum_luminance_pq: 2048,
            base: Some(DynamicHdrBaseParameters { m_p: 16384, ..base }),
            splines: vec![],
        };
        assert!(DynamicHdrMetadata::from_parts(stats, vec![invalid], vec![]).is_err());
    }

    #[test]
    fn jpeg_base_declaration_roundtrips_and_rejects_other_payloads() {
        let declaration = RegisteredJpegBaseDeclaration {
            image_count: 2,
            offsets: vec![(1_234_567, 1_233_000)],
        };
        let payload = declaration.to_app8_payload().unwrap();
        assert_eq!(
            payload.len(),
            registered_jpeg_base_payload_len(2),
            "two images declare one offset pair"
        );
        assert_eq!(&payload[..6], b"ITUT35");
        assert_eq!(payload[6], 0x26);
        assert_eq!(&payload[12..16], &0x0100_0000_u32.to_be_bytes());
        assert_eq!(
            RegisteredJpegBaseDeclaration::parse_app8_payload(&payload).unwrap(),
            declaration
        );

        // The enhanced payload of the same carrier is not a base declaration.
        let registered = RegisteredHdrMetadata {
            static_metadata: None,
            dynamic_metadata: Some(DynamicHdrMetadata::parse(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap()),
            mapping_extension: None,
        };
        let enhanced = registered.to_app8_payload().unwrap();
        assert!(RegisteredJpegBaseDeclaration::parse_app8_payload(&enhanced).is_err());

        // A zero, reversed, or miscounted offset is refused.
        for offsets in [vec![(0, 0)], vec![(10, 20)], vec![(10, 5), (20, 15)]] {
            assert!(RegisteredJpegBaseDeclaration {
                image_count: 2,
                offsets,
            }
            .to_app8_payload()
            .is_err());
        }
        assert!(RegisteredJpegBaseDeclaration {
            image_count: 1,
            offsets: Vec::new(),
        }
        .to_app8_payload()
        .is_err());
        for end in 0..payload.len() {
            assert!(
                RegisteredJpegBaseDeclaration::parse_app8_payload(&payload[..end]).is_err(),
                "truncated at {end}"
            );
        }
    }

    #[test]
    fn carriers_preserve_checked_source_metadata_and_reject_bad_lengths() {
        let data = RegisteredHdrMetadata {
            static_metadata: None,
            dynamic_metadata: Some(DynamicHdrMetadata::parse(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap()),
            mapping_extension: None,
        };
        let bytes = data.to_it35_payload().unwrap();
        assert_eq!(
            bytes,
            [0x26, 0, 4, 0, 7, 2, 2, 1, 0, 0, 0, 12, 0, 0, 0, 8, 1, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(RegisteredHdrMetadata::parse_it35(&bytes).unwrap(), data);
        assert_eq!(
            RegisteredHdrMetadata::parse_app8_payload(&data.to_app8_payload().unwrap()).unwrap(),
            data
        );
        for end in 0..bytes.len() {
            assert!(RegisteredHdrMetadata::parse_it35(&bytes[..end]).is_err());
        }
        for index in [0, 2, 4, 5, 7, 9, 11, 13, 15] {
            let mut bad = bytes.clone();
            bad[index] ^= 0xff;
            assert!(
                RegisteredHdrMetadata::parse_it35(&bad).is_err(),
                "byte {index}"
            );
        }
        let mut bad = bytes.clone();
        bad[8] = 1;
        assert!(RegisteredHdrMetadata::parse_it35(&bad).is_err());
        assert!(DynamicHdrMetadata::parse(&[1, 0, 0, 0, 0, 0, 0, 1]).is_err());
        assert!(DynamicHdrMetadata::parse(&[2, 0, 0, 0, 0, 0, 0, 0]).is_err());
    }

    #[test]
    fn registered_mapping_extension_survives_both_wire_containers() {
        // Hand-assembled 16-bit matrix (identity) followed by a 2x2x2 scalar LUT.
        let mut mapping = vec![16_u8];
        for index in 0..17 {
            let value = if matches!(index, 0 | 4 | 8) {
                0x3c00_u16
            } else {
                0_u16
            };
            mapping.extend_from_slice(&value.to_be_bytes());
        }
        let mut extension = Vec::new();
        extension.extend_from_slice(&(mapping.len() as u16).to_be_bytes());
        extension.extend_from_slice(&mapping);
        extension.extend_from_slice(&0_u16.to_be_bytes());
        let data = RegisteredHdrMetadata {
            static_metadata: None,
            dynamic_metadata: Some(DynamicHdrMetadata::parse(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap()),
            mapping_extension: Some(RegisteredMappingExtension::parse(&extension).unwrap()),
        };
        let it35 = data.to_it35_payload().unwrap();
        assert_eq!(&it35[6..10], &0x0201_0100_u32.to_be_bytes());
        assert_eq!(&it35[it35.len() - extension.len()..], extension);
        assert_eq!(RegisteredHdrMetadata::parse_it35(&it35).unwrap(), data);
        assert_eq!(
            RegisteredHdrMetadata::parse_app8_payload(&data.to_app8_payload().unwrap()).unwrap(),
            data
        );

        let mut missing_extension = it35.clone();
        missing_extension.truncate(missing_extension.len() - extension.len() - 2);
        assert!(RegisteredHdrMetadata::parse_it35(&missing_extension).is_err());
        let mut unsupported_flag = it35;
        let offset = unsupported_flag.len() - extension.len() + 2;
        unsupported_flag[offset] = 8;
        assert!(RegisteredHdrMetadata::parse_it35(&unsupported_flag).is_err());
    }
}
