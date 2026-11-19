//! OpenHarmony motion container observed on Mate 70 HEIC files.
//!
//! BeetMan/XDRemux-Flutter `xdremux/rust/src/motion_photo.rs` at
//! `78df7994b51ec79cf9730fd6d7584b40196e1ed6` (GPL-3.0, a fork of this
//! repository) reads this layout and writes an Apple Live Photo. That reader
//! is not evidence that a Harmony gallery plays a file we assemble.
//! `photoAccessHelper` in that app asks the picker for the original HEIC and
//! does not create a moving-photo asset. This module only recognizes a file
//! that already has the container and reports the fields it actually carries.

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HarmonyMotion {
    pub cover_time_ms: f64,
    /// Source text. The observed samples use a wall-clock value, not a duration.
    pub start_time_text: Option<String>,
    /// Source capture field. Never generated for another file.
    pub video_id: String,
    pub deferred_video_enhance: Option<bool>,
    pub video_start: usize,
    pub movie_bytes: usize,
    pub trailer: Vec<u8>,
}

#[derive(Clone)]
struct Box {
    start: usize,
    end: usize,
    header: usize,
    kind: [u8; 4],
}

impl Box {
    fn payload(&self) -> usize {
        self.start + self.header
    }
}

pub fn is_harmony_motion_photo(data: &[u8]) -> bool {
    parse_harmony_motion(data).is_some()
}

pub(crate) fn parse_harmony_motion(data: &[u8]) -> Option<HarmonyMotion> {
    let boxes = walk_boxes(data, 0, data.len())?;
    let first = boxes.first()?;
    if first.start != 0 || &first.kind != b"ftyp" || !is_heif_ftyp(data, first) {
        return None;
    }
    let video = boxes
        .iter()
        .find(|box_| box_.start > 0 && &box_.kind == b"ftyp")?;
    let video_boxes: Vec<&Box> = boxes
        .iter()
        .filter(|box_| box_.start >= video.start)
        .collect();
    if !video_boxes.iter().any(|box_| &box_.kind == b"mdat") {
        return None;
    }
    let moov = video_boxes.iter().find(|box_| &box_.kind == b"moov")?;
    let metadata = movie_metadata(data, moov)?;
    if metadata.cover_time_ms.is_none() || metadata.video_id.is_none() {
        return None;
    }
    let consumed = boxes.last()?.end;
    if consumed < video.start || consumed > data.len() {
        return None;
    }
    Some(HarmonyMotion {
        cover_time_ms: metadata.cover_time_ms?,
        start_time_text: metadata.start_time_text,
        video_id: metadata.video_id?,
        deferred_video_enhance: metadata.deferred_video_enhance,
        video_start: video.start,
        movie_bytes: consumed - video.start,
        trailer: data[consumed..].to_vec(),
    })
}

struct MovieMetadata {
    cover_time_ms: Option<f64>,
    start_time_text: Option<String>,
    video_id: Option<String>,
    deferred_video_enhance: Option<bool>,
}

fn movie_metadata(data: &[u8], moov: &Box) -> Option<MovieMetadata> {
    let moov_children = children(data, moov, false, true)?;
    let mut metas: Vec<Box> = moov_children
        .iter()
        .filter(|child| &child.kind == b"meta")
        .cloned()
        .collect();
    for udta in moov_children.iter().filter(|child| &child.kind == b"udta") {
        if let Some(nested) = children(data, udta, false, true) {
            metas.extend(nested.into_iter().filter(|child| &child.kind == b"meta"));
        }
    }
    for meta in metas {
        let Some(body) = children(data, &meta, true, true) else {
            continue;
        };
        let Some(keys) = body.iter().find(|child| &child.kind == b"keys") else {
            continue;
        };
        let Some(ilst) = body.iter().find(|child| &child.kind == b"ilst") else {
            continue;
        };
        let Some(names) = mdta_keys(data, keys) else {
            continue;
        };
        let Some(values) = mdta_values(data, ilst, &names) else {
            continue;
        };
        return Some(metadata_from_values(&values));
    }
    None
}

