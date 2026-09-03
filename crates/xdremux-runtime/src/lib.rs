#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
mod apple_adapter;
#[cfg(target_os = "macos")]
mod apple_styles;
mod batch;
mod batch_checkpoint;
mod categorize;
mod live_photo;
mod oppo_portrait;
mod oppo_tail;
mod validation;

#[cfg(target_os = "macos")]
pub use apple_adapter::AppleStylePropertiesFacts;
pub use batch::{
    plan_batch_items, BatchAssetKind, BatchExecutionOptions, BatchFailure, BatchItem,
    BatchPlanOptions, BatchReceipt, BatchSuccess, BatchSuccessDisposition,
};
pub use batch_checkpoint::{
    motion_photo_checkpoint_path, DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME,
    MOTION_PHOTO_CHECKPOINT_SCHEMA_VERSION,
};
pub use categorize::{CategorizeDisposition, CategorizeItemReceipt, CategorizeReceipt};
pub use live_photo::LivePhotoFileReceipt;
pub use oppo_portrait::ApplePortraitSourcePreflight;
pub use validation::{
    validate_media_file, IsoHdrValidationReport, LivePhotoValidationReport, ValidationReport,
};
pub use xdremux_heif::PhotographicStylesAssembly;

use std::error::Error;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use std::fs;

use atomic_write_file::AtomicWriteFile;
use xdremux_codec::{
    GainMapTileEncodeRequest, JpegRasterDecodeRequest, LibHeifProvider, Raster8, RasterPixelFormat,
    ZuneJpegProvider,
};
use xdremux_container::{extract, ExtractedLhdr, ExtractionMode as ContainerExtractionMode};
use xdremux_engine::{
    execute_conversion, gain_map_source_profile_from_jpeg, ArtifactBuilder, ArtifactPublisher,
    ArtifactValidator, CapabilityInventory, ConversionAnalysis, ConversionPlan, ConversionRequest,
    ExecutionError, ExecutionStage, GainMapChannels, GainMapTileEncoder, OperationCapability,
    RasterDecoder,
};
use xdremux_format::probe_jpeg_frame_profile;
use xdremux_hdr::{
    make_private_gain_map_info_floats, reconstruct_gain_map, resolve,
    ExtractionMode as HdrExtractionMode, GainMapRaster, ResolvedScale,
};
use xdremux_heif::{
    assemble_iso_gain_map_heif, assemble_photographic_styles_heif, validate_gain_map_structure,
    DirectHevcGainMap, GainMapChannels as HeifGainMapChannels,
    GainMapEncodeProfile as HeifGainMapEncodeProfile, GainMapTile, IsoGainMapAssembly,
};
#[cfg(target_os = "macos")]
use xdremux_metadata::make_apple_portrait_focus_xmp;
use xdremux_metadata::{make_apple_tmap_payload, make_hdrgm_xmp};

#[cfg(target_os = "macos")]
const APPLE_PORTRAIT_BASE_QUALITY: f64 = 0.9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    context: &'static str,
    detail: String,
}

impl RuntimeError {
    pub(crate) fn new(context: &'static str, detail: impl Into<String>) -> Self {
        Self {
            context,
            detail: detail.into(),
        }
    }

    pub(crate) fn external(context: &'static str, error: impl fmt::Display) -> Self {
        Self::new(context, error.to_string())
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.context, self.detail)
    }
}

