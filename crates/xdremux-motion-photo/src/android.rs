use std::collections::BTreeMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;

use crate::error::{MotionPhotoError, Result};
use crate::heif::{is_heif_mime, resolve_heif_motion_photo_ranges};
use crate::model::{
    ByteRange, MotionPhotoAsset, MotionPhotoItem, MotionPhotoSourceKind, PresentationSource,
};
use crate::scanner::is_ftyp_box_start;

const MAX_XMP_SCAN_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIRECTORY_ITEMS: usize = 64;
const MAX_METADATA_STRING_LENGTH: usize = 4096;

#[derive(Debug, Default)]
struct ParsedXmp {
    motion_photo_enabled: bool,
    version: Option<i64>,
    presentation_timestamp_us: Option<i64>,
    legacy_micro_video_offset: Option<i64>,
    items: Vec<MotionPhotoItem>,
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}

fn extract_xmp(data: &[u8]) -> Result<Option<&[u8]>> {
    let scan_len = data.len().min(MAX_XMP_SCAN_BYTES);
    let prefix = &data[..scan_len];
    let start = [b"<x:xmpmeta".as_slice(), b"<xmpmeta".as_slice()]
        .into_iter()
        .filter_map(|needle| find_bytes(prefix, needle, 0))
        .min();
    let Some(start) = start else {
        return Ok(None);
    };

    let mut end = None;
    for closing in [b"</x:xmpmeta>".as_slice(), b"</xmpmeta>".as_slice()] {
        if let Some(position) = find_bytes(prefix, closing, start) {
            let candidate = position
                .checked_add(closing.len())
                .ok_or(MotionPhotoError::ArithmeticOverflow)?;
            end = Some(end.map_or(candidate, |current: usize| current.min(candidate)));
        }
    }
    match end {
        Some(end) => Ok(Some(&prefix[start..end])),
        None if data.len() > scan_len => Err(MotionPhotoError::XmpTooLarge),
        None => Err(MotionPhotoError::MalformedXmp),
    }
}

fn attributes(element: &BytesStart<'_>) -> Result<BTreeMap<String, String>> {
    let mut output = BTreeMap::new();
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|_| MotionPhotoError::MalformedXmp)?;
        let key = attribute.key.as_ref();
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| MotionPhotoError::MalformedXmp)?;
        output.insert(key.to_owned(), value.into_owned());
    }
    Ok(output)
}

fn bounded_string(value: Option<&String>) -> Option<String> {
    let value = value?;
    (!value.is_empty() && value.len() <= MAX_METADATA_STRING_LENGTH).then(|| value.clone())
}

fn nonnegative(value: Option<&String>, default_value: u64) -> Option<u64> {
    let Some(value) = value else {
        return Some(default_value);
    };
    if value.len() > 32 {
        return None;
    }
    let parsed = value.parse::<i64>().ok()?;
    (parsed >= 0).then_some(parsed as u64)
}

fn first_i64(attributes: &BTreeMap<String, String>, names: &[&str]) -> Option<i64> {
    for name in names {
        let Some(raw) = attributes.get(*name) else {
            continue;
        };
        if raw.len() > 32 {
            return None;
        }
        return raw.parse::<i64>().ok();
    }
    None
}

fn parse_description(parsed: &mut ParsedXmp, attrs: &BTreeMap<String, String>) {
    if let Some(flag) = first_i64(attrs, &["Camera:MotionPhoto", "GCamera:MotionPhoto"]) {
        parsed.motion_photo_enabled = flag == 1;
    } else if let Some(flag) = first_i64(attrs, &["Camera:MicroVideo", "GCamera:MicroVideo"]) {
        parsed.motion_photo_enabled = flag == 1;
    }

    parsed.version = first_i64(
        attrs,
        &["Camera:MotionPhotoVersion", "GCamera:MotionPhotoVersion"],
    );
    if let Some(value) = first_i64(
        attrs,
        &[
            "Camera:MotionPhotoPresentationTimestampUs",
            "GCamera:MotionPhotoPresentationTimestampUs",
            "Camera:MicroVideoPresentationTimestampUs",
            "GCamera:MicroVideoPresentationTimestampUs",
        ],
    ) {
        parsed.presentation_timestamp_us = (value != -1).then_some(value);
    }
    parsed.legacy_micro_video_offset = first_i64(
        attrs,
        &["Camera:MicroVideoOffset", "GCamera:MicroVideoOffset"],
    );
}

