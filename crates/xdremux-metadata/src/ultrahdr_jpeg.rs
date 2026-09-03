use std::collections::BTreeMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;

use crate::{MetadataError, Result};

const ISO_NAMESPACE: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";
const XMP_CONTEXT: &str = "Ultra HDR hdrgm XMP";
const ISO_CONTEXT: &str = "ISO 21496-1 JPEG metadata";
const MAX_XMP_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct UltraHdrGainMapMetadata {
    /// Adobe/ISO gain-map values are expressed in log2 stops.
    pub gain_map_min: [f64; 3],
    pub gain_map_max: [f64; 3],
    pub gamma: [f64; 3],
    pub offset_sdr: [f64; 3],
    pub offset_hdr: [f64; 3],
    pub hdr_capacity_min: f64,
    pub hdr_capacity_max: f64,
    pub channel_count: u8,
    pub use_base_color_space: bool,
}

impl UltraHdrGainMapMetadata {
    /// Translate public Ultra HDR log-domain metadata into XDRemux's canonical
    /// 20-float linear ratio representation consumed by the ISO HEIF writer.
    pub fn to_info_floats(&self) -> Result<[f64; 20]> {
        self.validate()?;
        let mut info = [0.0_f64; 20];
        for channel in 0..3 {
            info[channel] = 2.0_f64.powf(self.gain_map_min[channel]);
            info[4 + channel] = 2.0_f64.powf(self.gain_map_max[channel]);
            info[7 + channel] = self.gamma[channel];
            info[10 + channel] = self.offset_sdr[channel];
            info[13 + channel] = self.offset_hdr[channel];
        }
        info[3] = 1.0;
        info[16] = 2.0_f64.powf(self.hdr_capacity_min);
        info[17] = 2.0_f64.powf(self.hdr_capacity_max);
        info[18] = info[17];
        info[19] = 0.0;
        if info.iter().any(|value| !value.is_finite()) {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "metadata overflows the canonical linear representation",
            ));
        }
        Ok(info)
    }

    fn validate(&self) -> Result<()> {
        if !matches!(self.channel_count, 1 | 3) {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                format!("unsupported gain-map channel count {}", self.channel_count),
            ));
        }
        if self
            .gain_map_min
            .iter()
            .chain(self.gain_map_max.iter())
            .chain(self.gamma.iter())
            .chain(self.offset_sdr.iter())
            .chain(self.offset_hdr.iter())
            .chain([&self.hdr_capacity_min, &self.hdr_capacity_max])
            .any(|value| !value.is_finite())
        {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "gain-map metadata contains a non-finite number",
            ));
        }
        if self.gamma.iter().any(|value| *value <= 0.0) {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "gain-map gamma must be positive",
            ));
        }
        Ok(())
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .and_then(|offset| start.checked_add(offset))
}

fn extract_xmp(jpeg: &[u8]) -> Result<Option<&[u8]>> {
    let scan = &jpeg[..jpeg.len().min(MAX_XMP_BYTES)];
    let start = [b"<x:xmpmeta".as_slice(), b"<xmpmeta".as_slice()]
        .into_iter()
        .filter_map(|needle| find_bytes(scan, needle, 0))
        .min();
    let Some(start) = start else {
        return Ok(None);
    };
    let end = [b"</x:xmpmeta>".as_slice(), b"</xmpmeta>".as_slice()]
        .into_iter()
        .filter_map(|closing| {
            find_bytes(scan, closing, start)
                .and_then(|position| position.checked_add(closing.len()))
        })
        .min()
        .ok_or_else(|| MetadataError::invalid(XMP_CONTEXT, "XMP packet is not terminated"))?;
    Ok(Some(&scan[start..end]))
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn attributes(element: &BytesStart<'_>) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute
            .map_err(|_| MetadataError::invalid(XMP_CONTEXT, "malformed XML attribute"))?;
        let key = local_name(attribute.key.as_ref());
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| MetadataError::invalid(XMP_CONTEXT, "invalid XML attribute value"))?;
        values.insert(key.to_owned(), value.into_owned());
    }
    Ok(values)
}

