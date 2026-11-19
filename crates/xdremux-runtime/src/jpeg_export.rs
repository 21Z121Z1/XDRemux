//! HEIF base plus Gain Map to a two-image JPEG.
//!
//! Both images are re-encoded as eight-bit JPEG. The Gain Map retains its
//! dimensions and declared operation, not exact decoded sample values. The ISO
//! 21496-1 APP2 carriers and the MPF index describe the pair; the base ICC
//! profile is preserved so the SDR image keeps its colorimetry.
use xdremux_codec::{LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_format::dual_jpeg::{
    assemble_dual_layer_jpeg, split_icc_app2, AppSegment, DualLayerJpegAssembly,
    ISO_GAIN_MAP_APP2_IDENTIFIER,
};
use xdremux_metadata::UltraHdrGainMapMetadata;

use crate::{Result, RuntimeError};

/// JPEG quality for both images. The value matches the independent encoder
/// default so a reader can compare the pair without a quality difference.
pub(crate) const JPEG_QUALITY: u8 = 95;

fn external(
    context: &'static str,
    error: impl std::error::Error + Send + Sync + 'static,
) -> RuntimeError {
    RuntimeError::external(context, error)
}

fn encode_jpeg(
    width: u32,
    height: u32,
    format: RasterPixelFormat,
    samples: &[u8],
) -> Result<Vec<u8>> {
    let color = match format {
        RasterPixelFormat::Mono8 => jpeg_encoder::ColorType::Luma,
        RasterPixelFormat::Rgb8 => jpeg_encoder::ColorType::Rgb,
    };
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, JPEG_QUALITY)
        .encode(
            samples,
            u16::try_from(width)
                .map_err(|_| RuntimeError::new("JPEG encode", "width exceeds u16"))?,
            u16::try_from(height)
                .map_err(|_| RuntimeError::new("JPEG encode", "height exceeds u16"))?,
            color,
        )
        .map_err(|error| external("JPEG encode", error))?;
    Ok(jpeg)
}

/// Pack a raster into the tightly packed samples that the JPEG encoder reads.
fn packed_samples(raster: &Raster8) -> Result<Vec<u8>> {
    let row_bytes = usize::try_from(raster.width)
        .ok()
        .and_then(|width| {
            width.checked_mul(match raster.format {
                RasterPixelFormat::Mono8 => 1,
                RasterPixelFormat::Rgb8 => 3,
            })
        })
        .ok_or_else(|| RuntimeError::new("JPEG encode", "row length overflows"))?;
    let height = usize::try_from(raster.height)
        .map_err(|_| RuntimeError::new("JPEG encode", "height exceeds usize"))?;
    let capacity = row_bytes
        .checked_mul(height)
        .ok_or_else(|| RuntimeError::new("JPEG encode", "raster byte count overflows"))?;
    let mut packed = Vec::with_capacity(capacity);
    for row in 0..height {
        let start = row
            .checked_mul(raster.bytes_per_row)
            .ok_or_else(|| RuntimeError::new("JPEG encode", "row offset overflows"))?;
        let end = start
            .checked_add(row_bytes)
            .ok_or_else(|| RuntimeError::new("JPEG encode", "row end overflows"))?;
        let slice = raster.data.get(start..end).ok_or_else(|| {
            RuntimeError::new("JPEG encode", "raster rows do not match its geometry")
        })?;
        packed.extend_from_slice(slice);
    }
    Ok(packed)
}

/// Convert the base samples to eight-bit JPEG codes without a gamma change.
fn eight_bit_samples(raster: &xdremux_codec::NormalizedRaster) -> Result<Vec<u8>> {
    if raster.channels != 3 {
        return Err(RuntimeError::new(
            "JPEG encode",
            "the primary image must decode to three components",
        ));
    }
    let mut samples = Vec::with_capacity(raster.samples.len());
    for value in &raster.samples {
        if !value.is_finite() {
            return Err(RuntimeError::new(
                "JPEG encode",
                "the primary image contains a non-finite sample",
            ));
        }
        samples.push((value.clamp(0.0, 1.0) * 255.0).round() as u8);
    }
    Ok(samples)
}

/// Registered APP8 carriers to add to a two-image JPEG.
pub(crate) struct RegisteredJpegCarriers<'a> {
    /// Enhanced APP8 payload from the source's registered metadata.
    pub enhanced: &'a [u8],
}

