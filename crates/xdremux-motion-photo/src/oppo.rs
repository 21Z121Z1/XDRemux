use crate::android::parse_android_motion_photo;
use crate::error::{MotionPhotoError, Result};
use crate::lpex::parse_first_lpex_object;
use crate::model::{
    ByteRange, MotionPhotoAsset, MotionPhotoItem, MotionPhotoSourceKind, OppoMetadata,
    PresentationSource,
};
use crate::scanner::{ftyp_box_offsets, is_ftyp_box_start};
use crate::topology::enrich_oppo_video_range;

const MAX_HEADER_BYTES: usize = 4 * 1024 * 1024;
const MAX_TAIL_SCAN_BYTES: u64 = 512 * 1024 * 1024;
const MIN_PLAUSIBLE_VIDEO_LENGTH: i64 = 100_000;

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .and_then(|relative| start.checked_add(relative))
}

fn extract_xmp_string(data: &[u8]) -> Option<&str> {
    let prefix = &data[..data.len().min(MAX_HEADER_BYTES)];
    let start = [b"<x:xmpmeta".as_slice(), b"<xmpmeta".as_slice()]
        .into_iter()
        .filter_map(|needle| find_bytes(prefix, needle, 0))
        .min()?;

    let end = [b"</x:xmpmeta>".as_slice(), b"</xmpmeta>".as_slice()]
        .into_iter()
        .filter_map(|closing| {
            find_bytes(prefix, closing, start).and_then(|position| position.checked_add(closing.len()))
        })
        .min()?;
    std::str::from_utf8(prefix.get(start..end)?).ok()
}

fn parse_decimal_prefix(text: &str) -> Option<i64> {
    let digits = text
        .as_bytes()
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits == 0 || digits > 19 {
        return None;
    }
    text.get(..digits)?.parse::<i64>().ok()
}

fn integer_matches_for_length_label(xmp: &str, label: &str) -> Vec<i64> {
    let mut values = Vec::new();
    let mut search_start = 0usize;
    while let Some(found) = xmp
        .get(search_start..)
        .and_then(|remaining| remaining.find(label))
        .and_then(|relative| search_start.checked_add(relative))
    {
        let Some(after_label) = found.checked_add(label.len()) else {
            break;
        };
        let Some(rest) = xmp.get(after_label..) else {
            break;
        };
        let trimmed = rest.trim_start_matches(char::is_whitespace);
        let value_text = if let Some(after_equals) = trimmed.strip_prefix('=') {
            let after_equals = after_equals.trim_start_matches(char::is_whitespace);
            after_equals
                .strip_prefix('"')
                .or_else(|| after_equals.strip_prefix('\''))
                .unwrap_or(after_equals)
        } else if let Some(after_close) = trimmed.strip_prefix('>') {
            after_close
        } else {
            search_start = after_label;
            continue;
        };
        if let Some(value) = parse_decimal_prefix(value_text) {
            if value > 0 {
                values.push(value);
            }
        }
        search_start = after_label;
    }
    values
}

fn extract_xmp_value<'a>(xmp: &'a str, tag_name: &str) -> Option<&'a str> {
    let opening = format!("<{tag_name}>");
    let closing = format!("</{tag_name}>");
    if let Some(start) = xmp.find(&opening) {
        let value_start = start.checked_add(opening.len())?;
        let relative_end = xmp.get(value_start..)?.find(&closing)?;
        let end = value_start.checked_add(relative_end)?;
        return Some(xmp.get(value_start..end)?.trim());
    }

    let attribute = format!("{tag_name}=");
    let start = xmp.find(&attribute)?.checked_add(attribute.len())?;
    let remainder = xmp.get(start..)?;
    let quote = remainder.as_bytes().first().copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let content = remainder.get(1..)?;
    let end = content.as_bytes().iter().position(|byte| *byte == quote)?;
    Some(content.get(..end)?.trim())
}

