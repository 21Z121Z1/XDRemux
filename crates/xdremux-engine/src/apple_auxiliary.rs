use std::error::Error;
use std::fmt;

use xdremux_format::FourCC;

use crate::{ApplePortraitCameraCalibration, ApplePortraitDisparity, AppleSemanticRole};

pub const APPLE_DEPTH_DATA_NAMESPACE: AppleMetadataNamespace = AppleMetadataNamespace {
    uri: "http://ns.apple.com/depthData/1.0/",
    prefix: "depthData",
};
pub const APPLE_DEPTH_BLUR_EFFECT_NAMESPACE: AppleMetadataNamespace = AppleMetadataNamespace {
    uri: "http://ns.apple.com/depthBlurEffect/1.0/",
    prefix: "depthBlurEffect",
};
pub const APPLE_PORTRAIT_LIGHTING_EFFECT_NAMESPACE: AppleMetadataNamespace =
    AppleMetadataNamespace {
        uri: "http://ns.apple.com/portraitLightingEffect/1.0/",
        prefix: "portraitLightingEffect",
    };
pub const APPLE_PORTRAIT_EFFECTS_MATTE_NAMESPACE: AppleMetadataNamespace = AppleMetadataNamespace {
    uri: "http://ns.apple.com/portraitEffectsMatte/1.0/",
    prefix: "portraitEffectsMatte",
};
pub const APPLE_SEMANTIC_SEGMENTATION_MATTE_NAMESPACE: AppleMetadataNamespace =
    AppleMetadataNamespace {
        uri: "http://ns.apple.com/semanticSegmentationMatte/1.0/",
        prefix: "semanticSegmentationMatte",
    };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleAuxiliaryKind {
    Disparity,
    PortraitEffectsMatte,
    SemanticSegmentation(AppleSemanticRole),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleAuxiliaryDescription {
    pub width: u32,
    pub height: u32,
    pub bytes_per_row: u32,
    pub pixel_format: FourCC,
    pub orientation: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleMetadataNamespace {
    pub uri: &'static str,
    pub prefix: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppleMetadataValue {
    Text(String),
    Numbers(Vec<f64>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppleMetadataTag {
    pub path: &'static str,
    pub value: AppleMetadataValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppleAuxiliaryPayload {
    pub kind: AppleAuxiliaryKind,
    pub data: Vec<u8>,
    pub description: AppleAuxiliaryDescription,
    pub namespaces: Vec<AppleMetadataNamespace>,
    pub metadata: Vec<AppleMetadataTag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleAuxiliaryError {
    InvalidGeometry,
    DataSizeMismatch { expected: usize, actual: usize },
    InvalidOrientation(u8),
    InvalidSimulatedAperture,
    MissingRenderingParameters,
    UnsupportedSemanticRole(AppleSemanticRole),
}

impl fmt::Display for AppleAuxiliaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeometry => formatter.write_str("Apple auxiliary geometry is invalid"),
            Self::DataSizeMismatch { expected, actual } => write!(
                formatter,
                "Apple auxiliary payload has {actual} bytes; expected {expected}"
            ),
            Self::InvalidOrientation(value) => {
                write!(formatter, "Apple auxiliary orientation {value} is invalid")
            }
            Self::InvalidSimulatedAperture => formatter
                .write_str("Apple Portrait simulated aperture must be finite and between 1 and 32"),
            Self::MissingRenderingParameters => {
                formatter.write_str("Apple Portrait rendering parameters are empty")
            }
            Self::UnsupportedSemanticRole(role) => write!(
                formatter,
                "Apple semantic role {role:?} is not an ImageIO semantic matte auxiliary type"
            ),
        }
    }
}

impl Error for AppleAuxiliaryError {}

pub fn build_apple_portrait_disparity_payload(
    disparity: ApplePortraitDisparity,
    orientation: u8,
    calibration: &ApplePortraitCameraCalibration,
    rendering_parameters: &[u8],
    simulated_aperture: f64,
) -> Result<AppleAuxiliaryPayload, AppleAuxiliaryError> {
    if !(1..=8).contains(&orientation) {
        return Err(AppleAuxiliaryError::InvalidOrientation(orientation));
    }
    if !simulated_aperture.is_finite() || !(1.0..=32.0).contains(&simulated_aperture) {
        return Err(AppleAuxiliaryError::InvalidSimulatedAperture);
    }
    if rendering_parameters.is_empty() {
        return Err(AppleAuxiliaryError::MissingRenderingParameters);
    }

    let bytes_per_row = disparity
        .width
        .checked_mul(2)
        .ok_or(AppleAuxiliaryError::InvalidGeometry)?;
    validate_data_size(
        disparity.width,
        disparity.height,
        2,
        disparity.pixels_le_f16.len(),
    )?;

    let metadata = vec![
        text("depthData:Quality", "high"),
        text("depthData:Accuracy", "relative"),
        text("depthData:Filtered", "True"),
        text("depthData:DepthDataVersion", "65541"),
        text(
            "depthData:IntrinsicMatrixReferenceWidth",
            calibration.reference_width.to_string(),
        ),
        text(
            "depthData:IntrinsicMatrixReferenceHeight",
            calibration.reference_height.to_string(),
        ),
        text(
            "depthData:LensDistortionCenterOffsetX",
            format!("{:.12}", calibration.distortion_center_x),
        ),
        text(
            "depthData:LensDistortionCenterOffsetY",
            format!("{:.12}", calibration.distortion_center_y),
        ),
        text(
            "depthData:PixelSize",
            format!("{:.12}", calibration.pixel_size_mm),
        ),
        numbers(
            "depthData:IntrinsicMatrix",
            calibration.intrinsic_matrix().to_vec(),
        ),
        numbers(
            "depthData:ExtrinsicMatrix",
            vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        ),
        numbers(
            "depthData:LensDistortionCoefficients",
            calibration.distortion_coefficients.to_vec(),
        ),
        numbers(
            "depthData:InverseLensDistortionCoefficients",
            calibration.inverse_distortion_coefficients.to_vec(),
        ),
        text(
            "depthBlurEffect:RenderingParameters",
            base64_standard(rendering_parameters),
        ),
        text(
            "depthBlurEffect:SimulatedAperture",
            format!("{simulated_aperture:.6}"),
        ),
        text("portraitLightingEffect:EffectStrength", "0.500000"),
    ];

    Ok(AppleAuxiliaryPayload {
        kind: AppleAuxiliaryKind::Disparity,
        data: disparity.pixels_le_f16,
        description: AppleAuxiliaryDescription {
            width: disparity.width,
            height: disparity.height,
            bytes_per_row,
            pixel_format: FourCC::new(*b"hdis"),
            orientation: Some(orientation),
        },
        namespaces: vec![
            APPLE_DEPTH_DATA_NAMESPACE,
            APPLE_DEPTH_BLUR_EFFECT_NAMESPACE,
            APPLE_PORTRAIT_LIGHTING_EFFECT_NAMESPACE,
        ],
        metadata,
    })
}

pub fn build_apple_portrait_effects_matte_payload(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
) -> Result<AppleAuxiliaryPayload, AppleAuxiliaryError> {
    build_l8_payload(
        AppleAuxiliaryKind::PortraitEffectsMatte,
        width,
        height,
        pixels,
        APPLE_PORTRAIT_EFFECTS_MATTE_NAMESPACE,
        "portraitEffectsMatte:PortraitEffectsMatteVersion",
        "65537",
    )
}

pub fn build_apple_semantic_matte_payload(
    role: AppleSemanticRole,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
) -> Result<AppleAuxiliaryPayload, AppleAuxiliaryError> {
    if matches!(role, AppleSemanticRole::Person) {
        return Err(AppleAuxiliaryError::UnsupportedSemanticRole(role));
    }
    build_l8_payload(
        AppleAuxiliaryKind::SemanticSegmentation(role),
        width,
        height,
        pixels,
        APPLE_SEMANTIC_SEGMENTATION_MATTE_NAMESPACE,
        "semanticSegmentationMatte:SemanticSegmentationMatteVersion",
        "65536",
    )
}

fn build_l8_payload(
    kind: AppleAuxiliaryKind,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    namespace: AppleMetadataNamespace,
    version_path: &'static str,
    version: &'static str,
) -> Result<AppleAuxiliaryPayload, AppleAuxiliaryError> {
    validate_data_size(width, height, 1, pixels.len())?;
    Ok(AppleAuxiliaryPayload {
        kind,
        data: pixels,
        description: AppleAuxiliaryDescription {
            width,
            height,
            bytes_per_row: width,
            pixel_format: FourCC::new(*b"L008"),
            orientation: None,
        },
        namespaces: vec![namespace],
        metadata: vec![text(version_path, version)],
    })
}

fn validate_data_size(
    width: u32,
    height: u32,
    bytes_per_pixel: usize,
    actual: usize,
) -> Result<(), AppleAuxiliaryError> {
    if width == 0 || height == 0 {
        return Err(AppleAuxiliaryError::InvalidGeometry);
    }
    let expected = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(bytes_per_pixel))
        .ok_or(AppleAuxiliaryError::InvalidGeometry)?;
    if expected != actual {
        return Err(AppleAuxiliaryError::DataSizeMismatch { expected, actual });
    }
    Ok(())
}

fn base64_standard(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut output = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);

        output.push(ALPHABET[usize::from(first >> 2)] as char);
        output.push(
            ALPHABET[usize::from(((first & 0x03) << 4) | (second >> 4))] as char,
        );
        if chunk.len() > 1 {
            output.push(
                ALPHABET[usize::from(((second & 0x0f) << 2) | (third >> 6))] as char,
            );
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[usize::from(third & 0x3f)] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn text(path: &'static str, value: impl Into<String>) -> AppleMetadataTag {
    AppleMetadataTag {
        path,
        value: AppleMetadataValue::Text(value.into()),
    }
}

fn numbers(path: &'static str, value: Vec<f64>) -> AppleMetadataTag {
    AppleMetadataTag {
        path,
        value: AppleMetadataValue::Numbers(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        derive_apple_portrait_camera_calibration, ApplePortraitCaptureFacts,
        ApplePortraitLensProfileId,
    };

    fn calibration() -> ApplePortraitCameraCalibration {
        derive_apple_portrait_camera_calibration(&ApplePortraitCaptureFacts {
            physical_focal_length_mm: 15.0,
            equivalent_focal_length_mm: 77.0,
            digital_zoom_ratio: Some(1.0),
            lens_model: Some("OPPO camera 77mm".to_owned()),
            base_width: 4032,
            base_height: 3024,
        })
        .expect("calibration")
    }

    fn metadata_text<'a>(payload: &'a AppleAuxiliaryPayload, path: &str) -> Option<&'a str> {
        payload.metadata.iter().find_map(|tag| {
            if tag.path != path {
                return None;
            }
            match &tag.value {
                AppleMetadataValue::Text(value) => Some(value.as_str()),
                AppleMetadataValue::Numbers(_) => None,
            }
        })
    }

    #[test]
    fn standard_base64_matches_rfc_4648_vectors() {
        for (input, expected) in [
            (b"".as_slice(), ""),
            (b"f".as_slice(), "Zg=="),
            (b"fo".as_slice(), "Zm8="),
            (b"foo".as_slice(), "Zm9v"),
            (b"foob".as_slice(), "Zm9vYg=="),
            (b"fooba".as_slice(), "Zm9vYmE="),
            (b"foobar".as_slice(), "Zm9vYmFy"),
        ] {
            assert_eq!(base64_standard(input), expected);
        }
    }

    #[test]
    fn disparity_payload_matches_the_legacy_imageio_contract() {
        let calibration = calibration();
        assert_eq!(calibration.profile, ApplePortraitLensProfileId::Tele3x);
        let disparity = ApplePortraitDisparity {
            width: 2,
            height: 2,
            far: 0.0,
            near: 1.0,
            pixels_le_f16: vec![0; 8],
        };
        let payload = build_apple_portrait_disparity_payload(
            disparity,
            6,
            &calibration,
            b"REND",
            2.8,
        )
        .expect("depth payload");

        assert_eq!(payload.kind, AppleAuxiliaryKind::Disparity);
        assert_eq!(payload.description.width, 2);
        assert_eq!(payload.description.height, 2);
        assert_eq!(payload.description.bytes_per_row, 4);
        assert_eq!(payload.description.pixel_format, FourCC::new(*b"hdis"));
        assert_eq!(payload.description.orientation, Some(6));
        assert_eq!(
            payload.namespaces,
            vec![
                APPLE_DEPTH_DATA_NAMESPACE,
                APPLE_DEPTH_BLUR_EFFECT_NAMESPACE,
                APPLE_PORTRAIT_LIGHTING_EFFECT_NAMESPACE,
            ]
        );
        assert_eq!(metadata_text(&payload, "depthData:Quality"), Some("high"));
        assert_eq!(
            metadata_text(&payload, "depthData:DepthDataVersion"),
            Some("65541")
        );
        assert_eq!(
            metadata_text(&payload, "depthBlurEffect:RenderingParameters"),
            Some("UkVORA==")
        );
        assert_eq!(
            metadata_text(&payload, "depthBlurEffect:SimulatedAperture"),
            Some("2.800000")
        );
        assert_eq!(
            metadata_text(&payload, "portraitLightingEffect:EffectStrength"),
            Some("0.500000")
        );
    }

    #[test]
    fn l8_matte_payloads_encode_only_rust_owned_role_policy() {
        let portrait = build_apple_portrait_effects_matte_payload(2, 2, vec![1, 2, 3, 4])
            .expect("portrait matte");
        assert_eq!(portrait.kind, AppleAuxiliaryKind::PortraitEffectsMatte);
        assert_eq!(portrait.description.bytes_per_row, 2);
        assert_eq!(portrait.description.pixel_format, FourCC::new(*b"L008"));
        assert_eq!(portrait.description.orientation, None);
        assert_eq!(
            metadata_text(
                &portrait,
                "portraitEffectsMatte:PortraitEffectsMatteVersion"
            ),
            Some("65537")
        );

        let hair =
            build_apple_semantic_matte_payload(AppleSemanticRole::Hair, 2, 2, vec![4, 3, 2, 1])
                .expect("hair matte");
        assert_eq!(
            hair.kind,
            AppleAuxiliaryKind::SemanticSegmentation(AppleSemanticRole::Hair)
        );
        assert_eq!(
            metadata_text(
                &hair,
                "semanticSegmentationMatte:SemanticSegmentationMatteVersion"
            ),
            Some("65536")
        );
        assert_eq!(
            build_apple_semantic_matte_payload(AppleSemanticRole::Person, 1, 1, vec![255]),
            Err(AppleAuxiliaryError::UnsupportedSemanticRole(
                AppleSemanticRole::Person
            ))
        );
    }

    #[test]
    fn payload_builders_reject_invalid_shape_and_product_values() {
        let calibration = calibration();
        let invalid_disparity = ApplePortraitDisparity {
            width: 2,
            height: 2,
            far: 0.0,
            near: 1.0,
            pixels_le_f16: vec![0; 6],
        };
        assert_eq!(
            build_apple_portrait_disparity_payload(
                invalid_disparity,
                1,
                &calibration,
                b"REND",
                2.8,
            ),
            Err(AppleAuxiliaryError::DataSizeMismatch {
                expected: 8,
                actual: 6,
            })
        );
        assert!(build_apple_portrait_effects_matte_payload(0, 2, Vec::new()).is_err());
    }
}