fn parse_number_list(value: &str) -> Result<Vec<f64>> {
    let values = value
        .split(|character: char| character == ',' || character.is_ascii_whitespace())
        .filter(|piece| !piece.is_empty())
        .map(|piece| {
            piece.parse::<f64>().map_err(|_| {
                MetadataError::invalid(XMP_CONTEXT, format!("invalid numeric value {piece:?}"))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(MetadataError::invalid(
            XMP_CONTEXT,
            "numeric field is empty or non-finite",
        ));
    }
    Ok(values)
}

fn expand_three(values: Option<&Vec<f64>>, default: f64, field: &str) -> Result<([f64; 3], u8)> {
    let Some(values) = values else {
        return Ok(([default; 3], 1));
    };
    match values.as_slice() {
        [value] => Ok(([*value; 3], 1)),
        [a, b, c] => Ok(([*a, *b, *c], 3)),
        _ => Err(MetadataError::invalid(
            XMP_CONTEXT,
            format!("{field} must contain one or three values"),
        )),
    }
}

fn parse_hdrgm_xmp(xmp: &[u8]) -> Result<Option<UltraHdrGainMapMetadata>> {
    let text = std::str::from_utf8(xmp)
        .map_err(|_| MetadataError::invalid(XMP_CONTEXT, "XMP is not UTF-8"))?;
    let upper = text.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err(MetadataError::invalid(
            XMP_CONTEXT,
            "DTD/entity declarations are forbidden",
        ));
    }

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut numeric: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut base_is_hdr = false;
    let mut active_field: Option<String> = None;

    loop {
        match reader
            .read_event()
            .map_err(|_| MetadataError::invalid(XMP_CONTEXT, "malformed XML"))?
        {
            Event::Start(element) | Event::Empty(element) => {
                let name = local_name(element.name().as_ref()).to_owned();
                for (key, value) in attributes(&element)? {
                    if is_numeric_field(&key) {
                        numeric.insert(key, parse_number_list(&value)?);
                    } else if key == "BaseRenditionIsHDR" {
                        base_is_hdr = parse_boolean(&value);
                    }
                }
                if is_numeric_field(&name) || name == "BaseRenditionIsHDR" {
                    active_field = Some(name);
                }
            }
            Event::Text(text) => {
                let Some(field) = active_field.as_deref() else {
                    continue;
                };
                let value = text.as_ref().trim();
                if value.is_empty() {
                    continue;
                }
                if field == "BaseRenditionIsHDR" {
                    base_is_hdr = parse_boolean(value);
                } else if is_numeric_field(field) {
                    let parsed = parse_number_list(value)?;
                    numeric.entry(field.to_owned()).or_default().extend(parsed);
                }
            }
            Event::End(element) => {
                let name = local_name(element.name().as_ref()).to_owned();
                if active_field.as_deref() == Some(name.as_str()) {
                    active_field = None;
                }
            }
            Event::DocType(_) => {
                return Err(MetadataError::invalid(
                    XMP_CONTEXT,
                    "DTD declarations are forbidden",
                ));
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let Some(gain_max_values) = numeric.get("GainMapMax") else {
        return Ok(None);
    };
    if base_is_hdr {
        return Err(MetadataError::invalid(
            XMP_CONTEXT,
            "base-HDR gain-map direction is unsupported for JPEG Ultra HDR",
        ));
    }
    let (gain_min, min_channels) = expand_three(numeric.get("GainMapMin"), 0.0, "GainMapMin")?;
    let (gain_max, max_channels) = expand_three(Some(gain_max_values), 1.0, "GainMapMax")?;
    let (gamma, gamma_channels) = expand_three(numeric.get("Gamma"), 1.0, "Gamma")?;
    let (offset_sdr, sdr_channels) = expand_three(numeric.get("OffsetSDR"), 0.0, "OffsetSDR")?;
    let (offset_hdr, hdr_channels) = expand_three(numeric.get("OffsetHDR"), 0.0, "OffsetHDR")?;
    let channel_count = [
        min_channels,
        max_channels,
        gamma_channels,
        sdr_channels,
        hdr_channels,
    ]
    .into_iter()
    .max()
    .unwrap_or(1);
    let hdr_capacity_min = numeric
        .get("HDRCapacityMin")
        .and_then(|values| values.first())
        .copied()
        .unwrap_or(0.0);
    let hdr_capacity_max = numeric
        .get("HDRCapacityMax")
        .and_then(|values| values.first())
        .copied()
        .unwrap_or(gain_max[0]);
    let metadata = UltraHdrGainMapMetadata {
        gain_map_min: gain_min,
        gain_map_max: gain_max,
        gamma,
        offset_sdr,
        offset_hdr,
        hdr_capacity_min,
        hdr_capacity_max,
        channel_count,
        use_base_color_space: true,
    };
    metadata.validate()?;
    Ok(Some(metadata))
}

fn is_numeric_field(name: &str) -> bool {
    matches!(
        name,
        "GainMapMin"
            | "GainMapMax"
            | "Gamma"
            | "OffsetSDR"
            | "OffsetHDR"
            | "HDRCapacityMin"
            | "HDRCapacityMax"
    )
}

fn parse_boolean(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes"
    )
}

struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self
            .position
            .checked_add(N)
            .ok_or_else(|| MetadataError::overflow(ISO_CONTEXT))?;
        let bytes = self.data.get(self.position..end).ok_or_else(|| {
            MetadataError::invalid(ISO_CONTEXT, "truncated gain-map metadata payload")
        })?;
        self.position = end;
        Ok(bytes.try_into().expect("fixed-size metadata field"))
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take::<2>()?))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take::<4>()?))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.take::<4>()?))
    }
}

