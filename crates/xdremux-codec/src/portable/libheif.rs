use std::sync::{Mutex, OnceLock};

use libheif_rs::{
    Channel, ColorPrimaries, ColorSpace, CompressionFormat, DecodingOptions, EncoderParameterValue,
    EncoderQuality, HeifContext, Image, ImageHandle, LibHeif, MatrixCoefficients, RgbChroma,
    TransferCharacteristics,
};
use xdremux_engine::{
    CapabilityInventory, GainMapChannels, GainMapCodec, GainMapCodecLayout,
    GainMapEncoderCapabilities, GainMapTileEncoder, OperationCapability, RasterDecoder,
    RasterDecoderCapabilities,
};
use xdremux_format::isobmff::{parse_boxes, parse_meta_box, scan_top_level_boxes, META};
use xdremux_format::{hvcc_vui_color, parse_hvcc_profile, ChromaSampling, FourCC};

use crate::{
    CodecError, EncodedGainMapTiles, EncodedHevcResource, EncodedHevcTile,
    GainMapTileEncodeRequest, HeifRasterDecodeRequest, Raster8, RasterPixelFormat, Result,
};

const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");
const GRID: FourCC = FourCC::new(*b"grid");
const DIMG: FourCC = FourCC::new(*b"dimg");

pub(super) fn checked_libheif() -> Result<&'static LibHeif> {
    // libheif owns one plugin registry per process. Keep it alive across
    // parallel encodes, decodes and metadata reads, including context teardown.
    static LIBHEIF: OnceLock<LibHeif> = OnceLock::new();
    static INIT: Mutex<()> = Mutex::new(());
    if let Some(lib) = LIBHEIF.get() {
        return Ok(lib);
    }
    let _init = INIT.lock().map_err(CodecError::libheif)?;
    if let Some(lib) = LIBHEIF.get() {
        return Ok(lib);
    }
    let lib = LibHeif::new_checked().map_err(CodecError::libheif)?;
    Ok(LIBHEIF.get_or_init(|| lib))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LibHeifProvider;

impl LibHeifProvider {
    pub const fn new() -> Self {
        Self
    }

    /// Decode a selected item in coded coordinates, retaining 8..16-bit sample
    /// precision. Color conversion here only undoes the codec's YCbCr packing;
    /// ICC/gamut/transfer semantics remain with the caller.
    pub fn decode_coded_item_normalized(
        &self,
        source: &[u8],
        item_id: u32,
        channels: u8,
    ) -> Result<crate::NormalizedRaster> {
        if !matches!(channels, 1 | 3) {
            return Err(CodecError::invalid(
                "normalized decode requires one or three channels",
            ));
        }
        let lib = checked_libheif()?;
        let context = HeifContext::read_from_bytes(source).map_err(CodecError::libheif)?;
        let handle = context.image_handle(item_id).map_err(CodecError::libheif)?;
        let mut options = DecodingOptions::new()
            .ok_or_else(|| CodecError::invalid("cannot allocate HEIF decoding options"))?;
        options.set_ignore_transformations(true);
        options.set_convert_hdr_to_8bit(false);
        options.set_strict_decoding(true);
        let space = if channels == 1 {
            ColorSpace::Monochrome
        } else {
            ColorSpace::Rgb(RgbChroma::HdrRgbLe)
        };
        let image = lib
            .decode(&handle, space, Some(options))
            .map_err(CodecError::libheif)?;
        let plane = if channels == 1 {
            image.planes().y
        } else {
            image.planes().interleaved
        }
        .ok_or_else(|| CodecError::invalid("normalized decoder returned no component plane"))?;
        if !(8..=16).contains(&plane.bits_per_pixel) {
            return Err(CodecError::invalid(
                "unsupported normalized decoder precision",
            ));
        }
        let bytes_per_sample = if channels == 3 || plane.storage_bits_per_pixel > 8 {
            2
        } else {
            1
        };
        let row_samples = (image.width() as usize)
            .checked_mul(usize::from(channels))
            .ok_or_else(|| CodecError::invalid("normalized row size overflows"))?;
        let count = row_samples
            .checked_mul(image.height() as usize)
            .ok_or_else(|| CodecError::invalid("normalized raster size overflows"))?;
        let maximum = (1_u32 << plane.bits_per_pixel) - 1;
        let mut samples = Vec::with_capacity(count);
        for y in 0..image.height() as usize {
            let start = y
                .checked_mul(plane.stride)
                .ok_or_else(|| CodecError::invalid("normalized plane offset overflows"))?;
            let row_bytes = row_samples
                .checked_mul(bytes_per_sample)
                .ok_or_else(|| CodecError::invalid("normalized row byte size overflows"))?;
            let row = plane
                .data
                .get(start..start + row_bytes)
                .ok_or_else(|| CodecError::invalid("normalized plane is truncated"))?;
            for x in 0..row_samples {
                let value = if bytes_per_sample == 1 {
                    u32::from(row[x])
                } else {
                    u32::from(u16::from_le_bytes([row[x * 2], row[x * 2 + 1]]))
                };
                if value > maximum {
                    return Err(CodecError::invalid(
                        "decoded component exceeds its bit-depth range",
                    ));
                }
                samples.push(value as f32 / maximum as f32);
            }
        }
        Ok(crate::NormalizedRaster {
            width: image.width(),
            height: image.height(),
            channels,
            bit_depth: plane.bits_per_pixel,
            samples,
        })
    }

    /// Extract the primary HEVC item and its dimensions from a complete HEIF.
    ///
    /// This is intentionally a codec-level operation. The caller receives
    /// only validated compressed-resource facts and remains responsible for
    /// deciding which ISO-BMFF item graph, if any, should reference them.
    pub fn extract_primary_hevc_resource(&self, source: &[u8]) -> Result<EncodedHevcResource> {
        if source.is_empty() {
            return Err(CodecError::invalid("HEIF resource input is empty"));
        }
        checked_libheif()?;
        let context = HeifContext::read_from_bytes(source).map_err(CodecError::libheif)?;
        let handle = context
            .primary_image_handle()
            .map_err(CodecError::libheif)?;
        let item = extract_primary_hevc_item(source)?;
        Ok(EncodedHevcResource {
            payload: item.payload,
            hvcc: item.hvcc,
            width: handle.width(),
            height: handle.height(),
        })
    }

    /// Decode one image item by its ISO-BMFF item ID.
    ///
    /// Styles and semantic analysis need the source Gain Map as a separate
    /// raster while the primary item remains the display base. Keeping item
    /// selection here avoids making the runtime guess through the primary
    /// image API or reimplementing libheif's grid decoder.
    pub fn decode_item_raster(
        &self,
        source: &[u8],
        item_id: u32,
        format: RasterPixelFormat,
    ) -> Result<Raster8> {
        if source.is_empty() {
            return Err(CodecError::invalid("HEIF item decode input is empty"));
        }
        let lib = checked_libheif()?;
        let context = HeifContext::read_from_bytes(source).map_err(CodecError::libheif)?;
        let handle = context.image_handle(item_id).map_err(CodecError::libheif)?;
        self.decode_handle(lib, &handle, format)
    }

    pub fn verified_encoder_layouts() -> [GainMapCodecLayout; 3] {
        [
            layout(ChromaSampling::Mono400),
            layout(ChromaSampling::Yuv420),
            layout(ChromaSampling::Yuv444),
        ]
    }

