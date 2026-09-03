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
