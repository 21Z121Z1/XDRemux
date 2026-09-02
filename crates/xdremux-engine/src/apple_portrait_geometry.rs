use std::error::Error;
use std::fmt;

use half::f16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplePortraitLensProfileId {
    Main1x,
    Fusion2x,
    Tele3x,
    Tetraprism5x,
}

impl ApplePortraitLensProfileId {
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Main1x => "Apple-1x-main-24mm",
            Self::Fusion2x => "Apple-2x-fusion-48mm",
            Self::Tele3x => "Apple-3x-tele-77mm",
            Self::Tetraprism5x => "Apple-5x-tetraprism-120mm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApplePortraitLensProfile {
    pub id: ApplePortraitLensProfileId,
    pub anchor_equivalent_focal_length_mm: f64,
    pub maximum_validated_equivalent_focal_length_mm: f64,
    pub reference_width: u32,
    pub reference_height: u32,
    pub focal_length_pixels: f64,
    pub principal_point_x: f64,
    pub principal_point_y: f64,
    pub distortion_center_x: f64,
    pub distortion_center_y: f64,
    pub pixel_size_mm: f64,
    pub distortion_coefficients: [f64; 8],
    pub inverse_distortion_coefficients: [f64; 8],
}

impl ApplePortraitLensProfile {
    pub const fn identifier(self) -> &'static str {
        self.id.identifier()
    }
}

pub const APPLE_PORTRAIT_MAIN_1X_PROFILE: ApplePortraitLensProfile = ApplePortraitLensProfile {
    id: ApplePortraitLensProfileId::Main1x,
    anchor_equivalent_focal_length_mm: 24.0,
    maximum_validated_equivalent_focal_length_mm: 44.0,
    reference_width: 4032,
    reference_height: 3024,
    focal_length_pixels: 2860.378_906_25,
    principal_point_x: 2010.311_035_156_25,
    principal_point_y: 1525.014_038_085_937_5,
    distortion_center_x: 2017.552_734_375,
    distortion_center_y: 1523.492_919_921_875,
    pixel_size_mm: 0.002_440,
    distortion_coefficients: [
        0.0,
        -0.555_219_471_454_620_4,
        0.053_949_449_211_359_024,
        -0.001_890_133_484_266_698_4,
        -0.000_004_621_016_614_692_053,
        0.000_001_959_401_970_452_745_4,
        -0.000_000_045_183_909_946_899_8,
        0.000_000_000_314_308_579_163_480_32,
    ],
    inverse_distortion_coefficients: [
        0.0,
        0.544_874_846_935_272_2,
        -0.050_807_286_053_895_95,
        0.001_680_599_059_909_582_1,
        0.000_007_370_583_261_945_285,
        -0.000_001_793_332_558_008_842_2,
        0.000_000_039_592_695_344_481_39,
        -0.000_000_000_268_914_474_021_997_3,
    ],
};

pub const APPLE_PORTRAIT_FUSION_2X_PROFILE: ApplePortraitLensProfile = ApplePortraitLensProfile {
    id: ApplePortraitLensProfileId::Fusion2x,
    anchor_equivalent_focal_length_mm: 48.0,
    maximum_validated_equivalent_focal_length_mm: 59.0,
    reference_width: 4032,
    reference_height: 3024,
    focal_length_pixels: 5666.130_371_093_75,
    principal_point_x: 2001.774_414_062_5,
    principal_point_y: 1543.746_093_75,
    distortion_center_x: 2008.567_138_671_875,
    distortion_center_y: 1553.952_880_859_375,
    pixel_size_mm: 0.001_219_999_976_456_165_3,
    distortion_coefficients: [
        0.0,
        -0.569_230_556_488_037_1,
        0.053_089_812_397_956_85,
        -0.001_865_589_176_304_638_4,
        -0.000_004_458_999_683_265_574,
        0.000_001_950_455_043_697_729_7,
        -0.000_000_044_818_150_968_239_934,
        0.000_000_000_305_347_469_531_369_6,
    ],
    inverse_distortion_coefficients: [
        0.0,
        0.557_631_433_010_101_3,
        -0.049_865_160_137_414_93,
        0.001_656_634_500_250_220_3,
        0.000_007_109_898_888_302_268_5,
        -0.000_001_782_454_432_941_449_3,
        0.000_000_039_320_074_307_624_964,
        -0.000_000_000_263_188_970_617_278_53,
    ],
};

