use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

use xdremux_format::FourCC;

/// Apple consumer facts reported by a platform capability adapter.
///
/// These are observations, not platform policy. The adapter reports what
/// ImageIO exposes; the Rust engine decides which facts a product feature
/// requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppleImageAuxiliaryFacts {
    pub iso_gain_map: bool,
    pub disparity: bool,
    pub portrait_effects_matte: bool,
    pub skin_matte: bool,
    pub hair_matte: bool,
    pub teeth_matte: bool,
    pub glasses_matte: bool,
}

impl AppleImageAuxiliaryFacts {
    /// Resource contract required for Apple Photos portrait editing output.
    ///
    /// Keep this policy in Rust rather than teaching the platform adapter to
    /// answer a business-level "valid portrait" question.
    pub const fn satisfies_portrait_editing(self) -> bool {
        self.iso_gain_map
            && self.disparity
            && self.portrait_effects_matte
            && self.skin_matte
            && self.hair_matte
            && self.teeth_matte
            && self.glasses_matte
    }
}

/// Semantic image resources used by Apple photo features.
///
/// Product code deals in these roles. Framework class names, selectors, and
/// whether a role currently requires public API or SPI stay behind the Apple
/// adapter and are not part of Rust product policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppleSemanticRole {
    Person,
    Skin,
    Hair,
    Teeth,
    Glasses,
    Sky,
}

/// Semantic resources required by the current Apple Photos Portrait contract.
///
/// This is product policy. The adapter is given these roles explicitly rather
/// than choosing a feature profile itself.
pub const APPLE_PORTRAIT_SEMANTIC_ROLES: [AppleSemanticRole; 5] = [
    AppleSemanticRole::Person,
    AppleSemanticRole::Skin,
    AppleSemanticRole::Hair,
    AppleSemanticRole::Teeth,
    AppleSemanticRole::Glasses,
];

/// One tightly packed linear 8-bit matte.
///
/// Apple Vision exposes semantic mattes as one-component 8-bit pixel buffers.
/// Keep the framework-specific pixel-buffer object at the adapter boundary;
/// the Rust engine owns the bytes once they cross that boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleL8Mask {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl AppleL8Mask {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, AppleL8MaskError> {
        let expected = mask_len(width, height)?;
        if pixels.len() != expected {
            return Err(AppleL8MaskError::DataSizeMismatch {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleL8MaskError {
    InvalidGeometry,
    DataSizeMismatch { expected: usize, actual: usize },
    GeometryMismatch,
}

impl fmt::Display for AppleL8MaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeometry => formatter.write_str("Apple L8 matte geometry is invalid"),
            Self::DataSizeMismatch { expected, actual } => write!(
                formatter,
                "Apple L8 matte has {actual} bytes; expected {expected}"
            ),
            Self::GeometryMismatch => {
                formatter.write_str("Apple L8 mattes do not share the same geometry")
            }
        }
    }
}

impl Error for AppleL8MaskError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplePortraitPersonFusion {
    pub mask: AppleL8Mask,
    pub used_prior: bool,
}

/// Fuse a Vision person matte with the OPPO subject topology prior.
///
/// This is the product policy recovered from the legacy Swift implementation:
/// an OPPO prior is accepted only when it has meaningful overlap with Vision.
/// Accepted prior pixels are clipped to a circularly dilated Vision support
/// region before taking the per-pixel maximum. The Apple adapter is therefore
/// responsible only for producing the Vision L8 observation.
pub fn fuse_apple_portrait_person_mask(
    vision: &AppleL8Mask,
    prior: Option<&AppleL8Mask>,
) -> Result<ApplePortraitPersonFusion, AppleL8MaskError> {
    validate_mask(vision)?;
    let Some(prior) = prior else {
        return Ok(ApplePortraitPersonFusion {
            mask: vision.clone(),
            used_prior: false,
        });
    };
    validate_mask(prior)?;
    if vision.width != prior.width || vision.height != prior.height {
        return Err(AppleL8MaskError::GeometryMismatch);
    }

    let mut overlap = 0_usize;
    let mut prior_support = 0_usize;
    for (&vision_pixel, &prior_pixel) in vision.pixels.iter().zip(&prior.pixels) {
        if prior_pixel >= 64 {
            prior_support += 1;
            if vision_pixel >= 64 {
                overlap += 1;
            }
        }
    }
    if overlap < 16 || prior_support == 0 {
        return Ok(ApplePortraitPersonFusion {
            mask: vision.clone(),
            used_prior: false,
        });
    }

    let radius = 2.0_f64.max(f64::from(vision.width.min(vision.height)) / 256.0);
    let support = circular_maximum(&vision.pixels, vision.width, vision.height, radius);
    let pixels = vision
        .pixels
        .iter()
        .zip(&prior.pixels)
        .zip(support)
        .map(|((&vision_pixel, &prior_pixel), support_pixel)| {
            vision_pixel.max(prior_pixel.min(support_pixel))
        })
        .collect();

    Ok(ApplePortraitPersonFusion {
        mask: AppleL8Mask {
            width: vision.width,
            height: vision.height,
            pixels,
        },
        used_prior: true,
    })
}

