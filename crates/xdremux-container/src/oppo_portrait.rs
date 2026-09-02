use crate::{ContainerError, Result};

const CONFIG_CONTEXT: &str = "rear.depth.config";
const MAX_DIMENSION: i32 = 16_384;
const BLUR_SAMPLE_COUNT: usize = 32;
const FACE_KEYPOINT_COUNT: usize = 296;
const MAX_FACE_COUNT: i32 = 10;
const V4_MINIMUM_BYTES: usize = 27_260;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OppoPortraitFace {
    pub rectangle: [i32; 4],
    pub angle: i32,
    pub keypoint_x: [i32; FACE_KEYPOINT_COUNT],
    pub keypoint_y: [i32; FACE_KEYPOINT_COUNT],
    pub keypoint_confidence: [i8; FACE_KEYPOINT_COUNT],
}

#[derive(Debug, Clone, PartialEq)]
pub struct OppoPortraitConfig {
    pub version: f32,
    pub processing_width: u32,
    pub processing_height: u32,
    pub focus_x: i32,
    pub focus_y: i32,
    pub blur_apertures: [f32; BLUR_SAMPLE_COUNT],
    pub blur_values: [f32; BLUR_SAMPLE_COUNT],
    pub current_blur_strength: i32,
    pub camera_roll: i32,
    pub spotlight_width: Option<i32>,
    pub spotlight_height: Option<i32>,
    pub current_f_number: Option<f32>,
    pub object_distance: Option<u32>,
    pub tele_master: Option<bool>,
    pub focus_rectangle: Option<[i32; 4]>,
    pub focus_rectangle_is_valid: bool,
    pub mirror_enabled: Option<bool>,
    pub refocus_mode: Option<i32>,
    pub foreground_blur_scale: Option<i32>,
    pub big_face_enabled: Option<bool>,
    pub pets_enabled: Option<bool>,
    pub multi_semantic_segmentation_enabled: Option<bool>,
    pub bokeh_version: Option<i32>,
    pub iso: Option<i32>,
    pub zoom_ratio: Option<i32>,
    pub focus_roi_type: Option<i32>,
    pub shutter: Option<f32>,
    pub aec_lux_index: Option<f32>,
    pub faces: Vec<OppoPortraitFace>,
}

fn truncated(field: &str) -> ContainerError {
    ContainerError::invalid(CONFIG_CONTEXT, format!("truncated at {field}"))
}

fn invalid(field: &str) -> ContainerError {
    ContainerError::invalid(CONFIG_CONTEXT, format!("invalid {field}"))
}

fn read_bytes<const N: usize>(data: &[u8], offset: usize, field: &str) -> Result<[u8; N]> {
    data.get(offset..)
        .and_then(|remaining| remaining.first_chunk::<N>())
        .copied()
        .ok_or_else(|| truncated(field))
}

fn read_i32_le(data: &[u8], offset: usize, field: &str) -> Result<i32> {
    Ok(i32::from_le_bytes(read_bytes(data, offset, field)?))
}

fn read_f32_le(data: &[u8], offset: usize, field: &str) -> Result<f32> {
    let value = f32::from_le_bytes(read_bytes(data, offset, field)?);
    if !value.is_finite() {
        return Err(invalid(field));
    }
    Ok(value)
}

fn read_bool_byte(data: &[u8], offset: usize, field: &str) -> Result<bool> {
    data.get(offset)
        .copied()
        .map(|value| value != 0)
        .ok_or_else(|| truncated(field))
}

fn read_i32_array<const N: usize>(
    data: &[u8],
    offset: usize,
    field: &str,
) -> Result<[i32; N]> {
    let mut values = [0_i32; N];
    for (index, value) in values.iter_mut().enumerate() {
        let byte_offset = offset
            .checked_add(index.checked_mul(4).ok_or_else(|| invalid(field))?)
            .ok_or_else(|| invalid(field))?;
        *value = read_i32_le(data, byte_offset, field)?;
    }
    Ok(values)
}

