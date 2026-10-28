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
    /// ISO APP2 metadata defines an alternate-image ICC binding. Android
    /// hdrgm instead defines HDR colorimetry from the primary image profile.
    pub uses_iso_colorimetry: bool,
    // Required ISO bytes retain the exact signed/unsigned rational fields.
    // Optional future tails are ignored, as permitted by Annex C.2.1.
    iso_payload: Option<Vec<u8>>,
}

impl UltraHdrGainMapMetadata {
    /// Remux ISO rationals directly. Only the application-space flag may be
    /// resolved by the caller (C.4.7); XMP uses the checked float writer.
    pub fn to_tmap_payload(&self, use_base_colour_space: bool) -> Result<Vec<u8>> {
        self.validate()?;
        if let Some(source) = &self.iso_payload {
            let mut payload = vec![0]; // HEIF ToneMapImage version
            payload.extend_from_slice(source);
            payload[5] = (payload[5] & !0x40) | if use_base_colour_space { 0x40 } else { 0 };
            decode_iso_payload(&payload[1..])?;
            return Ok(payload);
        }
        crate::iso21496::make_tmap_from_log_metadata(self, use_base_colour_space)
    }

    /// XMP is a floating-point compatibility projection; the ISO rationals
    /// remain authoritative. Avoid exp2/log2 overflow and decimal truncation.
    pub fn to_hdrgm_xmp(&self) -> Result<Vec<u8>> {
        self.validate()?;
        if let Some(source) = &self.iso_payload {
            let pair = |offset| {
                (
                    u32::from_be_bytes(
                        source[offset..offset + 4]
                            .try_into()
                            .expect("validated headroom"),
                    ),
                    u32::from_be_bytes(
                        source[offset + 4..offset + 8]
                            .try_into()
                            .expect("validated headroom"),
                    ),
                )
            };
            let (base, alternate) = (pair(5), pair(13));
            return Ok(crate::iso21496::write_hdrgm_xmp_with_headrooms(
                self.gain_map_min,
                self.gain_map_max,
                self.gamma,
                [self.offset_sdr, self.offset_hdr],
                [decimal_headroom(base), decimal_headroom(alternate)],
                u64::from(base.0) * u64::from(alternate.1)
                    > u64::from(alternate.0) * u64::from(base.1),
            ));
        }
        Ok(crate::iso21496::write_hdrgm_xmp(
            self.gain_map_min,
            self.gain_map_max,
            self.gamma,
            [self.offset_sdr, self.offset_hdr],
            [self.hdr_capacity_min, self.hdr_capacity_max],
        ))
    }

    /// Float compatibility view for calculations and diagnostics. ISO remux
    /// must use to_tmap_payload to retain source rational values and ordering.
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
        if info.iter().any(|value| !value.is_finite())
            || info[..3]
                .iter()
                .chain(&info[4..7])
                .any(|value| *value <= 0.0)
            || info[16] == info[17]
        {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "metadata cannot retain its constraints in the float compatibility representation",
            ));
        }
        Ok(info)
    }

    fn validate(&self) -> Result<()> {
        if let Some(source) = &self.iso_payload {
            // Public float observations must not be edited independently of
            // the exact source facts that will be serialized.
            let mut expected = decode_iso_payload(source)?;
            expected.iso_payload = None;
            let mut observed = self.clone();
            observed.iso_payload = None;
            if observed != expected {
                return Err(MetadataError::invalid(
                    ISO_CONTEXT,
                    "ISO observations differ from source rational values",
                ));
            }
            return Ok(());
        }
        if self.uses_iso_colorimetry {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "ISO metadata lacks source rational values",
            ));
        }
        let context = XMP_CONTEXT;
        if !matches!(self.channel_count, 1 | 3) {
            return Err(MetadataError::invalid(
                context,
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
                context,
                "gain-map metadata contains a non-finite number",
            ));
        }
        if self.gamma.iter().any(|value| *value <= 0.0) {
            return Err(MetadataError::invalid(
                context,
                "gain-map gamma must be positive",
            ));
        }
        for channel in 0..usize::from(self.channel_count) {
            if self.gain_map_min[channel] > self.gain_map_max[channel] {
                return Err(MetadataError::invalid(
                    context,
                    format!("gainMapMin[{channel}] exceeds gainMapMax[{channel}]"),
                ));
            }
        }
        if self.hdr_capacity_min < 0.0 || self.hdr_capacity_max <= self.hdr_capacity_min {
            return Err(MetadataError::invalid(
                context,
                "HDRCapacityMin must be nonnegative and HDRCapacityMax must exceed HDRCapacityMin",
            ));
        }
        Ok(())
    }
}

