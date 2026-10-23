use xdremux_engine::GainMapEncodeProfile;

use crate::{CodecError, Result};

/// Restore tightly packed RGB16LE display codes to HEIF coded order without
/// reducing their precision. Mirroring and rotation follow ipma order; the
/// inverse samples each coded pixel from its forward-mapped display position.
pub fn undo_heif_rgb16(
    data: &[u8],
    width: u32,
    height: u32,
    quarter_turns_ccw: u8,
    mirror_axis: Option<u8>,
    mirror_before_rotation: bool,
) -> Result<(u32, u32, Vec<u8>)> {
    if width == 0
        || height == 0
        || quarter_turns_ccw > 3
        || mirror_axis.is_some_and(|axis| axis > 1)
    {
        return Err(CodecError::invalid("invalid HEIF RGB16 transform geometry"));
    }
    let display_width =
        usize::try_from(width).map_err(|_| CodecError::invalid("RGB16 width exceeds usize"))?;
    let display_height =
        usize::try_from(height).map_err(|_| CodecError::invalid("RGB16 height exceeds usize"))?;
    let expected = display_width
        .checked_mul(display_height)
        .and_then(|pixels| pixels.checked_mul(6))
        .ok_or_else(|| CodecError::invalid("RGB16 byte size overflows"))?;
    if data.len() != expected {
        return Err(CodecError::invalid(
            "RGB16 data does not match its geometry",
        ));
    }
    let (coded_width, coded_height) = if quarter_turns_ccw & 1 == 1 {
        (display_height, display_width)
    } else {
        (display_width, display_height)
    };
    let mirror = |x: usize, y: usize, w: usize, h: usize| match mirror_axis {
        Some(0) => (x, h - 1 - y),
        Some(1) => (w - 1 - x, y),
        _ => (x, y),
    };
    let mut coded = vec![0; expected];
    for y in 0..coded_height {
        for x in 0..coded_width {
            let (px, py) = if mirror_before_rotation {
                mirror(x, y, coded_width, coded_height)
            } else {
                (x, y)
            };
            let (px, py) = match quarter_turns_ccw {
                0 => (px, py),
                1 => (py, coded_width - 1 - px),
                2 => (coded_width - 1 - px, coded_height - 1 - py),
                3 => (coded_height - 1 - py, px),
                _ => unreachable!("rotation was validated"),
            };
            let (px, py) = if mirror_before_rotation {
                (px, py)
            } else {
                mirror(px, py, display_width, display_height)
            };
            let source = (py * display_width + px) * 6;
            let target = (y * coded_width + x) * 6;
            coded[target..target + 6].copy_from_slice(&data[source..source + 6]);
        }
    }
    Ok((coded_width as u32, coded_height as u32, coded))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterPixelFormat {
    Mono8,
    Rgb8,
}

impl RasterPixelFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Mono8 => 1,
            Self::Rgb8 => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster8 {
    pub width: u32,
    pub height: u32,
    pub bytes_per_row: usize,
    pub format: RasterPixelFormat,
    pub data: Vec<u8>,
}

impl Raster8 {
    pub fn new(
        width: u32,
        height: u32,
        bytes_per_row: usize,
        format: RasterPixelFormat,
        data: Vec<u8>,
    ) -> Result<Self> {
        let raster = Self {
            width,
            height,
            bytes_per_row,
            format,
            data,
        };
        raster.validate()?;
        Ok(raster)
    }

    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(CodecError::invalid("raster dimensions must be non-zero"));
        }
        let row_bytes = usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(self.format.bytes_per_pixel()))
            .ok_or_else(|| CodecError::invalid("raster row size overflows usize"))?;
        if self.bytes_per_row < row_bytes {
            return Err(CodecError::invalid(format!(
                "bytes_per_row {} is smaller than required {row_bytes}",
                self.bytes_per_row
            )));
        }
        let required = self
            .bytes_per_row
            .checked_mul(
                usize::try_from(self.height)
                    .map_err(|_| CodecError::invalid("raster height exceeds usize"))?,
            )
            .ok_or_else(|| CodecError::invalid("raster byte size overflows"))?;
        if self.data.len() < required {
            return Err(CodecError::invalid(format!(
                "raster data is too short: need at least {required}, got {}",
                self.data.len()
            )));
        }
        Ok(())
    }

    /// Undo a HEIF `irot` that libheif has already applied to this raster.
    ///
    /// `quarter_turns_ccw` is the angle stored on the item. The returned raster
    /// is the coded orientation Apple keeps for a gain map, Style Delta Map,
    /// and Linear Thumbnail.
    pub fn undo_irot(&self, quarter_turns_ccw: u8) -> Result<Self> {
        self.validate()?;
        let inverse_ccw = (4 - (quarter_turns_ccw & 0x03)) & 0x03;
        self.rotate_ccw(inverse_ccw)
    }

    fn rotate_ccw(&self, quarter_turns_ccw: u8) -> Result<Self> {
        match quarter_turns_ccw & 0x03 {
            0 => Ok(self.clone()),
            1 => self.rotate_ccw_once(),
            turns => {
                let mut raster = self.rotate_ccw_once()?;
                for _ in 1..turns {
                    raster = raster.rotate_ccw_once()?;
                }
                Ok(raster)
            }
        }
    }

    fn rotate_ccw_once(&self) -> Result<Self> {
        let bytes_per_pixel = self.format.bytes_per_pixel();
        let source_width = usize::try_from(self.width)
            .map_err(|_| CodecError::invalid("raster width exceeds usize"))?;
        let source_height = usize::try_from(self.height)
            .map_err(|_| CodecError::invalid("raster height exceeds usize"))?;
        let destination_width = source_height;
        let destination_height = source_width;
        let destination_row = destination_width
            .checked_mul(bytes_per_pixel)
            .ok_or_else(|| CodecError::invalid("rotated row size overflows"))?;
        let destination_len = destination_row
            .checked_mul(destination_height)
            .ok_or_else(|| CodecError::invalid("rotated raster size overflows"))?;
        let mut data = vec![0_u8; destination_len];
        for y in 0..source_height {
            for x in 0..source_width {
                let destination_x = y;
                let destination_y = source_width - 1 - x;
                let source_offset = y
                    .checked_mul(self.bytes_per_row)
                    .and_then(|row| row.checked_add(x.checked_mul(bytes_per_pixel)?))
                    .ok_or_else(|| CodecError::invalid("source pixel offset overflows"))?;
                let destination_offset = destination_y
                    .checked_mul(destination_row)
                    .and_then(|row| row.checked_add(destination_x.checked_mul(bytes_per_pixel)?))
                    .ok_or_else(|| CodecError::invalid("rotated pixel offset overflows"))?;
                data[destination_offset..destination_offset + bytes_per_pixel]
                    .copy_from_slice(&self.data[source_offset..source_offset + bytes_per_pixel]);
            }
        }
        Self::new(
            u32::try_from(destination_width)
                .map_err(|_| CodecError::invalid("rotated width exceeds u32"))?,
            u32::try_from(destination_height)
                .map_err(|_| CodecError::invalid("rotated height exceeds u32"))?,
            destination_row,
            self.format,
            data,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainMapTileEncodeRequest {
    pub raster: Raster8,
    pub target: GainMapEncodeProfile,
    pub tile_size: u32,
    pub quality: u8,
}

impl GainMapTileEncodeRequest {
    pub const PYTHON_SWIFT_TILE_SIZE: u32 = 512;
    pub const PYTHON_SWIFT_QUALITY: u8 = 90;

    pub fn reference_compatible(raster: Raster8, target: GainMapEncodeProfile) -> Self {
        Self {
            raster,
            target,
            tile_size: Self::PYTHON_SWIFT_TILE_SIZE,
            quality: Self::PYTHON_SWIFT_QUALITY,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedHevcTile {
    pub payload: Vec<u8>,
    /// Coded storage width of this tile. Grid edge cropping belongs to the
    /// parent grid item, so padded edge tiles retain the full tile width here.
    pub width: u32,
    /// Coded storage height of this tile; see `width`.
    pub height: u32,
}

/// One primary HEVC item extracted from a complete HEIF container.
///
/// The item payload and hvcC record are kept together because an ISO-BMFF
/// graph cannot safely reuse one without the other. Higher layers can use
/// this for a resource-oriented remux (for example, the Apple Photographic
/// Styles Linear Thumbnail) without reaching into libheif's container model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedHevcResource {
    pub payload: Vec<u8>,
    /// HEVCDecoderConfigurationRecord payload without the outer hvcC box.
    pub hvcc: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedGainMapTiles {
    pub gain_map_width: u32,
    pub gain_map_height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tiles: Vec<EncodedHevcTile>,
    /// HEVCDecoderConfigurationRecord payload without the outer hvcC box.
    pub hvcc: Vec<u8>,
    pub profile: GainMapEncodeProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeifRasterDecodeRequest {
    pub data: Vec<u8>,
    pub format: RasterPixelFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JpegRasterDecodeRequest {
    pub data: Vec<u8>,
    pub format: RasterPixelFormat,
}

/// Portable request for a complete single-image HEIC base rendition.
///
/// This is intentionally separate from Gain Map tile encoding: callers that
/// transcode JPEG Motion Photo stills need one normal primary image container,
/// while HDR conversion continues to preserve an already-compressed HEIF base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryHeifEncodeRequest {
    pub raster: Raster8,
    pub quality: u8,
    pub icc_profile: Option<Vec<u8>>,
    /// Raw EXIF beginning at the TIFF `II`/`MM` header, as required by libheif.
    pub exif_tiff: Option<Vec<u8>>,
}

impl PrimaryHeifEncodeRequest {
    pub const LIVE_PHOTO_QUALITY: u8 = 95;

    pub fn live_photo(raster: Raster8, icc_profile: Option<Vec<u8>>) -> Self {
        Self {
            raster,
            quality: Self::LIVE_PHOTO_QUALITY,
            icc_profile,
            exif_tiff: None,
        }
    }

    pub fn with_exif_tiff(mut self, exif_tiff: Vec<u8>) -> Self {
        self.exif_tiff = Some(exif_tiff);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb16_inverse_preserves_channels_for_all_rotations_and_mirror_orders() {
        // Six distinct RGB16 colors, including low bits that RGB8 cannot keep.
        let pixels: Vec<[u16; 3]> = (0..6).map(|n| [1001 + n, 20003 + n, 50007 + n]).collect();
        let pack = |indices: &[usize; 6]| {
            indices
                .iter()
                .flat_map(|index| pixels[*index])
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>()
        };
        let coded = pack(&[0, 1, 2, 3, 4, 5]);
        let cases = [
            (
                None,
                false,
                [
                    [0, 1, 2, 3, 4, 5],
                    [2, 5, 1, 4, 0, 3],
                    [5, 4, 3, 2, 1, 0],
                    [3, 0, 4, 1, 5, 2],
                ],
            ),
            (
                Some(0),
                true,
                [
                    [3, 4, 5, 0, 1, 2],
                    [5, 2, 4, 1, 3, 0],
                    [2, 1, 0, 5, 4, 3],
                    [0, 3, 1, 4, 2, 5],
                ],
            ),
            (
                Some(1),
                true,
                [
                    [2, 1, 0, 5, 4, 3],
                    [0, 3, 1, 4, 2, 5],
                    [3, 4, 5, 0, 1, 2],
                    [5, 2, 4, 1, 3, 0],
                ],
            ),
            (
                Some(0),
                false,
                [
                    [3, 4, 5, 0, 1, 2],
                    [0, 3, 1, 4, 2, 5],
                    [2, 1, 0, 5, 4, 3],
                    [5, 2, 4, 1, 3, 0],
                ],
            ),
            (
                Some(1),
                false,
                [
                    [2, 1, 0, 5, 4, 3],
                    [5, 2, 4, 1, 3, 0],
                    [3, 4, 5, 0, 1, 2],
                    [0, 3, 1, 4, 2, 5],
                ],
            ),
        ];
        for (axis, before, rotations) in cases {
            for (turns, permutation) in rotations.iter().enumerate() {
                let (width, height) = if turns % 2 == 0 { (3, 2) } else { (2, 3) };
                let (w, h, actual) =
                    undo_heif_rgb16(&pack(permutation), width, height, turns as u8, axis, before)
                        .unwrap();
                assert_eq!((w, h), (3, 2));
                assert_eq!(
                    actual, coded,
                    "rotation={turns}, axis={axis:?}, mirror_first={before}"
                );
            }
        }
        assert!(undo_heif_rgb16(&coded[..35], 3, 2, 0, None, false).is_err());
        assert!(undo_heif_rgb16(&coded, 3, 2, 0, Some(2), false).is_err());
    }

    #[test]
    fn undo_irot_returns_coded_pixels_for_a_quarter_turn() {
        let display =
            Raster8::new(2, 1, 6, RasterPixelFormat::Rgb8, vec![1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(display.undo_irot(0).unwrap(), display);
        let coded = display.undo_irot(3).unwrap();
        assert_eq!((coded.width, coded.height), (1, 2));
        assert_eq!(coded.data, vec![4, 5, 6, 1, 2, 3]);
        let restored = coded.undo_irot(1).unwrap();
        assert_eq!(restored.data, display.data);
        assert_eq!((restored.width, restored.height), (2, 1));
    }
}