fn read_f32_array<const N: usize>(
    data: &[u8],
    offset: usize,
    field: &str,
) -> Result<[f32; N]> {
    let mut values = [0.0_f32; N];
    for (index, value) in values.iter_mut().enumerate() {
        let byte_offset = offset
            .checked_add(index.checked_mul(4).ok_or_else(|| invalid(field))?)
            .ok_or_else(|| invalid(field))?;
        *value = read_f32_le(data, byte_offset, field)?;
    }
    Ok(values)
}

fn positive_u32(value: i32, field: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| invalid(field))
}

/// Parse OPPO's versioned `rear.depth.config` producer record.
///
/// This parser owns only the portable binary contract. It deliberately does not
/// decide Apple Portrait policy, invoke Apple frameworks, or infer values that
/// are absent from the producer payload.
pub fn parse_oppo_portrait_config(data: &[u8]) -> Result<OppoPortraitConfig> {
    let version = read_f32_le(data, 0, "version")?;
    if !(1.0..=4.0).contains(&version) {
        return Err(invalid("version"));
    }

    let width_raw = read_i32_le(data, 4, "depth width")?;
    let height_raw = read_i32_le(data, 8, "depth height")?;
    if !(1..=MAX_DIMENSION).contains(&width_raw) || !(1..=MAX_DIMENSION).contains(&height_raw) {
        return Err(invalid("dimensions"));
    }

    let blur_apertures = read_f32_array::<BLUR_SAMPLE_COUNT>(data, 20, "blur aperture")?;
    let blur_values = read_f32_array::<BLUR_SAMPLE_COUNT>(data, 148, "blur value")?;

    let mut spotlight_width = None;
    let mut spotlight_height = None;
    let mut current_f_number = None;
    let mut object_distance = None;
    let mut tele_master = None;
    let mut focus_rectangle = None;
    let mut focus_rectangle_is_valid = false;
    let mut mirror_enabled = None;
    let mut refocus_mode = None;
    let mut foreground_blur_scale = None;
    let mut big_face_enabled = None;
    let mut pets_enabled = None;
    let mut multi_semantic_segmentation_enabled = None;
    let mut bokeh_version = None;
    let mut iso = None;
    let mut zoom_ratio = None;
    let mut focus_roi_type = None;
    let mut shutter = None;
    let mut aec_lux_index = None;
    let mut faces = Vec::new();

    if version >= 2.0 {
        spotlight_width = Some(read_i32_le(data, 284, "spotlight width")?);
        spotlight_height = Some(read_i32_le(data, 288, "spotlight height")?);
        let aperture = read_f32_le(data, 292, "current f-number")?;
        current_f_number = (1.0..=64.0).contains(&aperture).then_some(aperture);
        let distance = read_i32_le(data, 296, "object distance")?;
        object_distance = (distance > 0).then(|| positive_u32(distance, "object distance")).transpose()?;
        tele_master = Some(read_bool_byte(data, 300, "tele-master flag")?);
        let _ = read_i32_le(data, 304, "reference EV")?;
        let _ = read_i32_le(data, 308, "minimum EV")?;
        let _ = read_i32_le(data, 312, "scene mode")?;
    }

    if version >= 2.2 {
        focus_rectangle = Some(read_i32_array::<4>(data, 316, "focus rectangle")?);
        focus_rectangle_is_valid = read_bool_byte(data, 332, "focus rectangle validity")?;
    }

    if version >= 2.3 {
        mirror_enabled = Some(read_i32_le(data, 336, "mirror flag")? != 0);
    }

    if version >= 2.4 {
        refocus_mode = Some(read_i32_le(data, 340, "refocus mode")?);
        let _ = read_i32_le(data, 344, "light spot strength")?;
        let _ = read_i32_le(data, 348, "bright spot trigger")?;
        let _ = read_f32_le(data, 352, "curve value")?;
        let _ = read_i32_le(data, 356, "shine threshold")?;
        let _ = read_i32_le(data, 360, "shine level")?;
        let _ = read_i32_le(data, 364, "spot sharpen amount")?;
        let _ = read_i32_le(data, 368, "spot sharpen radius")?;
        foreground_blur_scale = Some(read_i32_le(data, 372, "foreground blur scale")?);
        let _ = read_i32_le(data, 376, "master type")?;
    }

    if version >= 2.5 {
        big_face_enabled = Some(read_i32_le(data, 380, "big-face flag")? != 0);
        pets_enabled = Some(read_i32_le(data, 384, "pet flag")? != 0);
        multi_semantic_segmentation_enabled =
            Some(read_i32_le(data, 388, "multi-semantic flag")? != 0);
    }

    if version >= 4.0 {
        bokeh_version = Some(read_i32_le(data, 392, "bokeh version")?);
        iso = Some(read_i32_le(data, 396, "ISO")?);
        zoom_ratio = Some(read_i32_le(data, 400, "zoom ratio")?);
        focus_roi_type = Some(read_i32_le(data, 404, "focus ROI type")?);
        shutter = Some(read_f32_le(data, 408, "shutter")?);
        aec_lux_index = Some(read_f32_le(data, 412, "AEC lux index")?);
        let face_count = read_i32_le(data, 416, "face count")?;
        if !(0..=MAX_FACE_COUNT).contains(&face_count) || data.len() < V4_MINIMUM_BYTES {
            return Err(ContainerError::invalid(
                CONFIG_CONTEXT,
                "v4 face table is invalid or truncated",
            ));
        }

        faces.reserve(usize::try_from(face_count).unwrap_or_default());
        for face_index in 0..usize::try_from(face_count).unwrap_or_default() {
            let rectangle_offset = 420 + face_index * 16;
            let angle_offset = 580 + face_index * 4;
            let keypoint_x_offset = 620 + face_index * FACE_KEYPOINT_COUNT * 4;
            let keypoint_y_offset = 12_460 + face_index * FACE_KEYPOINT_COUNT * 4;
            let confidence_offset = 24_300 + face_index * FACE_KEYPOINT_COUNT;
            let confidence_bytes = data
                .get(confidence_offset..confidence_offset + FACE_KEYPOINT_COUNT)
                .ok_or_else(|| truncated("face keypoint confidence"))?;
            let mut confidence = [0_i8; FACE_KEYPOINT_COUNT];
            for (destination, source) in confidence.iter_mut().zip(confidence_bytes) {
                *destination = i8::from_ne_bytes([*source]);
            }
            faces.push(OppoPortraitFace {
                rectangle: read_i32_array::<4>(data, rectangle_offset, "face rectangle")?,
                angle: read_i32_le(data, angle_offset, "face angle")?,
                keypoint_x: read_i32_array::<FACE_KEYPOINT_COUNT>(
                    data,
                    keypoint_x_offset,
                    "face keypoint X",
                )?,
                keypoint_y: read_i32_array::<FACE_KEYPOINT_COUNT>(
                    data,
                    keypoint_y_offset,
                    "face keypoint Y",
                )?,
                keypoint_confidence: confidence,
            });
        }
    }

    Ok(OppoPortraitConfig {
        version,
        processing_width: positive_u32(width_raw, "depth width")?,
        processing_height: positive_u32(height_raw, "depth height")?,
        focus_x: read_i32_le(data, 12, "focus X")?,
        focus_y: read_i32_le(data, 16, "focus Y")?,
        blur_apertures,
        blur_values,
        current_blur_strength: read_i32_le(data, 276, "current blur strength")?,
        camera_roll: read_i32_le(data, 280, "camera roll")?,
        spotlight_width,
        spotlight_height,
        current_f_number,
        object_distance,
        tele_master,
        focus_rectangle,
        focus_rectangle_is_valid,
        mirror_enabled,
        refocus_mode,
        foreground_blur_scale,
        big_face_enabled,
        pets_enabled,
        multi_semantic_segmentation_enabled,
        bokeh_version,
        iso,
        zoom_ratio,
        focus_roi_type,
        shutter,
        aec_lux_index,
        faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn put_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_f32(data: &mut [u8], offset: usize, value: f32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn base_config(version: f32, size: usize) -> Vec<u8> {
        let mut data = vec![0_u8; size];
        put_f32(&mut data, 0, version);
        put_i32(&mut data, 4, 900);
        put_i32(&mut data, 8, 1200);
        put_i32(&mut data, 12, 300);
        put_i32(&mut data, 16, 500);
        for index in 0..BLUR_SAMPLE_COUNT {
            put_f32(&mut data, 20 + index * 4, 1.0 + index as f32 * 0.1);
            put_f32(&mut data, 148 + index * 4, index as f32);
        }
        put_i32(&mut data, 276, 20);
        put_i32(&mut data, 280, 0);
        data
    }

    #[test]
    fn parses_version_one_without_inventing_newer_fields() {
        let config = parse_oppo_portrait_config(&base_config(1.0, 284)).unwrap();
        assert_eq!(config.version, 1.0);
        assert_eq!((config.processing_width, config.processing_height), (900, 1200));
        assert_eq!((config.focus_x, config.focus_y), (300, 500));
        assert_eq!(config.current_blur_strength, 20);
        assert_eq!(config.current_f_number, None);
        assert_eq!(config.focus_rectangle, None);
        assert!(config.faces.is_empty());
    }

    #[test]
    fn parses_version_four_face_table_into_fixed_shape_records() {
        let mut data = base_config(4.0, V4_MINIMUM_BYTES);
        put_i32(&mut data, 284, 640);
        put_i32(&mut data, 288, 480);
        put_f32(&mut data, 292, 2.8);
        put_i32(&mut data, 296, 123);
        data[300] = 1;
        put_i32(&mut data, 316, 10);
        put_i32(&mut data, 320, 20);
        put_i32(&mut data, 324, 110);
        put_i32(&mut data, 328, 220);
        data[332] = 1;
        put_i32(&mut data, 336, 1);
        put_i32(&mut data, 340, 2);
        put_f32(&mut data, 352, 0.5);
        put_i32(&mut data, 372, 7);
        put_i32(&mut data, 380, 1);
        put_i32(&mut data, 384, 1);
        put_i32(&mut data, 388, 1);
        put_i32(&mut data, 392, 4);
        put_i32(&mut data, 396, 100);
        put_i32(&mut data, 400, 3);
        put_i32(&mut data, 404, 1);
        put_f32(&mut data, 408, 0.01);
        put_f32(&mut data, 412, 42.0);
        put_i32(&mut data, 416, 1);
        put_i32(&mut data, 420, 1);
        put_i32(&mut data, 424, 2);
        put_i32(&mut data, 428, 3);
        put_i32(&mut data, 432, 4);
        put_i32(&mut data, 580, 90);
        for index in 0..FACE_KEYPOINT_COUNT {
            put_i32(&mut data, 620 + index * 4, index as i32);
            put_i32(&mut data, 12_460 + index * 4, 1000 + index as i32);
            data[24_300 + index] = (index % 128) as u8;
        }

        let config = parse_oppo_portrait_config(&data).unwrap();
        assert_eq!(config.current_f_number, Some(2.8));
        assert_eq!(config.object_distance, Some(123));
        assert_eq!(config.tele_master, Some(true));
        assert_eq!(config.focus_rectangle, Some([10, 20, 110, 220]));
        assert!(config.focus_rectangle_is_valid);
        assert_eq!(config.faces.len(), 1);
        assert_eq!(config.faces[0].rectangle, [1, 2, 3, 4]);
        assert_eq!(config.faces[0].angle, 90);
        assert_eq!(config.faces[0].keypoint_x[295], 295);
        assert_eq!(config.faces[0].keypoint_y[295], 1295);
    }

    #[test]
    fn rejects_non_finite_or_truncated_records() {
        let mut invalid_version = base_config(1.0, 284);
        put_f32(&mut invalid_version, 0, f32::NAN);
        assert!(parse_oppo_portrait_config(&invalid_version).is_err());

        let truncated = base_config(2.0, 300);
        assert!(parse_oppo_portrait_config(&truncated).is_err());
    }

    #[test]
    fn parses_committed_find_x9_ultra_portrait_config() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic");
        let source = fs::read(fixture).expect("read committed portrait fixture");
        let blocks = crate::portrait_blocks(&source).expect("extract OPPO portrait blocks");
        let config = blocks
            .get("rear.depth.config")
            .expect("fixture contains rear.depth.config");
        let parsed = parse_oppo_portrait_config(config).expect("parse real OPPO portrait config");
        assert!((1.0..=4.0).contains(&parsed.version));
        assert!(parsed.processing_width > 0);
        assert!(parsed.processing_height > 0);
    }
}