fn mask_len(width: u32, height: u32) -> Result<usize, AppleL8MaskError> {
    if width == 0 || height == 0 {
        return Err(AppleL8MaskError::InvalidGeometry);
    }
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(AppleL8MaskError::InvalidGeometry)
}

fn validate_mask(mask: &AppleL8Mask) -> Result<(), AppleL8MaskError> {
    let expected = mask_len(mask.width, mask.height)?;
    if mask.pixels.len() != expected {
        return Err(AppleL8MaskError::DataSizeMismatch {
            expected,
            actual: mask.pixels.len(),
        });
    }
    Ok(())
}

/// Grayscale morphology maximum with a circular structuring element.
///
/// Core Image documents `CIMorphologyMaximum` as a circular maximum filter.
/// Each horizontal span is evaluated with a monotonic deque, keeping the
/// implementation O(radius * pixels) without a new image-processing dependency.
fn circular_maximum(pixels: &[u8], width: u32, height: u32, radius: f64) -> Vec<u8> {
    let width = usize::try_from(width).expect("validated u32 width fits usize");
    let height = usize::try_from(height).expect("validated u32 height fits usize");
    let mut output = vec![0_u8; pixels.len()];
    let mut horizontal = vec![0_u8; width];
    let vertical_radius = radius.floor() as isize;
    let radius_squared = radius * radius;

    for dy in -vertical_radius..=vertical_radius {
        let dy_squared = (dy as f64) * (dy as f64);
        let horizontal_radius = (radius_squared - dy_squared).max(0.0).sqrt().floor() as usize;
        for source_y in 0..height {
            let output_y = source_y as isize - dy;
            if !(0..height as isize).contains(&output_y) {
                continue;
            }
            let row_start = source_y * width;
            horizontal_maximum(
                &pixels[row_start..row_start + width],
                horizontal_radius,
                &mut horizontal,
            );
            let output_start = usize::try_from(output_y).expect("bounded output row") * width;
            for (destination, &value) in output[output_start..output_start + width]
                .iter_mut()
                .zip(&horizontal)
            {
                *destination = (*destination).max(value);
            }
        }
    }
    output
}

fn horizontal_maximum(row: &[u8], radius: usize, output: &mut [u8]) {
    debug_assert_eq!(row.len(), output.len());
    if row.is_empty() {
        return;
    }

    let mut deque = VecDeque::with_capacity(radius.saturating_mul(2).saturating_add(1));
    let mut next = 0_usize;
    for x in 0..row.len() {
        let right = x.saturating_add(radius).min(row.len() - 1);
        while next <= right {
            while deque
                .back()
                .is_some_and(|&index| row[index] <= row[next])
            {
                deque.pop_back();
            }
            deque.push_back(next);
            next += 1;
        }

        let left = x.saturating_sub(radius);
        while deque.front().is_some_and(|&index| index < left) {
            deque.pop_front();
        }
        output[x] = row[*deque.front().expect("non-empty morphology window")];
    }
}

/// ImageIO observations about one ISO Gain Map auxiliary image.
///
/// The platform adapter reports the raw FourCC and geometry. Product policy
/// such as which pixel formats are accepted for Portrait remains in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleGainMapFacts {
    pub pixel_format: FourCC,
    pub width: u32,
    pub height: u32,
}

impl AppleGainMapFacts {
    pub fn supports_portrait_source(self) -> bool {
        matches!(self.pixel_format.as_bytes(), b"444f" | b"L008")
    }

