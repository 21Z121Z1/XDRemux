use std::error::Error;
use std::fmt;

use crate::{OppoPortraitConfig, OppoPortraitDepth, OppoPortraitDepthHeader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OppoPortraitFocusBranch {
    TappedFace,
    PortraitFace,
    PortraitWithoutFace,
    PetFace,
    PetRegion,
    NearObject,
    TappedRegion,
    CenterRegion,
    DisparityHistogram,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OppoPortraitFocusRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl OppoPortraitFocusRegion {
    pub fn normalized(self) -> Self {
        let x = self.x.clamp(0.0, 1.0);
        let y = self.y.clamp(0.0, 1.0);
        Self {
            x,
            y,
            width: self.width.clamp(0.0, 1.0 - x),
            height: self.height.clamp(0.0, 1.0 - y),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OppoPortraitFocusSelection {
    pub branch: OppoPortraitFocusBranch,
    pub source_roi: OppoPortraitFocusRegion,
    pub selected_rank: f64,
    pub internal_disparity: Option<f64>,
    pub config_distance: Option<f64>,
    pub confidence: f64,
    pub sample_count: usize,
    pub rejected_sample_count: usize,
    pub roi_is_producer_exact: bool,
    pub statistic_is_producer_exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OppoPortraitBlurSample {
    pub aperture: f64,
    pub blur_value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OppoPortraitZoomRegion {
    OneToBelowTwo,
    TwoToBelowThree,
    ThreeToBelowSix,
    SixToTen,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OppoPortraitBlurResponse {
    pub samples: Vec<OppoPortraitBlurSample>,
    pub selected_aperture: f64,
    pub selected_blur_value: f64,
    pub foreground_blur_scale: f64,
    pub zoom_region: OppoPortraitZoomRegion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OppoPortraitFocusError {
    InvalidGeometry,
    RankPlaneSizeMismatch,
    NoValidBlurCurve,
    NoFocusCandidates,
}

impl fmt::Display for OppoPortraitFocusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeometry => formatter.write_str("OPPO Portrait focus geometry is invalid"),
            Self::RankPlaneSizeMismatch => {
                formatter.write_str("OPPO Portrait rank plane does not match its geometry")
            }
            Self::NoValidBlurCurve => {
                formatter.write_str("OPPO Portrait config has no valid aperture/blur curve")
            }
            Self::NoFocusCandidates => {
                formatter.write_str("OPPO Portrait focus selection has no rank candidates")
            }
        }
    }
}

impl Error for OppoPortraitFocusError {}

/// Recovered `CalFocusDepthEngine::calcFocusDepth` branch dispatch.
///
/// This is OPPO producer behavior and intentionally contains no Apple policy.
pub const fn oppo_portrait_focus_branch(
    near_object_detected: bool,
    scene_class: i32,
    focus_roi_type: i32,
    focused_face_available: bool,
    portrait_plane_available: bool,
    pet_plane_available: bool,
) -> OppoPortraitFocusBranch {
    if near_object_detected {
        if scene_class == 0 {
            return OppoPortraitFocusBranch::NearObject;
        }
        if scene_class == 2 && focus_roi_type == 3 {
            return OppoPortraitFocusBranch::PetRegion;
        }
        return OppoPortraitFocusBranch::CenterRegion;
    }

    match focus_roi_type {
        1 if focused_face_available => OppoPortraitFocusBranch::TappedFace,
        1 if portrait_plane_available => OppoPortraitFocusBranch::PortraitFace,
        1 => OppoPortraitFocusBranch::PortraitWithoutFace,
        2 => OppoPortraitFocusBranch::CenterRegion,
        3 if pet_plane_available => OppoPortraitFocusBranch::PetRegion,
        3 => OppoPortraitFocusBranch::PortraitWithoutFace,
        _ if portrait_plane_available => OppoPortraitFocusBranch::PortraitWithoutFace,
        _ => OppoPortraitFocusBranch::DisparityHistogram,
    }
}

/// Recovered producer-shaped 20-bin depth histogram.
pub fn oppo_portrait_focus_histogram(
    values: &[u8],
    header: &OppoPortraitDepthHeader,
    target_fraction: f64,
) -> f64 {
    let depths = values
        .iter()
        .filter_map(|value| header.native_float_depth(f64::from(*value)))
        .collect::<Vec<_>>();
    let Some(raw_minimum) = depths.iter().copied().reduce(f64::min) else {
        return 0.0;
    };
    let Some(raw_maximum) = depths.iter().copied().reduce(f64::max) else {
        return 0.0;
    };

    let minimum = raw_minimum;
    let maximum = raw_maximum.min(15_000.0);
    if maximum <= minimum {
        return header
            .rank_for_native_float_depth(minimum)
            .unwrap_or_else(|| f64::from(values[values.len() / 2]));
    }

    let mut counts = [0_usize; 20];
    let mut sums = [0.0_f64; 20];
    let span = maximum - minimum;
    for depth in &depths {
        let delta = *depth - minimum;
        let normalized = (delta.clamp(0.0, span) / span) * 20.0;
        let bin = (normalized.floor() as usize).min(19);
        counts[bin] += 1;
        sums[bin] += delta;
    }

    let target = ((depths.len() as f64) * target_fraction.max(0.0)).floor() as usize;
    let mut cumulative = 0_usize;
    for index in 0..counts.len() {
        cumulative += counts[index];
        if cumulative > target {
            let selected_depth = if counts[index] > 0 {
                sums[index] / counts[index] as f64 + minimum
            } else {
                minimum
            };
            return header
                .rank_for_native_float_depth(selected_depth)
                .unwrap_or_else(|| f64::from(values[values.len() / 2]));
        }
    }

    header
        .rank_for_native_float_depth(maximum)
        .unwrap_or_else(|| f64::from(values.last().copied().unwrap_or(0)))
}

pub fn oppo_portrait_blur_response(
    config: &OppoPortraitConfig,
    header: &OppoPortraitDepthHeader,
) -> Result<OppoPortraitBlurResponse, OppoPortraitFocusError> {
    let samples = config
        .blur_apertures
        .iter()
        .copied()
        .zip(config.blur_values.iter().copied())
        .filter_map(|(aperture, blur_value)| {
            let aperture = f64::from(aperture);
            let blur_value = f64::from(blur_value);
            (aperture.is_finite() && blur_value.is_finite() && aperture > 0.0 && blur_value >= 0.0)
                .then_some(OppoPortraitBlurSample {
                    aperture,
                    blur_value,
                })
        })
        .collect::<Vec<_>>();
    let Some(first) = samples.first() else {
        return Err(OppoPortraitFocusError::NoValidBlurCurve);
    };

    let selected_aperture = config
        .current_f_number
        .map(f64::from)
        .unwrap_or(first.aperture);
    let zoom = header
        .app_zoom_ratio
        .map(f64::from)
        .unwrap_or_else(|| f64::from(config.zoom_ratio.unwrap_or(100)) / 100.0);
    let zoom_region = if zoom < 2.0 {
        OppoPortraitZoomRegion::OneToBelowTwo
    } else if zoom < 3.0 {
        OppoPortraitZoomRegion::TwoToBelowThree
    } else if zoom < 6.0 {
        OppoPortraitZoomRegion::ThreeToBelowSix
    } else {
        OppoPortraitZoomRegion::SixToTen
    };

    Ok(OppoPortraitBlurResponse {
        samples,
        selected_aperture,
        selected_blur_value: f64::from(config.current_blur_strength),
        foreground_blur_scale: f64::from(config.foreground_blur_scale.unwrap_or(100)),
        zoom_region,
    })
}

pub fn select_oppo_portrait_focus(
    depth: &OppoPortraitDepth,
    config: &OppoPortraitConfig,
    source_width: u32,
    source_height: u32,
    focus: OppoPortraitFocusRegion,
) -> Result<OppoPortraitFocusSelection, OppoPortraitFocusError> {
    if source_width == 0 || source_height == 0 || depth.header.width == 0 || depth.header.height == 0 {
        return Err(OppoPortraitFocusError::InvalidGeometry);
    }
    let width = usize::try_from(depth.header.width).map_err(|_| OppoPortraitFocusError::InvalidGeometry)?;
    let height = usize::try_from(depth.header.height).map_err(|_| OppoPortraitFocusError::InvalidGeometry)?;
    let expected = width
        .checked_mul(height)
        .ok_or(OppoPortraitFocusError::InvalidGeometry)?;
    if depth.ranks.len() != expected {
        return Err(OppoPortraitFocusError::RankPlaneSizeMismatch);
    }

    let source_width_i64 = i64::from(source_width);
    let source_height_i64 = i64::from(source_height);
    let normalized_ltwh = |rectangle: [i32; 4]| -> Option<OppoPortraitFocusRegion> {
        if rectangle[2] <= 0 || rectangle[3] <= 0 {
            return None;
        }
        let x0 = i64::from(rectangle[0]).clamp(0, source_width_i64);
        let y0 = i64::from(rectangle[1]).clamp(0, source_height_i64);
        let x1 = (i64::from(rectangle[0]) + i64::from(rectangle[2]))
            .clamp(x0, source_width_i64);
        let y1 = (i64::from(rectangle[1]) + i64::from(rectangle[3]))
            .clamp(y0, source_height_i64);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(OppoPortraitFocusRegion {
            x: x0 as f64 / f64::from(source_width),
            y: y0 as f64 / f64::from(source_height),
            width: (x1 - x0) as f64 / f64::from(source_width),
            height: (y1 - y0) as f64 / f64::from(source_height),
        })
    };
    let contains_focus = |rectangle: [i32; 4]| {
        rectangle[2] > 0
            && rectangle[3] > 0
            && config.focus_x >= rectangle[0]
            && config.focus_x < rectangle[0].saturating_add(rectangle[2])
            && config.focus_y >= rectangle[1]
            && config.focus_y < rectangle[1].saturating_add(rectangle[3])
    };
    let focused_face = config
        .faces
        .iter()
        .find(|face| contains_focus(face.rectangle));

    let focus = focus.normalized();
    let focus_depth_x = ((focus.x * width as f64).floor() as usize).min(width - 1);
    let focus_depth_y = ((focus.y * height as f64).floor() as usize).min(height - 1);
    let has_portrait = depth
        .portrait
        .as_deref()
        .is_some_and(|plane| plane.iter().any(|value| *value >= 127));
    let has_pet = depth
        .pet
        .as_deref()
        .is_some_and(|plane| plane.iter().any(|value| *value >= 127));

    let branch = oppo_portrait_focus_branch(
        depth.header.near_object_detected,
        depth.header.scene_class,
        config.focus_roi_type.unwrap_or(0),
        focused_face.is_some(),
        has_portrait,
        has_pet,
    );
    let exact_pet_histogram_fallback =
        branch == OppoPortraitFocusBranch::PetRegion && config.faces.is_empty();
    let exact_full_image_histogram =
        branch == OppoPortraitFocusBranch::DisparityHistogram || exact_pet_histogram_fallback;

    let mut source_roi = if branch == OppoPortraitFocusBranch::CenterRegion {
        let x0 = focus_depth_x.saturating_sub(2);
        let y0 = focus_depth_y.saturating_sub(2);
        let x1 = (focus_depth_x + 3).min(width);
        let y1 = (focus_depth_y + 3).min(height);
        OppoPortraitFocusRegion {
            x: x0 as f64 / width as f64,
            y: y0 as f64 / height as f64,
            width: (x1 - x0) as f64 / width as f64,
            height: (y1 - y0) as f64 / height as f64,
        }
    } else if exact_full_image_histogram {
        OppoPortraitFocusRegion {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    } else if matches!(
        branch,
        OppoPortraitFocusBranch::TappedFace
            | OppoPortraitFocusBranch::PortraitFace
            | OppoPortraitFocusBranch::PetFace
    ) {
        focused_face
            .and_then(|face| normalized_ltwh(face.rectangle))
            .or_else(|| {
                config
                    .focus_rectangle
                    .filter(|_| config.focus_rectangle_is_valid)
                    .and_then(normalized_ltwh)
            })
            .unwrap_or(focus)
    } else {
        config
            .focus_rectangle
            .filter(|_| config.focus_rectangle_is_valid)
            .and_then(normalized_ltwh)
            .unwrap_or(focus)
    };
    source_roi = source_roi.normalized();

    let min_x = ((source_roi.x * width as f64).floor() as usize).min(width - 1);
    let min_y = ((source_roi.y * height as f64).floor() as usize).min(height - 1);
    let max_x = (((source_roi.x + source_roi.width) * width as f64).ceil() as usize)
        .saturating_sub(1)
        .clamp(min_x, width - 1);
    let max_y = (((source_roi.y + source_roi.height) * height as f64).ceil() as usize)
        .saturating_sub(1)
        .clamp(min_y, height - 1);

    let mask = if exact_full_image_histogram {
        None
    } else {
        match branch {
            OppoPortraitFocusBranch::TappedFace
            | OppoPortraitFocusBranch::PortraitFace
            | OppoPortraitFocusBranch::PortraitWithoutFace => depth.portrait.as_deref(),
            OppoPortraitFocusBranch::PetFace | OppoPortraitFocusBranch::PetRegion => {
                depth.pet.as_deref()
            }
            _ => None,
        }
    };

    let mut candidates = Vec::new();
    let mut rejected = 0_usize;
    let sample_step = if branch == OppoPortraitFocusBranch::CenterRegion {
        1
    } else {
        2
    };
    for y in (min_y..=max_y).step_by(sample_step) {
        for x in (min_x..=max_x).step_by(sample_step) {
            let index = y * width + x;
            if mask.is_some_and(|plane| plane.len() == depth.ranks.len() && plane[index] < 127) {
                rejected += 1;
                continue;
            }
            candidates.push(depth.ranks[index]);
        }
    }

    if candidates.len() < 9 {
        rejected += candidates.len();
        candidates.clear();
        let radius = 3_usize.max(width.min(height) / 64);
        let y0 = focus_depth_y.saturating_sub(radius);
        let y1 = (focus_depth_y + radius).min(height - 1);
        let x0 = focus_depth_x.saturating_sub(radius);
        let x1 = (focus_depth_x + radius).min(width - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                candidates.push(depth.ranks[y * width + x]);
            }
        }
    }
    if candidates.is_empty() {
        return Err(OppoPortraitFocusError::NoFocusCandidates);
    }
    candidates.sort_unstable();

    let selected_rank = match branch {
        OppoPortraitFocusBranch::CenterRegion => {
            let mut depth_sum = 0.0;
            let mut count = 0_usize;
            let y0 = focus_depth_y.saturating_sub(2);
            let y1 = (focus_depth_y + 2).min(height - 1);
            let x0 = focus_depth_x.saturating_sub(2);
            let x1 = (focus_depth_x + 2).min(width - 1);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if let Some(value) = depth
                        .header
                        .native_float_depth(f64::from(depth.ranks[y * width + x]))
                    {
                        depth_sum += value;
                        count += 1;
                    }
                }
            }
            if count > 0 {
                depth
                    .header
                    .rank_for_native_float_depth(depth_sum / count as f64)
                    .unwrap_or_else(|| f64::from(candidates[candidates.len() / 2]))
            } else {
                f64::from(candidates[candidates.len() / 2])
            }
        }
        OppoPortraitFocusBranch::NearObject | OppoPortraitFocusBranch::DisparityHistogram => {
            oppo_portrait_focus_histogram(&candidates, &depth.header, 0.05)
        }
        OppoPortraitFocusBranch::PetRegion if exact_pet_histogram_fallback => {
            oppo_portrait_focus_histogram(&candidates, &depth.header, 0.02)
        }
        OppoPortraitFocusBranch::PortraitFace
        | OppoPortraitFocusBranch::PortraitWithoutFace
        | OppoPortraitFocusBranch::TappedFace
        | OppoPortraitFocusBranch::PetFace
        | OppoPortraitFocusBranch::PetRegion => {
            oppo_portrait_focus_histogram(&candidates, &depth.header, 0.20)
        }
        OppoPortraitFocusBranch::TappedRegion => {
            let lower = candidates.len() / 10;
            let upper = (candidates.len() - lower).max(lower + 1);
            f64::from(candidates[lower + (upper - lower) / 2])
        }
    };

    let mut deviations = candidates
        .iter()
        .map(|value| (f64::from(*value) - selected_rank).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(f64::total_cmp);
    let mad = deviations[deviations.len() / 2];
    let acceptance = candidates.len() as f64 / (candidates.len() + rejected).max(1) as f64;
    let confidence = (acceptance * (1.0 - mad / 128.0)).clamp(0.0, 1.0);
    let producer_exact = branch == OppoPortraitFocusBranch::CenterRegion || exact_full_image_histogram;

    Ok(OppoPortraitFocusSelection {
        branch,
        source_roi,
        selected_rank,
        internal_disparity: internal_disparity(&depth.header, selected_rank),
        config_distance: config.object_distance.map(f64::from),
        confidence,
        sample_count: candidates.len(),
        rejected_sample_count: rejected,
        roi_is_producer_exact: producer_exact,
        statistic_is_producer_exact: producer_exact,
    })
}

fn internal_disparity(header: &OppoPortraitDepthHeader, rank: f64) -> Option<f64> {
    if !rank.is_finite() || !(1..=2).contains(&header.disparity_exponentiation) {
        return None;
    }
    let minimum = f64::from(header.disparity_minimum);
    let maximum = f64::from(header.disparity_maximum);
    if maximum <= minimum {
        return None;
    }
    let normalized = (rank / 255.0)
        .clamp(0.0, 1.0)
        .powi(i32::from(header.disparity_exponentiation));
    Some(65_535.0 - (minimum + normalized * (maximum - minimum)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OppoPortraitDepth, OppoPortraitFace};

    fn header(exponentiation: u8) -> OppoPortraitDepthHeader {
        OppoPortraitDepthHeader {
            width: 16,
            height: 16,
            rank_disparity_scale: 0.003_450_42,
            focal_length_pixels: 4098.0234,
            stereo_baseline: 38.844524,
            hair_plane_present: false,
            portrait_plane_present: true,
            pet_plane_present: false,
            near_object_detected: false,
            near_object_confidence: None,
            plant_object_state: 0,
            disparity_minimum: 11_560,
            disparity_maximum: 38_858,
            disparity_exponentiation: exponentiation,
            auxiliary_width: None,
            auxiliary_height: None,
            model_output_present: false,
            scene_class: 3,
            object_distance: Some(102),
            aec_lux_index: Some(324.54495),
            app_zoom_ratio: Some(6.0),
        }
    }

    #[test]
    fn focus_dispatch_matches_recovered_swift_regression_vectors() {
        let vectors = [
            (false, 3, 1, true, true, false, OppoPortraitFocusBranch::TappedFace),
            (false, 3, 1, false, true, false, OppoPortraitFocusBranch::PortraitFace),
            (
                false,
                3,
                1,
                false,
                false,
                false,
                OppoPortraitFocusBranch::PortraitWithoutFace,
            ),
            (false, 3, 2, false, true, false, OppoPortraitFocusBranch::CenterRegion),
            (false, 3, 3, false, true, true, OppoPortraitFocusBranch::PetRegion),
            (
                false,
                3,
                3,
                false,
                true,
                false,
                OppoPortraitFocusBranch::PortraitWithoutFace,
            ),
            (
                false,
                3,
                0,
                false,
                false,
                false,
                OppoPortraitFocusBranch::DisparityHistogram,
            ),
            (true, 0, 1, true, true, false, OppoPortraitFocusBranch::NearObject),
            (true, 2, 3, false, true, true, OppoPortraitFocusBranch::PetRegion),
            (true, 2, 2, false, true, false, OppoPortraitFocusBranch::CenterRegion),
            (true, 1, 3, false, true, true, OppoPortraitFocusBranch::CenterRegion),
        ];
        for (near, scene, roi, face, portrait, pet, expected) in vectors {
            assert_eq!(
                oppo_portrait_focus_branch(near, scene, roi, face, portrait, pet),
                expected
            );
        }
    }

    #[test]
    fn pet_no_rectangle_histogram_keeps_recovered_two_percent_behavior() {
        let values = [vec![0_u8; 3], vec![255_u8; 97]].concat();
        let header = header(1);
        let pet_rank = oppo_portrait_focus_histogram(&values, &header, 0.02);
        let portrait_rank = oppo_portrait_focus_histogram(&values, &header, 0.20);
        assert!(pet_rank < 1.0);
        assert!(portrait_rank > pet_rank + 1.0);
    }

    #[test]
    fn center_focus_averages_native_depth_not_rank() {
        let mut depth = OppoPortraitDepth {
            header: header(1),
            ranks: vec![200; 16 * 16],
            hair: None,
            portrait: Some(vec![255; 16 * 16]),
            pet: None,
        };
        depth.ranks[8 * 16 + 8] = 10;
        let config = config_with_focus(8, 8, 2);
        let selection = select_oppo_portrait_focus(
            &depth,
            &config,
            16,
            16,
            OppoPortraitFocusRegion {
                x: 0.5,
                y: 0.5,
                width: 0.12,
                height: 0.12,
            },
        )
        .unwrap();
        assert_eq!(selection.branch, OppoPortraitFocusBranch::CenterRegion);
        assert!(selection.roi_is_producer_exact);
        assert!(selection.statistic_is_producer_exact);
        assert!(selection.selected_rank.is_finite());
    }

    #[test]
    fn blur_response_uses_source_curve_and_zoom_region() {
        let mut config = config_with_focus(8, 8, 2);
        config.blur_apertures[0] = 1.4;
        config.blur_values[0] = 12.0;
        config.current_f_number = Some(2.8);
        config.current_blur_strength = 37;
        config.foreground_blur_scale = Some(120);
        let response = oppo_portrait_blur_response(&config, &header(1)).unwrap();
        assert_eq!(response.selected_aperture, f64::from(2.8_f32));
        assert_eq!(response.selected_blur_value, 37.0);
        assert_eq!(response.foreground_blur_scale, 120.0);
        assert_eq!(response.zoom_region, OppoPortraitZoomRegion::SixToTen);
        assert_eq!(response.samples[0].blur_value, 12.0);
    }

    fn config_with_focus(focus_x: i32, focus_y: i32, focus_roi_type: i32) -> OppoPortraitConfig {
        OppoPortraitConfig {
            version: 4.0,
            processing_width: 16,
            processing_height: 16,
            focus_x,
            focus_y,
            blur_apertures: [0.0; 32],
            blur_values: [0.0; 32],
            current_blur_strength: 0,
            camera_roll: 0,
            spotlight_width: None,
            spotlight_height: None,
            current_f_number: None,
            object_distance: Some(102),
            tele_master: None,
            focus_rectangle: None,
            focus_rectangle_is_valid: false,
            mirror_enabled: None,
            refocus_mode: None,
            foreground_blur_scale: None,
            big_face_enabled: None,
            pets_enabled: None,
            multi_semantic_segmentation_enabled: None,
            bokeh_version: None,
            iso: None,
            zoom_ratio: Some(100),
            focus_roi_type: Some(focus_roi_type),
            shutter: None,
            aec_lux_index: None,
            faces: Vec::<OppoPortraitFace>::new(),
        }
    }
}