impl Error for RuntimeError {}

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Clone)]
pub struct PreparedProXdr {
    pub analysis: ConversionAnalysis,
    pub extracted: ExtractedLhdr,
    pub scale: ResolvedScale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteConversionReceipt {
    pub plan: ConversionPlan,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConversionReceipt {
    pub plan: ConversionPlan,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotographicStylesFileReceipt {
    pub output: PathBuf,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplePortraitFileReceipt {
    pub output: PathBuf,
    pub auxiliary: xdremux_engine::AppleImageAuxiliaryFacts,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PortableRuntime {
    jpeg: ZuneJpegProvider,
    heif: LibHeifProvider,
}

impl PortableRuntime {
    pub const fn new() -> Self {
        Self {
            jpeg: ZuneJpegProvider::new(),
            heif: LibHeifProvider::new(),
        }
    }

    pub fn capability_inventory(&self) -> Result<CapabilityInventory> {
        let heif = self
            .heif
            .capability_inventory()
            .map_err(|error| RuntimeError::external("portable HEIF capabilities", error))?;
        let mut operations = heif.iter().collect::<Vec<_>>();
        operations.push(OperationCapability::RasterDecoder(
            xdremux_engine::GainMapCodec::Jpeg,
        ));
        Ok(CapabilityInventory::new(operations))
    }

    #[cfg(target_os = "macos")]
    pub fn capability_inventory_with_apple_adapter(
        &self,
        executable: impl AsRef<Path>,
    ) -> Result<CapabilityInventory> {
        let mut operations = self.capability_inventory()?.iter().collect::<Vec<_>>();
        let adapter = apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf());
        operations.extend(adapter.capabilities()?.operation_capabilities());
        Ok(CapabilityInventory::new(operations))
    }

    #[cfg(target_os = "macos")]
    pub fn apple_image_auxiliary_facts(
        &self,
        executable: impl AsRef<Path>,
        input: impl AsRef<Path>,
    ) -> Result<xdremux_engine::AppleImageAuxiliaryFacts> {
        apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf())
            .imageio_auxiliary_facts(input.as_ref())
    }

    #[cfg(target_os = "macos")]
    pub fn apple_semantic_style_properties_facts(
        &self,
        executable: impl AsRef<Path>,
        metadata: &[u8],
        expected_style_data: &[u8],
    ) -> Result<AppleStylePropertiesFacts> {
        apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf())
            .semantic_style_properties_facts(metadata, expected_style_data)
    }

    /// Convert a ProXDR source into a Rust-assembled Photographic Styles
    /// graph. The Apple adapter is restricted to Vision/ImageIO primitives;
    /// Rust owns the resource policy, metadata, graph, validation, and
    /// atomic publication transaction.
    #[cfg(target_os = "macos")]
    pub fn convert_apple_photographic_styles_file(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
        output: impl AsRef<Path>,
    ) -> Result<PhotographicStylesFileReceipt> {
        apple_styles::convert_file(self, executable, source, output)
    }

    #[cfg(target_os = "macos")]
    pub fn apple_write_auxiliary_payloads(
        &self,
        executable: impl AsRef<Path>,
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        payloads: &[xdremux_engine::AppleAuxiliaryPayload],
    ) -> Result<()> {
        apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf())
            .imageio_write_auxiliary(input.as_ref(), output.as_ref(), payloads)
    }

    #[cfg(target_os = "macos")]
    pub fn apple_portrait_semantic_masks(
        &self,
        executable: impl AsRef<Path>,
        input: impl AsRef<Path>,
        orientation: Option<u32>,
    ) -> Result<
        std::collections::BTreeMap<xdremux_engine::AppleSemanticRole, xdremux_engine::AppleL8Mask>,
    > {
        apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf())
            .vision_semantic_mattes(
                input.as_ref(),
                &xdremux_engine::APPLE_PORTRAIT_SEMANTIC_ROLES,
                orientation,
            )
    }

    #[cfg(target_os = "macos")]
    pub fn preflight_apple_portrait_source(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
    ) -> Result<ApplePortraitSourcePreflight> {
        oppo_portrait::prepare_apple_portrait_source(executable.as_ref(), source)
    }

    /// Convert an OPPO Portrait source through one Rust-owned file transaction.
    ///
    /// Rust selects and derives every product artifact, while the Apple
    /// adapter only performs ImageIO encoding/writing and returns factual
    /// consumer observations. All framework output stays in a sibling staging
    /// directory until structural and ImageIO validation both pass.
    #[cfg(target_os = "macos")]
    pub fn convert_apple_portrait_file(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
        output: impl AsRef<Path>,
    ) -> Result<ApplePortraitFileReceipt> {
        let output = output.as_ref();
        let parent = publication_parent(output);
        if !parent.is_dir() {
            return Err(RuntimeError::new(
                "Apple Portrait publication",
                format!("output parent is not a directory: {}", parent.display()),
            ));
        }

        let staging = tempfile::Builder::new()
            .prefix(".xdremux-portrait-")
            .tempdir_in(parent)
            .map_err(|error| RuntimeError::external("Apple Portrait staging", error))?;
        let input_path = staging.path().join("input.heic");
        fs::write(&input_path, source)
            .map_err(|error| RuntimeError::external("Apple Portrait input staging", error))?;

        let preflight = self.preflight_apple_portrait_source(executable.as_ref(), source)?;
        let expected_gain_map = preflight.gain_map;
        let mut source_image = preflight.base_jpeg.clone();
        source_image.extend_from_slice(&preflight.gain_map_jpeg);
        if source_image.is_empty() {
            return Err(RuntimeError::new(
                "Apple Portrait source",
                "extracted adjacent base/Gain Map JPEG source is empty",
            ));
        }
        let source_image_path = staging.path().join("source-image.jpg");
        fs::write(&source_image_path, source_image)
            .map_err(|error| RuntimeError::external("Apple Portrait source staging", error))?;

        let adapter = apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf());
        let carrier_path = staging.path().join("carrier.heic");
        adapter.imageio_encode_source_image(
            &source_image_path,
            &carrier_path,
            APPLE_PORTRAIT_BASE_QUALITY,
        )?;