pub const APPLE_PORTRAIT_TELE_3X_PROFILE: ApplePortraitLensProfile = ApplePortraitLensProfile {
    id: ApplePortraitLensProfileId::Tele3x,
    anchor_equivalent_focal_length_mm: 77.0,
    maximum_validated_equivalent_focal_length_mm: 134.0,
    reference_width: 4032,
    reference_height: 3024,
    focal_length_pixels: 9169.129_882_812_5,
    principal_point_x: 2023.225_585_937_5,
    principal_point_y: 1536.472_656_25,
    distortion_center_x: 2066.858_398_437_5,
    distortion_center_y: 1557.304_565_429_687_5,
    pixel_size_mm: 0.001_000_000_047_497_451_3,
    distortion_coefficients: [
        0.0,
        1.326_359_272_003_173_8,
        -0.799_688_637_256_622_3,
        0.186_875_805_258_750_92,
        -0.016_688_073_053_956_032,
        -0.001_481_974_148_191_511_6,
        0.000_467_687_001_219_019_3,
        -0.000_029_682_618_333_026_767,
    ],
    inverse_distortion_coefficients: [
        0.0,
        -1.303_797_483_444_213_9,
        0.781_151_235_103_607_2,
        -0.177_246_913_313_865_66,
        0.013_979_822_397_232_056,
        0.001_744_827_604_852_616_8,
        -0.000_452_920_416_137_203_6,
        0.000_026_913_012_334_262_02,
    ],
};

