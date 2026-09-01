use std::error::Error;
use std::fmt;

use little_exif::exif_tag::ExifTag;
use little_exif::filetype::FileExtension;
use little_exif::metadata::Metadata;

const APPLE_MAKERNOTE_PREFIX: &[u8] = b"Apple iOS\0\0\x01MM";
const APPLE_ASSET_IDENTIFIER_TAG: u16 = 0x0011;
const ASCII_TIFF_TYPE: u16 = 2;
const APPLE_MAKERNOTE_IFD_OFFSET: usize = 14;
const APPLE_MAKERNOTE_ENTRY_SIZE: usize = 12;
const APPLE_MAKERNOTE_NEXT_IFD_SIZE: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePhotoStillError {
    detail: String,
}

impl LivePhotoStillError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    fn external(context: &str, error: impl fmt::Display) -> Self {
        Self::new(format!("{context}: {error}"))
    }
}

impl fmt::Display for LivePhotoStillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for LivePhotoStillError {}

pub type LivePhotoStillResult<T> = std::result::Result<T, LivePhotoStillError>;

pub fn build_apple_makernote(content_identifier: &str) -> LivePhotoStillResult<Vec<u8>> {
    if content_identifier.is_empty()
        || !content_identifier.is_ascii()
        || content_identifier.as_bytes().contains(&0)
    {
        return Err(LivePhotoStillError::new(
            "Live Photo content identifier must be non-empty ASCII without NUL bytes",
        ));
    }

    let identifier = content_identifier.to_ascii_uppercase();
    let mut value = identifier.into_bytes();
    value.push(0);
    let value_count = u32::try_from(value.len())
        .map_err(|_| LivePhotoStillError::new("Live Photo content identifier is too large"))?;

    let mut output = APPLE_MAKERNOTE_PREFIX.to_vec();
    output.extend_from_slice(&1_u16.to_be_bytes());
    let value_offset = APPLE_MAKERNOTE_IFD_OFFSET
        .checked_add(2)
        .and_then(|offset| offset.checked_add(APPLE_MAKERNOTE_ENTRY_SIZE))
        .and_then(|offset| offset.checked_add(APPLE_MAKERNOTE_NEXT_IFD_SIZE))
        .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote offset overflow"))?;
    let value_offset = u32::try_from(value_offset)
        .map_err(|_| LivePhotoStillError::new("Apple MakerNote offset exceeds u32"))?;
    output.extend_from_slice(&APPLE_ASSET_IDENTIFIER_TAG.to_be_bytes());
    output.extend_from_slice(&ASCII_TIFF_TYPE.to_be_bytes());
    output.extend_from_slice(&value_count.to_be_bytes());
    output.extend_from_slice(&value_offset.to_be_bytes());
    output.extend_from_slice(&0_u32.to_be_bytes());
    output.extend_from_slice(&value);
    Ok(output)
}

fn apple_makernote_identifier(maker_note: &[u8]) -> LivePhotoStillResult<Option<String>> {
    if !maker_note.starts_with(APPLE_MAKERNOTE_PREFIX) {
        return Ok(None);
    }
    let count_offset = APPLE_MAKERNOTE_IFD_OFFSET;
    let count_end = count_offset
        .checked_add(2)
        .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote entry-count overflow"))?;
    let count_bytes = maker_note
        .get(count_offset..count_end)
        .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote is truncated before entry count"))?;
    let entry_count = usize::from(u16::from_be_bytes(
        count_bytes
            .try_into()
            .map_err(|_| LivePhotoStillError::new("invalid Apple MakerNote entry count"))?,
    ));

    let entries_start = count_end;
    for index in 0..entry_count {
        let entry_start = entries_start
            .checked_add(index.checked_mul(APPLE_MAKERNOTE_ENTRY_SIZE).ok_or_else(|| {
                LivePhotoStillError::new("Apple MakerNote entry offset overflow")
            })?)
            .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote entry offset overflow"))?;
        let entry_end = entry_start
            .checked_add(APPLE_MAKERNOTE_ENTRY_SIZE)
            .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote entry end overflow"))?;
        let entry = maker_note
            .get(entry_start..entry_end)
            .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote entry is truncated"))?;
        let tag = u16::from_be_bytes([entry[0], entry[1]]);
        let field_type = u16::from_be_bytes([entry[2], entry[3]]);
        if tag != APPLE_ASSET_IDENTIFIER_TAG || field_type != ASCII_TIFF_TYPE {
            continue;
        }
        let component_count = usize::try_from(u32::from_be_bytes([
            entry[4], entry[5], entry[6], entry[7],
        ]))
        .map_err(|_| LivePhotoStillError::new("Apple MakerNote value count exceeds usize"))?;
        if component_count == 0 {
            return Err(LivePhotoStillError::new(
                "Apple MakerNote asset identifier is empty",
            ));
        }
        let value = if component_count <= 4 {
            entry
                .get(8..8 + component_count)
                .ok_or_else(|| LivePhotoStillError::new("inline Apple MakerNote value is truncated"))?
        } else {
            let offset = usize::try_from(u32::from_be_bytes([
                entry[8], entry[9], entry[10], entry[11],
            ]))
            .map_err(|_| LivePhotoStillError::new("Apple MakerNote value offset exceeds usize"))?;
            let end = offset
                .checked_add(component_count)
                .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote value range overflow"))?;
            maker_note
                .get(offset..end)
                .ok_or_else(|| LivePhotoStillError::new("Apple MakerNote value is out of bounds"))?
        };
        let value = value.strip_suffix(&[0]).unwrap_or(value);
        if value.is_empty() || !value.is_ascii() {
            return Err(LivePhotoStillError::new(
                "Apple MakerNote asset identifier is not valid ASCII",
            ));
        }
        return String::from_utf8(value.to_vec())
            .map(Some)
            .map_err(|error| LivePhotoStillError::external("Apple MakerNote identifier", error));
    }
    Ok(None)
}