fn fraction_signed(numerator: i32, denominator: u32, field: &str) -> Result<f64> {
    if denominator == 0 {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            format!("zero denominator in {field}"),
        ));
    }
    Ok(f64::from(numerator) / f64::from(denominator))
}

fn fraction_unsigned(numerator: u32, denominator: u32, field: &str) -> Result<f64> {
    if denominator == 0 {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            format!("zero denominator in {field}"),
        ));
    }
    Ok(f64::from(numerator) / f64::from(denominator))
}

fn decode_iso_payload(payload: &[u8]) -> Result<UltraHdrGainMapMetadata> {
    let mut cursor = Cursor::new(payload);
    let minimum_version = cursor.u16()?;
    let _writer_version = cursor.u16()?;
    if minimum_version != 0 {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            format!("unsupported minimum version {minimum_version}"),
        ));
    }
    let flags = cursor.u8()?;
    let channel_count = if flags & 0x01 != 0 { 3 } else { 1 };
    let use_base_color_space = flags & 0x02 != 0;
    if flags & 0x04 != 0 {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            "backward-direction metadata is unsupported for JPEG Ultra HDR",
        ));
    }
    let common_denominator = flags & 0x08 != 0;

    let (base_headroom, alternate_headroom, gain_min, gain_max, gamma, base_offset, alt_offset) =
        if common_denominator {
            let denominator = cursor.u32()?;
            if denominator == 0 {
                return Err(MetadataError::invalid(
                    ISO_CONTEXT,
                    "zero common denominator",
                ));
            }
            let base_headroom = fraction_unsigned(cursor.u32()?, denominator, "baseHdrHeadroom")?;
            let alternate_headroom =
                fraction_unsigned(cursor.u32()?, denominator, "alternateHdrHeadroom")?;
            let mut gain_min = Vec::new();
            let mut gain_max = Vec::new();
            let mut gamma = Vec::new();
            let mut base_offset = Vec::new();
            let mut alt_offset = Vec::new();
            for channel in 0..channel_count {
                gain_min.push(fraction_signed(
                    cursor.i32()?,
                    denominator,
                    &format!("gainMapMin[{channel}]"),
                )?);
                gain_max.push(fraction_signed(
                    cursor.i32()?,
                    denominator,
                    &format!("gainMapMax[{channel}]"),
                )?);
                gamma.push(fraction_unsigned(
                    cursor.u32()?,
                    denominator,
                    &format!("gamma[{channel}]"),
                )?);
                base_offset.push(fraction_signed(
                    cursor.i32()?,
                    denominator,
                    &format!("baseOffset[{channel}]"),
                )?);
                alt_offset.push(fraction_signed(
                    cursor.i32()?,
                    denominator,
                    &format!("alternateOffset[{channel}]"),
                )?);
            }
            (
                base_headroom,
                alternate_headroom,
                gain_min,
                gain_max,
                gamma,
                base_offset,
                alt_offset,
            )
        } else {
            let base_headroom = fraction_unsigned(cursor.u32()?, cursor.u32()?, "baseHdrHeadroom")?;
            let alternate_headroom =
                fraction_unsigned(cursor.u32()?, cursor.u32()?, "alternateHdrHeadroom")?;
            let mut gain_min = Vec::new();
            let mut gain_max = Vec::new();
            let mut gamma = Vec::new();
            let mut base_offset = Vec::new();
            let mut alt_offset = Vec::new();
            for channel in 0..channel_count {
                gain_min.push(fraction_signed(
                    cursor.i32()?,
                    cursor.u32()?,
                    &format!("gainMapMin[{channel}]"),
                )?);
                gain_max.push(fraction_signed(
                    cursor.i32()?,
                    cursor.u32()?,
                    &format!("gainMapMax[{channel}]"),
                )?);
                gamma.push(fraction_unsigned(
                    cursor.u32()?,
                    cursor.u32()?,
                    &format!("gamma[{channel}]"),
                )?);
                base_offset.push(fraction_signed(
                    cursor.i32()?,
                    cursor.u32()?,
                    &format!("baseOffset[{channel}]"),
                )?);
                alt_offset.push(fraction_signed(
                    cursor.i32()?,
                    cursor.u32()?,
                    &format!("alternateOffset[{channel}]"),
                )?);
            }
            (
                base_headroom,
                alternate_headroom,
                gain_min,
                gain_max,
                gamma,
                base_offset,
                alt_offset,
            )
        };

    fn three(values: &[f64]) -> [f64; 3] {
        match values {
            [value] => [*value; 3],
            [a, b, c] => [*a, *b, *c],
            _ => unreachable!("ISO parser only creates one or three channels"),
        }
    }

    let metadata = UltraHdrGainMapMetadata {
        gain_map_min: three(&gain_min),
        gain_map_max: three(&gain_max),
        gamma: three(&gamma),
        offset_sdr: three(&base_offset),
        offset_hdr: three(&alt_offset),
        hdr_capacity_min: base_headroom,
        hdr_capacity_max: alternate_headroom,
        channel_count,
        use_base_color_space,
    };
    metadata.validate()?;
    Ok(metadata)
}

