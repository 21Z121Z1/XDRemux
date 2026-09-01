#![forbid(unsafe_code)]

mod batch;
mod batch_checkpoint;
mod categorize;
mod live_photo;
mod validation;

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
pub use validation::{
    validate_media_file, IsoHdrValidationReport, LivePhotoValidationReport, ValidationReport,
};

use std::error::Error;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use atomic_write_file::AtomicWriteFile;
use xdremux_codec::{
    GainMapTileEncodeRequest, JpegRasterDecodeRequest, LibHeifProvider, Raster8, RasterPixelFormat,
    ZuneJpegProvider,
};
use xdremux_container::{
    extract, is_oppo_private_hdr_tail_entry, pack_filtered_oppo_camera_tail, ExtractedLhdr,
    ExtractionMode as ContainerExtractionMode,
};
use xdremux_engine::{
    detect_source_family, execute_conversion, gain_map_source_profile_from_jpeg, ArtifactBuilder,
    ArtifactPublisher, ArtifactValidator, CapabilityInventory, ConversionAnalysis, ConversionPlan,
    ConversionRequest, ExecutionError, ExecutionStage, GainMapChannels, GainMapTileEncoder,
    InputProcessingBranch, OperationCapability, OppoCameraTail, OppoCompatibility, RasterDecoder,
    SourceFamily, SourceHdrMode, TmapFormat,
};
use xdremux_format::probe_jpeg_frame_profile;
use xdremux_hdr::{
    make_private_gain_map_info_floats, reconstruct_gain_map, resolve,
    ExtractionMode as HdrExtractionMode, Family as HdrFamily, GainMapRaster, ResolvedScale,
};
use xdremux_heif::{
    assemble_iso_gain_map_heif, validate_gain_map_structure, DirectHevcGainMap,
    GainMapChannels as HeifGainMapChannels, GainMapEncodeProfile as HeifGainMapEncodeProfile,
    GainMapTile, IsoGainMapAssembly,
};
use xdremux_metadata::{make_apple_tmap_payload, make_hdrgm_xmp, make_strict_tmap_payload};

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
            bytes: publisher.output.ok_or_else(|| {
                RuntimeError::new("runtime execution", "publisher produced no output")
            })?,
        })
    }

    pub fn convert_proxdr_file<Observe>(
        &self,
        source: &[u8],
        output: &Path,
        request: ConversionRequest,
        observe: Observe,
    ) -> Result<FileConversionReceipt>
    where
        Observe: FnMut(ExecutionStage),
    {
        let converted = self.convert_proxdr_bytes(source, request, observe)?;
        let parent = output.parent().unwrap_or_else(|| Path::new("."));
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|error| RuntimeError::external("output directory", error))?;
        }
        let mut file = AtomicWriteFile::options()
            .open(output)
            .map_err(|error| RuntimeError::external("output open", error))?;
        file.write_all(&converted.bytes)
            .map_err(|error| RuntimeError::external("output write", error))?;
        file.commit()
            .map_err(|error| RuntimeError::external("output commit", error))?;
        Ok(FileConversionReceipt {
            plan: converted.plan,
            output: output.to_path_buf(),
        })
    }

    pub fn convert_motion_photo_file(
        &self,
        source: &[u8],
        input: &Path,
        output_image: &Path,
    ) -> Result<LivePhotoFileReceipt> {
        live_photo::convert_motion_photo_file(&self.jpeg, &self.heif, source, input, output_image)
    }

    pub fn categorize_paths<I>(
        &self,
        inputs: I,
        output_root: &Path,
        operation: xdremux_classification::CategorizeOperation,
        dry_run: bool,
    ) -> CategorizeReceipt
    where
        I: IntoIterator<Item = PathBuf>,
    {
        categorize::categorize_paths(inputs, output_root, operation, dry_run)
    }
}

fn runtime_execution_error(error: ExecutionError) -> RuntimeError {
    RuntimeError::new("runtime execution", error.to_string())
}

fn analyze_proxdr(source: &[u8]) -> Result<PreparedProXdr> {
    let extracted = extract(source, ContainerExtractionMode::Lenient)
        .map_err(|error| RuntimeError::external("OPPO container extraction", error))?;
    let hdr = xdremux_hdr::extract(source, HdrExtractionMode::Lenient)
        .map_err(|error| RuntimeError::external("HDR extraction", error))?;
    let scale = resolve(hdr.metadata.as_ref())
        .map_err(|error| RuntimeError::external("HDR metadata resolution", error))?;
    let source_profile = gain_map_source_profile_from_jpeg(&extracted.gain_map_jpeg)
        .map_err(|error| RuntimeError::external("gain-map source profile", error))?;
    let source_hdr_mode = match hdr.family {
        HdrFamily::Lhdr1 => SourceHdrMode::Lhdr1,
        HdrFamily::Lhdr2 => SourceHdrMode::Lhdr2,
        HdrFamily::Uhdr => SourceHdrMode::Uhdr,
    };
    let analysis = ConversionAnalysis {
        source_family: detect_source_family(source),
        source_hdr_mode,
        source_gain_map: Some(source_profile),
        source_tail: Some(OppoCameraTail {
            available: extracted.oppo_tail.is_some(),
            mode: SourceHdrMode::from(source_hdr_mode),
        }),
    };
    Ok(PreparedProXdr {
        analysis,
        extracted,
        scale,
    })
}