    pub fn capability_inventory(&self) -> Result<CapabilityInventory> {
        let lib = checked_libheif()?;
        if lib
            .encoder_descriptors(1, Some(CompressionFormat::Hevc), None)
            .is_empty()
        {
            return Err(CodecError::unsupported(
                "libheif has no HEVC encoder plugin available",
            ));
        }
        if lib
            .decoder_descriptors(1, Some(CompressionFormat::Hevc))
            .is_empty()
        {
            return Err(CodecError::unsupported(
                "libheif has no HEVC decoder plugin available",
            ));
        }

        let mut operations = vec![OperationCapability::RasterDecoder(GainMapCodec::Hevc)];
        operations.extend(
            Self::verified_encoder_layouts()
                .into_iter()
                .map(OperationCapability::GainMapTileEncoder),
        );
        Ok(CapabilityInventory::new(operations))
    }

    fn encode_tiles(&self, request: &GainMapTileEncodeRequest) -> Result<EncodedGainMapTiles> {
        validate_encode_request(request)?;
        let columns = ceil_div(request.raster.width, request.tile_size)?;
        let rows = ceil_div(request.raster.height, request.tile_size)?;
        let count = usize::try_from(
            rows.checked_mul(columns)
                .ok_or_else(|| CodecError::invalid("tile count overflows u32"))?,
        )
        .map_err(|_| CodecError::invalid("tile count exceeds usize"))?;
        let mut tiles = Vec::with_capacity(count);
        let mut common_hvcc: Option<Vec<u8>> = None;

        for row in 0..rows {
            for column in 0..columns {
                let origin_x = column
                    .checked_mul(request.tile_size)
                    .ok_or_else(|| CodecError::invalid("tile x offset overflows"))?;
                let origin_y = row
                    .checked_mul(request.tile_size)
                    .ok_or_else(|| CodecError::invalid("tile y offset overflows"))?;
                let encoded = encode_tile_container(request, origin_x, origin_y)?;
                let item = extract_primary_hevc_item(&encoded)?;
                validate_encoded_profile(&item.hvcc, request.target.layout)?;

                if let Some(expected) = &common_hvcc {
                    if expected != &item.hvcc {
                        return Err(CodecError::InconsistentEncoderConfiguration(
                            "libheif emitted different hvcC records for fixed-size tiles"
                                .to_owned(),
                        ));
                    }
                } else {
                    common_hvcc = Some(item.hvcc.clone());
                }
                tiles.push(EncodedHevcTile {
                    payload: item.payload,
                    width: request.tile_size,
                    height: request.tile_size,
                });
            }
        }

        Ok(EncodedGainMapTiles {
            gain_map_width: request.raster.width,
            gain_map_height: request.raster.height,
            tile_width: request.tile_size,
            tile_height: request.tile_size,
            tiles,
            hvcc: common_hvcc.ok_or_else(|| CodecError::invalid("encoder produced no tiles"))?,
            profile: request.target,
        })
    }

    /// Decode the primary item as 8-bit sRGB for a disclosed SDR JPEG.
    ///
    /// Pixels come from the coded YCbCr planes. This program applies one
    /// admitted sRGB or Display P3 transform. libheif's interleaved RGB is
    /// only a witness: on a build without LCMS it matches the YCbCr codes,
    /// and on a build with LCMS it should match that one transform. Neither
    /// result is transformed again. `irot` is applied. The source Orientation
    /// tag is not returned.
    pub fn decode_sdr_display_rgb8(&self, source: &[u8]) -> Result<Raster8> {
        if source.is_empty() {
            return Err(CodecError::invalid("HEIF decode input is empty"));
        }
        let lib = checked_libheif()?;
        let context = HeifContext::read_from_bytes(source).map_err(CodecError::libheif)?;
        let handle = context
            .primary_image_handle()
            .map_err(CodecError::libheif)?;
        refuse_signaled_hdr_transfer(&handle, source)?;
        let icc = handle.color_profile_raw().map(|profile| profile.data);
        let space = if let Some(profile) = icc.as_deref() {
            xdremux_metadata::identify_sdr_color_space(Some(profile), None).map_err(|error| {
                CodecError::unsupported(format!(
                    "primary ICC is not an admitted sRGB or Display P3 SDR profile: {error}"
                ))
            })?
        } else {
            admitted_nclx_sdr(&handle)?
        };
        let matrix = source_rgb_matrix(&handle)?;
        let mut coded_options = DecodingOptions::new()
            .ok_or_else(|| CodecError::invalid("cannot allocate HEIF decoding options"))?;
        coded_options.set_convert_hdr_to_8bit(false);
        let preferred = handle
            .preferred_decoding_colorspace()
            .map_err(CodecError::libheif)?;
        let ColorSpace::YCbCr(chroma) = preferred else {
            return Err(CodecError::unsupported(
                "primary is not YCbCr; refusing to treat an RGB decode tag as sRGB",
            ));
        };
        let coded_image = lib
            .decode(&handle, ColorSpace::YCbCr(chroma), Some(coded_options))
            .map_err(CodecError::libheif)?;
        let coded = ycbcr_full_range_rgb(&coded_image, matrix)?;
        let mut witness_options = DecodingOptions::new()
            .ok_or_else(|| CodecError::invalid("cannot allocate HEIF decoding options"))?;
        witness_options.set_convert_hdr_to_8bit(false);
        let witness_image = lib
            .decode(
                &handle,
                ColorSpace::Rgb(RgbChroma::Rgb),
                Some(witness_options),
            )
            .map_err(CodecError::libheif)?;
        if witness_image.width() != coded_image.width()
            || witness_image.height() != coded_image.height()
        {
            return Err(CodecError::unsupported(
                "libheif RGB size does not match the YCbCr decode",
            ));
        }
        let witness_plane = witness_image
            .planes()
            .interleaved
            .ok_or_else(|| CodecError::invalid("decoded RGB witness has no interleaved plane"))?;
        if witness_plane.bits_per_pixel != 8 {
            return Err(CodecError::unsupported(
                "decoded primary is not 8-bit; refusing to shift HDR or wide samples into JPEG",
            ));
        }
        let witness = copy_plane_to_raster(
            witness_plane.data,
            witness_plane.stride,
            witness_image.width(),
            witness_image.height(),
            3,
        )?;
        let transformed = match (icc.as_deref(), space) {
            (Some(profile), _) => xdremux_metadata::sdr_icc_rgb8_to_srgb8(profile, &coded)
                .map_err(|error| {
                    CodecError::unsupported(format!(
                        "the embedded ICC could not be applied to sRGB: {error}"
                    ))
                })?,
            (None, xdremux_metadata::SdrColorSpace::DisplayP3) => {
                xdremux_metadata::sdr_nclx_rgb8_to_srgb8(space, &coded).map_err(|error| {
                    CodecError::unsupported(format!(
                        "Display P3 nclx could not be applied to sRGB: {error}"
                    ))
                })?
            }
            (None, xdremux_metadata::SdrColorSpace::Srgb) => coded.clone(),
        };
        confirm_one_srgb_transform(&coded, &transformed, &witness)?;
        Raster8::new(
            coded_image.width(),
            coded_image.height(),
            usize::try_from(coded_image.width())
                .ok()
                .and_then(|width| width.checked_mul(3))
                .ok_or_else(|| CodecError::invalid("decoded RGB row size overflows"))?,
            RasterPixelFormat::Rgb8,
            transformed,
        )
    }