        let metadata_carrier_path = staging.path().join("carrier-metadata.heic");
        adapter.imageio_merge_metadata(&carrier_path, &input_path, &metadata_carrier_path)?;

        let carrier_gain_map = adapter.imageio_gain_map_facts(&metadata_carrier_path)?;
        if carrier_gain_map != expected_gain_map {
            return Err(RuntimeError::new(
                "Apple Portrait Gain Map carrier",
                format!(
                    "ImageIO changed source Gain Map facts from {:?} to {:?}",
                    expected_gain_map, carrier_gain_map
                ),
            ));
        }
        let carrier_facts = adapter.imageio_auxiliary_facts(&metadata_carrier_path)?;
        if !carrier_facts.iso_gain_map {
            return Err(RuntimeError::new(
                "Apple Portrait Gain Map carrier",
                "ImageIO did not expose the encoded carrier as an ISO Gain Map",
            ));
        }

        let focus_xmp = make_apple_portrait_focus_xmp(
            preflight.base_width,
            preflight.base_height,
            preflight.focus_region.x,
            preflight.focus_region.y,
            preflight.focus_region.width,
            preflight.focus_region.height,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait Focus XMP", error))?;
        let payloads = preflight.into_auxiliary_payloads()?;
        let assembled_path = staging.path().join("assembled.heic");
        adapter.imageio_write_auxiliary(&metadata_carrier_path, &assembled_path, &payloads)?;

        let output_path = staging.path().join("validated.heic");
        adapter.imageio_merge_xmp_metadata(&assembled_path, &focus_xmp, &output_path)?;

        let facts = adapter.imageio_auxiliary_facts(&output_path)?;
        if !facts.satisfies_portrait_editing() {
            return Err(RuntimeError::new(
                "Apple Portrait consumer validation",
                format!("ImageIO did not expose the complete Portrait resource set: {facts:?}"),
            ));
        }
        let bytes = fs::read(&output_path)
            .map_err(|error| RuntimeError::external("Apple Portrait output read", error))?;
        validate_gain_map_structure(&bytes).map_err(|error| {
            RuntimeError::external("Apple Portrait structural validation", error)
        })?;

        let mut publisher = AtomicFilePublisher::new(output.to_path_buf());
        let published = publisher.publish_bytes(bytes)?;
        Ok(ApplePortraitFileReceipt {
            output: published,
            auxiliary: facts,
        })
    }

    pub fn analyze_proxdr(&self, source: &[u8]) -> Result<PreparedProXdr> {
        analyze_proxdr(source)
    }

    pub fn convert_proxdr_bytes<Observe>(
        &self,
        source: &[u8],
        request: ConversionRequest,
        observe: Observe,
    ) -> Result<ByteConversionReceipt>
    where
        Observe: FnMut(ExecutionStage),
    {
        let prepared = self.analyze_proxdr(source)?;
        let capabilities = self.capability_inventory()?;
        let mut builder = ProXdrArtifactBuilder {
            source,
            prepared: &prepared,
            jpeg: &self.jpeg,
            heif: &self.heif,
        };
        let mut validator = IsoGainMapValidator;
        let mut publisher = MemoryPublisher;
        let receipt = execute_conversion(
            &prepared.analysis,
            request,
            &capabilities,
            &mut builder,
            &mut validator,
            &mut publisher,
            observe,
        )
        .map_err(runtime_execution_error)?;
        Ok(ByteConversionReceipt {
            plan: receipt.plan,
            bytes: receipt.published,
        })
    }