// Two distinct u32/u32 values differ by at least 1/(u32::MAX^2).
// Forty decimal places preserve that ordering in XMP text even when f64 cannot.
fn decimal_headroom((numerator, denominator): (u32, u32)) -> String {
    let mut value = (numerator / denominator).to_string();
    let mut remainder = u64::from(numerator % denominator);
    if remainder != 0 {
        value.push('.');
    }
    for _ in 0..40 {
        if remainder == 0 {
            break;
        }
        remainder *= 10;
        value.push(char::from(
            b'0' + (remainder / u64::from(denominator)) as u8,
        ));
        remainder %= u64::from(denominator);
    }
    value
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
        uses_iso_colorimetry: false,
        iso_payload: None,
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
    // ISO 21496-1:2025, Annex C.2.2: flag bits are in MSB order.
    let flags = cursor.u8()?;
    if flags & 0x3f != 0 {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            format!("nonzero reserved gain-map flags 0x{flags:02x}"),
        ));
    }
    let channel_count: u8 = if flags & 0x80 != 0 { 3 } else { 1 };
    let use_base_color_space = flags & 0x40 != 0;

    // Each rational carries its own denominator; the 2025 layout has
    // no optional common-denominator or reverse-direction fields.
    let base = (cursor.u32()?, cursor.u32()?);
    let alternate = (cursor.u32()?, cursor.u32()?);
    let base_headroom = fraction_unsigned(base.0, base.1, "baseHdrHeadroom")?;
    let alternate_headroom = fraction_unsigned(alternate.0, alternate.1, "alternateHdrHeadroom")?;
    if u64::from(base.0) * u64::from(alternate.1) == u64::from(alternate.0) * u64::from(base.1) {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            "baseline and alternate HDR headrooms must differ",
        ));
    }
    let mut gain_min = Vec::with_capacity(usize::from(channel_count));
    let mut gain_max = Vec::with_capacity(usize::from(channel_count));
    let mut gamma = Vec::with_capacity(usize::from(channel_count));
    let mut base_offset = Vec::with_capacity(usize::from(channel_count));
    let mut alt_offset = Vec::with_capacity(usize::from(channel_count));
    for channel in 0..channel_count {
        let minimum = (cursor.i32()?, cursor.u32()?);
        let maximum = (cursor.i32()?, cursor.u32()?);
        gain_min.push(fraction_signed(
            minimum.0,
            minimum.1,
            &format!("gainMapMin[{channel}]"),
        )?);
        gain_max.push(fraction_signed(
            maximum.0,
            maximum.1,
            &format!("gainMapMax[{channel}]"),
        )?);
        // i32 * u32 fits i64, including i32::MIN and u32::MAX.
        if i64::from(minimum.0) * i64::from(maximum.1) > i64::from(maximum.0) * i64::from(minimum.1)
        {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                format!("gainMapMin[{channel}] exceeds gainMapMax[{channel}]"),
            ));
        }
        let gamma_value =
            fraction_unsigned(cursor.u32()?, cursor.u32()?, &format!("gamma[{channel}]"))?;
        if gamma_value <= 0.0 {
            return Err(MetadataError::invalid(
                ISO_CONTEXT,
                "gain-map gamma must be positive",
            ));
        }
        gamma.push(gamma_value);
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

    fn three(values: &[f64]) -> [f64; 3] {
        match values {
            [value] => [*value; 3],
            [a, b, c] => [*a, *b, *c],
            _ => unreachable!("ISO parser only creates one or three channels"),
        }
    }

    Ok(UltraHdrGainMapMetadata {
        gain_map_min: three(&gain_min),
        gain_map_max: three(&gain_max),
        gamma: three(&gamma),
        offset_sdr: three(&base_offset),
        offset_hdr: three(&alt_offset),
        hdr_capacity_min: base_headroom,
        hdr_capacity_max: alternate_headroom,
        channel_count,
        use_base_color_space,
        uses_iso_colorimetry: true,
        iso_payload: Some(payload[..cursor.position].to_vec()),
    })
}

