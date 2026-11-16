//! Turn an HEIC primary into an 8-bit sRGB JPEG for galleries that do not
//! play HEIC Motion Photos. The gain map is not applied.

use jpeg_encoder::ColorType;

use xdremux_codec::LibHeifProvider;

use crate::{Result, RuntimeError};

pub(crate) const JPEG_QUALITY: u8 = 90;

pub(crate) fn encode_sdr_jpeg(heif: &[u8]) -> Result<Vec<u8>> {
    let raster = LibHeifProvider::new()
        .decode_sdr_display_rgb8(heif)
        .map_err(|error| RuntimeError::external("HEIC SDR JPEG", error))?;
    let width = u16::try_from(raster.width).map_err(|_| {
        RuntimeError::new(
            "HEIC SDR JPEG",
            "decoded still is wider than a JPEG dimension",
        )
    })?;
    let height = u16::try_from(raster.height).map_err(|_| {
        RuntimeError::new(
            "HEIC SDR JPEG",
            "decoded still is taller than a JPEG dimension",
        )
    })?;
    let packed = if raster.bytes_per_row == usize::from(width) * 3 {
        raster.data
    } else {
        let mut tight = Vec::with_capacity(usize::from(width) * usize::from(height) * 3);
        for row in 0..usize::from(height) {
            let start = row * raster.bytes_per_row;
            tight.extend_from_slice(&raster.data[start..start + usize::from(width) * 3]);
        }
        tight
    };
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, JPEG_QUALITY)
        .encode(&packed, width, height, ColorType::Rgb)
        .map_err(|error| RuntimeError::external("HEIC SDR JPEG encode", error))?;
    if !jpeg.starts_with(&[0xff, 0xd8]) {
        return Err(RuntimeError::new(
            "HEIC SDR JPEG",
            "encoder did not write a JPEG SOI",
        ));
    }
    Ok(jpeg)
}
