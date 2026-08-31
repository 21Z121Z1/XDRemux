use libheif_rs::{
    Channel, ColorSpace, CompressionFormat, EncoderParameterValue, EncoderQuality, HeifContext,
    Image, LibHeif, RgbChroma,
};
use xdremux_engine::{
    CapabilityInventory, GainMapChannels, GainMapCodec, GainMapCodecLayout,
    GainMapEncoderCapabilities, GainMapTileEncoder, OperationCapability, RasterDecoder,
    RasterDecoderCapabilities,
};
use xdremux_format::isobmff::{parse_boxes, parse_meta_box, scan_top_level_boxes, META};
use xdremux_format::{parse_hvcc_profile, ChromaSampling, FourCC};

use crate::{
    CodecError, EncodedGainMapTiles, EncodedHevcTile, GainMapTileEncodeRequest,
    HeifRasterDecodeRequest, Raster8, RasterPixelFormat, Result,
};

const HVC1: FourCC = FourCC::new(*b"hvc1");
const HVCC: FourCC = FourCC::new(*b"hvcC");

#[derive(Debug, Clone, Copy, Default)]
pub struct LibHeifProvider;

impl LibHeifProvider {
    pub const fn new() -> Self {
        Self
    }

    pub fn verified_encoder_layouts() -> [GainMapCodecLayout; 3] {
        [
            layout(ChromaSampling::Mono400),
            layout(ChromaSampling::Yuv420),
            layout(ChromaSampling::Yuv444),
        ]
    }

    pub fn capability_inventory(&self) -> Result<CapabilityInventory> {
        let lib = LibHeif::new_checked().map_err(CodecError::libheif)?;
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
                let logical_width = request
                    .raster
                    .width
                    .checked_sub(origin_x)
                    .ok_or_else(|| CodecError::invalid("tile x offset exceeds raster"))?
                    .min(request.tile_size);
                let logical_height = request
                    .raster
                    .height
                    .checked_sub(origin_y)
                    .ok_or_else(|| CodecError::invalid("tile y offset exceeds raster"))?
                    .min(request.tile_size);
                let encoded = encode_tile_container(request, origin_x, origin_y)?;
                let item = extract_primary_hevc_item(&encoded)?;
                validate_encoded_profile(&item.hvcc, request.target.layout)?;

                if let Some(expected) = &common_hvcc {
                    if expected != &item.hvcc {
                        return Err(CodecError::InconsistentEncoderConfiguration(
                            "libheif emitted different hvcC records for fixed-size tiles".to_owned(),
                        ));
                    }
                } else {
                    common_hvcc = Some(item.hvcc.clone());
                }
                tiles.push(EncodedHevcTile {
                    payload: item.payload,
                    width: logical_width,
                    height: logical_height,
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

    fn decode_heif(&self, request: &HeifRasterDecodeRequest) -> Result<Raster8> {
        if request.data.is_empty() {
            return Err(CodecError::invalid("HEIF decode input is empty"));
        }
        let context = HeifContext::read_from_bytes(&request.data).map_err(CodecError::libheif)?;
        let handle = context
            .primary_image_handle()
            .map_err(CodecError::libheif)?;
        let lib = LibHeif::new_checked().map_err(CodecError::libheif)?;
        match request.format {
            RasterPixelFormat::Mono8 => {
                let image = lib
                    .decode(&handle, ColorSpace::Monochrome, None)
                    .map_err(CodecError::libheif)?;
                let plane = image
                    .planes()
                    .y
                    .ok_or_else(|| CodecError::invalid("decoded monochrome image has no Y plane"))?;
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
                    .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)
                    .map_err(CodecError::libheif)?;
                let plane = image
                    .planes()
                    .interleaved
                    .ok_or_else(|| CodecError::invalid("decoded RGB image has no interleaved plane"))?;
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
    let mut image = match request.raster.format {
        RasterPixelFormat::Mono8 => Image::new(
            request.tile_size,
            request.tile_size,
            ColorSpace::Monochrome,
        )
        .map_err(CodecError::libheif)?,
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

    let lib = LibHeif::new_checked().map_err(CodecError::libheif)?;
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
        if !encoder.parameters_names().iter().any(|name| name == "chroma") {
            return Err(CodecError::unsupported(
                "selected libheif HEVC encoder does not expose the chroma parameter",
            ));
        }
        encoder
            .set_parameter_value("chroma", EncoderParameterValue::String(chroma.to_owned()))
            .map_err(CodecError::libheif)?;
        match encoder.parameter("chroma").map_err(CodecError::libheif)? {
            Some(EncoderParameterValue::String(value)) if value == chroma => {}
            other => {
                return Err(CodecError::InconsistentEncoderConfiguration(format!(
                    "requested chroma {chroma}, encoder reports {other:?}"
                )));
            }
        }
    }

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
        let source_y = origin_y
            .checked_add(y)
            .unwrap_or(u32::MAX)
            .min(source.height - 1);
        let output_row = usize::try_from(y)
            .map_err(|_| CodecError::invalid("tile y exceeds usize"))?
            .checked_mul(output_stride)
            .ok_or_else(|| CodecError::invalid("output row offset overflows"))?;
        let source_row = usize::try_from(source_y)
            .map_err(|_| CodecError::invalid("source y exceeds usize"))?
            .checked_mul(source.bytes_per_row)
            .ok_or_else(|| CodecError::invalid("source row offset overflows"))?;
        for x in 0..tile_size {
            let source_x = origin_x
                .checked_add(x)
                .unwrap_or(u32::MAX)
                .min(source.width - 1);
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
        let source_y = origin_y
            .checked_add(y)
            .unwrap_or(u32::MAX)
            .min(source.height - 1);
        let source_row = usize::try_from(source_y)
            .map_err(|_| CodecError::invalid("source y exceeds usize"))?
            .checked_mul(source.bytes_per_row)
            .ok_or_else(|| CodecError::invalid("source row offset overflows"))?;
        let ry = usize::try_from(y)
            .map_err(|_| CodecError::invalid("tile y exceeds usize"))?;
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
            let source_x = origin_x
                .checked_add(x)
                .unwrap_or(u32::MAX)
                .min(source.width - 1);
            let source_pixel = source_row
                .checked_add(
                    usize::try_from(source_x)
                        .map_err(|_| CodecError::invalid("source x exceeds usize"))?
                        .checked_mul(3)
                        .ok_or_else(|| CodecError::invalid("source pixel offset overflows"))?,
                )
                .ok_or_else(|| CodecError::invalid("source pixel offset overflows"))?;
            let output_x = usize::try_from(x)
                .map_err(|_| CodecError::invalid("tile x exceeds usize"))?;
            r[r_row + output_x] = source.data[source_pixel];
            g[g_row + output_x] = source.data[source_pixel + 1];
            b[b_row + output_x] = source.data[source_pixel + 2];
        }
    }
    Ok(())
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
    let height = usize::try_from(height)
        .map_err(|_| CodecError::invalid("decoded height exceeds usize"))?;
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
        meta.properties
            .iter()
            .find(|property| {
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
                let offset = usize::try_from(y).unwrap() * row_bytes
                    + usize::try_from(x).unwrap() * 3;
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
            provider.raster_decoder_capabilities().iter().collect::<Vec<_>>(),
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
}