fn parse_item(
    parsed: &mut ParsedXmp,
    attrs: &BTreeMap<String, String>,
    directory_prefix: &str,
) -> Result<()> {
    if parsed.items.len() >= MAX_DIRECTORY_ITEMS {
        return Err(MotionPhotoError::InvalidDirectory);
    }
    let attribute_prefix = if directory_prefix == "Container" {
        "Item"
    } else {
        "GContainerItem"
    };
    let mime_key = format!("{attribute_prefix}:Mime");
    let semantic_key = format!("{attribute_prefix}:Semantic");
    let length_key = format!("{attribute_prefix}:Length");
    let padding_key = format!("{attribute_prefix}:Padding");
    let mime = bounded_string(attrs.get(&mime_key)).ok_or(MotionPhotoError::InvalidDirectory)?;
    let semantic =
        bounded_string(attrs.get(&semantic_key)).ok_or(MotionPhotoError::InvalidDirectory)?;
    let length = nonnegative(attrs.get(&length_key), 0).ok_or(MotionPhotoError::InvalidDirectory)?;
    let padding =
        nonnegative(attrs.get(&padding_key), 0).ok_or(MotionPhotoError::InvalidDirectory)?;
    parsed.items.push(MotionPhotoItem {
        mime,
        semantic,
        length,
        padding,
    });
    Ok(())
}

fn parse_xmp(xmp: &[u8]) -> Result<ParsedXmp> {
    let text = std::str::from_utf8(xmp).map_err(|_| MotionPhotoError::MalformedXmp)?;
    let upper = text.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err(MotionPhotoError::MalformedXmp);
    }

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    let mut parsed = ParsedXmp::default();
    let mut directory_prefix: Option<&'static str> = None;

    loop {
        let event = reader
            .read_event()
            .map_err(|_| MotionPhotoError::MalformedXmp)?;
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let name = element.name().as_ref().to_owned();
                if name == "rdf:Description" {
                    parse_description(&mut parsed, &attributes(&element)?);
                } else if name == "Container:Directory" {
                    directory_prefix = Some("Container");
                } else if name == "GContainer:Directory" {
                    directory_prefix = Some("GContainer");
                } else if let Some(prefix) = directory_prefix {
                    let expected = format!("{prefix}:Item");
                    if name == expected {
                        parse_item(&mut parsed, &attributes(&element)?, prefix)?;
                    }
                }
            }
            Event::End(element) => {
                let name = element.name().as_ref();
                if name == "Container:Directory" || name == "GContainer:Directory" {
                    directory_prefix = None;
                }
            }
            Event::DocType(_) => return Err(MotionPhotoError::MalformedXmp),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(parsed)
}

fn validate_directory(items: &[MotionPhotoItem]) -> Result<()> {
    if !(2..=MAX_DIRECTORY_ITEMS).contains(&items.len()) {
        return Err(MotionPhotoError::InvalidDirectory);
    }
    let primary = &items[0];
    if !primary.semantic.eq_ignore_ascii_case("Primary")
        || primary.length != 0
        || items[1..]
            .iter()
            .any(|item| item.semantic.eq_ignore_ascii_case("Primary"))
    {
        return Err(MotionPhotoError::InvalidPrimaryItem);
    }
    if items[1..].iter().any(|item| item.padding != 0) {
        return Err(MotionPhotoError::InvalidItemLength);
    }
    let motion_indices = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            item.semantic
                .eq_ignore_ascii_case("MotionPhoto")
                .then_some(index)
        })
        .collect::<Vec<_>>();
    if motion_indices.as_slice() != [items.len() - 1] {
        return Err(MotionPhotoError::InvalidMotionPhotoItem);
    }
    let motion = items.last().ok_or(MotionPhotoError::InvalidDirectory)?;
    if !(motion.mime.eq_ignore_ascii_case("video/mp4")
        || motion.mime.eq_ignore_ascii_case("video/quicktime"))
        || motion.length == 0
    {
        return Err(MotionPhotoError::InvalidMotionPhotoItem);
    }
    Ok(())
}

fn derive_jpeg_style_ranges(
    items: &[MotionPhotoItem],
    file_size: u64,
) -> Result<(ByteRange, ByteRange)> {
    let mut item_start = file_size;
    let mut primary_encoding_end = None;
    let mut video_start = None;
    let mut video_end = None;

    for index in (0..items.len()).rev() {
        let item = &items[index];
        let mut item_end = item_start;
        if index == 0 {
            let unpadded_end = item_end
                .checked_sub(item.padding)
                .ok_or(MotionPhotoError::ArithmeticOverflow)?;
            item_start = 0;
            item_end = unpadded_end;
            primary_encoding_end = Some(item_end);
        } else {
            item_start = item_start
                .checked_sub(item.length)
                .ok_or(MotionPhotoError::ArithmeticOverflow)?;
        }
        let is_video = item.mime.eq_ignore_ascii_case("video/mp4")
            || item.mime.eq_ignore_ascii_case("video/quicktime");
        if is_video && item_start != item_end {
            video_start = Some(item_start);
            video_end = Some(item_end);
        }
    }

    let primary_encoding_end = primary_encoding_end.ok_or(MotionPhotoError::InvalidByteRange)?;
    let video_start = video_start.ok_or(MotionPhotoError::InvalidByteRange)?;
    let video_end = video_end.ok_or(MotionPhotoError::InvalidByteRange)?;
    if primary_encoding_end > video_start || video_end != file_size {
        return Err(MotionPhotoError::InvalidByteRange);
    }
    Ok((
        ByteRange::new(0, video_start)?,
        ByteRange::new(video_start, video_end)?,
    ))
}

