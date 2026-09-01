use libheif_rs::{
    color_profile_types, Channel, ColorProfileRaw, ColorSpace, CompressionFormat,
    EncoderParameterValue, EncoderQuality, HeifContext, Image, LibHeif, RgbChroma,
};

use crate::{CodecError, LibHeifProvider, PrimaryHeifEncodeRequest, RasterPixelFormat, Result};

impl LibHeifProvider {
    /// Encode a complete RGB primary rendition as a standalone HEIC container.
    ///
    /// JPEG Motion Photo conversion needs this operation because, unlike ProXDR,
    /// its base rendition is JPEG and cannot be remuxed into HEIF byte-for-byte.
    /// Codec/container details stay in the provider; higher layers only exchange
    /// typed rasters and compressed HEIF bytes.
    pub fn encode_primary_heif(&self, request: &PrimaryHeifEncodeRequest) -> Result<Vec<u8>> {
        request.raster.validate()?;
        if request.raster.format != RasterPixelFormat::Rgb8 {
            return Err(CodecError::invalid(
                "primary HEIF encoding requires an RGB8 raster",
            ));
        }
        if request.quality > 100 {
            return Err(CodecError::invalid(
                "primary HEIF quality must be in the 0...100 range",
            ));
        }
        if let Some(exif) = request.exif_tiff.as_ref()
            && !(exif.starts_with(b"II") || exif.starts_with(b"MM"))
        {
            return Err(CodecError::invalid(
                "primary HEIF EXIF must begin at a TIFF II/MM header",
            ));
        }

        let mut image = Image::new(
            request.raster.width,
            request.raster.height,
            ColorSpace::Rgb(RgbChroma::C444),
        )
        .map_err(CodecError::libheif)?;
        for channel in [Channel::R, Channel::G, Channel::B] {
            image
                .create_plane(
                    channel,
                    request.raster.width,
                    request.raster.height,
                    8,
                )
                .map_err(CodecError::libheif)?;
        }
        {
            let planes = image.planes_mut();
            let r = planes
                .r
                .ok_or_else(|| CodecError::invalid("libheif did not allocate the R plane"))?;
            let g = planes
                .g
                .ok_or_else(|| CodecError::invalid("libheif did not allocate the G plane"))?;
            let b = planes
                .b
                .ok_or_else(|| CodecError::invalid("libheif did not allocate the B plane"))?;
            copy_rgb_to_planes(
                &request.raster,
                r.data,
                r.stride,
                g.data,
                g.stride,
                b.data,
                b.stride,
            )?;
        }

        if let Some(icc) = request.icc_profile.as_ref() {
            if icc.is_empty() {
                return Err(CodecError::invalid("primary HEIF ICC profile is empty"));
            }
            image
                .set_color_profile_raw(&ColorProfileRaw::new(
                    color_profile_types::PROF,
                    icc.clone(),
                ))
                .map_err(CodecError::libheif)?;
        }

        let lib = LibHeif::new_checked().map_err(CodecError::libheif)?;
        let mut encoder = lib
            .encoder_for_format(CompressionFormat::Hevc)
            .map_err(CodecError::libheif)?;
        encoder
            .set_quality(EncoderQuality::Lossy(request.quality))
            .map_err(CodecError::libheif)?;
        // Apple-compatible stills conventionally use 4:2:0. Request it when the
        // chosen HEVC encoder exposes the same stable parameter already verified
        // by the Gain Map provider. The emitted container is decoded below, so an
        // encoder that ignores or cannot honor the request still fails validation.
        if encoder
            .parameters_names()
            .iter()
            .any(|name| name == "chroma")
        {
            encoder
                .set_parameter_value(
                    "chroma",
                    EncoderParameterValue::String("420".to_owned()),
                )
                .map_err(CodecError::libheif)?;
        }

        let mut context = HeifContext::new().map_err(CodecError::libheif)?;
        let handle = context
            .encode_image(&image, &mut encoder, None)
            .map_err(CodecError::libheif)?;
        if let Some(exif) = request.exif_tiff.as_ref() {
            context
                .add_exif_metadata(&handle, exif)
                .map_err(CodecError::libheif)?;
        }
        let encoded = context.write_to_bytes().map_err(CodecError::libheif)?;
        validate_primary_container(&encoded, request.raster.width, request.raster.height)?;
        Ok(encoded)
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_rgb_to_planes(
    source: &crate::Raster8,
    r: &mut [u8],
    r_stride: usize,
    g: &mut [u8],
    g_stride: usize,
    b: &mut [u8],
    b_stride: usize,
) -> Result<()> {
    let width = usize::try_from(source.width)
        .map_err(|_| CodecError::invalid("primary HEIF width exceeds usize"))?;
    let height = usize::try_from(source.height)
        .map_err(|_| CodecError::invalid("primary HEIF height exceeds usize"))?;
    for y in 0..height {
        let source_row = y
            .checked_mul(source.bytes_per_row)
            .ok_or_else(|| CodecError::invalid("primary HEIF source row overflows"))?;
        let r_row = y
            .checked_mul(r_stride)
            .ok_or_else(|| CodecError::invalid("primary HEIF R row overflows"))?;
        let g_row = y
            .checked_mul(g_stride)
            .ok_or_else(|| CodecError::invalid("primary HEIF G row overflows"))?;
        let b_row = y
            .checked_mul(b_stride)
            .ok_or_else(|| CodecError::invalid("primary HEIF B row overflows"))?;
        for x in 0..width {
            let input = source_row
                .checked_add(
                    x.checked_mul(3)
                        .ok_or_else(|| CodecError::invalid("primary HEIF pixel offset overflows"))?,
                )
                .ok_or_else(|| CodecError::invalid("primary HEIF pixel offset overflows"))?;
            r[r_row + x] = source.data[input];
            g[g_row + x] = source.data[input + 1];
            b[b_row + x] = source.data[input + 2];
        }
    }
    Ok(())
}

fn validate_primary_container(encoded: &[u8], width: u32, height: u32) -> Result<()> {
    let context = HeifContext::read_from_bytes(encoded).map_err(CodecError::libheif)?;
    let handle = context
        .primary_image_handle()
        .map_err(CodecError::libheif)?;
    if handle.width() != width || handle.height() != height {
        return Err(CodecError::InconsistentEncoderConfiguration(format!(
            "primary HEIF dimensions changed during encode: expected {width}x{height}, got {}x{}",
            handle.width(),
            handle.height()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HeifRasterDecodeRequest, Raster8};
    use xdremux_engine::RasterDecoder;

    fn raster() -> Raster8 {
        let width = 31_u32;
        let height = 23_u32;
        let row = usize::try_from(width).unwrap() * 3;
        let mut data = vec![0_u8; row * usize::try_from(height).unwrap()];
        for y in 0..usize::try_from(height).unwrap() {
            for x in 0..usize::try_from(width).unwrap() {
                let offset = y * row + x * 3;
                data[offset] = (x * 7) as u8;
                data[offset + 1] = (y * 9) as u8;
                data[offset + 2] = ((x + y) * 5) as u8;
            }
        }
        Raster8::new(width, height, row, RasterPixelFormat::Rgb8, data).unwrap()
    }

    #[test]
    fn primary_heif_roundtrips_geometry_through_libheif() {
        let provider = LibHeifProvider::new();
        let source = raster();
        let encoded = provider
            .encode_primary_heif(&PrimaryHeifEncodeRequest::live_photo(
                source.clone(),
                None,
            ))
            .unwrap();
        let decoded = provider
            .decode_raster(&HeifRasterDecodeRequest {
                data: encoded,
                format: RasterPixelFormat::Rgb8,
            })
            .unwrap();
        assert_eq!((decoded.width, decoded.height), (source.width, source.height));
        assert_eq!(decoded.format, RasterPixelFormat::Rgb8);
    }

    #[test]
    fn rejects_non_tiff_exif_before_calling_libheif() {
        let request = PrimaryHeifEncodeRequest::live_photo(raster(), None)
            .with_exif_tiff(b"Exif\0\0not-a-tiff".to_vec());
        let error = LibHeifProvider::new()
            .encode_primary_heif(&request)
            .expect_err("non-TIFF EXIF must fail before encode");
        assert!(error.to_string().contains("TIFF II/MM"));
    }
}
