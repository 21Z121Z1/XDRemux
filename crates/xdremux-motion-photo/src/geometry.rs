//! XDRemux presentation policy. This module is not part of LibLivePhoto.
use crate::OppoMetadata;
const LEGACY_COLOROS16_EIS_COMPENSATION_SCALE: f64 = 0.90;
fn invert3(matrix: [f64; 9]) -> Option<[f64; 9]> {
    let [a, b, c, d, e, f, g, h, i] = matrix;
    let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if !determinant.is_finite() || determinant.abs() <= 1e-10 {
        return None;
    }
    let inverse = 1.0 / determinant;
    Some([
        (e * i - f * h) * inverse,
        (c * h - b * i) * inverse,
        (b * f - c * e) * inverse,
        (f * g - d * i) * inverse,
        (a * i - c * g) * inverse,
        (c * d - a * f) * inverse,
        (d * h - e * g) * inverse,
        (b * g - a * h) * inverse,
        (a * e - b * d) * inverse,
    ])
}

fn multiply3(left: [f64; 9], right: [f64; 9]) -> [f64; 9] {
    let mut output = [0.0; 9];
    for row in 0..3 {
        for column in 0..3 {
            output[row * 3 + column] = (0..3)
                .map(|index| left[row * 3 + index] * right[index * 3 + column])
                .sum();
        }
    }
    output
}

fn normalized_axis_scale(value: f64) -> Option<f64> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let scale = if value > 1.0 { 1.0 / value } else { value };
    (scale.is_finite() && scale > 0.0 && scale <= 1.0).then_some(scale)
}

fn normalized_scale(values: Option<&[f64]>) -> Option<(f64, f64)> {
    let values = values?;
    let first = *values.first()?;
    let second = values.get(1).copied().unwrap_or(first);
    Some((
        normalized_axis_scale(first)?,
        normalized_axis_scale(second)?,
    ))
}

fn normalize_homography(matrix: [f64; 9]) -> Option<[f64; 9]> {
    if matrix.iter().any(|value| !value.is_finite()) || matrix[8].abs() <= 1e-12 {
        return None;
    }
    let denominator = matrix[8];
    let mut output = matrix;
    for value in &mut output {
        *value /= denominator;
    }
    Some(output)
}

pub fn oppo_live_photo_transform(metadata: &OppoMetadata) -> Option<[f64; 9]> {
    let result = if metadata.version >= 1 {
        let (scale_x, scale_y) = normalized_scale(metadata.photo_eis_crop_factor.as_deref())
            .or_else(|| normalized_scale(metadata.eis_crop_factor.as_deref()))
            .unwrap_or((
                LEGACY_COLOROS16_EIS_COMPENSATION_SCALE,
                LEGACY_COLOROS16_EIS_COMPENSATION_SCALE,
            ));
        let mut result = [scale_x, 0.0, 0.0, 0.0, scale_y, 0.0, 0.0, 0.0, 1.0];
        if let Some(matrix) = metadata.photo_crop_matrix.and_then(invert3) {
            result = multiply3(result, matrix);
        }
        if let Some(matrix) = metadata.photo_eis_matrix.and_then(invert3) {
            result = multiply3(result, matrix);
        }
        result
    } else {
        if metadata.matrix_count <= 0 || metadata.matrices.is_empty() {
            return None;
        }
        let cover = metadata.cover_frame_pts_us?;
        let matrix = metadata
            .matrices
            .iter()
            .filter_map(|(key, matrix)| key.parse::<i64>().ok().map(|pts| (pts, *matrix)))
            .min_by_key(|(pts, _)| (i128::from(*pts) - i128::from(cover)).abs())?
            .1;
        invert3(matrix).unwrap_or(matrix)
    };

    let normalized = normalize_homography(result)?;
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    if normalized
        .iter()
        .zip(identity)
        .all(|(left, right)| (*left - right).abs() <= 1e-6)
    {
        None
    } else {
        Some(normalized)
    }
}

/// Preserve the existing XDRemux presentation policy at the product boundary.
pub fn presentation_geometry(
    metadata: Option<&OppoMetadata>,
) -> liblivephoto::apple::GeometryMetadata {
    let transform = metadata.and_then(oppo_live_photo_transform);
    let dimension = |value: Option<i64>| {
        value
            .filter(|v| *v > 0 && *v <= 16_777_216)
            .map(|v| v as f32)
    };
    let dimensions = if transform.is_some() {
        metadata.and_then(|m| Some((dimension(m.video_width)?, dimension(m.video_height)?)))
    } else {
        None
    };
    liblivephoto::apple::GeometryMetadata {
        transform,
        dimensions,
    }
}

/// Legacy XDRemux API. Exact time is used by the runtime facade path.
pub fn write_live_photo_movie(
    source: &[u8],
    identifier: &str,
    seconds: f64,
    metadata: Option<&OppoMetadata>,
) -> crate::LivePhotoMovieResult<Vec<u8>> {
    liblivephoto::compat::write_live_photo_movie(
        source,
        identifier,
        seconds,
        Some(presentation_geometry(metadata)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn coloros16_transform_matches_existing_product_policy() {
        let metadata = OppoMetadata {
            version: 1,
            photo_eis_crop_factor: Some(vec![1.11, 1.12]),
            video_width: Some(1728),
            video_height: Some(1296),
            ..OppoMetadata::default()
        };
        let geometry = presentation_geometry(Some(&metadata));
        let transform = geometry.transform.unwrap();
        assert!((transform[0] - 1.0 / 1.11).abs() < 1e-12);
        assert!((transform[4] - 1.0 / 1.12).abs() < 1e-12);
        assert_eq!(transform[8], 1.0);
        assert_eq!(geometry.dimensions, Some((1728.0, 1296.0)));
    }

    #[test]
    fn coloros15_uses_closest_cover_frame_and_inverts_matrix() {
        let mut matrices = BTreeMap::new();
        matrices.insert("1000".into(), [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0]);
        matrices.insert("2000".into(), [4.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 1.0]);
        let metadata = OppoMetadata {
            cover_frame_pts_us: Some(1100),
            matrix_count: 2,
            matrices,
            ..OppoMetadata::default()
        };
        let transform = oppo_live_photo_transform(&metadata).unwrap();
        assert!((transform[0] - 0.5).abs() < 1e-12);
        assert!((transform[4] - 0.5).abs() < 1e-12);
        assert_eq!(transform[8], 1.0);
    }
}