/// Build a two-image JPEG from a preserved HEIF body and its Gain Map samples.
pub(crate) fn build_dual_layer_jpeg(
    heif: &LibHeifProvider,
    body: &[u8],
    gain_map: &Raster8,
    metadata: &UltraHdrGainMapMetadata,
    base_icc: Option<&[u8]>,
    exif_tiff: Option<&[u8]>,
    registered: Option<&RegisteredJpegCarriers<'_>>,
) -> Result<Vec<u8>> {
    let primary_item = xdremux_heif::primary_image_item_id(body)
        .map_err(|error| external("JPEG primary item", error))?;
    let base = heif
        .decode_coded_item_normalized(body, primary_item, 3)
        .map_err(|error| external("JPEG primary decode", error))?;
    let base_jpeg = encode_jpeg(
        base.width,
        base.height,
        RasterPixelFormat::Rgb8,
        &eight_bit_samples(&base)?,
    )?;
    let gain_jpeg = encode_jpeg(
        gain_map.width,
        gain_map.height,
        gain_map.format,
        &packed_samples(gain_map)?,
    )?;

    let mut base_segments: Vec<(u8, Vec<u8>)> = Vec::new();
    if let Some(tiff) = exif_tiff {
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(tiff);
        if payload.len() > xdremux_format::dual_jpeg::MAX_APP_SEGMENT_PAYLOAD {
            return Err(RuntimeError::new(
                "JPEG Exif",
                "the source Exif payload exceeds one APP1 segment",
            ));
        }
        base_segments.push((0xe1, payload));
    }
    if let Some(profile) = base_icc {
        for payload in split_icc_app2(profile).map_err(|error| external("JPEG base ICC", error))? {
            base_segments.push((0xe2, payload));
        }
    }
    // The primary image declares only the four-byte version; the parameters
    // stay with the gain-map image.
    let mut declaration = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
    declaration.extend_from_slice(&xdremux_metadata::iso_gain_map_base_app2_payload());
    base_segments.push((0xe2, declaration));

    // No alternate ICC is written, so the application primaries are the base
    // primaries even when the source flag was zero.
    let parameters = metadata
        .to_iso_app2_payload(true)
        .map_err(|error| external("JPEG ISO gain-map metadata", error))?;
    let mut gain_payload = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
    gain_payload.extend_from_slice(&parameters);
    let mut gain_segments: Vec<(u8, Vec<u8>)> = vec![(0xe2, gain_payload)];

    if let Some(registered) = registered {
        // The declaration is the last primary-image carrier, so its offsets
        // can be computed from the segment list that precedes it.
        let base_app = base_segments
            .iter()
            .map(|(marker, payload)| AppSegment {
                marker: *marker,
                payload,
            })
            .collect::<Vec<_>>();
        let gain_app = gain_segments
            .iter()
            .map(|(marker, payload)| AppSegment {
                marker: *marker,
                payload,
            })
            .collect::<Vec<_>>();
        let payload_len = xdremux_metadata::registered_jpeg_base_payload_len(2);
        let offsets = xdremux_format::dual_jpeg::registered_app8_offsets(
            &base_jpeg,
            &gain_jpeg,
            &base_app,
            &gain_app,
            payload_len,
        )
        .map_err(|error| external("registered APP8 offsets", error))?;
        let declaration = xdremux_metadata::RegisteredJpegBaseDeclaration {
            image_count: 2,
            offsets: vec![(offsets.raw, offsets.reduced)],
        };
        let payload = declaration
            .to_app8_payload()
            .map_err(|error| external("registered APP8 declaration", error))?;
        base_segments.push((0xe8, payload));
        gain_segments.push((0xe8, registered.enhanced.to_vec()));
    }

    let base_app = base_segments
        .iter()
        .map(|(marker, payload)| AppSegment {
            marker: *marker,
            payload,
        })
        .collect::<Vec<_>>();
    let gain_app = gain_segments
        .iter()
        .map(|(marker, payload)| AppSegment {
            marker: *marker,
            payload,
        })
        .collect::<Vec<_>>();

    assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
        base: &base_jpeg,
        gain_map: &gain_jpeg,
        base_segments: &base_app,
        gain_segments: &gain_app,
    })
    .map_err(|error| external("dual-layer JPEG assembly", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_refuses_unrepresentable_dimensions_without_panicking() {
        let raster = Raster8 {
            width: u32::MAX,
            height: u32::MAX,
            bytes_per_row: usize::MAX,
            format: RasterPixelFormat::Rgb8,
            data: Vec::new(),
        };
        assert!(packed_samples(&raster).is_err());
    }
}