    /// Raw Exif item from the primary image, including the HEIF four-byte TIFF offset.
    pub fn primary_exif_payload(&self, source: &[u8]) -> Result<Option<Vec<u8>>> {
        if source.is_empty() {
            return Ok(None);
        }
        checked_libheif()?;
        let context = HeifContext::read_from_bytes(source).map_err(CodecError::libheif)?;
        let handle = context
            .primary_image_handle()
            .map_err(CodecError::libheif)?;
        Ok(handle
            .all_metadata()
            .into_iter()
            .find_map(|item| (item.item_type.0 == *b"Exif").then_some(item.raw_data)))
    }

    fn decode_heif(&self, request: &HeifRasterDecodeRequest) -> Result<Raster8> {
        if request.data.is_empty() {
            return Err(CodecError::invalid("HEIF decode input is empty"));
        }
        let lib = checked_libheif()?;
        let context = HeifContext::read_from_bytes(&request.data).map_err(CodecError::libheif)?;
        let handle = context
            .primary_image_handle()
            .map_err(CodecError::libheif)?;
        self.decode_handle(lib, &handle, request.format)
    }

    fn decode_handle(
        &self,
        lib: &LibHeif,
        handle: &ImageHandle,
        format: RasterPixelFormat,
    ) -> Result<Raster8> {
        match format {
            RasterPixelFormat::Mono8 => {
                let image = lib
                    .decode(handle, ColorSpace::Monochrome, None)
                    .map_err(CodecError::libheif)?;
                let plane = image.planes().y.ok_or_else(|| {
                    CodecError::invalid("decoded monochrome image has no Y plane")
                })?;
                copy_plane_to_raster(plane.data, plane.stride, image.width(), image.height(), 1)
                    .and_then(|data| {
                        Raster8::new(
                            image.width(),
                            image.height(),
                            usize::try_from(image.width())
                                .map_err(|_| CodecError::invalid("decoded width exceeds usize"))?,
                            RasterPixelFormat::Mono8,
                            data,
                        )
                    })
            }
            RasterPixelFormat::Rgb8 => {
                let image = lib
                    .decode(handle, ColorSpace::Rgb(RgbChroma::Rgb), None)
                    .map_err(CodecError::libheif)?;
                let plane = image.planes().interleaved.ok_or_else(|| {
                    CodecError::invalid("decoded RGB image has no interleaved plane")
                })?;
                let row_bytes = usize::try_from(image.width())
                    .ok()
                    .and_then(|width| width.checked_mul(3))
                    .ok_or_else(|| CodecError::invalid("decoded RGB row size overflows"))?;
                let data = copy_plane_to_raster(
                    plane.data,
                    plane.stride,
                    image.width(),
                    image.height(),
                    3,
                )?;
                Raster8::new(
                    image.width(),
                    image.height(),
                    row_bytes,
                    RasterPixelFormat::Rgb8,
                    data,
                )
            }
        }
    }
}

impl GainMapTileEncoder for LibHeifProvider {
    type Request = GainMapTileEncodeRequest;
    type Output = EncodedGainMapTiles;
    type Error = CodecError;

    fn gain_map_encoder_capabilities(&self) -> GainMapEncoderCapabilities {
        GainMapEncoderCapabilities::new(Self::verified_encoder_layouts())
    }

    fn encode_gain_map_tiles(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error> {
        self.encode_tiles(request)
    }
}

impl RasterDecoder for LibHeifProvider {
    type Request = HeifRasterDecodeRequest;
    type Output = Raster8;
    type Error = CodecError;

    fn raster_decoder_capabilities(&self) -> RasterDecoderCapabilities {
        RasterDecoderCapabilities::new([GainMapCodec::Hevc])
    }