    pub fn convert_proxdr_file<Observe>(
        &self,
        source: &[u8],
        output: impl AsRef<Path>,
        request: ConversionRequest,
        observe: Observe,
    ) -> Result<FileConversionReceipt>
    where
        Observe: FnMut(ExecutionStage),
    {
        let prepared = self.analyze_proxdr(source)?;
        let capabilities = self.capability_inventory()?;
        let mut builder = ProXdrArtifactBuilder {
            source,
            prepared: &prepared,
            jpeg: &self.jpeg,
            heif: &self.heif,
        };
        let mut validator = IsoGainMapValidator;
        let mut publisher = AtomicFilePublisher::new(output.as_ref().to_path_buf());
        let receipt = execute_conversion(
            &prepared.analysis,
            request,
            &capabilities,
            &mut builder,
            &mut validator,
            &mut publisher,
            observe,
        )
        .map_err(runtime_execution_error)?;
        Ok(FileConversionReceipt {
            plan: receipt.plan,
            output: receipt.published,
        })
    }

    pub fn convert_motion_photo_file(
        &self,
        source: &[u8],
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Result<LivePhotoFileReceipt> {
        live_photo::convert_motion_photo_file(
            &self.jpeg,
            &self.heif,
            source,
            input.as_ref(),
            output.as_ref(),
        )
    }

    /// Convert a Motion Photo while enforcing product-intent applicability.
    pub fn convert_motion_photo_file_with_request(
        &self,
        source: &[u8],
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        request: ConversionRequest,
    ) -> Result<LivePhotoFileReceipt> {
        if request.requests_oppo_gallery_compatibility() {
            return Err(RuntimeError::new(
                "Motion Photo conversion",
                "OPPO-compatible output applies to ProXDR still images and cannot be combined with Motion Photo conversion",
            ));
        }
        if request.apple_features.any() {
            return Err(RuntimeError::new(
                "Motion Photo conversion",
                "Apple Portrait and Photographic Styles intents apply to ProXDR still images and cannot be combined with Motion Photo conversion",
            ));
        }
        self.convert_motion_photo_file(source, input, output)
    }

    /// Publish a Rust-assembled Photographic Styles graph as one atomic file.
    ///
    /// Product policy and all style resources are prepared by the caller's
    /// Rust engine path. This runtime operation owns the final graph assembly
    /// and publication boundary; it never invokes a second product pipeline
    /// or a subprocess for product policy.
    pub fn assemble_photographic_styles_file(
        &self,
        source: &[u8],
        output: impl AsRef<Path>,
        assembly: &PhotographicStylesAssembly<'_>,
    ) -> Result<PhotographicStylesFileReceipt> {
        let bytes = assemble_photographic_styles_heif(source, assembly)
            .map_err(|error| RuntimeError::external("Rust Photographic Styles assembly", error))?;
        let mut publisher = AtomicFilePublisher::new(output.as_ref().to_path_buf());
        let published = publisher.publish_bytes(bytes)?;
        Ok(PhotographicStylesFileReceipt { output: published })
    }
}

pub fn analyze_proxdr(source: &[u8]) -> Result<PreparedProXdr> {
    let extracted = extract(source)
        .map_err(|error| RuntimeError::external("ProXDR container analysis", error))?;
    let frame = probe_jpeg_frame_profile(&extracted.mask_jpeg_data)
        .map_err(|error| RuntimeError::external("private Gain Map JPEG profile", error))?;
    let gain_map = gain_map_source_profile_from_jpeg(&frame)
        .map_err(|error| RuntimeError::external("Gain Map source profile", error))?;
    let scale = resolve(&extracted.meta_floats, hdr_extraction_mode(extracted.mode))
        .map_err(|error| RuntimeError::external("HDR scale resolution", error))?;
    let analysis = ConversionAnalysis { gain_map };
    Ok(PreparedProXdr {
        analysis,
        extracted,
        scale,
    })
}

fn hdr_extraction_mode(mode: ContainerExtractionMode) -> HdrExtractionMode {
    match mode {
        ContainerExtractionMode::Lhdr => HdrExtractionMode::Lhdr,
        ContainerExtractionMode::Uhdr => HdrExtractionMode::Uhdr,
    }
}

fn runtime_execution_error(
    error: ExecutionError<RuntimeError, RuntimeError, RuntimeError>,
) -> RuntimeError {
    RuntimeError::external("conversion execution", error)
}

struct ProXdrArtifactBuilder<'a> {
    source: &'a [u8],
    prepared: &'a PreparedProXdr,
    jpeg: &'a ZuneJpegProvider,
    heif: &'a LibHeifProvider,
}