struct ProXdrArtifactBuilder<'a> {
    source: &'a [u8],
    prepared: &'a PreparedProXdr,
    jpeg: &'a ZuneJpegProvider,
    heif: &'a LibHeifProvider,
}

impl ArtifactBuilder for ProXdrArtifactBuilder<'_> {
    type Artifact = Vec<u8>;

    fn build(
        &mut self,
        plan: &ConversionPlan,
        _stage: ExecutionStage,
    ) -> std::result::Result<Self::Artifact, ExecutionError> {
        if plan.request.oppo_compatibility == OppoCompatibility::Enabled {
            return Err(ExecutionError::unsupported(
                "Rust runtime does not yet write OPPO-compatible output",
            ));
        }
        if plan.input_processing_branch != InputProcessingBranch::Hybrid {
            return Err(ExecutionError::unsupported(
                "initial Rust runtime slice only implements the canonical hybrid processing branch",
            ));
        }
        let gain_map = reconstruct_gain_map(
            GainMapRaster::Rgb8 {
                width: self.prepared.extracted.width,
                height: self.prepared.extracted.height,
                rgb: self.prepared.extracted.gain_map_rgb.clone(),
            },
            self.prepared.scale,
        )
        .map_err(|error| ExecutionError::build(error.to_string()))?;
        let encoded = self
            .jpeg
            .encode_gain_map(&GainMapTileEncodeRequest {
                raster: Raster8 {
                    width: gain_map.width,
                    height: gain_map.height,
                    channels: 3,
                    bytes: gain_map.rgb,
                },
                profile: plan.output_profile,
            })
            .map_err(|error| ExecutionError::build(error.to_string()))?;
        let base = self
            .heif
            .encode_primary_heif(self.source)
            .map_err(|error| ExecutionError::build(error.to_string()))?;
        let tmap = match plan.output_tmap_format {
            TmapFormat::AppleImageIoNative => make_apple_tmap_payload(&self.prepared.scale),
            TmapFormat::StrictIso => make_strict_tmap_payload(&self.prepared.scale),
        };
        let xmp = make_hdrgm_xmp(&self.prepared.scale);
        let output = assemble_iso_gain_map_heif(&IsoGainMapAssembly {
            primary_heif: base,
            gain_map: DirectHevcGainMap {
                width: encoded.width,
                height: encoded.height,
                rows: encoded.rows,
                columns: encoded.columns,
                channels: match encoded.channels {
                    GainMapChannels::Mono => HeifGainMapChannels::Mono,
                    GainMapChannels::Rgb => HeifGainMapChannels::Rgb,
                },
                profile: HeifGainMapEncodeProfile {
                    chroma: encoded.layout.chroma,
                    luma_bit_depth: encoded.layout.luma_bit_depth,
                    chroma_bit_depth: encoded.layout.chroma_bit_depth,
                },
                tiles: encoded
                    .tiles
                    .iter()
                    .map(|tile| GainMapTile {
                        width: tile.width,
                        height: tile.height,
                        hevc: tile.hevc.clone(),
                        decoder_config: tile.decoder_config.clone(),
                    })
                    .collect(),
            },
            tmap_payload: tmap,
            hdrgm_xmp: xmp,
        })
        .map_err(|error| ExecutionError::build(error.to_string()))?;

        if let Some(tail) = self.prepared.extracted.oppo_tail.as_ref() {
            if plan.request.source_tail_policy == xdremux_engine::SourceTailPolicy::CameraMetadata {
                let filtered = pack_filtered_oppo_camera_tail(tail, is_oppo_private_hdr_tail_entry)
                    .map_err(|error| ExecutionError::build(error.to_string()))?;
                let mut result = output;
                result.extend_from_slice(&filtered);
                return Ok(result);
            }
        }
        Ok(output)
    }
}

struct IsoGainMapValidator;

impl ArtifactValidator<Vec<u8>> for IsoGainMapValidator {
    fn validate(
        &mut self,
        artifact: &Vec<u8>,
        _plan: &ConversionPlan,
        _stage: ExecutionStage,
    ) -> std::result::Result<(), ExecutionError> {
        validate_gain_map_structure(artifact)
            .map(|_| ())
            .map_err(|error| ExecutionError::validate(error.to_string()))
    }
}

#[derive(Default)]
struct MemoryPublisher {
    output: Option<Vec<u8>>,
}

impl ArtifactPublisher<Vec<u8>> for MemoryPublisher {
    fn publish(
        &mut self,
        artifact: Vec<u8>,
        _plan: &ConversionPlan,
        _stage: ExecutionStage,
    ) -> std::result::Result<(), ExecutionError> {
        self.output = Some(artifact);
        Ok(())
    }
}