fn parse_iso_jpeg(jpeg: &[u8]) -> Result<Option<UltraHdrGainMapMetadata>> {
    if jpeg.get(..2) != Some(&[0xff, 0xd8]) {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            "gain-map resource is not a JPEG",
        ));
    }
    let mut cursor = 2usize;
    while cursor < jpeg.len() {
        if jpeg.get(cursor) != Some(&0xff) {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "malformed JPEG marker stream before SOS",
            ));
        }
        while jpeg.get(cursor) == Some(&0xff) {
            cursor = cursor
                .checked_add(1)
                .ok_or_else(|| MetadataError::overflow(ISO_CONTEXT))?;
        }
        let marker = *jpeg
            .get(cursor)
            .ok_or_else(|| MetadataError::invalid(ISO_CONTEXT, "truncated JPEG marker"))?;
        cursor = cursor
            .checked_add(1)
            .ok_or_else(|| MetadataError::overflow(ISO_CONTEXT))?;
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if marker == 0x01 || marker == 0xd8 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let length_end = cursor
            .checked_add(2)
            .ok_or_else(|| MetadataError::overflow(ISO_CONTEXT))?;
        let length_bytes = jpeg
            .get(cursor..length_end)
            .ok_or_else(|| MetadataError::invalid(ISO_CONTEXT, "truncated JPEG segment length"))?;
        let length = usize::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
        if length < 2 {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "invalid JPEG segment length",
            ));
        }
        let payload_start = length_end;
        let segment_end = cursor
            .checked_add(length)
            .ok_or_else(|| MetadataError::overflow(ISO_CONTEXT))?;
        let payload = jpeg.get(payload_start..segment_end).ok_or_else(|| {
            MetadataError::invalid(ISO_CONTEXT, "JPEG segment exceeds gain-map resource")
        })?;
        if marker == 0xe2 && payload.starts_with(ISO_NAMESPACE) {
            return decode_iso_payload(&payload[ISO_NAMESPACE.len()..]).map(Some);
        }
        cursor = segment_end;
    }
    Ok(None)
}