pub fn read_apple_content_identifier(data: &[u8]) -> LivePhotoStillResult<Option<String>> {
    let file = data.to_vec();
    let metadata = Metadata::new_from_vec(&file, FileExtension::HEIF)
        .map_err(|error| LivePhotoStillError::external("read HEIF EXIF", error))?;
    let Some(tag) = metadata.get_tag(&ExifTag::MakerNote(Vec::new())).next() else {
        return Ok(None);
    };
    let ExifTag::MakerNote(maker_note) = tag else {
        return Err(LivePhotoStillError::new(
            "HEIF MakerNote tag resolved to an unexpected EXIF variant",
        ));
    };
    apple_makernote_identifier(maker_note)
}

fn disable_motion_photo_flag(data: &mut [u8]) -> bool {
    const PATTERNS: &[&[u8]] = &[
        b"Camera:MotionPhoto=\"1\"",
        b"GCamera:MotionPhoto=\"1\"",
        b"Camera:MotionPhoto='1'",
        b"GCamera:MotionPhoto='1'",
        b"Camera:MicroVideo=\"1\"",
        b"GCamera:MicroVideo=\"1\"",
        b"Camera:MicroVideo='1'",
        b"GCamera:MicroVideo='1'",
    ];
    let mut changed = false;
    for pattern in PATTERNS {
        let value_index = pattern.len().saturating_sub(2);
        let mut start = 0usize;
        while start < data.len() {
            let Some(relative) = data[start..]
                .windows(pattern.len())
                .position(|window| window == *pattern)
            else {
                break;
            };
            let match_start = start + relative;
            let absolute_value = match_start + value_index;
            data[absolute_value] = b'0';
            changed = true;
            start = match_start + pattern.len();
        }
    }
    changed
}

pub fn write_live_photo_heif_still(
    static_heif: &[u8],
    content_identifier: &str,
) -> LivePhotoStillResult<Vec<u8>> {
    let mut output = static_heif.to_vec();
    let mut metadata = Metadata::new_from_vec(&output, FileExtension::HEIF)
        .map_err(|error| LivePhotoStillError::external("read source HEIF EXIF", error))?;
    metadata.set_tag(ExifTag::MakerNote(build_apple_makernote(content_identifier)?));
    metadata
        .write_to_vec(&mut output, FileExtension::HEIF)
        .map_err(|error| LivePhotoStillError::external("write Live Photo HEIF EXIF", error))?;
    disable_motion_photo_flag(&mut output);

    let expected = content_identifier.to_ascii_uppercase();
    let actual = read_apple_content_identifier(&output)?.ok_or_else(|| {
        LivePhotoStillError::new("written Live Photo HEIF is missing Apple asset identifier")
    })?;
    if actual != expected {
        return Err(LivePhotoStillError::new(format!(
            "written Live Photo HEIF asset identifier mismatch: expected {expected}, found {actual}"
        )));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_makernote_round_trips_asset_identifier() {
        let maker = build_apple_makernote("df64c2ae-ed3c-4778-bfca-c15277e521d2").unwrap();
        assert!(maker.starts_with(APPLE_MAKERNOTE_PREFIX));
        assert_eq!(
            apple_makernote_identifier(&maker).unwrap().as_deref(),
            Some("DF64C2AE-ED3C-4778-BFCA-C15277E521D2")
        );
    }

    #[test]
    fn motion_photo_flag_is_disabled_without_changing_length() {
        let mut xmp = b"<rdf:Description Camera:MotionPhoto=\"1\"/>".to_vec();
        let len = xmp.len();
        assert!(disable_motion_photo_flag(&mut xmp));
        assert_eq!(xmp.len(), len);
        assert!(xmp.windows(b"Camera:MotionPhoto=\"0\"".len()).any(|window| {
            window == b"Camera:MotionPhoto=\"0\""
        }));
    }
}