pub const APPLE_PORTRAIT_TETRAPRISM_5X_PROFILE: ApplePortraitLensProfile =
    ApplePortraitLensProfile {
        id: ApplePortraitLensProfileId::Tetraprism5x,
        anchor_equivalent_focal_length_mm: 120.0,
        maximum_validated_equivalent_focal_length_mm: 120.0,
        reference_width: 4032,
        reference_height: 3024,
        focal_length_pixels: 14_235.533_203_125,
        principal_point_x: 2012.309_082_031_25,
        principal_point_y: 1589.007_568_359_375,
        distortion_center_x: 2027.138_183_593_75,
        distortion_center_y: 1567.147_583_007_812_5,
        pixel_size_mm: 0.001_120_000_029_914_081,
        distortion_coefficients: [
            0.0,
            -0.098_828_054_964_542_39,
            0.000_012_278_825_124_667_492,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        inverse_distortion_coefficients: [
            0.0,
            0.102_295_719_087_123_87,
            -0.000_544_977_548_997_849_2,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
    };

#[derive(Debug, Clone, PartialEq)]
pub struct ApplePortraitCaptureFacts {
    pub physical_focal_length_mm: f64,
    pub equivalent_focal_length_mm: f64,
    pub digital_zoom_ratio: Option<f64>,
    pub lens_model: Option<String>,
    pub base_width: u32,
    pub base_height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplePortraitCameraCalibration {
    pub profile: ApplePortraitLensProfileId,
    pub profile_anchor_equivalent_focal_length_mm: f64,
    pub profile_maximum_validated_equivalent_focal_length_mm: f64,
    pub physical_focal_length_mm: f64,
    pub optical_equivalent_focal_length_mm: f64,
    pub source_equivalent_focal_length_mm: f64,
    pub render_equivalent_focal_length_mm: f64,
    pub digital_zoom_ratio: f64,
    pub reference_width: u32,
    pub reference_height: u32,
    pub focal_length_pixels: f64,
    pub effective_focal_length_pixels: f64,
    pub principal_point_x: f64,
    pub principal_point_y: f64,
    pub distortion_center_x: f64,
    pub distortion_center_y: f64,
    pub pixel_size_mm: f64,
    pub distortion_coefficients: [f64; 8],
    pub inverse_distortion_coefficients: [f64; 8],
}

impl ApplePortraitCameraCalibration {
    pub const fn profile_identifier(&self) -> &'static str {
        self.profile.identifier()
    }

    pub fn intrinsic_matrix(&self) -> [f64; 9] {
        [
            self.focal_length_pixels,
            0.0,
            0.0,
            0.0,
            self.focal_length_pixels,
            0.0,
            self.principal_point_x,
            self.principal_point_y,
            1.0,
        ]
    }

    pub fn profile_saturated(&self) -> bool {
        self.source_equivalent_focal_length_mm
            > self.profile_maximum_validated_equivalent_focal_length_mm + 0.001
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplePortraitImageGeometry {
    pub width: u32,
    pub height: u32,
    pub orientation: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplePortraitDisparity {
    pub width: u32,
    pub height: u32,
    pub far: f32,
    pub near: f32,
    pub pixels_le_f16: Vec<u8>,
}

impl ApplePortraitDisparity {
    pub const fn bytes_per_row(&self) -> u32 {
        self.width * 2
    }

    pub fn focus_disparity(
        &self,
        rank: f64,
        exponentiation: u8,
    ) -> Result<f64, ApplePortraitGeometryError> {
        focus_disparity(rank, f64::from(self.near - self.far), exponentiation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplePortraitGeometryError {
    InvalidCaptureFact(&'static str),
    InvalidGeometry,
    InvalidDisparityScale,
    UnsupportedDisparityExponentiation(u8),
    RankPlaneSizeMismatch { expected: usize, actual: usize },
}

impl fmt::Display for ApplePortraitGeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCaptureFact(name) => {
                write!(formatter, "Apple Portrait capture fact {name} is invalid")
            }
            Self::InvalidGeometry => {
                formatter.write_str("Apple Portrait image geometry is invalid")
            }
            Self::InvalidDisparityScale => {
                formatter.write_str("Apple Portrait disparity scale must be finite and positive")
            }
            Self::UnsupportedDisparityExponentiation(value) => write!(
                formatter,
                "Apple Portrait disparity exponentiation {value} is unsupported"
            ),
            Self::RankPlaneSizeMismatch { expected, actual } => write!(
                formatter,
                "Apple Portrait rank plane has {actual} bytes; expected {expected}"
            ),
        }
    }
}

impl Error for ApplePortraitGeometryError {}

pub const fn apple_portrait_lens_profile(
    physical_focal_length_mm: f64,
    equivalent_focal_length_mm: f64,
) -> ApplePortraitLensProfile {
    if physical_focal_length_mm <= 11.0 {
        if equivalent_focal_length_mm < 45.0 {
            APPLE_PORTRAIT_MAIN_1X_PROFILE
        } else {
            APPLE_PORTRAIT_FUSION_2X_PROFILE
        }
    } else if physical_focal_length_mm < 28.0 {
        APPLE_PORTRAIT_TELE_3X_PROFILE
    } else {
        APPLE_PORTRAIT_TETRAPRISM_5X_PROFILE
    }
}

/// Recover the optical equivalent-focal-length anchor encoded in OPPO LensModel.
///
/// This mirrors the established Swift pattern `camera\\s+<number>mm\\b` without
/// introducing a regex dependency into the product core.
pub fn optical_equivalent_focal_length_from_lens_model(lens_model: &str) -> Option<f64> {
    let mut words = lens_model.split_ascii_whitespace();
    while let Some(word) = words.next() {
        if !word.eq_ignore_ascii_case("camera") {
            continue;
        }
        let candidate = words.next()?;
        let bytes = candidate.as_bytes();
        let mut end = 0usize;
        let mut decimal_seen = false;
        while let Some(byte) = bytes.get(end).copied() {
            if byte.is_ascii_digit() {
                end += 1;
            } else if byte == b'.' && !decimal_seen {
                decimal_seen = true;
                end += 1;
            } else {
                break;
            }
        }
        if end == 0 || bytes.get(end..end + 2) != Some(b"mm") {
            return None;
        }
        if bytes
            .get(end + 2)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            return None;
        }
        let value = candidate.get(..end)?.parse::<f64>().ok()?;
        return (value.is_finite() && value > 0.0).then_some(value);
    }
    None
}

pub fn derive_apple_portrait_camera_calibration(
    facts: &ApplePortraitCaptureFacts,
) -> Result<ApplePortraitCameraCalibration, ApplePortraitGeometryError> {
    validate_positive_finite(facts.physical_focal_length_mm, "physical_focal_length_mm")?;
    validate_positive_finite(
        facts.equivalent_focal_length_mm,
        "equivalent_focal_length_mm",
    )?;
    if facts.base_width == 0 || facts.base_height == 0 {
        return Err(ApplePortraitGeometryError::InvalidGeometry);
    }

    let exif_zoom = facts
        .digital_zoom_ratio
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0);
    let lens_anchor = facts
        .lens_model
        .as_deref()
        .and_then(optical_equivalent_focal_length_from_lens_model);
    let fallback_anchor = facts.equivalent_focal_length_mm / exif_zoom.max(1.0);
    let optical_equivalent_focal_length_mm = lens_anchor.unwrap_or(fallback_anchor);
    validate_positive_finite(
        optical_equivalent_focal_length_mm,
        "optical_equivalent_focal_length_mm",
    )?;

    let equivalent_zoom = facts.equivalent_focal_length_mm / optical_equivalent_focal_length_mm;
    let digital_zoom_ratio = if exif_zoom >= 1.0
        && (exif_zoom - equivalent_zoom).abs() <= 0.08_f64.max(equivalent_zoom * 0.05)
    {
        exif_zoom
    } else {
        equivalent_zoom.max(1.0)
    };

    let profile = apple_portrait_lens_profile(
        facts.physical_focal_length_mm,
        facts.equivalent_focal_length_mm,
    );
    let render_equivalent_focal_length_mm = facts
        .equivalent_focal_length_mm
        .min(profile.maximum_validated_equivalent_focal_length_mm);
    let crop_scale = profile.anchor_equivalent_focal_length_mm / render_equivalent_focal_length_mm;
    let reference_width =
        rounded_multiple_of_four(f64::from(profile.reference_width) * crop_scale)?;
    let reference_height =
        rounded_multiple_of_four(f64::from(profile.reference_height) * crop_scale)?;
    let crop_offset_x = (f64::from(profile.reference_width) - f64::from(reference_width)) / 2.0;
    let crop_offset_y = (f64::from(profile.reference_height) - f64::from(reference_height)) / 2.0;

    Ok(ApplePortraitCameraCalibration {
        profile: profile.id,
        profile_anchor_equivalent_focal_length_mm: profile.anchor_equivalent_focal_length_mm,
        profile_maximum_validated_equivalent_focal_length_mm: profile
            .maximum_validated_equivalent_focal_length_mm,
        physical_focal_length_mm: facts.physical_focal_length_mm,
        optical_equivalent_focal_length_mm,
        source_equivalent_focal_length_mm: facts.equivalent_focal_length_mm,
        render_equivalent_focal_length_mm,
        digital_zoom_ratio,
        reference_width,
        reference_height,
        focal_length_pixels: profile.focal_length_pixels,
        effective_focal_length_pixels: profile.focal_length_pixels * f64::from(facts.base_width)
            / f64::from(reference_width),
        principal_point_x: profile.principal_point_x - crop_offset_x,
        principal_point_y: profile.principal_point_y - crop_offset_y,
        distortion_center_x: profile.distortion_center_x - crop_offset_x,
        distortion_center_y: profile.distortion_center_y - crop_offset_y,
        pixel_size_mm: profile.pixel_size_mm * crop_scale,
        distortion_coefficients: profile.distortion_coefficients,
        inverse_distortion_coefficients: profile.inverse_distortion_coefficients,
    })
}

pub fn resolve_apple_portrait_base_orientation(
    input: Option<ApplePortraitImageGeometry>,
    base: ApplePortraitImageGeometry,
) -> Result<u8, ApplePortraitGeometryError> {
    if base.width == 0 || base.height == 0 {
        return Err(ApplePortraitGeometryError::InvalidGeometry);
    }
    let normalized_input_orientation = input
        .and_then(|geometry| valid_orientation(geometry.orientation))
        .unwrap_or(1);
    let target_is_portrait = match input {
        Some(geometry)
            if geometry.width > 0 && geometry.height > 0 && geometry.width != geometry.height =>
        {
            displayed_is_portrait(
                geometry.width,
                geometry.height,
                normalized_input_orientation,
            )
        }
        _ => displayed_is_portrait(
            base.width,
            base.height,
            valid_orientation(base.orientation).unwrap_or(normalized_input_orientation),
        ),
    };

    if let Some(base_orientation) = valid_orientation(base.orientation) {
        if displayed_is_portrait(base.width, base.height, base_orientation) == target_is_portrait {
            return Ok(base_orientation);
        }
    }

    if (base.height > base.width) == target_is_portrait {
        Ok(1)
    } else {
        Ok(6)
    }
}

pub fn build_apple_portrait_disparity(
    ranks: &[u8],
    width: u32,
    height: u32,
    rank_disparity_scale: f32,
    exponentiation: u8,
) -> Result<ApplePortraitDisparity, ApplePortraitGeometryError> {
    if width == 0 || height == 0 {
        return Err(ApplePortraitGeometryError::InvalidGeometry);
    }
    if !rank_disparity_scale.is_finite() || rank_disparity_scale <= 0.0 {
        return Err(ApplePortraitGeometryError::InvalidDisparityScale);
    }
    if !(1..=2).contains(&exponentiation) {
        return Err(ApplePortraitGeometryError::UnsupportedDisparityExponentiation(exponentiation));
    }
    let expected = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(ApplePortraitGeometryError::InvalidGeometry)?;
    if ranks.len() != expected {
        return Err(ApplePortraitGeometryError::RankPlaneSizeMismatch {
            expected,
            actual: ranks.len(),
        });
    }

    let far = 0.0_f32;
    let span = 255.0_f32 * rank_disparity_scale;
    let near = far + span;
    let mut pixels_le_f16 = Vec::with_capacity(expected.saturating_mul(2));
    for rank in ranks {
        let normalized_rank = (f32::from(*rank) / 255.0).powf(f32::from(exponentiation));
        let value = near - normalized_rank * span;
        pixels_le_f16.extend_from_slice(&f16::from_f32(value).to_le_bytes());
    }

    Ok(ApplePortraitDisparity {
        width,
        height,
        far,
        near,
        pixels_le_f16,
    })
}

pub fn focus_disparity(
    rank: f64,
    disparity_span: f64,
    exponentiation: u8,
) -> Result<f64, ApplePortraitGeometryError> {
    if !rank.is_finite() || !disparity_span.is_finite() || disparity_span < 0.0 {
        return Err(ApplePortraitGeometryError::InvalidDisparityScale);
    }
    if !(1..=2).contains(&exponentiation) {
        return Err(ApplePortraitGeometryError::UnsupportedDisparityExponentiation(exponentiation));
    }
    let normalized = (rank / 255.0)
        .clamp(0.0, 1.0)
        .powf(f64::from(exponentiation));
    Ok((1.0 - normalized) * disparity_span)
}

fn validate_positive_finite(
    value: f64,
    name: &'static str,
) -> Result<(), ApplePortraitGeometryError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(ApplePortraitGeometryError::InvalidCaptureFact(name))
    }
}

fn rounded_multiple_of_four(value: f64) -> Result<u32, ApplePortraitGeometryError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(ApplePortraitGeometryError::InvalidGeometry);
    }
    let rounded = ((value / 4.0).round() * 4.0).max(4.0);
    if rounded > f64::from(u32::MAX) {
        return Err(ApplePortraitGeometryError::InvalidGeometry);
    }
    Ok(rounded as u32)
}