fn metadata_from_values(values: &[(String, Vec<u8>)]) -> MovieMetadata {
    let text = |name: &str| -> Option<String> {
        values
            .iter()
            .find(|(key, _)| key == name)
            .and_then(|(_, value)| std::str::from_utf8(value).ok())
            .map(|value| value.trim_end_matches('\0').trim().to_owned())
            .filter(|value| !value.is_empty() && value.len() <= 4096)
    };
    let cover_time_ms = values
        .iter()
        .find(|(key, value)| key == "com.openharmony.covertime" && value.len() == 4)
        .and_then(|(_, value)| {
            let value = f32::from_be_bytes(value[..4].try_into().ok()?) as f64;
            (value.is_finite() && value >= 0.0).then_some(value)
        });
    let deferred_video_enhance = match text("com.openharmony.deferredVideoEnhanceFlag").as_deref() {
        Some("0") => Some(false),
        Some("1") => Some(true),
        _ => None,
    };
    MovieMetadata {
        cover_time_ms,
        start_time_text: text("com.openharmony.starttime"),
        video_id: text("com.openharmony.videoId"),
        deferred_video_enhance,
    }
}

fn mdta_keys(data: &[u8], keys: &Box) -> Option<Vec<String>> {
    let payload = keys.payload();
    if payload + 8 > keys.end {
        return None;
    }
    let count = u32::from_be_bytes(data[payload + 4..payload + 8].try_into().ok()?) as usize;
    if count > 64 {
        return None;
    }
    let mut cursor = payload + 8;
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        if cursor + 8 > keys.end {
            return None;
        }
        let size = u32::from_be_bytes(data[cursor..cursor + 4].try_into().ok()?) as usize;
        if size < 8 || cursor + size > keys.end || &data[cursor + 4..cursor + 8] != b"mdta" {
            return None;
        }
        let name = std::str::from_utf8(&data[cursor + 8..cursor + size]).ok()?;
        if name.len() > 4096 {
            return None;
        }
        names.push(name.to_owned());
        cursor += size;
    }
    Some(names)
}

fn mdta_values(data: &[u8], ilst: &Box, keys: &[String]) -> Option<Vec<(String, Vec<u8>)>> {
    let mut values = Vec::new();
    for item in children(data, ilst, false, false)? {
        let id = u32::from_be_bytes(item.kind);
        if id == 0 {
            continue;
        }
        let name = keys.get(id as usize - 1)?;
        let data_box = children(data, &item, false, false)?
            .into_iter()
            .find(|child| &child.kind == b"data")?;
        let start = data_box.payload().checked_add(8)?;
        if start > data_box.end || data_box.end - start > 4096 {
            return None;
        }
        values.push((name.clone(), data[start..data_box.end].to_vec()));
    }
    Some(values)
}

fn children(data: &[u8], parent: &Box, full_box: bool, printable: bool) -> Option<Vec<Box>> {
    let mut cursor = parent.payload().checked_add(if full_box { 4 } else { 0 })?;
    if cursor > parent.end {
        return None;
    }
    let mut boxes = Vec::new();
    while cursor < parent.end {
        let child = read_box(data, cursor, parent.end, printable)?;
        cursor = child.end;
        boxes.push(child);
    }
    Some(boxes)
}

fn walk_boxes(data: &[u8], start: usize, end: usize) -> Option<Vec<Box>> {
    let mut cursor = start;
    let mut boxes = Vec::new();
    while cursor < end {
        if boxes.len() >= 4096 {
            return None;
        }
        let Some(child) = read_box(data, cursor, end, true) else {
            break;
        };
        cursor = child.end;
        boxes.push(child);
    }
    Some(boxes)
}

fn read_box(data: &[u8], offset: usize, end: usize, printable: bool) -> Option<Box> {
    if offset.checked_add(8)? > end {
        return None;
    }
    let size32 = u32::from_be_bytes(data[offset..offset + 4].try_into().ok()?);
    let mut kind = [0u8; 4];
    kind.copy_from_slice(&data[offset + 4..offset + 8]);
    if printable && !kind.iter().all(|byte| (0x20..=0x7e).contains(byte)) {
        return None;
    }
    let (size, header) = if size32 == 1 {
        if offset.checked_add(16)? > end {
            return None;
        }
        let size = u64::from_be_bytes(data[offset + 8..offset + 16].try_into().ok()?);
        (usize::try_from(size).ok()?, 16)
    } else if size32 < 8 {
        return None;
    } else {
        (size32 as usize, 8)
    };
    let box_end = offset.checked_add(size)?;
    if box_end > end || size < header {
        return None;
    }
    Some(Box {
        start: offset,
        end: box_end,
        header,
        kind,
    })
}