    fn decode_raster(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error> {
        self.decode_heif(request)
    }
}

fn layout(chroma: ChromaSampling) -> GainMapCodecLayout {
    GainMapCodecLayout {
        chroma,
        luma_bit_depth: 8,
        chroma_bit_depth: 8,
    }
}

fn ceil_div(value: u32, divisor: u32) -> Result<u32> {
    value
        .checked_add(divisor - 1)
        .map(|sum| sum / divisor)
        .ok_or_else(|| CodecError::invalid("tile geometry overflows"))
}

fn validate_encode_request(request: &GainMapTileEncodeRequest) -> Result<()> {
    request.raster.validate()?;
    if ![256, 512, 1024].contains(&request.tile_size) {
        return Err(CodecError::invalid(
            "tile_size must be 256, 512, or 1024 to match the product contract",
        ));
    }
    if request.quality > 100 {
        return Err(CodecError::invalid("quality must be in the 0...100 range"));
    }
    if request.target.width != request.raster.width
        || request.target.height != request.raster.height
    {
        return Err(CodecError::invalid(
            "Gain Map target dimensions must match the input raster",
        ));
    }
    if request.target.layout.luma_bit_depth != 8 || request.target.layout.chroma_bit_depth != 8 {
        return Err(CodecError::unsupported(
            "portable libheif provider currently verifies only 8-bit Gain Map encoding",
        ));
    }
    match (request.raster.format, request.target.channels, request.target.layout.chroma) {
        (RasterPixelFormat::Mono8, GainMapChannels::Mono, ChromaSampling::Mono400) => Ok(()),
        (
            RasterPixelFormat::Rgb8,
            GainMapChannels::Rgb,
            ChromaSampling::Yuv420 | ChromaSampling::Yuv444,
        ) => Ok(()),
        (_, _, ChromaSampling::Yuv422) => Err(CodecError::unsupported(
            "portable libheif provider does not advertise native 4:2:2; planner must promote to 4:4:4",
        )),
        _ => Err(CodecError::invalid(
            "raster semantics do not match the requested Gain Map encode profile",
        )),
    }
}

fn encode_tile_container(
    request: &GainMapTileEncodeRequest,
    origin_x: u32,
    origin_y: u32,
) -> Result<Vec<u8>> {
    let lib = checked_libheif()?;
    let mut image = match request.raster.format {
        RasterPixelFormat::Mono8 => {
            Image::new(request.tile_size, request.tile_size, ColorSpace::Monochrome)
                .map_err(CodecError::libheif)?
        }
        RasterPixelFormat::Rgb8 => Image::new(
            request.tile_size,
            request.tile_size,
            ColorSpace::Rgb(RgbChroma::C444),
        )
        .map_err(CodecError::libheif)?,
    };

    match request.raster.format {
        RasterPixelFormat::Mono8 => {
            image
                .create_plane(Channel::Y, request.tile_size, request.tile_size, 8)
                .map_err(CodecError::libheif)?;
            let plane = image
                .planes_mut()
                .y
                .ok_or_else(|| CodecError::invalid("libheif did not allocate the Y plane"))?;
            fill_mono_plane(
                plane.data,
                plane.stride,
                &request.raster,
                request.tile_size,
                origin_x,
                origin_y,
            )?;
        }
        RasterPixelFormat::Rgb8 => {
            image
                .create_plane(Channel::R, request.tile_size, request.tile_size, 8)
                .map_err(CodecError::libheif)?;
            image
                .create_plane(Channel::G, request.tile_size, request.tile_size, 8)
                .map_err(CodecError::libheif)?;
            image
                .create_plane(Channel::B, request.tile_size, request.tile_size, 8)
                .map_err(CodecError::libheif)?;
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
            fill_rgb_planes(
                r.data,
                r.stride,
                g.data,
                g.stride,
                b.data,
                b.stride,
                &request.raster,
                request.tile_size,
                origin_x,
                origin_y,
            )?;
        }
    }

    let mut encoder = lib
        .encoder_for_format(CompressionFormat::Hevc)
        .map_err(CodecError::libheif)?;
    encoder
        .set_quality(EncoderQuality::Lossy(request.quality))
        .map_err(CodecError::libheif)?;
    if request.raster.format == RasterPixelFormat::Rgb8 {
        let chroma = match request.target.layout.chroma {
            ChromaSampling::Yuv420 => "420",
            ChromaSampling::Yuv444 => "444",
            _ => {
                return Err(CodecError::unsupported(
                    "RGB libheif tile requested an unadvertised chroma layout",
                ));
            }
        };
        if !encoder
            .parameters_names()
            .iter()
            .any(|name| name == "chroma")
        {
            return Err(CodecError::unsupported(
                "selected libheif HEVC encoder does not expose the chroma parameter",
            ));
        }
        encoder
            .set_parameter_value("chroma", EncoderParameterValue::String(chroma.to_owned()))
            .map_err(CodecError::libheif)?;
        // Parameter readback is not a stable libheif cross-version contract.
        // The emitted hvcC is parsed and validated after encoding instead.
    }

    // libheif's default RGB conversion is BT.601 YCbCr, full range. The
    // final HEIF gain item declares (2, 2, 6), while the encoder's VUI may
    // retain its default sRGB primaries/transfer. Those fields are not the
    // gain application or alternate-image colorimetry. The final-file pixel
    // regression checks both supported RGB chroma layouts against this contract.
    let mut context = HeifContext::new().map_err(CodecError::libheif)?;
    context
        .encode_image(&image, &mut encoder, None)
        .map_err(CodecError::libheif)?;
    context.write_to_bytes().map_err(CodecError::libheif)
}

fn fill_mono_plane(
    output: &mut [u8],
    output_stride: usize,
    source: &Raster8,
    tile_size: u32,
    origin_x: u32,
    origin_y: u32,
) -> Result<()> {
    for y in 0..tile_size {
        let source_y = origin_y.saturating_add(y).min(source.height - 1);
        let output_row = usize::try_from(y)
            .map_err(|_| CodecError::invalid("tile y exceeds usize"))?
            .checked_mul(output_stride)
            .ok_or_else(|| CodecError::invalid("output row offset overflows"))?;
        let source_row = usize::try_from(source_y)
            .map_err(|_| CodecError::invalid("source y exceeds usize"))?
            .checked_mul(source.bytes_per_row)
            .ok_or_else(|| CodecError::invalid("source row offset overflows"))?;
        for x in 0..tile_size {
            let source_x = origin_x.saturating_add(x).min(source.width - 1);
            output[output_row
                + usize::try_from(x).map_err(|_| CodecError::invalid("tile x exceeds usize"))?] =
                source.data[source_row
                    + usize::try_from(source_x)
                        .map_err(|_| CodecError::invalid("source x exceeds usize"))?];
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn fill_rgb_planes(
    r: &mut [u8],
    r_stride: usize,
    g: &mut [u8],
    g_stride: usize,
    b: &mut [u8],
    b_stride: usize,
    source: &Raster8,
    tile_size: u32,
    origin_x: u32,
    origin_y: u32,
) -> Result<()> {
    for y in 0..tile_size {
        let source_y = origin_y.saturating_add(y).min(source.height - 1);
        let source_row = usize::try_from(source_y)
            .map_err(|_| CodecError::invalid("source y exceeds usize"))?
            .checked_mul(source.bytes_per_row)
            .ok_or_else(|| CodecError::invalid("source row offset overflows"))?;
        let ry = usize::try_from(y).map_err(|_| CodecError::invalid("tile y exceeds usize"))?;
        let r_row = ry
            .checked_mul(r_stride)
            .ok_or_else(|| CodecError::invalid("R row offset overflows"))?;
        let g_row = ry
            .checked_mul(g_stride)
            .ok_or_else(|| CodecError::invalid("G row offset overflows"))?;
        let b_row = ry
            .checked_mul(b_stride)
            .ok_or_else(|| CodecError::invalid("B row offset overflows"))?;
        for x in 0..tile_size {
            let source_x = origin_x.saturating_add(x).min(source.width - 1);
            let source_pixel = source_row
                .checked_add(
                    usize::try_from(source_x)
                        .map_err(|_| CodecError::invalid("source x exceeds usize"))?
                        .checked_mul(3)
                        .ok_or_else(|| CodecError::invalid("source pixel offset overflows"))?,
                )
                .ok_or_else(|| CodecError::invalid("source pixel offset overflows"))?;
            let output_x =
                usize::try_from(x).map_err(|_| CodecError::invalid("tile x exceeds usize"))?;
            r[r_row + output_x] = source.data[source_pixel];
            g[g_row + output_x] = source.data[source_pixel + 1];
            b[b_row + output_x] = source.data[source_pixel + 2];
        }
    }
    Ok(())
}

fn admitted_nclx_sdr(handle: &libheif_rs::ImageHandle) -> Result<xdremux_metadata::SdrColorSpace> {
    let Some(profile) = handle.color_profile_nclx() else {
        return Err(CodecError::unsupported(
            "primary has no ICC or nclx color description; refusing to assume sRGB",
        ));
    };
    if profile.full_range_flag() != 1 {
        return Err(CodecError::unsupported(
            "primary nclx is not full range; refusing to expand limited-range codes into JPEG",
        ));
    }
    let primaries = match profile.color_primaries() {
        ColorPrimaries::ITU_R_BT_709_5 => 1,
        ColorPrimaries::SMPTE_EG_432_1 => 12,
        _ => {
            return Err(CodecError::unsupported(
                "primary nclx primaries are not sRGB or Display P3; refusing to label those pixels as sRGB",
            ));
        }
    };
    if profile.transfer_characteristics() != TransferCharacteristics::IEC_61966_2_1 {
        return Err(CodecError::unsupported(
            "primary nclx transfer is not sRGB; refusing to relabel those code values",
        ));
    }
    xdremux_metadata::identify_sdr_color_space(None, Some([primaries, 13, 0])).map_err(|error| {
        CodecError::unsupported(format!(
            "primary nclx is not an admitted SDR encoding: {error}"
        ))
    })
}

fn refuse_signaled_hdr_transfer(handle: &ImageHandle, source: &[u8]) -> Result<()> {
    if let Some(profile) = handle.color_profile_nclx() {
        if hdr_transfer(profile.transfer_characteristics() as u8) {
            return Err(CodecError::unsupported(
                "primary nclx uses an HDR transfer; refusing to clamp those code values into an 8-bit JPEG",
            ));
        }
    }
    let Some(hvcc) = primary_hvcc_payload(source)? else {
        return Ok(());
    };
    let stored = parse_hvcc_profile(&hvcc).map_err(CodecError::format)?;
    if stored.luma_bit_depth != 8 || stored.chroma_bit_depth != 8 {
        return Err(CodecError::unsupported(
            "primary HEVC is not 8-bit; refusing to shift those samples into an 8-bit JPEG",
        ));
    }
    match hvcc_vui_color(&hvcc).map_err(CodecError::format)? {
        Some(color) if hdr_transfer(color.transfer) => Err(CodecError::unsupported(
            "primary HEVC VUI uses an HDR transfer; refusing to clamp those code values into an 8-bit JPEG",
        )),
        Some(_) | None => Ok(()),
    }
}

fn hdr_transfer(transfer: u8) -> bool {
    matches!(transfer, 8 | 9 | 10 | 14 | 15 | 16 | 18)
}

fn primary_hvcc_payload(source: &[u8]) -> Result<Option<Vec<u8>>> {
    let top = scan_top_level_boxes(source).map_err(CodecError::format)?;
    let Some(meta_header) = top.boxes.iter().find(|header| header.kind == META) else {
        return Ok(None);
    };
    let meta = parse_meta_box(source, meta_header).map_err(CodecError::format)?;
    let mut item_id = meta.primary_item_id;
    if meta
        .iinf
        .entries
        .iter()
        .any(|item| item.item_id == item_id && item.item_type == Some(GRID))
    {
        item_id = meta
            .iref
            .as_ref()
            .and_then(|iref| {
                iref.entries
                    .iter()
                    .find(|entry| entry.kind == DIMG && entry.from_item_id == meta.primary_item_id)
            })
            .and_then(|entry| entry.to_item_ids.first().copied())
            .ok_or_else(|| {
                CodecError::unsupported(
                    "primary grid has no tile item; refusing to skip the bitstream transfer",
                )
            })?;
    }
    let Some(entry) = meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item_id)
    else {
        return Ok(None);
    };
    let Some(property) = entry.associations.iter().find_map(|association| {
        meta.properties.iter().find(|property| {
            property.index == u32::from(association.property_index) && property.kind == HVCC
        })
    }) else {
        return Ok(None);
    };
    let raw = source
        .get(property.box_range.clone())
        .ok_or_else(|| CodecError::format("primary hvcC is outside the file"))?;
    let headers = parse_boxes(raw, 0..raw.len()).map_err(CodecError::format)?;
    let header = headers
        .iter()
        .find(|header| header.kind == HVCC)
        .ok_or_else(|| CodecError::format("primary hvcC property has no hvcC box"))?;
    let payload = raw
        .get(header.payload_range())
        .ok_or_else(|| CodecError::format("primary hvcC payload is outside its box"))?;
    if payload.is_empty() {
        return Err(CodecError::format("primary hvcC payload is empty"));
    }
    Ok(Some(payload.to_vec()))
}

#[derive(Clone, Copy)]
enum RgbFromYcbcr {
    Bt601,
    Bt709,
}

fn source_rgb_matrix(handle: &ImageHandle) -> Result<RgbFromYcbcr> {
    let Some(profile) = handle.color_profile_nclx() else {
        return Ok(RgbFromYcbcr::Bt601);
    };
    if profile.full_range_flag() != 1 {
        return Err(CodecError::unsupported(
            "primary nclx is not full range; refusing to expand limited-range codes into JPEG",
        ));
    }
    match profile.matrix_coefficients() {
        MatrixCoefficients::ITU_R_BT_601_6
        | MatrixCoefficients::ITU_R_BT_470_6_System_B_G
        | MatrixCoefficients::Unspecified => Ok(RgbFromYcbcr::Bt601),
        MatrixCoefficients::ITU_R_BT_709_5 => Ok(RgbFromYcbcr::Bt709),
        _ => Err(CodecError::unsupported(
            "primary nclx matrix is not BT.601 or BT.709; refusing to invent RGB coefficients",
        )),
    }
}

fn ycbcr_full_range_rgb(image: &Image, matrix: RgbFromYcbcr) -> Result<Vec<u8>> {
    let planes = image.planes();
    let y = planes
        .y
        .ok_or_else(|| CodecError::invalid("decoded YCbCr image has no Y plane"))?;
    let cb = planes
        .cb
        .ok_or_else(|| CodecError::invalid("decoded YCbCr image has no Cb plane"))?;
    let cr = planes
        .cr
        .ok_or_else(|| CodecError::invalid("decoded YCbCr image has no Cr plane"))?;
    if y.bits_per_pixel != 8 || cb.bits_per_pixel != 8 || cr.bits_per_pixel != 8 {
        return Err(CodecError::unsupported(
            "decoded primary is not 8-bit; refusing to shift HDR or wide samples into JPEG",
        ));
    }
    let width = y.width as usize;
    let height = y.height as usize;
    if width == 0 || height == 0 || cb.width == 0 || cb.height == 0 {
        return Err(CodecError::invalid("decoded YCbCr plane is empty"));
    }
    let mut rgb = vec![0u8; width * height * 3];
    for row in 0..height {
        for col in 0..width {
            let yv = y.data[row * y.stride + col] as f32;
            let chroma_x = col * cb.width as usize / width;
            let chroma_y = row * cb.height as usize / height;
            let cbv = cb.data[chroma_y * cb.stride + chroma_x] as f32 - 128.0;
            let crv = cr.data[chroma_y * cr.stride + chroma_x] as f32 - 128.0;
            let (red, green, blue) = match matrix {
                RgbFromYcbcr::Bt601 => (
                    yv + 1.402 * crv,
                    yv - 0.344_136 * cbv - 0.714_136 * crv,
                    yv + 1.772 * cbv,
                ),
                RgbFromYcbcr::Bt709 => (
                    yv + 1.5748 * crv,
                    yv - 0.187_324 * cbv - 0.468_124 * crv,
                    yv + 1.8556 * cbv,
                ),
            };
            let at = (row * width + col) * 3;
            rgb[at] = red.round().clamp(0.0, 255.0) as u8;
            rgb[at + 1] = green.round().clamp(0.0, 255.0) as u8;
            rgb[at + 2] = blue.round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(rgb)
}

fn confirm_one_srgb_transform(coded: &[u8], transformed: &[u8], witness: &[u8]) -> Result<()> {
    if coded.len() != transformed.len()
        || coded.len() != witness.len()
        || !coded.len().is_multiple_of(3)
    {
        return Err(CodecError::invalid(
            "color planes do not have the same RGB length",
        ));
    }
    let pixels = coded.len() / 3;
    let step = (pixels / 4096).max(1);
    let mut samples = 0usize;
    let mut coded_differs = 0usize;
    let mut witness_is_coded = 0usize;
    let mut witness_is_transformed = 0usize;
    for index in (0..pixels).step_by(step) {
        samples += 1;
        let at = index * 3;
        let coded_delta = max_channel_delta(&coded[at..at + 3], &transformed[at..at + 3]);
        let witness_coded = max_channel_delta(&witness[at..at + 3], &coded[at..at + 3]);
        let witness_transformed = max_channel_delta(&witness[at..at + 3], &transformed[at..at + 3]);
        if coded_delta > 2 {
            coded_differs += 1;
        }
        if witness_coded <= 2 {
            witness_is_coded += 1;
        }
        if witness_transformed <= 4 {
            witness_is_transformed += 1;
        }
    }
    let explains_coded = witness_is_coded * 100 / samples >= 90;
    let explains_transform = witness_is_transformed * 100 / samples >= 90;
    let transform_changes_codes = coded_differs * 20 > samples;
    let explained = if transform_changes_codes {
        explains_coded || explains_transform
    } else {
        explains_coded
    };
    if explained {
        return Ok(());
    }
    Err(CodecError::unsupported(
        "libheif RGB matches neither the YCbCr code values nor one sRGB transform; refusing to guess which conversion already ran",
    ))
}

fn max_channel_delta(left: &[u8], right: &[u8]) -> i32 {
    left.iter()
        .zip(right)
        .map(|(a, b)| i32::from(*a) - i32::from(*b))
        .map(i32::abs)
        .max()
        .unwrap_or(0)
}

fn copy_plane_to_raster(
    source: &[u8],
    source_stride: usize,
    width: u32,
    height: u32,
    bytes_per_pixel: usize,
) -> Result<Vec<u8>> {
    let row_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(bytes_per_pixel))
        .ok_or_else(|| CodecError::invalid("decoded row size overflows"))?;
    if source_stride < row_bytes {
        return Err(CodecError::invalid(
            "decoded libheif plane stride is shorter than its logical row",
        ));
    }
    let height =
        usize::try_from(height).map_err(|_| CodecError::invalid("decoded height exceeds usize"))?;
    let output_len = row_bytes
        .checked_mul(height)
        .ok_or_else(|| CodecError::invalid("decoded raster size overflows"))?;
    let mut output = vec![0u8; output_len];
    for y in 0..height {
        let source_start = y
            .checked_mul(source_stride)
            .ok_or_else(|| CodecError::invalid("decoded source row offset overflows"))?;
        let output_start = y
            .checked_mul(row_bytes)
            .ok_or_else(|| CodecError::invalid("decoded output row offset overflows"))?;
        let source_end = source_start
            .checked_add(row_bytes)
            .ok_or_else(|| CodecError::invalid("decoded source row end overflows"))?;
        let output_end = output_start
            .checked_add(row_bytes)
            .ok_or_else(|| CodecError::invalid("decoded output row end overflows"))?;
        let row = source
            .get(source_start..source_end)
            .ok_or_else(|| CodecError::invalid("decoded libheif plane is shorter than declared"))?;
        output[output_start..output_end].copy_from_slice(row);
    }
    Ok(output)
}

struct HevcItem {
    payload: Vec<u8>,
    hvcc: Vec<u8>,
}

fn extract_primary_hevc_item(source: &[u8]) -> Result<HevcItem> {
    let top = scan_top_level_boxes(source).map_err(CodecError::format)?;
    let meta_header = top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| CodecError::format("libheif output has no meta box"))?;
    let meta = parse_meta_box(source, meta_header).map_err(CodecError::format)?;
    let item = meta
        .iinf
        .entries
        .iter()
        .find(|item| item.item_type == Some(HVC1))
        .ok_or_else(|| CodecError::format("libheif output has no hvc1 image item"))?;
    let location = meta
        .iloc
        .entries
        .iter()
        .find(|entry| entry.item_id == item.item_id)
        .ok_or_else(|| CodecError::format("libheif hvc1 item has no iloc entry"))?;
    if location.data_reference_index != 0 {
        return Err(CodecError::format(
            "libheif hvc1 item uses an external data reference",
        ));
    }

    let mut payload = Vec::new();
    for extent in &location.extents {
        let relative = location
            .base_offset
            .checked_add(extent.offset)
            .ok_or_else(|| CodecError::format("hvc1 extent offset overflows"))?;
        let relative = usize::try_from(relative)
            .map_err(|_| CodecError::format("hvc1 extent offset exceeds usize"))?;
        let start = match location.construction_method {
            0 => relative,
            1 => meta
                .idat
                .as_ref()
                .ok_or_else(|| CodecError::format("hvc1 item references missing idat"))?
                .data_start
                .checked_add(relative)
                .ok_or_else(|| CodecError::format("hvc1 idat offset overflows"))?,
            method => {
                return Err(CodecError::format(format!(
                    "hvc1 item uses unsupported construction method {method}"
                )));
            }
        };
        let length = usize::try_from(extent.length)
            .map_err(|_| CodecError::format("hvc1 extent length exceeds usize"))?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| CodecError::format("hvc1 extent end overflows"))?;
        payload.extend_from_slice(
            source
                .get(start..end)
                .ok_or_else(|| CodecError::format("hvc1 extent is outside libheif output"))?,
        );
    }
    if payload.is_empty() {
        return Err(CodecError::format("libheif hvc1 item payload is empty"));
    }

    let ipma = meta
        .ipma
        .entries
        .iter()
        .find(|entry| entry.item_id == item.item_id)
        .ok_or_else(|| CodecError::format("libheif hvc1 item has no ipma entry"))?;
    let mut hvcc_matches = ipma.associations.iter().filter_map(|association| {
        meta.properties.iter().find(|property| {
            property.index == u32::from(association.property_index) && property.kind == HVCC
        })
    });
    let property = hvcc_matches
        .next()
        .ok_or_else(|| CodecError::format("libheif hvc1 item has no hvcC property"))?;
    if hvcc_matches.next().is_some() {
        return Err(CodecError::format(
            "libheif hvc1 item has more than one hvcC property",
        ));
    }
    let raw = source
        .get(property.box_range.clone())
        .ok_or_else(|| CodecError::format("hvcC property is outside libheif output"))?;
    let headers = parse_boxes(raw, 0..raw.len()).map_err(CodecError::format)?;
    if headers.len() != 1 || headers[0].kind != HVCC {
        return Err(CodecError::format(
            "hvcC property does not contain exactly one hvcC box",
        ));
    }
    let hvcc = raw
        .get(headers[0].payload_range())
        .ok_or_else(|| CodecError::format("hvcC payload is outside its property box"))?
        .to_vec();
    if hvcc.is_empty() {
        return Err(CodecError::format("libheif hvcC payload is empty"));
    }
    Ok(HevcItem { payload, hvcc })
}

fn validate_encoded_profile(hvcc: &[u8], requested: GainMapCodecLayout) -> Result<()> {
    let profile = parse_hvcc_profile(hvcc).map_err(CodecError::format)?;
    if profile.chroma_sampling != requested.chroma
        || profile.luma_bit_depth != requested.luma_bit_depth
        || profile.chroma_bit_depth != requested.chroma_bit_depth
    {
        return Err(CodecError::InconsistentEncoderConfiguration(format!(
            "requested {requested:?}, encoded hvcC reports chroma={:?}, lumaDepth={}, chromaDepth={}",
            profile.chroma_sampling, profile.luma_bit_depth, profile.chroma_bit_depth
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_engine::GainMapEncodeProfile;

    fn rgb_raster(width: u32, height: u32) -> Raster8 {
        let row_bytes = usize::try_from(width).unwrap() * 3;
        let mut data = vec![0u8; row_bytes * usize::try_from(height).unwrap()];
        for y in 0..height {
            for x in 0..width {
                let value = ((x * 3 + y * 5) & 0xff) as u8;
                let offset =
                    usize::try_from(y).unwrap() * row_bytes + usize::try_from(x).unwrap() * 3;
                data[offset] = value;
                data[offset + 1] = value.wrapping_add(17);
                data[offset + 2] = value.wrapping_add(31);
            }
        }
        Raster8::new(width, height, row_bytes, RasterPixelFormat::Rgb8, data).unwrap()
    }

    fn target(width: u32, height: u32, chroma: ChromaSampling) -> GainMapEncodeProfile {
        GainMapEncodeProfile {
            width,
            height,
            channels: if chroma == ChromaSampling::Mono400 {
                GainMapChannels::Mono
            } else {
                GainMapChannels::Rgb
            },
            layout: layout(chroma),
        }
    }

    #[test]
    fn advertises_only_reference_verified_operations() {
        let provider = LibHeifProvider::new();
        let encoder = provider.gain_map_encoder_capabilities();
        assert!(encoder.supports(layout(ChromaSampling::Mono400)));
        assert!(encoder.supports(layout(ChromaSampling::Yuv420)));
        assert!(encoder.supports(layout(ChromaSampling::Yuv444)));
        assert!(!encoder.supports(layout(ChromaSampling::Yuv422)));
        assert_eq!(
            provider
                .raster_decoder_capabilities()
                .iter()
                .collect::<Vec<_>>(),
            vec![GainMapCodec::Hevc]
        );
    }

    #[test]
    fn rejects_native_422_instead_of_silently_falling_back() {
        let raster = rgb_raster(64, 64);
        let request = GainMapTileEncodeRequest {
            target: target(64, 64, ChromaSampling::Yuv422),
            raster,
            tile_size: 512,
            quality: 90,
        };
        let error = LibHeifProvider::new()
            .encode_gain_map_tiles(&request)
            .expect_err("native 422 is not part of the verified provider contract");
        assert!(matches!(error, CodecError::Unsupported(_)));
    }

    #[test]
    fn reference_compatible_request_pins_512_and_quality_90() {
        let raster = rgb_raster(64, 64);
        let request = GainMapTileEncodeRequest::reference_compatible(
            raster,
            target(64, 64, ChromaSampling::Yuv444),
        );
        assert_eq!(request.tile_size, 512);
        assert_eq!(request.quality, 90);
    }

    #[test]
    fn edge_tiles_report_padded_coded_geometry() {
        let raster = rgb_raster(257, 257);
        let request = GainMapTileEncodeRequest {
            target: target(257, 257, ChromaSampling::Yuv444),
            raster,
            tile_size: 256,
            quality: 90,
        };
        let encoded = LibHeifProvider::new()
            .encode_gain_map_tiles(&request)
            .unwrap();
        assert_eq!(
            (encoded.gain_map_width, encoded.gain_map_height),
            (257, 257)
        );
        assert_eq!((encoded.tile_width, encoded.tile_height), (256, 256));
        assert_eq!(encoded.tiles.len(), 4);
        assert!(encoded
            .tiles
            .iter()
            .all(|tile| (tile.width, tile.height) == (256, 256)));
    }

    #[test]
    fn gain_ramp_quantization_and_hevc_error_stay_within_the_measured_budget() {
        // This measures the gain branch, including 8-bit quantization and the
        // actual quality-90 codec. It is not a native HDR-rendering assertion.
        let (w, h) = (93_u32, 71_u32);
        let mut ideal = Vec::new();
        for y in 0..h {
            for x in 0..w {
                ideal.extend([
                    f64::from(x) / f64::from(w - 1),
                    f64::from(y) / f64::from(h - 1),
                    f64::from(x + y) / f64::from(w + h - 2),
                ]);
            }
        }
        for (chroma, max_budget, p99_budget) in [
            (ChromaSampling::Yuv444, 4., 3.),
            (ChromaSampling::Yuv420, 6., 4.),
        ] {
            let raster = Raster8::new(
                w,
                h,
                w as usize * 3,
                RasterPixelFormat::Rgb8,
                ideal.iter().map(|v| (v * 255.).round() as u8).collect(),
            )
            .unwrap();
            let request =
                GainMapTileEncodeRequest::reference_compatible(raster, target(w, h, chroma));
            let decoded = LibHeifProvider::new()
                .decode_raster(&HeifRasterDecodeRequest {
                    data: encode_tile_container(&request, 0, 0).unwrap(),
                    format: RasterPixelFormat::Rgb8,
                })
                .unwrap();
            let mut errors = Vec::new();
            for y in 0..h as usize {
                for x in 0..w as usize {
                    for c in 0..3 {
                        let got = f64::from(decoded.data[y * decoded.bytes_per_row + x * 3 + c]);
                        errors.push((got - ideal[(y * w as usize + x) * 3 + c] * 255.).abs());
                    }
                }
            }
            errors.sort_by(f64::total_cmp);
            let maximum = errors[errors.len() - 1];
            let p99 = errors[(errors.len() * 99).div_ceil(100) - 1];
            println!("gain-ramp {chroma:?}: max={maximum:.6} codes p99={p99:.6} codes; at 24 EV max={:.6} EV p99={:.6} EV", maximum * 24. / 255., p99 * 24. / 255.);
            assert!(
                maximum <= max_budget,
                "{chroma:?}: maximum code error {maximum}"
            );
            assert!(p99 <= p99_budget, "{chroma:?}: p99 code error {p99}");
        }
    }

    #[test]
    fn libheif_rgb444_tile_roundtrips_through_raster_decoder() {
        let raster = rgb_raster(93, 71);
        let request = GainMapTileEncodeRequest::reference_compatible(
            raster,
            target(93, 71, ChromaSampling::Yuv444),
        );
        let encoded = encode_tile_container(&request, 0, 0).unwrap();
        let decoded = LibHeifProvider::new()
            .decode_raster(&HeifRasterDecodeRequest {
                data: encoded,
                format: RasterPixelFormat::Rgb8,
            })
            .unwrap();
        assert_eq!(decoded.width, 512);
        assert_eq!(decoded.height, 512);
        assert_eq!(decoded.format, RasterPixelFormat::Rgb8);
    }

    #[test]
    fn samsung_ultra_hdr_primary_decodes_as_full_range_srgb() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic");
        let data = std::fs::read(path).expect("samsung heic fixture");
        let decoded = LibHeifProvider::new()
            .decode_sdr_display_rgb8(&data)
            .expect("Display P3 primary should become sRGB");
        assert_eq!(decoded.width, 3000);
        assert_eq!(decoded.height, 4000);
        let count = (decoded.data.len() / 3) as u64;
        let mut sum = [0u64; 3];
        let mut max = 0u8;
        for pixel in decoded.data.as_chunks::<3>().0 {
            sum[0] += u64::from(pixel[0]);
            sum[1] += u64::from(pixel[1]);
            sum[2] += u64::from(pixel[2]);
            max = max.max(pixel[0]).max(pixel[1]).max(pixel[2]);
        }
        assert!(sum[1] / count > 20, "primary looks clamped dark");
        assert!(max > 200);
        assert_samsung_primary_matches_independent_srgb(&data, &decoded);
    }

    fn assert_samsung_primary_matches_independent_srgb(source: &[u8], claimed: &Raster8) {
        let context = libheif_rs::HeifContext::read_from_bytes(source).unwrap();
        let handle = context.primary_image_handle().unwrap();
        let icc = handle.color_profile_raw().unwrap().data;
        assert_eq!(
            xdremux_metadata::identify_sdr_color_space(Some(&icc), None).unwrap(),
            xdremux_metadata::SdrColorSpace::DisplayP3
        );
        let lib = checked_libheif().unwrap();
        let ycc = lib
            .decode(
                &handle,
                libheif_rs::ColorSpace::YCbCr(libheif_rs::Chroma::C420),
                None,
            )
            .unwrap();
        let planes = ycc.planes();
        let y = planes.y.unwrap();
        let cb = planes.cb.unwrap();
        let cr = planes.cr.unwrap();
        let width = ycc.width() as usize;
        let height = ycc.height() as usize;
        let mut coded = vec![0u8; width * height * 3];
        for row in 0..height {
            for col in 0..width {
                let yv = y.data[row * y.stride + col] as f32;
                let cbv = cb.data[(row / 2) * cb.stride + col / 2] as f32 - 128.0;
                let crv = cr.data[(row / 2) * cr.stride + col / 2] as f32 - 128.0;
                let at = (row * width + col) * 3;
                coded[at] = (yv + 1.402 * crv).round().clamp(0.0, 255.0) as u8;
                coded[at + 1] = (yv - 0.344136 * cbv - 0.714136 * crv)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                coded[at + 2] = (yv + 1.772 * cbv).round().clamp(0.0, 255.0) as u8;
            }
        }
        let independent = xdremux_metadata::sdr_icc_rgb8_to_srgb8(&icc, &coded).unwrap();
        let irot = source
            .windows(4)
            .position(|window| window == b"irot")
            .unwrap();
        assert_eq!(
            source[irot + 4],
            3,
            "fixture irot is 270 degrees anti-clockwise"
        );
        assert_eq!(claimed.width, 3000);
        assert_eq!(
            claimed.height, 4000,
            "display size, not the coded 4000x3000"
        );
        let mut sampled = 0usize;
        let mut moved = 0usize;
        let mut close = 0usize;
        let mut max_independent = 0i32;
        for (index, pixel) in claimed.data.as_chunks::<3>().0.iter().enumerate() {
            if index % 64 != 0 {
                continue;
            }
            sampled += 1;
            let coded_px = &coded[index * 3..index * 3 + 3];
            let other = &independent[index * 3..index * 3 + 3];
            let coded_delta = pixel
                .iter()
                .zip(coded_px)
                .map(|(a, b)| (*a as i32 - *b as i32).abs())
                .max()
                .unwrap();
            let independent_delta = pixel
                .iter()
                .zip(other)
                .map(|(a, b)| (*a as i32 - *b as i32).abs())
                .max()
                .unwrap();
            if coded_delta > 2 {
                moved += 1;
            }
            if independent_delta <= 2 {
                close += 1;
            }
            max_independent = max_independent.max(independent_delta);
        }
        // Pillow 13 LittleCMS, relative colorimetric, on this file's ICC.
        // These are not the product's own return values.
        let pillow = [
            (0u32, 0u32, [144, 138, 117]),
            (100, 100, [148, 143, 130]),
            (1500, 2000, [139, 200, 252]),
            (2999, 3999, [202, 201, 181]),
        ];
        for (x, y, expected) in pillow {
            let index = (y as usize * claimed.width as usize + x as usize) * 3;
            let actual = &claimed.data[index..index + 3];
            let coded_px = &coded[index..index + 3];
            let delta = actual
                .iter()
                .zip(expected)
                .map(|(a, b)| i32::from(*a).abs_diff(b))
                .max()
                .unwrap();
            assert!(
                delta <= 1,
                "{x},{y} product {actual:?} is not the LittleCMS sRGB {expected:?}"
            );
            if (x, y) == (1500, 2000) {
                assert!(
                    (coded_px[0] as i32 - actual[0] as i32).abs() >= 10,
                    "saturated pixel was left as Display P3 codes {coded_px:?}"
                );
            }
        }
        assert!(
            moved * 10 > sampled,
            "decoded RGB still matches Display P3 code values; sampled {sampled}, moved {moved}"
        );
        assert!(
            close * 100 / sampled >= 99,
            "decoded RGB misses the independent ICC transform; {close}/{sampled}, max {max_independent}"
        );
        assert!(
            max_independent <= 8,
            "max independent delta {max_independent}"
        );
    }

    #[test]
    fn pq_and_hlg_vui_are_refused_before_a_jpeg_exists() {
        let encoded = tiny_rgb_heic();
        let untouched = LibHeifProvider::new()
            .decode_sdr_display_rgb8(&encoded)
            .expect_err("VUI sRGB without ICC or nclx");
        let untouched = untouched.to_string();
        assert!(
            untouched.contains("ICC") || untouched.contains("nclx"),
            "{untouched}"
        );
        for (transfer, label) in [(16u8, "PQ"), (18u8, "HLG")] {
            let patched = patch_sps_transfer(&encoded, transfer);
            let error = LibHeifProvider::new()
                .decode_sdr_display_rgb8(&patched)
                .expect_err(label);
            let message = error.to_string();
            assert!(
                message.contains("HDR"),
                "{label} was not refused: {message}"
            );
        }
    }

    #[test]
    fn unspecified_vui_transfer_is_not_treated_as_srgb() {
        let encoded = patch_sps_transfer(&tiny_rgb_heic(), 2);
        let error = LibHeifProvider::new()
            .decode_sdr_display_rgb8(&encoded)
            .expect_err("unspecified transfer");
        let message = error.to_string();
        assert!(
            message.contains("ICC") || message.contains("nclx") || message.contains("sRGB"),
            "{message}"
        );
        assert!(!message.contains("JPEG"), "{message}");
    }

    fn tiny_rgb_heic() -> Vec<u8> {
        let mut image = Image::new(16, 16, ColorSpace::Rgb(RgbChroma::Rgb)).unwrap();
        image.create_plane(Channel::Interleaved, 16, 16, 8).unwrap();
        {
            let planes = image.planes_mut();
            let plane = planes.interleaved.unwrap();
            for pixel in plane.data.chunks_mut(3) {
                pixel[0] = 220;
                pixel[1] = 20;
                pixel[2] = 40;
            }
        }
        let lib = checked_libheif().unwrap();
        let mut encoder = lib.encoder_for_format(CompressionFormat::Hevc).unwrap();
        encoder.set_quality(EncoderQuality::Lossy(80)).unwrap();
        let mut context = HeifContext::new().unwrap();
        context.encode_image(&image, &mut encoder, None).unwrap();
        context.write_to_bytes().unwrap()
    }

    fn patch_sps_transfer(bytes: &[u8], transfer: u8) -> Vec<u8> {
        // libheif 1.17 / x265 writes this 16x16 Main Still Picture SPS with
        // transfer 13 at a fixed byte pair. Replacing those two bytes keeps
        // the NAL length and does not add an emulation-prevention sequence.
        let original = [0xe0, 0x21, 0xa0, 0xc0];
        let offset = bytes
            .windows(original.len())
            .position(|window| window == original)
            .expect("encoder SPS transfer bytes moved; the VUI patch is no longer aligned");
        let (first, second) = match transfer {
            16 => (0x22, 0x00),
            18 => (0x22, 0x40),
            2 => (0x20, 0x40),
            _ => panic!("unexpected transfer {transfer}"),
        };
        let mut patched = bytes.to_vec();
        patched[offset + 1] = first;
        patched[offset + 2] = second;
        patched
    }
}