impl ArtifactBuilder for ProXdrArtifactBuilder<'_> {
    type Artifact = Vec<u8>;
    type Error = RuntimeError;

    fn build_artifact(&mut self, plan: &ConversionPlan) -> Result<Self::Artifact> {
        let decoded = self
            .jpeg
            .decode_raster(&JpegRasterDecodeRequest {
                data: self.prepared.extracted.mask_jpeg_data.clone(),
                format: match self.prepared.analysis.gain_map.channels {
                    GainMapChannels::Mono => RasterPixelFormat::Mono8,
                    GainMapChannels::Rgb => RasterPixelFormat::Rgb8,
                },
            })
            .map_err(|error| RuntimeError::external("private Gain Map JPEG decode", error))?;
        let raster = self.normalized_gain_map_raster(decoded, plan)?;
        let encoded = self
            .heif
            .encode_gain_map_tiles(&GainMapTileEncodeRequest::reference_compatible(
                raster,
                plan.gain_map_target,
            ))
            .map_err(|error| RuntimeError::external("HEVC Gain Map encode", error))?;

        let info_floats = match self.prepared.extracted.mode {
            ContainerExtractionMode::Uhdr => self.prepared.extracted.meta_floats.clone(),
            ContainerExtractionMode::Lhdr => {
                make_private_gain_map_info_floats(&self.prepared.scale).to_vec()
            }
        };
        let tmap_payload = make_apple_tmap_payload(&info_floats)
            .map_err(|error| RuntimeError::external("ISO Gain Map tmap", error))?;
        let xmp_payload = make_hdrgm_xmp(&info_floats)
            .map_err(|error| RuntimeError::external("ISO Gain Map XMP", error))?;

        let body = standard_heif_body(
            self.source,
            self.prepared.extracted.manifest_info.extension_start,
        )?;
        let tiles = encoded
            .tiles
            .iter()
            .map(|tile| GainMapTile {
                payload: &tile.payload,
                width: tile.width,
                height: tile.height,
            })
            .collect::<Vec<_>>();
        let gain_map = DirectHevcGainMap {
            gain_map_width: encoded.gain_map_width,
            gain_map_height: encoded.gain_map_height,
            tile_width: encoded.tile_width,
            tile_height: encoded.tile_height,
            tiles: &tiles,
            hvcc: &encoded.hvcc,
            profile: HeifGainMapEncodeProfile {
                channels: match encoded.profile.channels {
                    GainMapChannels::Mono => HeifGainMapChannels::Mono,
                    GainMapChannels::Rgb => HeifGainMapChannels::Rgb,
                },
                chroma: encoded.profile.layout.chroma,
                luma_bit_depth: encoded.profile.layout.luma_bit_depth,
                chroma_bit_depth: encoded.profile.layout.chroma_bit_depth,
            },
        };
        let mut output = assemble_iso_gain_map_heif(
            body,
            &IsoGainMapAssembly {
                gain_map,
                tmap_payload: &tmap_payload,
                xmp_payload: &xmp_payload,
            },
        )
        .map_err(|error| RuntimeError::external("native Rust HEIF assembly", error))?;

        let tail = oppo_tail::build_tail(self.source, &self.prepared.extracted, plan.output)?;
        output.extend_from_slice(&tail);
        Ok(output)
    }
}