/// Parse public gain-map metadata from a compressed Ultra HDR gain-map JPEG.
///
/// Adobe hdrgm XMP is preferred when present, matching existing product
/// semantics. ISO 21496-1 APP2 is the standards fallback used by newer Android
/// encoders. A JPEG containing neither form is not considered a gain map.
pub fn parse_ultrahdr_gain_map_metadata(jpeg: &[u8]) -> Result<Option<UltraHdrGainMapMetadata>> {
    if let Some(xmp) = extract_xmp(jpeg)? {
        if let Some(metadata) = parse_hdrgm_xmp(xmp)? {
            return Ok(Some(metadata));
        }
    }
    parse_iso_jpeg(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso_payload(flags: u8) -> Vec<u8> {
        let denominator = 1000_u32;
        let mut output = Vec::new();
        output.extend_from_slice(&0_u16.to_be_bytes());
        output.extend_from_slice(&0_u16.to_be_bytes());
        output.push(flags | 0x08);
        output.extend_from_slice(&denominator.to_be_bytes());
        output.extend_from_slice(&0_u32.to_be_bytes());
        output.extend_from_slice(&2000_u32.to_be_bytes());
        let channels = if flags & 0x01 != 0 { 3 } else { 1 };
        for channel in 0..channels {
            output.extend_from_slice(&0_i32.to_be_bytes());
            output.extend_from_slice(&(1000_i32 + channel * 100).to_be_bytes());
            output.extend_from_slice(&1000_u32.to_be_bytes());
            output.extend_from_slice(&0_i32.to_be_bytes());
            output.extend_from_slice(&0_i32.to_be_bytes());
        }
        output
    }

    #[test]
    fn decodes_iso_common_denominator_metadata() {
        let metadata = decode_iso_payload(&iso_payload(0x01 | 0x02)).unwrap();
        assert_eq!(metadata.channel_count, 3);
        assert_eq!(metadata.gain_map_max, [1.0, 1.1, 1.2]);
        assert_eq!(metadata.hdr_capacity_max, 2.0);
        assert!(metadata.use_base_color_space);
        let info = metadata.to_info_floats().unwrap();
        assert_eq!(info[0], 1.0);
        assert!((info[4] - 2.0).abs() < 1e-12);
        assert!((info[17] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn parses_hdrgm_scalar_values() {
        let xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description hdrgm:GainMapMax="2.0" hdrgm:Gamma="1.0" hdrgm:HDRCapacityMax="2.0"/></rdf:RDF></x:xmpmeta>"#;
        let metadata = parse_hdrgm_xmp(xmp).unwrap().unwrap();
        assert_eq!(metadata.channel_count, 1);
        assert_eq!(metadata.gain_map_max, [2.0; 3]);
        assert_eq!(metadata.gamma, [1.0; 3]);
    }

    #[test]
    fn rejects_backward_iso_direction() {
        assert!(decode_iso_payload(&iso_payload(0x04)).is_err());
    }
}