const fn valid_orientation(orientation: Option<u8>) -> Option<u8> {
    match orientation {
        Some(value @ 1..=8) => Some(value),
        _ => None,
    }
}

const fn orientation_swaps_axes(orientation: u8) -> bool {
    matches!(orientation, 5..=8)
}

const fn displayed_is_portrait(width: u32, height: u32, orientation: u8) -> bool {
    if orientation_swaps_axes(orientation) {
        width > height
    } else {
        height > width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lens_model_parser_matches_swift_camera_number_contract() {
        assert_eq!(
            optical_equivalent_focal_length_from_lens_model(
                "OPPO Find X9 Ultra back camera 24mm f/1.8"
            ),
            Some(24.0)
        );
        assert_eq!(
            optical_equivalent_focal_length_from_lens_model("tele CAMERA\t77.5mm profile"),
            Some(77.5)
        );
        assert_eq!(
            optical_equivalent_focal_length_from_lens_model("camera 24mmx"),
            None
        );
    }

    #[test]
    fn lens_profile_dispatch_matches_swift_thresholds() {
        assert_eq!(
            apple_portrait_lens_profile(8.0, 24.0).id,
            ApplePortraitLensProfileId::Main1x
        );
        assert_eq!(
            apple_portrait_lens_profile(8.0, 48.0).id,
            ApplePortraitLensProfileId::Fusion2x
        );
        assert_eq!(
            apple_portrait_lens_profile(18.0, 77.0).id,
            ApplePortraitLensProfileId::Tele3x
        );
        assert_eq!(
            apple_portrait_lens_profile(30.0, 120.0).id,
            ApplePortraitLensProfileId::Tetraprism5x
        );
    }

    #[test]
    fn calibration_uses_lens_anchor_and_repairs_inconsistent_exif_zoom() {
        let calibration = derive_apple_portrait_camera_calibration(&ApplePortraitCaptureFacts {
            physical_focal_length_mm: 8.0,
            equivalent_focal_length_mm: 48.0,
            digital_zoom_ratio: Some(1.0),
            lens_model: Some("OPPO camera 24mm".to_owned()),
            base_width: 4032,
            base_height: 3024,
        })
        .unwrap();
        assert_eq!(calibration.profile, ApplePortraitLensProfileId::Fusion2x);
        assert_eq!(calibration.digital_zoom_ratio, 2.0);
        assert_eq!(calibration.reference_width, 4032);
        assert_eq!(calibration.reference_height, 3024);
        assert_eq!(calibration.focal_length_pixels, 5666.130_371_093_75);
        assert!(!calibration.profile_saturated());
    }

    #[test]
    fn calibration_saturates_tele_profile_without_fabricating_new_profile() {
        let calibration = derive_apple_portrait_camera_calibration(&ApplePortraitCaptureFacts {
            physical_focal_length_mm: 18.0,
            equivalent_focal_length_mm: 230.0,
            digital_zoom_ratio: Some(3.0),
            lens_model: Some("OPPO camera 77mm".to_owned()),
            base_width: 4096,
            base_height: 3072,
        })
        .unwrap();
        assert_eq!(calibration.profile, ApplePortraitLensProfileId::Tele3x);
        assert_eq!(calibration.render_equivalent_focal_length_mm, 134.0);
        assert!(calibration.profile_saturated());
        assert!(calibration.reference_width < APPLE_PORTRAIT_TELE_3X_PROFILE.reference_width);
        assert!(calibration.pixel_size_mm < APPLE_PORTRAIT_TELE_3X_PROFILE.pixel_size_mm);
    }

    #[test]
    fn orientation_resolution_matches_swift_fallback_policy() {
        assert_eq!(
            resolve_apple_portrait_base_orientation(
                Some(ApplePortraitImageGeometry {
                    width: 4032,
                    height: 3024,
                    orientation: Some(6),
                }),
                ApplePortraitImageGeometry {
                    width: 4032,
                    height: 3024,
                    orientation: Some(6),
                },
            )
            .unwrap(),
            6
        );
        assert_eq!(
            resolve_apple_portrait_base_orientation(
                Some(ApplePortraitImageGeometry {
                    width: 3024,
                    height: 4032,
                    orientation: Some(1),
                }),
                ApplePortraitImageGeometry {
                    width: 4032,
                    height: 3024,
                    orientation: None,
                },
            )
            .unwrap(),
            6
        );
    }

    #[test]
    fn disparity_pixels_match_swift_endpoint_semantics() {
        let disparity = build_apple_portrait_disparity(&[0, 255], 2, 1, 0.01, 1).unwrap();
        assert_eq!(disparity.pixels_le_f16.len(), 4);
        let near = f16::from_le_bytes(disparity.pixels_le_f16[0..2].try_into().unwrap()).to_f32();
        let far = f16::from_le_bytes(disparity.pixels_le_f16[2..4].try_into().unwrap()).to_f32();
        assert_eq!(near, f16::from_f32(2.55).to_f32());
        assert_eq!(far, 0.0);
        assert_eq!(disparity.bytes_per_row(), 4);
    }

    #[test]
    fn focus_disparity_uses_source_exponent_without_extra_focal_scaling() {
        let span = 255.0 * 0.003_450_42;
        assert!((focus_disparity(0.0, span, 2).unwrap() - span).abs() < 1e-12);
        assert_eq!(focus_disparity(255.0, span, 2).unwrap(), 0.0);
        let linear_mid = focus_disparity(127.5, span, 1).unwrap();
        let squared_mid = focus_disparity(127.5, span, 2).unwrap();
        assert!(squared_mid > linear_mid);
    }
}