impl ProXdrArtifactBuilder<'_> {
    fn normalized_gain_map_raster(
        &self,
        decoded: Raster8,
        plan: &ConversionPlan,
    ) -> Result<Raster8> {
        let normalized = match self.prepared.extracted.mode {
            ContainerExtractionMode::Uhdr => decoded,
            ContainerExtractionMode::Lhdr => {
                if decoded.format != RasterPixelFormat::Mono8 {
                    return Err(RuntimeError::new(
                        "LHDR reconstruction",
                        "LHDR private mask did not decode as monochrome",
                    ));
                }
                let mask = GainMapRaster {
                    width: usize::try_from(decoded.width).map_err(|_| {
                        RuntimeError::new("LHDR reconstruction", "width exceeds usize")
                    })?,
                    height: usize::try_from(decoded.height).map_err(|_| {
                        RuntimeError::new("LHDR reconstruction", "height exceeds usize")
                    })?,
                    bytes_per_row: decoded.bytes_per_row,
                    channel_count: 1,
                    data: decoded.data,
                };
                let (gain_map, _) = reconstruct_gain_map(
                    &mask,
                    &self.prepared.scale,
                    &self.prepared.extracted.meta_floats,
                )
                .map_err(|error| RuntimeError::external("LHDR Gain Map reconstruction", error))?;
                Raster8::new(
                    u32::try_from(gain_map.width).map_err(|_| {
                        RuntimeError::new("LHDR reconstruction", "width exceeds u32")
                    })?,
                    u32::try_from(gain_map.height).map_err(|_| {
                        RuntimeError::new("LHDR reconstruction", "height exceeds u32")
                    })?,
                    gain_map.bytes_per_row,
                    RasterPixelFormat::Mono8,
                    gain_map.data,
                )
                .map_err(|error| RuntimeError::external("LHDR normalized raster", error))?
            }
        };
        conform_raster_channels(normalized, plan.gain_map_target.channels)
    }
}

fn conform_raster_channels(raster: Raster8, target: GainMapChannels) -> Result<Raster8> {
    match (raster.format, target) {
        (RasterPixelFormat::Mono8, GainMapChannels::Mono)
        | (RasterPixelFormat::Rgb8, GainMapChannels::Rgb) => Ok(raster),
        (RasterPixelFormat::Mono8, GainMapChannels::Rgb) => replicate_mono_to_rgb(raster),
        (RasterPixelFormat::Rgb8, GainMapChannels::Mono) => Err(RuntimeError::new(
            "Gain Map channel conformance",
            "refusing to collapse an RGB Gain Map into monochrome",
        )),
    }
}

fn replicate_mono_to_rgb(raster: Raster8) -> Result<Raster8> {
    raster
        .validate()
        .map_err(|error| RuntimeError::external("Gain Map channel conformance", error))?;
    if raster.format != RasterPixelFormat::Mono8 {
        return Err(RuntimeError::new(
            "Gain Map channel conformance",
            "replication requires a monochrome raster",
        ));
    }
    let width = usize::try_from(raster.width)
        .map_err(|_| RuntimeError::new("Gain Map channel conformance", "width exceeds usize"))?;
    let height = usize::try_from(raster.height)
        .map_err(|_| RuntimeError::new("Gain Map channel conformance", "height exceeds usize"))?;
    let row_bytes = width.checked_mul(3).ok_or_else(|| {
        RuntimeError::new("Gain Map channel conformance", "RGB row size overflows")
    })?;
    let mut data = vec![
        0_u8;
        row_bytes.checked_mul(height).ok_or_else(|| {
            RuntimeError::new("Gain Map channel conformance", "RGB raster size overflows")
        })?
    ];
    for y in 0..height {
        let source_row = y.checked_mul(raster.bytes_per_row).ok_or_else(|| {
            RuntimeError::new("Gain Map channel conformance", "source row overflows")
        })?;
        let output_row = y.checked_mul(row_bytes).ok_or_else(|| {
            RuntimeError::new("Gain Map channel conformance", "output row overflows")
        })?;
        for x in 0..width {
            let value = raster.data[source_row + x];
            let output = output_row + x * 3;
            data[output] = value;
            data[output + 1] = value;
            data[output + 2] = value;
        }
    }
    Raster8::new(
        raster.width,
        raster.height,
        row_bytes,
        RasterPixelFormat::Rgb8,
        data,
    )
    .map_err(|error| RuntimeError::external("Gain Map channel conformance", error))
}

fn standard_heif_body(source: &[u8], extension_start: usize) -> Result<&[u8]> {
    if extension_start == 0 || extension_start > source.len() {
        return Err(RuntimeError::new(
            "source HEIF boundary",
            format!(
                "parsed extension start {extension_start} is outside input length {}",
                source.len()
            ),
        ));
    }
    source.get(..extension_start).ok_or_else(|| {
        RuntimeError::new(
            "source HEIF boundary",
            "parsed extension boundary could not be sliced from input",
        )
    })
}

#[derive(Debug, Clone, Copy)]
struct IsoGainMapValidator;

