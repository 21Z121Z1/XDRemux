//! Bounded registered HDR metadata carriers. Dynamic payloads are checked for
//! syntax and retained exactly; applying their display mapping is separate.
use crate::{ContentLightLevel, MasteringDisplay, MetadataError, Result};

const PREFIX: &[u8] = &[0x26, 0, 4, 0, 7, 2];

fn invalid(message: &str) -> MetadataError {
    MetadataError::invalid("registered HDR metadata", message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicHdrMetadata {
    payload: Vec<u8>,
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
    fn skip(&mut self, mut n: usize) -> Result<()> {
        while n > 0 {
            let step = n.min(32);
            self.read(step)?;
            n -= step;
        }
        Ok(())
    }
}

impl DynamicHdrMetadata {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > u16::MAX as usize {
            return Err(invalid("invalid dynamic metadata length"));
        }
        let mut bits = Bits { bytes, position: 0 };
        if bits.read(8)? != 1 {
            return Err(invalid("unsupported dynamic metadata version"));
        }
        let minimum = bits.read(12)?;
        let average = bits.read(12)?;
        let _variance = bits.read(12)?;
        let maximum = bits.read(12)?;
        if minimum > average || average > maximum {
            return Err(invalid("dynamic luminance statistics are reversed"));
        }
        if bits.read(1)? != 0 {
            let count = bits.read(1)? + 1;
            for _ in 0..count {
                bits.read(12)?;
                if bits.read(1)? != 0 {
                    bits.skip(64)?;
                }
                if bits.read(1)? != 0 {
                    let splines = bits.read(1)? + 1;
                    for _ in 0..splines {
                        let mode = bits.read(2)?;
                        if mode == 0 || mode == 2 {
                            bits.read(8)?;
                        }
                        bits.skip(40)?;
                    }
                }
            }
        }
        if bits.read(1)? != 0 {
            let count = bits.read(3)?;
            bits.skip(count as usize * 8)?;
        }
        let remaining = bytes.len() * 8 - bits.position;
        if remaining > 7 || bits.read(remaining)? != 0 {
            return Err(invalid(
                "unexpected dynamic metadata tail or nonzero padding",
            ));
        }
        Ok(Self {
            payload: bytes.to_vec(),
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.payload
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
        if (kind >> 8) & 255 != 0 {
            return Err(invalid(
                "additional matrix/LUT mapping requires a supported mapping decoder",
            ));
        }
        if cursor.position != bytes.len() {
            return Err(invalid("unexpected registered metadata tail"));
        }
        if static_metadata.is_none() && dynamic_metadata.is_none() {
            return Err(invalid(
                "registered HDR metadata contains no source information",
            ));
        }
        Ok(Self {
            static_metadata,
            dynamic_metadata,
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
        result.extend_from_slice(&0x02010000_u32.to_be_bytes());
        result.extend_from_slice(&size.to_be_bytes());
        result.extend(section);
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
        assert!(RegisteredHdrMetadata::parse_it35(&bad)
            .unwrap_err()
            .to_string()
            .contains("matrix/LUT"));
        assert!(DynamicHdrMetadata::parse(&[1, 0, 0, 0, 0, 0, 0, 1]).is_err());
        assert!(DynamicHdrMetadata::parse(&[2, 0, 0, 0, 0, 0, 0, 0]).is_err());
    }
}