fn extract_video_length(xmp: &str) -> Option<i64> {
    let mut generic_lengths = integer_matches_for_length_label(xmp, "Item:Length");
    generic_lengths.extend(integer_matches_for_length_label(xmp, "Length"));
    if let Some(max_length) = generic_lengths.into_iter().max() {
        if max_length > MIN_PLAUSIBLE_VIDEO_LENGTH {
            return Some(max_length);
        }
    }

    for tag in ["OpCamera:VideoLength", "GCamera:VideoLength", "VideoLength"] {
        let Some(value) = extract_xmp_value(xmp, tag) else {
            continue;
        };
        if value.len() <= 32 {
            if let Ok(length) = value.parse::<i64>() {
                if length > MIN_PLAUSIBLE_VIDEO_LENGTH {
                    return Some(length);
                }
            }
        }
    }
    None
}

fn extract_presentation_timestamp(xmp: &str) -> Option<i64> {
    for tag in [
        "GCamera:MotionPhotoPresentationTimestampUs",
        "MotionPhotoPresentationTimestampUs",
        "GCamera:MicroVideoPresentationTimestampUs",
    ] {
        let Some(value) = extract_xmp_value(xmp, tag) else {
            continue;
        };
        if value.len() <= 32 {
            if let Ok(timestamp) = value.parse::<i64>() {
                return (timestamp != -1).then_some(timestamp);
            }
        }
    }
    None
}

fn has_oppo_signature(xmp: Option<&str>, lpex: Option<&OppoMetadata>) -> bool {
    if lpex.is_some() {
        return true;
    }
    let Some(xmp) = xmp else {
        return false;
    };
    if xmp.contains("OpCamera:") {
        return true;
    }
    let lower = xmp.to_lowercase();
    lower.contains("oppo") || lower.contains("oplus")
}

fn resolve_fallback_video_range(
    data: &[u8],
    declared_length: Option<i64>,
    lpex: Option<&OppoMetadata>,
) -> Result<Option<(ByteRange, usize)>> {
    let file_size = u64::try_from(data.len()).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
    let tail_start = file_size.saturating_sub(MAX_TAIL_SCAN_BYTES);
    let scan_range = ByteRange::new(tail_start, file_size)?;

    if lpex.is_some_and(|metadata| metadata.version >= 1) {
        let offsets = ftyp_box_offsets(data, scan_range, 1 << 20)?;
        if offsets.len() >= 2 {
            return Ok(Some((
                ByteRange::new(offsets[offsets.len() - 2], file_size)?,
                2,
            )));
        }
    }

    if let Some(declared_length) = declared_length {
        if declared_length > 0 {
            let declared_length = u64::try_from(declared_length)
                .map_err(|_| MotionPhotoError::InvalidItemLength)?;
            if declared_length <= file_size {
                let start = file_size
                    .checked_sub(declared_length)
                    .ok_or(MotionPhotoError::ArithmeticOverflow)?;
                if is_ftyp_box_start(data, start, file_size)? {
                    let range = ByteRange::new(start, file_size)?;
                    let stream_count = ftyp_box_offsets(data, range, 1 << 20)?.len().max(1);
                    return Ok(Some((range, stream_count)));
                }
            }
        }
    }

    let offsets = ftyp_box_offsets(data, scan_range, 1 << 20)?;
    let Some(last) = offsets.last().copied() else {
        return Ok(None);
    };
    Ok(Some((ByteRange::new(last, file_size)?, 1)))
}

pub fn parse_oppo_fallback(data: &[u8]) -> Result<Option<MotionPhotoAsset>> {
    if data.len() < 16 {
        return Ok(None);
    }

    let xmp = extract_xmp_string(data);
    let lpex = parse_first_lpex_object(data);
    if !has_oppo_signature(xmp, lpex.as_ref()) {
        return Ok(None);
    }

    let declared_length = xmp.and_then(extract_video_length);
    let presentation = xmp.and_then(extract_presentation_timestamp);
    let Some((video_range, stream_count)) =
        resolve_fallback_video_range(data, declared_length, lpex.as_ref())?
    else {
        return Ok(None);
    };

    let still_range = ByteRange::new(0, video_range.lower_bound)?;
    let mut metadata = lpex.unwrap_or_default();
    metadata.stream_count = stream_count.max(1);
    let selected_presentation = presentation.or(metadata.cover_frame_pts_us);
    let selected_source = if presentation.is_some() {
        Some(PresentationSource::AndroidXmp)
    } else if metadata.cover_frame_pts_us.is_some() {
        Some(PresentationSource::OppoCoverFrame)
    } else {
        None
    };

    Ok(Some(MotionPhotoAsset {
        source_kind: MotionPhotoSourceKind::OppoLivePhoto,
        items: vec![
            MotionPhotoItem {
                mime: "image/jpeg".into(),
                semantic: "Primary".into(),
                length: 0,
                padding: 0,
            },
            MotionPhotoItem {
                mime: "video/mp4".into(),
                semantic: "MotionPhoto".into(),
                length: video_range.length(),
                padding: 0,
            },
        ],
        still_resource_range: still_range,
        video_resource_range: video_range,
        presentation_timestamp_us: selected_presentation,
        presentation_source: selected_source,
        vendor_metadata: Some(metadata),
    }))
}