impl ArtifactValidator<Vec<u8>> for IsoGainMapValidator {
    type Error = RuntimeError;

    fn validate_artifact(&mut self, _plan: &ConversionPlan, artifact: &Vec<u8>) -> Result<()> {
        validate_gain_map_structure(artifact)
            .map(|_| ())
            .map_err(|error| RuntimeError::external("ISO Gain Map validation", error))
    }
}

#[derive(Debug, Clone, Copy)]
struct MemoryPublisher;

impl ArtifactPublisher<Vec<u8>> for MemoryPublisher {
    type Output = Vec<u8>;
    type Error = RuntimeError;

    fn publish_artifact(&mut self, _plan: &ConversionPlan, artifact: Vec<u8>) -> Result<Vec<u8>> {
        Ok(artifact)
    }
}

#[derive(Debug, Clone)]
pub struct AtomicFilePublisher {
    output: PathBuf,
}

impl AtomicFilePublisher {
    pub fn new(output: PathBuf) -> Self {
        Self { output }
    }

    pub fn publish_bytes(&mut self, artifact: Vec<u8>) -> Result<PathBuf> {
        let parent = publication_parent(&self.output);
        if !parent.is_dir() {
            return Err(RuntimeError::new(
                "atomic publication",
                format!("output parent is not a directory: {}", parent.display()),
            ));
        }
        let mut file = AtomicWriteFile::open(&self.output)
            .map_err(|error| RuntimeError::external("atomic publication open", error))?;
        file.write_all(&artifact)
            .map_err(|error| RuntimeError::external("atomic publication write", error))?;
        file.sync_all()
            .map_err(|error| RuntimeError::external("atomic publication sync", error))?;
        file.commit()
            .map_err(|error| RuntimeError::external("atomic publication commit", error))?;
        Ok(self.output.clone())
    }
}

fn publication_parent(output: &Path) -> &Path {
    match output.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

impl ArtifactPublisher<Vec<u8>> for AtomicFilePublisher {
    type Output = PathBuf;
    type Error = RuntimeError;

    fn publish_artifact(&mut self, _plan: &ConversionPlan, artifact: Vec<u8>) -> Result<PathBuf> {
        self.publish_bytes(artifact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use xdremux_engine::OutputIntent;
    use xdremux_format::isobmff::{make_box, MDAT};
    use xdremux_format::FourCC;

    #[test]
    fn standard_heif_body_drops_post_mdat_vendor_tail() {
        let mut source = make_box(FourCC::new(*b"ftyp"), b"heic").unwrap();
        source.extend_from_slice(&make_box(MDAT, b"base").unwrap());
        let body_len = source.len();
        source.extend_from_slice(b"vendor-tail");
        assert_eq!(
            standard_heif_body(&source, body_len).unwrap().len(),
            body_len
        );
    }

    #[test]
    fn mono_replication_preserves_logical_pixels_not_padding() {
        let raster = Raster8::new(
            2,
            2,
            4,
            RasterPixelFormat::Mono8,
            vec![1, 2, 99, 99, 3, 4, 88, 88],
        )
        .unwrap();
        let rgb = replicate_mono_to_rgb(raster).unwrap();
        assert_eq!(rgb.bytes_per_row, 6);
        assert_eq!(rgb.data, vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4]);
    }

    #[test]
    fn relative_publication_uses_current_directory_parent() {
        assert_eq!(publication_parent(Path::new("output.heic")), Path::new("."));
        assert_eq!(
            publication_parent(Path::new("artifacts/output.heic")),
            Path::new("artifacts")
        );
    }

    #[test]
    fn atomic_publisher_replaces_destination_only_on_commit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "xdremux-runtime-{}-{unique}.heic",
            std::process::id()
        ));
        fs::write(&path, b"old").unwrap();
        let mut publisher = AtomicFilePublisher::new(path.clone());
        let plan = ConversionPlan {
            output: OutputIntent::Standard,
            gain_map_target: xdremux_engine::GainMapEncodeProfile {
                width: 1,
                height: 1,
                channels: GainMapChannels::Mono,
                layout: xdremux_engine::GainMapCodecLayout {
                    chroma: xdremux_format::ChromaSampling::Mono400,
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
            required_capabilities: Vec::new(),
        };
        publisher.publish_artifact(&plan, b"new".to_vec()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        fs::remove_file(path).unwrap();
    }
}
