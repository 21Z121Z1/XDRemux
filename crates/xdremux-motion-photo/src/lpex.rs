use std::collections::BTreeMap;

use serde_json::Value;

use crate::model::OppoMetadata;

const NEEDLES: [&[u8]; 3] = [
    b"lpexLivePhotoExtension",
    b"LivePhotoExtension",
    b"pexLivePhotoExtension",
];
const MAX_JSON_BYTES: usize = 256 * 1024;

fn balanced_json_range(data: &[u8], start: usize) -> Option<std::ops::Range<usize>> {
    if data.get(start).copied()? != b'{' {
        return None;
    }
    let limit = data.len().min(start.checked_add(MAX_JSON_BYTES + 1)?);
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaping = false;
    for index in start..limit {
        let byte = data[index];
        if in_string {
            if escaping {
                escaping = false;
            } else if byte == b'\\' {
                escaping = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth = depth.checked_add(1)?,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(start..index + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn number_as_i64(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_u64().and_then(|value| i64::try_from(value).ok()))
            .or_else(|| {
                number.as_f64().and_then(|value| {
                    value.is_finite()
                        .then(|| value.trunc())
                        .filter(|value| *value >= i64::MIN as f64 && *value <= i64::MAX as f64)
                        .map(|value| value as i64)
                })
            }),
        Value::String(text) if text.len() <= 32 => text.parse().ok(),
        Value::Bool(flag) => Some(i64::from(*flag)),
        _ => None,
    }
}

fn number_as_f64(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) if text.len() <= 64 => text.parse().ok(),
        Value::Bool(flag) => Some(if *flag { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn number_array(value: Option<&Value>, max_count: usize) -> Option<Vec<f64>> {
    let values = value?.as_array()?;
    if values.len() > max_count {
        return None;
    }
    let parsed: Option<Vec<_>> = values
        .iter()
        .map(|value| number_as_f64(Some(value)))
        .collect();
    let parsed = parsed?;
    parsed
        .iter()
        .all(|value| value.is_finite())
        .then_some(parsed)
}

fn matrix(value: Option<&Value>) -> Option<[f64; 9]> {
    let values = number_array(value, 9)?;
    if values.len() != 9 {
        return None;
    }
    values.try_into().ok()
}

fn size(value: Option<&Value>) -> (Option<i64>, Option<i64>) {
    let Some(values) = value.and_then(Value::as_array) else {
        return (None, None);
    };
    if values.len() < 2 {
        return (None, None);
    }
    let width = number_as_i64(values.first());
    let height = number_as_i64(values.get(1));
    match (width, height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => (Some(width), Some(height)),
        _ => (None, None),
    }
}

fn parse_json(raw: &[u8]) -> Option<OppoMetadata> {
    if raw.len() > MAX_JSON_BYTES {
        return None;
    }
    let object = serde_json::from_slice::<Value>(raw).ok()?;
    let dictionary = object.as_object()?;
    let (video_width, video_height) = size(dictionary.get("videoSize"));
    let (origin_photo_width, origin_photo_height) = size(dictionary.get("originPhotoSize"));

    let mut matrices = BTreeMap::new();
    if let Some(raw_matrices) = dictionary.get("matrices").and_then(Value::as_object) {
        if raw_matrices.len() <= 4096 {
            for (key, value) in raw_matrices {
                if key.len() <= 128 {
                    if let Some(parsed) = matrix(Some(value)) {
                        matrices.insert(key.clone(), parsed);
                    }
                }
            }
        }
    }

    Some(OppoMetadata {
        cover_frame_pts_us: number_as_i64(dictionary.get("coverFramePts")),
        version: number_as_i64(dictionary.get("version")).unwrap_or(0),
        matrix_count: number_as_i64(dictionary.get("matrixCount")).unwrap_or(0),
        photo_crop_matrix: matrix(dictionary.get("photoCropMatrix")),
        photo_eis_matrix: matrix(dictionary.get("photoEisMatrix")),
        matrices,
        video_width,
        video_height,
        origin_photo_width,
        origin_photo_height,
        photo_eis_crop_factor: number_array(dictionary.get("photoEisCropFactor"), 8),
        eis_crop_factor: number_array(dictionary.get("eisCropFactor"), 8),
        photo_crop_factor: number_as_f64(dictionary.get("photoCropFactor")),
        stream_count: 1,
    })
}

pub fn parse_first_lpex_object(data: &[u8]) -> Option<OppoMetadata> {
    for needle in NEEDLES {
        let mut search_start = 0usize;
        while search_start < data.len() {
            let Some(relative) = data[search_start..]
                .windows(needle.len())
                .position(|window| window == needle)
            else {
                break;
            };
            let found = search_start.checked_add(relative)?;
            let after = found.checked_add(needle.len())?;
            if after >= data.len() {
                break;
            }
            let search_end = data.len().min(after.checked_add(33)?);
            if let Some(relative_brace) = data[after..search_end]
                .iter()
                .position(|byte| *byte == b'{')
            {
                let brace = after.checked_add(relative_brace)?;
                if let Some(range) = balanced_json_range(data, brace) {
                    if range.len() <= MAX_JSON_BYTES {
                        if let Some(parsed) = parse_json(&data[range]) {
                            return Some(parsed);
                        }
                    }
                }
            }
            search_start = after;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_swift_contract_fields_without_normalizing_matrix_keys() {
        let data = br#"prefix lpexLivePhotoExtension {"version":1,"matrixCount":2,"coverFramePts":1433000,"photoCropMatrix":[1,0,0,0,1,0,0,0,1],"matrices":{"frame-A":[1,0,0,0,1,0,0,0,1]},"videoSize":[1728,1296],"originPhotoSize":[4096,3072],"photoEisCropFactor":[1.11,1.12],"eisCropFactor":[0.9,0.91],"photoCropFactor":0.9} suffix"#;
        let metadata = parse_first_lpex_object(data).unwrap();
        assert_eq!(metadata.version, 1);
        assert_eq!(metadata.cover_frame_pts_us, Some(1_433_000));
        assert!(metadata.matrices.contains_key("frame-A"));
        assert_eq!(metadata.video_width, Some(1728));
        assert_eq!(metadata.photo_eis_crop_factor, Some(vec![1.11, 1.12]));
    }

    #[test]
    fn parses_compatibility_needle_when_primary_needle_is_absent() {
        let data = br#"prefix LivePhotoExtension {"version":2,"coverFramePts":7}"#;
        let metadata = parse_first_lpex_object(data).unwrap();
        assert_eq!(metadata.version, 2);
        assert_eq!(metadata.cover_frame_pts_us, Some(7));
    }

    #[test]
    fn rejects_non_finite_matrix_but_keeps_object() {
        let data = br#"lpexLivePhotoExtension {"version":1,"photoCropMatrix":["nan",0,0,0,1,0,0,0,1]}"#;
        let metadata = parse_first_lpex_object(data).unwrap();
        assert!(metadata.photo_crop_matrix.is_none());
    }

    #[test]
    fn ignores_braces_inside_strings() {
        let data = br#"lpexLivePhotoExtension {"version":1,"note":"} { escaped \\" still string","coverFramePts":42}"#;
        let metadata = parse_first_lpex_object(data).unwrap();
        assert_eq!(metadata.cover_frame_pts_us, Some(42));
    }
}
