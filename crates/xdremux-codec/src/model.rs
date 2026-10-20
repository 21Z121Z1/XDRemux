use xdremux_engine::GainMapEncodeProfile;

use crate::{CodecError, Result};

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

    /// Restore coded pixels after libheif applied the item's ordered irot/imir.
    pub fn undo_heif_transform(
        &self,
        quarter_turns_ccw: u8,
        mirror_axis: Option<u8>,
        mirror_before_rotation: bool,
    ) -> Result<Self> {
        self.validate()?;
        if mirror_axis.is_some_and(|axis| axis > 1) {
            return Err(CodecError::invalid("HEIF mirror axis must be zero or one"));
        }
        // Invert the association order: the last applied transform goes first.
        if mirror_before_rotation {
            let mut coded = self.undo_irot(quarter_turns_ccw)?;
            if let Some(axis) = mirror_axis {
                coded.mirror(axis);
            }
            Ok(coded)
        } else {
            let mut display = self.clone();
            if let Some(axis) = mirror_axis {
                display.mirror(axis);
            }
            display.undo_irot(quarter_turns_ccw)
        }
    }

    fn mirror(&mut self, axis: u8) {
        let width = self.width as usize;
        let height = self.height as usize;
        if axis == 0 {
            for y in 0..height / 2 {
                for byte in 0..self.bytes_per_row {
                    self.data.swap(
                        y * self.bytes_per_row + byte,
                        (height - 1 - y) * self.bytes_per_row + byte,
                    );
                }
            }
        } else {
            let bytes_per_pixel = self.format.bytes_per_pixel();
            for y in 0..height {
                for x in 0..width / 2 {
                    for byte in 0..bytes_per_pixel {
                        self.data.swap(
                            y * self.bytes_per_row + x * bytes_per_pixel + byte,
                            y * self.bytes_per_row + (width - 1 - x) * bytes_per_pixel + byte,
                        );
                    }
                }
            }
        }
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
    fn inverse_display_transform_restores_mirrored_rgb8_pixels() {
        let pixels = |ids: &[u8]| {
            ids.iter()
                .flat_map(|value| [*value, value * 11, 250 - value])
                .collect::<Vec<_>>()
        };
        // Literal displayed layouts for a 3x2 coded image [1,2,3;4,5,6].
        // The two orders differ because reflection and quarter turns do not commute.
        for (turns, axis, mirror_first, display) in [
            (0, 0, false, [4, 5, 6, 1, 2, 3]),
            (0, 1, false, [3, 2, 1, 6, 5, 4]),
            (1, 0, false, [1, 4, 2, 5, 3, 6]),
            (1, 0, true, [6, 3, 5, 2, 4, 1]),
            (1, 1, false, [6, 3, 5, 2, 4, 1]),
            (1, 1, true, [1, 4, 2, 5, 3, 6]),
            (2, 0, false, [3, 2, 1, 6, 5, 4]),
            (2, 1, false, [4, 5, 6, 1, 2, 3]),
            (3, 0, false, [6, 3, 5, 2, 4, 1]),
            (3, 0, true, [1, 4, 2, 5, 3, 6]),
            (3, 1, false, [1, 4, 2, 5, 3, 6]),
            (3, 1, true, [6, 3, 5, 2, 4, 1]),
        ] {
            let (width, height) = if turns & 1 == 1 { (2, 3) } else { (3, 2) };
            let raster = Raster8::new(
                width,
                height,
                width as usize * 3,
                RasterPixelFormat::Rgb8,
                pixels(&display),
            )
            .unwrap();
            let coded = raster
                .undo_heif_transform(turns, Some(axis), mirror_first)
                .unwrap();
            assert_eq!((coded.width, coded.height), (3, 2));
            assert_eq!(
                coded.data,
                pixels(&[1, 2, 3, 4, 5, 6]),
                "axis={axis} mirror_first={mirror_first}"
            );
        }
    }

    #[test]
    fn inverse_display_transform_handles_padded_mono_rows_and_rejects_invalid_input() {
        let raster =
            Raster8::new(2, 2, 3, RasterPixelFormat::Mono8, vec![1, 2, 99, 3, 4, 88]).unwrap();
        for (axis, expected) in [(0, [3, 4, 1, 2]), (1, [2, 1, 4, 3])] {
            let output = raster.undo_heif_transform(0, Some(axis), false).unwrap();
            let pixels = output
                .data
                .chunks(output.bytes_per_row)
                .flat_map(|row| &row[..2])
                .copied()
                .collect::<Vec<_>>();
            assert_eq!(pixels, expected);
        }
        assert!(raster.undo_heif_transform(0, Some(2), false).is_err());
        let mut truncated = raster;
        truncated.data.truncate(1);
        assert!(truncated.undo_heif_transform(1, Some(0), true).is_err());
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