pub fn parse_android_motion_photo(data: &[u8]) -> Result<Option<MotionPhotoAsset>> {
    if data.len() < 16 {
        return Err(MotionPhotoError::FileTooSmall);
    }
    let Some(xmp) = extract_xmp(data)? else {
        return Ok(None);
    };
    let description = parse_xmp(xmp)?;
    if !description.motion_photo_enabled {
        return Ok(None);
    }

    let (source_kind, items, timestamp_source) = if !description.items.is_empty() {
        if description.version != Some(1) {
            return Err(MotionPhotoError::UnsupportedVersion(description.version));
        }
        let source_kind = if is_heif_mime(&description.items[0].mime) {
            MotionPhotoSourceKind::AndroidHeifMotionPhotoV1
        } else {
            MotionPhotoSourceKind::AndroidMotionPhotoV1
        };
        (
            source_kind,
            description.items,
            description
                .presentation_timestamp_us
                .map(|_| PresentationSource::AndroidXmp),
        )
    } else if let Some(legacy_offset) = description.legacy_micro_video_offset {
        if legacy_offset <= 0 {
            return Err(MotionPhotoError::InvalidItemLength);
        }
        let length = u64::try_from(legacy_offset).map_err(|_| MotionPhotoError::InvalidItemLength)?;
        (
            MotionPhotoSourceKind::LegacyMicroVideoV1b,
            vec![
                MotionPhotoItem {
                    mime: "image/jpeg".into(),
                    semantic: "Primary".into(),
                    length: 0,
                    padding: 0,
                },
                MotionPhotoItem {
                    mime: "video/mp4".into(),
                    semantic: "MotionPhoto".into(),
                    length,
                    padding: 0,
                },
            ],
            description
                .presentation_timestamp_us
                .map(|_| PresentationSource::LegacyMicroVideoXmp),
        )
    } else {
        return Err(MotionPhotoError::InvalidDirectory);
    };

    validate_directory(&items)?;
    let file_size = u64::try_from(data.len()).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
    let (still_resource_range, video_resource_range) =
        if source_kind == MotionPhotoSourceKind::AndroidHeifMotionPhotoV1 {
            resolve_heif_motion_photo_ranges(data, &items)?
        } else {
            let ranges = derive_jpeg_style_ranges(&items, file_size)?;
            if !is_ftyp_box_start(data, ranges.1.lower_bound, ranges.1.upper_bound)? {
                return Err(MotionPhotoError::InvalidVideoPayload);
            }
            ranges
        };

    Ok(Some(MotionPhotoAsset {
        source_kind,
        items,
        still_resource_range,
        video_resource_range,
        presentation_timestamp_us: description.presentation_timestamp_us,
        presentation_source: timestamp_source,
        vendor_metadata: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(payload.len() + 8).unwrap();
        let mut output = size.to_be_bytes().to_vec();
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        output
    }

    fn fake_mp4() -> Vec<u8> {
        let mut output = make_box(b"ftyp", b"isom\0\0\x02\0");
        output.extend_from_slice(&make_box(b"mdat", &[]));
        output
    }

    fn standard_xmp(video_length: u64, timestamp: Option<i64>, extra: &str) -> String {
        let timestamp = timestamp
            .map(|value| format!(" Camera:MotionPhotoPresentationTimestampUs=\"{value}\""))
            .unwrap_or_default();
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1"{timestamp}><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>{extra}<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{video_length}" Item:Padding="0"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#
        )
    }

    #[test]
    fn parses_android_directory_and_timestamp() {
        let video = fake_mp4();
        let xmp = standard_xmp(video.len() as u64, Some(1_417_000), "");
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        let still_len = data.len() as u64;
        data.extend_from_slice(&video);
        let asset = parse_android_motion_photo(&data).unwrap().unwrap();
        assert_eq!(asset.source_kind, MotionPhotoSourceKind::AndroidMotionPhotoV1);
        assert_eq!(asset.presentation_timestamp_us, Some(1_417_000));
        assert_eq!(asset.presentation_source, Some(PresentationSource::AndroidXmp));
        assert_eq!(asset.still_resource_range, ByteRange::new(0, still_len).unwrap());
    }

    #[test]
    fn preserves_positive_length_gain_map_in_static_resource() {
        let video = fake_mp4();
        let gain = vec![0xab; 64];
        let extra = format!(r#"<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="GainMap" Item:Length="{}" Item:Padding="0"/></rdf:li>"#, gain.len());
        let xmp = standard_xmp(video.len() as u64, None, &extra);
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        data.extend_from_slice(&gain);
        let still_len = data.len() as u64;
        data.extend_from_slice(&video);
        let asset = parse_android_motion_photo(&data).unwrap().unwrap();
        assert_eq!(asset.items[1].semantic, "GainMap");
        assert_eq!(asset.still_resource_range.upper_bound, still_len);
    }

    #[test]
    fn parses_legacy_micro_video() {
        let video = fake_mp4();
        let xmp = format!(r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/" GCamera:MicroVideo="1" GCamera:MicroVideoOffset="{}" GCamera:MicroVideoPresentationTimestampUs="900000"/></rdf:RDF></x:xmpmeta>"#, video.len());
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        data.extend_from_slice(&video);
        let asset = parse_android_motion_photo(&data).unwrap().unwrap();
        assert_eq!(asset.source_kind, MotionPhotoSourceKind::LegacyMicroVideoV1b);
        assert_eq!(asset.presentation_timestamp_us, Some(900_000));
    }

    #[test]
    fn parses_gcontainer_namespace() {
        let video = fake_mp4();
        let xmp = format!(r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/" xmlns:GContainer="http://ns.google.com/photos/1.0/container/" xmlns:GContainerItem="http://ns.google.com/photos/1.0/container/item/" GCamera:MotionPhoto="1" GCamera:MotionPhotoVersion="1" GCamera:MotionPhotoPresentationTimestampUs="500000"><GContainer:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="image/jpeg" GContainerItem:Semantic="Primary" GContainerItem:Length="0" GContainerItem:Padding="0"/></rdf:li><rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="video/mp4" GContainerItem:Semantic="MotionPhoto" GContainerItem:Length="{}" GContainerItem:Padding="0"/></rdf:li></rdf:Seq></GContainer:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#, video.len());
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        data.extend_from_slice(&video);
        let asset = parse_android_motion_photo(&data).unwrap().unwrap();
        assert_eq!(asset.presentation_timestamp_us, Some(500_000));
    }

    #[test]
    fn rejects_dtd_and_entities_before_xml_parse() {
        let video = fake_mp4();
        let xmp = format!(r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><!DOCTYPE rdf:RDF [<!ENTITY injected "MotionPhoto">]><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1"><Container:Directory><rdf:Seq><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/><Container:Item Item:Mime="video/mp4" Item:Semantic="&injected;" Item:Length="{}" Item:Padding="0"/></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#, video.len());
        let mut data = xmp.into_bytes();
        data.extend_from_slice(&video);
        assert_eq!(parse_android_motion_photo(&data), Err(MotionPhotoError::MalformedXmp));
    }

    #[test]
    fn rejects_missing_version() {
        let video = fake_mp4();
        let xmp = standard_xmp(video.len() as u64, None, "").replace(" Camera:MotionPhotoVersion=\"1\"", "");
        let mut data = xmp.into_bytes();
        data.extend_from_slice(&video);
        assert_eq!(
            parse_android_motion_photo(&data),
            Err(MotionPhotoError::UnsupportedVersion(None))
        );
    }

    #[test]
    fn rejects_stale_directory_without_real_video() {
        let xmp = standard_xmp(32, None, "");
        let mut data = xmp.into_bytes();
        data.extend_from_slice(&[0u8; 32]);
        assert_eq!(
            parse_android_motion_photo(&data),
            Err(MotionPhotoError::InvalidVideoPayload)
        );
    }

    #[test]
    fn rejects_motion_photo_item_that_is_not_last() {
        let video = fake_mp4();
        let trailing = r#"<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="application/octet-stream" Item:Semantic="Auxiliary" Item:Length="0" Item:Padding="0"/></rdf:li>"#;
        let xmp = standard_xmp(video.len() as u64, None, "").replace("</rdf:Seq>", &format!("{trailing}</rdf:Seq>"));
        let mut data = xmp.into_bytes();
        data.extend_from_slice(&video);
        assert_eq!(
            parse_android_motion_photo(&data),
            Err(MotionPhotoError::InvalidMotionPhotoItem)
        );
    }
}