    pub const fn has_geometry(self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(width: u32, height: u32, pixels: Vec<u8>) -> AppleL8Mask {
        AppleL8Mask::new(width, height, pixels).unwrap()
    }

    fn index(width: usize, x: usize, y: usize) -> usize {
        y * width + x
    }

    #[test]
    fn portrait_policy_requires_the_complete_auxiliary_resource_set() {
        let complete = AppleImageAuxiliaryFacts {
            iso_gain_map: true,
            disparity: true,
            portrait_effects_matte: true,
            skin_matte: true,
            hair_matte: true,
            teeth_matte: true,
            glasses_matte: true,
        };
        assert!(complete.satisfies_portrait_editing());

        let missing_glasses = AppleImageAuxiliaryFacts {
            glasses_matte: false,
            ..complete
        };
        assert!(!missing_glasses.satisfies_portrait_editing());
    }

    #[test]
    fn portrait_semantic_contract_is_explicit_and_stable() {
        assert_eq!(
            APPLE_PORTRAIT_SEMANTIC_ROLES,
            [
                AppleSemanticRole::Person,
                AppleSemanticRole::Skin,
                AppleSemanticRole::Hair,
                AppleSemanticRole::Teeth,
                AppleSemanticRole::Glasses,
            ]
        );
    }

    #[test]
    fn l8_mask_rejects_invalid_shape() {
        assert_eq!(
            AppleL8Mask::new(2, 2, vec![0; 3]),
            Err(AppleL8MaskError::DataSizeMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            AppleL8Mask::new(0, 2, Vec::new()),
            Err(AppleL8MaskError::InvalidGeometry)
        );
    }

    #[test]
    fn portrait_person_prior_is_ignored_without_legacy_overlap_threshold() {
        let mut vision = vec![0_u8; 81];
        for y in 0..4 {
            for x in 0..4 {
                vision[index(9, x, y)] = 255;
            }
        }
        let mut prior = vec![0_u8; 81];
        for y in 5..9 {
            for x in 5..9 {
                prior[index(9, x, y)] = 255;
            }
        }
        let vision = mask(9, 9, vision);
        let prior = mask(9, 9, prior);

        let fusion = fuse_apple_portrait_person_mask(&vision, Some(&prior)).unwrap();
        assert!(!fusion.used_prior);
        assert_eq!(fusion.mask, vision);
    }

    #[test]
    fn portrait_person_prior_is_clipped_to_dilated_vision_support() {
        let mut vision = vec![0_u8; 81];
        let mut prior = vec![0_u8; 81];
        for y in 2..6 {
            for x in 2..6 {
                vision[index(9, x, y)] = 255;
                prior[index(9, x, y)] = 255;
            }
        }
        prior[index(9, 7, 3)] = 200;
        prior[index(9, 8, 8)] = 200;
        let vision = mask(9, 9, vision);
        let prior = mask(9, 9, prior);

        let fusion = fuse_apple_portrait_person_mask(&vision, Some(&prior)).unwrap();
        assert!(fusion.used_prior);
        assert_eq!(fusion.mask.pixels[index(9, 7, 3)], 200);
        assert_eq!(fusion.mask.pixels[index(9, 8, 8)], 0);
    }

    #[test]
    fn morphology_support_uses_a_disk_not_a_square() {
        let mut pixels = vec![0_u8; 49];
        pixels[index(7, 3, 3)] = 255;
        let dilated = circular_maximum(&pixels, 7, 7, 2.0);

        assert_eq!(dilated[index(7, 5, 3)], 255);
        assert_eq!(dilated[index(7, 4, 4)], 255);
        assert_eq!(dilated[index(7, 5, 5)], 0);
    }

    #[test]
    fn portrait_source_gain_map_policy_accepts_only_observed_apple_formats() {
        for pixel_format in [FourCC::new(*b"444f"), FourCC::new(*b"L008")] {
            let facts = AppleGainMapFacts {
                pixel_format,
                width: 1024,
                height: 768,
            };
            assert!(facts.supports_portrait_source());
            assert!(facts.has_geometry(1024, 768));
        }

        let unsupported = AppleGainMapFacts {
            pixel_format: FourCC::new(*b"420f"),
            width: 1024,
            height: 768,
        };
        assert!(!unsupported.supports_portrait_source());
    }
}