/// Check the actual HEIF metadata bytes after a writer has changed representation.
pub(crate) fn validate_tmap_payload(payload: &[u8]) -> Result<()> {
    if payload.first() != Some(&0) {
        return Err(MetadataError::invalid(
            ISO_CONTEXT,
            "unsupported HEIF tone-map version",
        ));
    }
    decode_iso_payload(&payload[1..]).map(|_| ())
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
/// Prefer ISO APP2 as recommended by Android's cross-format compatibility guide.
/// An invalid or unsupported ISO payload is an error under XDRemux's conversion
/// policy, even when XMP is valid. Only absent ISO permits the Android hdrgm path.
/// The caller must pass the located gain JPEG, not the baseline's version-only APP2.
pub fn parse_ultrahdr_gain_map_metadata(jpeg: &[u8]) -> Result<Option<UltraHdrGainMapMetadata>> {
    if let Some(metadata) = parse_iso_jpeg(jpeg)? {
        return Ok(Some(metadata));
    }
    if let Some(xmp) = extract_xmp(jpeg)? {
        return parse_hdrgm_xmp(xmp);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso_payload(flags: u8) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(&0_u16.to_be_bytes());
        output.extend_from_slice(&0_u16.to_be_bytes());
        output.push(flags);
        output.extend_from_slice(&0_u32.to_be_bytes());
        output.extend_from_slice(&1_u32.to_be_bytes());
        output.extend_from_slice(&2_u32.to_be_bytes());
        output.extend_from_slice(&1_u32.to_be_bytes());
        let channels = if flags & 0x80 != 0 { 3 } else { 1 };
        for channel in 0..channels {
            output.extend_from_slice(&0_i32.to_be_bytes());
            output.extend_from_slice(&1_u32.to_be_bytes());
            output.extend_from_slice(&(1000_i32 + channel * 100).to_be_bytes());
            output.extend_from_slice(&1000_u32.to_be_bytes());
            output.extend_from_slice(&1_u32.to_be_bytes());
            output.extend_from_slice(&1_u32.to_be_bytes());
            output.extend_from_slice(&0_i32.to_be_bytes());
            output.extend_from_slice(&1_u32.to_be_bytes());
            output.extend_from_slice(&0_i32.to_be_bytes());
            output.extend_from_slice(&1_u32.to_be_bytes());
        }
        output
    }

    #[test]
    fn decodes_iso_multichannel_metadata() {
        let metadata = decode_iso_payload(&iso_payload(0x80 | 0x40)).unwrap();
        assert_eq!(metadata.channel_count, 3);
        assert_eq!(metadata.gain_map_max, [1.0, 1.1, 1.2]);
        assert_eq!(metadata.hdr_capacity_max, 2.0);
        assert!(metadata.use_base_color_space);
        assert!(metadata.uses_iso_colorimetry);
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
        assert!(!metadata.uses_iso_colorimetry);
    }

    #[test]
    fn rejects_nonzero_iso_reserved_bits() {
        assert!(decode_iso_payload(&iso_payload(0x04)).is_err());
    }

    const XMP: &[u8] = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/"><description hdrgm:GainMapMax="2" hdrgm:HDRCapacityMax="2"/></x:xmpmeta>"#;

    fn gain_jpeg(iso: Option<&[u8]>, xmp: Option<&[u8]>) -> Vec<u8> {
        let mut jpeg = vec![0xff, 0xd8];
        for (marker, prefix, payload) in [
            (0xe1, b"http://ns.adobe.com/xap/1.0/\0".as_slice(), xmp),
            (0xe2, ISO_NAMESPACE, iso),
        ] {
            if let Some(payload) = payload {
                jpeg.extend_from_slice(&[0xff, marker]);
                jpeg.extend_from_slice(
                    &u16::try_from(prefix.len() + payload.len() + 2)
                        .unwrap()
                        .to_be_bytes(),
                );
                jpeg.extend_from_slice(prefix);
                jpeg.extend_from_slice(payload);
            }
        }
        jpeg.extend_from_slice(&[0xff, 0xd9]);
        jpeg
    }

    #[test]
    fn gain_jpeg_selects_iso_values_and_colorimetry_before_reading_xmp() {
        for flags in [0, 0x40, 0x80, 0xc0] {
            let iso = iso_payload(flags);
            let expected = decode_iso_payload(&iso).unwrap();
            // XMP conflicts with ISO's gains and flag; malformed XMP must also
            // be irrelevant when the gain JPEG supplies valid ISO metadata.
            for xmp in [Some(XMP), Some(b"<x:xmpmeta broken".as_slice()), None] {
                let parsed = parse_ultrahdr_gain_map_metadata(&gain_jpeg(Some(&iso), xmp))
                    .unwrap()
                    .unwrap();
                assert_eq!(parsed, expected);
            }
        }
        let parsed = parse_ultrahdr_gain_map_metadata(&gain_jpeg(None, Some(XMP)))
            .unwrap()
            .unwrap();
        assert!(!parsed.uses_iso_colorimetry);
        assert!(parsed.use_base_color_space);
        assert_eq!(parsed.gain_map_max, [2.0; 3]);
        assert!(parse_ultrahdr_gain_map_metadata(&gain_jpeg(None, None))
            .unwrap()
            .is_none());
    }

    #[test]
    fn invalid_iso_never_falls_back_to_valid_xmp() {
        let valid = iso_payload(0x40);
        let mut unsupported = valid.clone();
        unsupported[..2].copy_from_slice(&1_u16.to_be_bytes());
        let mut reserved = valid.clone();
        reserved[4] |= 1;
        let mut zero_denominator = valid.clone();
        zero_denominator[9..13].fill(0);
        let mut inverted_gain = valid.clone();
        inverted_gain[21..25].copy_from_slice(&2_i32.to_be_bytes());
        let mut equal_headrooms = valid.clone();
        equal_headrooms[13..17].fill(0);
        for invalid in [
            &valid[..4],
            &valid[..valid.len() - 1],
            &unsupported,
            &reserved,
            &zero_denominator,
            &inverted_gain,
            &equal_headrooms,
        ] {
            let error = parse_ultrahdr_gain_map_metadata(&gain_jpeg(Some(invalid), Some(XMP)))
                .expect_err("declared invalid ISO must fail even when XMP is usable");
            assert!(error.to_string().contains(ISO_CONTEXT), "{error}");
        }
    }

    #[test]
    fn iso_checks_each_gain_interval_and_distinct_headrooms() {
        for flags in [0, 0x80] {
            for channel in 0..if flags == 0 { 1 } else { 3 } {
                let mut payload = iso_payload(flags);
                let offset = 21 + channel * 40;
                payload[offset..offset + 4].copy_from_slice(&2_i32.to_be_bytes());
                assert!(decode_iso_payload(&payload).is_err(), "channel {channel}");
            }
        }
        let mut payload = iso_payload(0);
        payload[13..17].fill(0);
        assert!(decode_iso_payload(&payload).is_err());
        // ISO permits the HDR baseline -> SDR alternate direction. It also
        // permits a constant gain interval, signed offsets and future tails.
        payload[5..9].copy_from_slice(&2_u32.to_be_bytes());
        payload[29..33].copy_from_slice(&0_i32.to_be_bytes());
        payload[45..49].copy_from_slice(&(-1_i32).to_be_bytes());
        payload[49..53].copy_from_slice(&4_u32.to_be_bytes());
        payload[2..4].copy_from_slice(&7_u16.to_be_bytes());
        let expected = decode_iso_payload(&payload).unwrap();
        assert_eq!(expected.hdr_capacity_min, 2.0);
        assert_eq!(expected.hdr_capacity_max, 0.0);
        assert_eq!(expected.gain_map_min, expected.gain_map_max);
        assert_eq!(expected.offset_sdr, [-0.25; 3]);
        payload.extend_from_slice(&[0xff, 0xff, 1, 2, 3]);
        assert_eq!(decode_iso_payload(&payload).unwrap(), expected);
    }

    #[test]
    fn exact_iso_ordering_and_headrooms_are_not_decided_by_f64() {
        let mut payload = iso_payload(0);
        for (offset, value) in [
            (21, 2_147_483_646_u32),
            (25, 2_147_483_647),
            (29, 2_147_483_645),
            (33, 2_147_483_646),
        ] {
            payload[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        assert!(decode_iso_payload(&payload)
            .unwrap_err()
            .to_string()
            .contains("gainMapMin"));
        // The reverse ordering is valid and must retain all rational bytes.
        payload[21..25].copy_from_slice(&2_147_483_645_u32.to_be_bytes());
        payload[25..29].copy_from_slice(&2_147_483_646_u32.to_be_bytes());
        payload[29..33].copy_from_slice(&2_147_483_646_u32.to_be_bytes());
        payload[33..37].copy_from_slice(&2_147_483_647_u32.to_be_bytes());
        for (offset, value) in [
            (5, 2_147_483_645_u32),
            (9, 2_147_483_646),
            (13, 2_147_483_646),
            (17, 2_147_483_647),
        ] {
            payload[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        let metadata = decode_iso_payload(&payload).unwrap();
        assert_eq!(metadata.hdr_capacity_min, metadata.hdr_capacity_max);
        assert!(metadata.to_info_floats().is_err());
        assert_eq!(&metadata.to_tmap_payload(false).unwrap()[1..], payload);
        let xmp = String::from_utf8(metadata.to_hdrgm_xmp().unwrap()).unwrap();
        assert!(xmp.contains("<hdrgm:HDRCapacityMin>0.9999999995343387122585798727"));
        assert!(xmp.contains("<hdrgm:HDRCapacityMax>0.9999999995343387124754203075"));
        let mut reverse = payload.clone();
        reverse[5..13].copy_from_slice(&payload[13..21]);
        reverse[13..21].copy_from_slice(&payload[5..13]);
        let xmp = String::from_utf8(
            decode_iso_payload(&reverse)
                .unwrap()
                .to_hdrgm_xmp()
                .unwrap(),
        )
        .unwrap();
        assert!(xmp.contains("<hdrgm:BaseRenditionIsHDR>True</hdrgm:BaseRenditionIsHDR>"));
        let mut edited = metadata.clone();
        edited.gamma[0] = 2.;
        assert!(edited.to_tmap_payload(false).is_err());
        // Equal values expressed with different denominators are still invalid.
        payload[5..9].copy_from_slice(&1_u32.to_be_bytes());
        payload[9..13].copy_from_slice(&2_u32.to_be_bytes());
        payload[13..17].copy_from_slice(&2_u32.to_be_bytes());
        payload[17..21].copy_from_slice(&4_u32.to_be_bytes());
        assert!(decode_iso_payload(&payload).is_err());
        // No canonical exp2 normalization may destroy otherwise valid ISO data.
        payload[13..17].copy_from_slice(&u32::MAX.to_be_bytes());
        payload[17..21].copy_from_slice(&1_u32.to_be_bytes());
        payload[37..41].copy_from_slice(&u32::MAX.to_be_bytes());
        let metadata = decode_iso_payload(&payload).unwrap();
        assert_eq!(&metadata.to_tmap_payload(false).unwrap()[1..], payload);
        assert!(metadata.to_hdrgm_xmp().is_ok());
    }

    #[test]
    fn xmp_capacity_constraints_are_separate_from_iso_direction() {
        for (minimum, maximum) in [("-1", "2"), ("2", "2"), ("2", "1")] {
            let xmp = format!(
                r#"<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/"><description hdrgm:GainMapMax="2" hdrgm:HDRCapacityMin="{minimum}" hdrgm:HDRCapacityMax="{maximum}"/></x:xmpmeta>"#
            );
            let error = parse_ultrahdr_gain_map_metadata(&gain_jpeg(None, Some(xmp.as_bytes())))
                .expect_err("invalid Android HDR capacities must be rejected");
            assert!(error.to_string().contains(XMP_CONTEXT), "{error}");
        }
    }
}