fn is_heif_ftyp(data: &[u8], ftyp: &Box) -> bool {
    let Some(payload) = data.get(ftyp.payload()..ftyp.end) else {
        return false;
    };
    if payload.len() < 8 {
        return false;
    }
    let heif = |brand: &[u8]| {
        matches!(
            brand,
            b"heic" | b"heix" | b"heis" | b"hevs" | b"mif1" | b"msf1"
        )
    };
    heif(&payload[..4])
        || payload[8..]
            .as_chunks::<4>()
            .0
            .iter()
            .any(|brand| heif(brand))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&((payload.len() + 8) as u32).to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn sample(trailer: &[u8]) -> Vec<u8> {
        let names = [
            b"com.openharmony.covertime".as_slice(),
            b"com.openharmony.deferredVideoEnhanceFlag".as_slice(),
            b"com.openharmony.starttime".as_slice(),
            b"com.openharmony.videoId".as_slice(),
        ];
        let values: [&[u8]; 4] = [
            &1234.5f32.to_be_bytes(),
            b"0",
            b"1788718230582",
            b"20260907021031595",
        ];
        let mut keys_payload = vec![0, 0, 0, 0];
        keys_payload.extend_from_slice(&(names.len() as u32).to_be_bytes());
        for name in names {
            keys_payload.extend_from_slice(&((name.len() + 8) as u32).to_be_bytes());
            keys_payload.extend_from_slice(b"mdta");
            keys_payload.extend_from_slice(name);
        }
        let mut ilst_payload = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            let mut data_payload = vec![0; 8];
            data_payload.extend_from_slice(value);
            let data = make_box(b"data", &data_payload);
            let item_id = (index as u32 + 1).to_be_bytes();
            ilst_payload.extend_from_slice(&make_box(&item_id, &data));
        }
        let mut meta_payload = vec![0, 0, 0, 0];
        meta_payload.extend_from_slice(&make_box(b"hdlr", &[0; 8]));
        meta_payload.extend_from_slice(&make_box(b"keys", &keys_payload));
        meta_payload.extend_from_slice(&make_box(b"ilst", &ilst_payload));
        let mut still = make_box(b"ftyp", b"heic\0\0\0\0heicmif1");
        still.extend_from_slice(&make_box(b"mdat", b"heif-still-hdr"));
        still.extend_from_slice(&make_box(b"ftyp", b"mp42\0\0\0\0isommp42"));
        still.extend_from_slice(&make_box(b"mdat", b"video-samples"));
        still.extend_from_slice(&make_box(b"moov", &make_box(b"meta", &meta_payload)));
        still.extend_from_slice(trailer);
        still
    }

    #[test]
    fn reads_source_fields_and_keeps_the_v6_trailer_unparsed() {
        let trailer = {
            let mut bytes = b"v6_f27LIVE_".to_vec();
            bytes.extend(std::iter::repeat_n(b' ', 49));
            assert_eq!(bytes.len(), 60);
            bytes
        };
        let data = sample(&trailer);
        let parsed = parse_harmony_motion(&data).expect("harmony container");
        assert_eq!(parsed.cover_time_ms, 1234.5);
        assert_eq!(parsed.start_time_text.as_deref(), Some("1788718230582"));
        assert_eq!(parsed.video_id, "20260907021031595");
        assert_eq!(parsed.deferred_video_enhance, Some(false));
        assert_eq!(
            &data[parsed.video_start..parsed.video_start + 4],
            &24u32.to_be_bytes()
        );
        assert_eq!(
            &data[parsed.video_start + 4..parsed.video_start + 8],
            b"ftyp"
        );
        assert_eq!(parsed.trailer, trailer);
        assert!(parsed.trailer.starts_with(b"v6_"));
        assert!(parsed.trailer.windows(5).any(|window| window == b"LIVE_"));
        assert_eq!(
            parsed.video_start + parsed.movie_bytes + parsed.trailer.len(),
            data.len()
        );
    }

    #[test]
    fn missing_video_id_is_not_a_harmony_motion_photo() {
        let mut data = sample(b"v6_f27");
        let marker = b"com.openharmony.videoId";
        let start = data
            .windows(marker.len())
            .position(|window| window == marker)
            .unwrap();
        data[start..start + marker.len()].fill(b'x');
        assert!(parse_harmony_motion(&data).is_none());
    }

    #[test]
    fn jpeg_motion_is_not_the_harmony_container() {
        assert!(parse_harmony_motion(b"\xff\xd8jpeg-not-heic").is_none());
    }
}
