use xdremux_engine::{GainMapCodec, RasterDecoder, RasterDecoderCapabilities};
use zune_jpeg::zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
use zune_jpeg::JpegDecoder;

use crate::{CodecError, JpegRasterDecodeRequest, Raster8, RasterPixelFormat, Result};

#[derive(Debug, Clone, Copy, Default)]
pub struct ZuneJpegProvider;

impl ZuneJpegProvider {
    pub const fn new() -> Self {
        Self
    }

    fn decode_jpeg(&self, request: &JpegRasterDecodeRequest) -> Result<Raster8> {
        if request.data.is_empty() {
            return Err(CodecError::invalid("JPEG decode input is empty"));
        }

        let output_colorspace = match request.format {
            RasterPixelFormat::Mono8 => ColorSpace::Luma,
            RasterPixelFormat::Rgb8 => ColorSpace::RGB,
        };
        let options = DecoderOptions::default()
            .set_strict_mode(true)
            .jpeg_set_out_colorspace(output_colorspace);
        let mut decoder =
            JpegDecoder::new_with_options(ZCursor::new(request.data.as_slice()), options);
        let data = decoder
            .decode()
            .map_err(|error| CodecError::invalid(format!("JPEG decode failed: {error}")))?;
        let (width, height) = decoder
            .dimensions()
            .ok_or_else(|| CodecError::invalid("JPEG decoder returned no dimensions"))?;
        let width =
            u32::try_from(width).map_err(|_| CodecError::invalid("JPEG width exceeds u32"))?;
        let height =
            u32::try_from(height).map_err(|_| CodecError::invalid("JPEG height exceeds u32"))?;
        let bytes_per_row = usize::try_from(width)
            .ok()
            .and_then(|value| value.checked_mul(request.format.bytes_per_pixel()))
            .ok_or_else(|| CodecError::invalid("JPEG decoded row size overflows usize"))?;

        Raster8::new(width, height, bytes_per_row, request.format, data)
    }
}

impl RasterDecoder for ZuneJpegProvider {
    type Request = JpegRasterDecodeRequest;
    type Output = Raster8;
    type Error = CodecError;

    fn raster_decoder_capabilities(&self) -> RasterDecoderCapabilities {
        RasterDecoderCapabilities::new([GainMapCodec::Jpeg])
    }

    fn decode_raster(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error> {
        self.decode_jpeg(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use xdremux_engine::{CapabilityInventory, OperationCapability};

    fn jpeg_fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg")
    }

    #[test]
    fn advertises_only_jpeg_raster_decode() {
        let provider = ZuneJpegProvider::new();
        let capabilities = provider.raster_decoder_capabilities();
        assert!(capabilities.supports(GainMapCodec::Jpeg));
        assert!(!capabilities.supports(GainMapCodec::Hevc));

        let inventory = CapabilityInventory::new([OperationCapability::RasterDecoder(
            GainMapCodec::Jpeg,
        )]);
        assert!(inventory.supports(OperationCapability::RasterDecoder(GainMapCodec::Jpeg)));
    }

    #[test]
    fn decodes_real_jpeg_with_trailing_motion_data_as_mono_and_rgb() {
        let jpeg = fs::read(jpeg_fixture()).expect("read real JPEG fixture");
        let provider = ZuneJpegProvider::new();

        for format in [RasterPixelFormat::Mono8, RasterPixelFormat::Rgb8] {
            let raster = provider
                .decode_raster(&JpegRasterDecodeRequest {
                    data: jpeg.clone(),
                    format,
                })
                .expect("decode real JPEG fixture");
            assert!(raster.width > 0);
            assert!(raster.height > 0);
            assert_eq!(raster.format, format);
            assert_eq!(
                raster.bytes_per_row,
                usize::try_from(raster.width).unwrap() * format.bytes_per_pixel()
            );
            raster.validate().unwrap();
        }
    }
}