pub fn enrich_oppo_asset(data: &[u8], asset: MotionPhotoAsset) -> Result<MotionPhotoAsset> {
    let Some(mut metadata) = parse_first_lpex_object(data) else {
        return Ok(asset);
    };
    let (still_resource_range, video_resource_range, stream_count) = enrich_oppo_video_range(
        data,
        asset.still_resource_range,
        asset.video_resource_range,
        metadata.version,
    )?;
    metadata.stream_count = stream_count.max(1);

    let selected_presentation = asset
        .presentation_timestamp_us
        .or(metadata.cover_frame_pts_us);
    let selected_source = if asset.presentation_timestamp_us.is_some() {
        asset.presentation_source
    } else if metadata.cover_frame_pts_us.is_some() {
        Some(PresentationSource::OppoCoverFrame)
    } else {
        None
    };

    Ok(MotionPhotoAsset {
        source_kind: MotionPhotoSourceKind::OppoLivePhoto,
        items: asset.items,
        still_resource_range,
        video_resource_range,
        presentation_timestamp_us: selected_presentation,
        presentation_source: selected_source,
        vendor_metadata: Some(metadata),
    })
}

pub fn parse_oppo_motion_photo(data: &[u8]) -> Result<Option<MotionPhotoAsset>> {
    match parse_android_motion_photo(data) {
        Ok(Some(asset)) => enrich_oppo_asset(data, asset).map(Some),
        Ok(None) => parse_oppo_fallback(data),
        Err(android_error) => match parse_oppo_fallback(data) {
            Ok(Some(asset)) => Ok(Some(asset)),
            Ok(None) => Err(android_error),
            Err(fallback_error) => Err(fallback_error),
        },
    }
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

    fn fake_mp4(brand: &[u8; 4], payload_size: usize, payload_byte: u8) -> Vec<u8> {
        let mut ftyp_payload = brand.to_vec();
        ftyp_payload.extend_from_slice(&[0, 0, 0, 0]);
        let mut output = make_box(b"ftyp", &ftyp_payload);
        output.extend_from_slice(&make_box(b"mdat", &vec![payload_byte; payload_size]));
        output
    }

    fn standard_xmp(video_length: usize) -> String {
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1" Camera:MotionPhotoPresentationTimestampUs="1634640"><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{video_length}" Item:Padding="0"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#
        )
    }

    #[test]
    fn unsigned_jpeg_with_appended_mp4_is_not_misclassified_as_oppo() {
        let mut data = vec![0xff, 0xd8, 0xff, 0xd9];
        data.extend_from_slice(&fake_mp4(b"isom", 128, 0x11));
        assert_eq!(parse_oppo_fallback(&data).unwrap(), None);
    }

    #[test]
    fn fallback_uses_valid_vendor_length_and_android_presentation_timestamp() {
        let video = fake_mp4(b"isom", 120_000, 0x22);
        let xmp = format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/" OpCamera:VideoLength="{}" GCamera:MotionPhotoPresentationTimestampUs="1634640"/></rdf:RDF></x:xmpmeta>"#,
            video.len()
        );
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        let video_start = data.len() as u64;
        data.extend_from_slice(&video);

        let asset = parse_oppo_fallback(&data).unwrap().unwrap();
        assert_eq!(asset.source_kind, MotionPhotoSourceKind::OppoLivePhoto);
        assert_eq!(asset.video_resource_range.lower_bound, video_start);
        assert_eq!(asset.presentation_timestamp_us, Some(1_634_640));
        assert_eq!(asset.presentation_source, Some(PresentationSource::AndroidXmp));
        assert_eq!(asset.vendor_metadata.unwrap().stream_count, 1);
    }

    #[test]
    fn stale_vendor_length_falls_back_to_last_valid_ftyp() {
        let video = fake_mp4(b"isom", 120_000, 0x33);
        let xmp = r#"<x:xmpmeta><rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/" OpCamera:VideoLength="100001"/></rdf:RDF></x:xmpmeta>"#;
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        let video_start = data.len() as u64;
        data.extend_from_slice(&video);

        let asset = parse_oppo_fallback(&data).unwrap().unwrap();
        assert_eq!(asset.video_resource_range.lower_bound, video_start);
    }

    #[test]
    fn lpex_v1_dual_stream_beats_length_pointing_only_to_stream2() {
        let stream1 = fake_mp4(b"isom", 128, 0x44);
        let stream2 = fake_mp4(b"mp42", 128, 0x55);
        let xmp = format!(
            r#"<x:xmpmeta><rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/" OpCamera:VideoLength="{}" GCamera:MotionPhotoPresentationTimestampUs="1634640"/></rdf:RDF></x:xmpmeta>"#,
            stream2.len()
        );
        let lpex = r#"lpexLivePhotoExtension {"version":1,"coverFramePts":1666666,"matrixCount":0}"#;
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(lpex.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        let stream1_start = data.len() as u64;
        data.extend_from_slice(&stream1);
        let stream2_start = data.len() as u64;
        data.extend_from_slice(&stream2);

        let asset = parse_oppo_fallback(&data).unwrap().unwrap();
        assert_eq!(asset.video_resource_range.lower_bound, stream1_start);
        assert_eq!(asset.presentation_timestamp_us, Some(1_634_640));
        let metadata = asset.vendor_metadata.unwrap();
        assert_eq!(metadata.cover_frame_pts_us, Some(1_666_666));
        assert_eq!(metadata.stream_count, 2);
        let layout = crate::topology::resolve_video_stream_layout(
            &data,
            asset.video_resource_range,
            true,
            metadata.stream_count,
        )
        .unwrap();
        assert_eq!(layout.primary.range.upper_bound, stream2_start);
    }

    #[test]
    fn standard_android_directory_is_corrected_by_lpex_dual_stream_topology() {
        let stream1 = fake_mp4(b"isom", 64, 0x66);
        let stream2 = fake_mp4(b"mp42", 64, 0x77);
        let xmp = standard_xmp(stream2.len());
        let lpex = r#"lpexLivePhotoExtension {"version":1,"coverFramePts":1666666}"#;
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(xmp.as_bytes());
        data.extend_from_slice(lpex.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        let stream1_start = data.len() as u64;
        data.extend_from_slice(&stream1);
        let stream2_start = data.len() as u64;
        data.extend_from_slice(&stream2);

        let generic = parse_android_motion_photo(&data).unwrap().unwrap();
        assert_eq!(generic.video_resource_range.lower_bound, stream2_start);
        let asset = parse_oppo_motion_photo(&data).unwrap().unwrap();
        assert_eq!(asset.source_kind, MotionPhotoSourceKind::OppoLivePhoto);
        assert_eq!(asset.video_resource_range.lower_bound, stream1_start);
        assert_eq!(asset.presentation_timestamp_us, Some(1_634_640));
        assert_eq!(asset.presentation_source, Some(PresentationSource::AndroidXmp));
        assert_eq!(asset.vendor_metadata.unwrap().stream_count, 2);
    }

    #[test]
    fn lpex_only_signature_uses_cover_frame_when_xmp_is_absent() {
        let video = fake_mp4(b"isom", 128, 0x88);
        let lpex = r#"lpexLivePhotoExtension {"version":0,"coverFramePts":777777}"#;
        let mut data = vec![0xff, 0xd8];
        data.extend_from_slice(lpex.as_bytes());
        data.extend_from_slice(&[0xff, 0xd9]);
        data.extend_from_slice(&video);

        let asset = parse_oppo_fallback(&data).unwrap().unwrap();
        assert_eq!(asset.presentation_timestamp_us, Some(777_777));
        assert_eq!(asset.presentation_source, Some(PresentationSource::OppoCoverFrame));
    }
}