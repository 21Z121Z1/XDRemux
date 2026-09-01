#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

use atomic_write_file::AtomicWriteFile;
use xdremux_codec::{
    GainMapTileEncodeRequest, JpegRasterDecodeRequest, LibHeifProvider, Raster8,
    RasterPixelFormat, ZuneJpegProvider,
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
use xdremux_format::isobmff::{scan_top_level_boxes, MDAT};
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
    fn new(context: &'static str, detail: impl Into<String>) -> Self {
        Self {
            context,
            detail: detail.into(),
        }
    }

    fn external(context: &'static str, error: impl fmt::Display) -> Self {
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
}

pub fn analyze_proxdr(source: &[u8]) -> Result<PreparedProXdr> {
    let extracted = extract(source)
        .map_err(|error| RuntimeError::external("ProXDR container analysis", error))?;
    let frame = probe_jpeg_frame_profile(&extracted.mask_jpeg_data)
        .map_err(|error| RuntimeError::external("private Gain Map JPEG profile", error))?;
    let gain_map = gain_map_source_profile_from_jpeg(&frame)
        .map_err(|error| RuntimeError::external("Gain Map source profile", error))?;
    let hdr_mode = source_hdr_mode(extracted.mode);
    let scale = resolve(&extracted.meta_floats, hdr_extraction_mode(extracted.mode))
        .map_err(|error| RuntimeError::external("HDR scale resolution", error))?;
    let analysis = ConversionAnalysis {
        source_family: detect_source_family(hdr_mode, &extracted.meta_floats),
        hdr_mode,
        gain_map,
    };
    Ok(PreparedProXdr {
        analysis,
        extracted,
        scale,
    })
}

fn source_hdr_mode(mode: ContainerExtractionMode) -> SourceHdrMode {
    match mode {
        ContainerExtractionMode::Lhdr => SourceHdrMode::Lhdr,
        ContainerExtractionMode::Uhdr => SourceHdrMode::Uhdr,
    }
}

fn hdr_extraction_mode(mode: ContainerExtractionMode) -> HdrExtractionMode {
    match mode {
        ContainerExtractionMode::Lhdr => HdrExtractionMode::Lhdr,
        ContainerExtractionMode::Uhdr => HdrExtractionMode::Uhdr,
    }
}

fn hdr_family(family: SourceFamily) -> HdrFamily {
    match family {
        SourceFamily::X6 => HdrFamily::X6,
        SourceFamily::X7 => HdrFamily::X7,
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
        self.validate_supported_plan(plan)?;

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
        let imageio_tmap = make_apple_tmap_payload(&info_floats)
            .map_err(|error| RuntimeError::external("ISO Gain Map tmap", error))?;
        let tmap_payload = match plan.tmap_format {
            TmapFormat::ImageIo => imageio_tmap,
            TmapFormat::Strict => make_strict_tmap_payload(&imageio_tmap)
                .map_err(|error| RuntimeError::external("strict ISO Gain Map tmap", error))?,
        };
        let xmp_payload = make_hdrgm_xmp(&info_floats)
            .map_err(|error| RuntimeError::external("ISO Gain Map XMP", error))?;

        let body = standard_heif_body(self.source)?;
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

        if plan.oppo_camera_tail == OppoCameraTail::PreserveWithoutPrivateHdr {
            let tail = pack_filtered_oppo_camera_tail(
                self.source,
                &self.prepared.extracted.manifest_info,
                self.prepared.extracted.data_base,
                |entry| !is_oppo_private_hdr_tail_entry(&entry.name),
            )
            .map_err(|error| RuntimeError::external("OPPO camera tail", error))?;
            output.extend_from_slice(&tail);
        }
        Ok(output)
    }
}

impl ProXdrArtifactBuilder<'_> {
    fn validate_supported_plan(&self, plan: &ConversionPlan) -> Result<()> {
        if plan.oppo_compatibility != OppoCompatibility::Off {
            return Err(RuntimeError::new(
                "portable runtime",
                "OPPO-compatible output is not wired into the Rust runtime yet",
            ));
        }
        if plan.effective_input_processing_branch != InputProcessingBranch::Hybrid {
            return Err(RuntimeError::new(
                "portable runtime",
                "the first Rust runtime slice supports the canonical Hybrid path only",
            ));
        }
        if !matches!(
            plan.oppo_camera_tail,
            OppoCameraTail::Off | OppoCameraTail::PreserveWithoutPrivateHdr
        ) {
            return Err(RuntimeError::new(
                "portable runtime",
                "requested OPPO camera-tail policy is not wired into the Rust runtime yet",
            ));
        }
        Ok(())
    }

    fn normalized_gain_map_raster(&self, decoded: Raster8, plan: &ConversionPlan) -> Result<Raster8> {
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
                    width: usize::try_from(decoded.width)
                        .map_err(|_| RuntimeError::new("LHDR reconstruction", "width exceeds usize"))?,
                    height: usize::try_from(decoded.height)
                        .map_err(|_| RuntimeError::new("LHDR reconstruction", "height exceeds usize"))?,
                    bytes_per_row: decoded.bytes_per_row,
                    channel_count: 1,
                    data: decoded.data,
                };
                let (gain_map, _) = reconstruct_gain_map(
                    &mask,
                    hdr_family(plan.effective_family),
                    &self.prepared.scale,
                    &self.prepared.extracted.meta_floats,
                )
                .map_err(|error| RuntimeError::external("LHDR Gain Map reconstruction", error))?;
                Raster8::new(
                    u32::try_from(gain_map.width)
                        .map_err(|_| RuntimeError::new("LHDR reconstruction", "width exceeds u32"))?,
                    u32::try_from(gain_map.height)
                        .map_err(|_| RuntimeError::new("LHDR reconstruction", "height exceeds u32"))?,
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
    let row_bytes = width
        .checked_mul(3)
        .ok_or_else(|| RuntimeError::new("Gain Map channel conformance", "RGB row size overflows"))?;
    let mut data = vec![0_u8; row_bytes.checked_mul(height).ok_or_else(|| {
        RuntimeError::new("Gain Map channel conformance", "RGB raster size overflows")
    })?];
    for y in 0..height {
        let source_row = y
            .checked_mul(raster.bytes_per_row)
            .ok_or_else(|| RuntimeError::new("Gain Map channel conformance", "source row overflows"))?;
        let output_row = y
            .checked_mul(row_bytes)
            .ok_or_else(|| RuntimeError::new("Gain Map channel conformance", "output row overflows"))?;
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

fn standard_heif_body(source: &[u8]) -> Result<&[u8]> {
    let top = scan_top_level_boxes(source)
        .map_err(|error| RuntimeError::external("source HEIF scan", error))?;
    let mut mdats = top.boxes.iter().filter(|header| header.kind == MDAT);
    let mdat = mdats
        .next()
        .ok_or_else(|| RuntimeError::new("source HEIF scan", "source mdat is missing"))?;
    if mdats.next().is_some() {
        return Err(RuntimeError::new(
            "source HEIF scan",
            "multiple top-level mdat boxes are not supported by the canonical runtime path",
        ));
    }
    source
        .get(..mdat.data_end)
        .ok_or_else(|| RuntimeError::new("source HEIF scan", "source mdat end exceeds input"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use xdremux_format::isobmff::make_box;
    use xdremux_format::FourCC;

    #[test]
    fn standard_heif_body_drops_post_mdat_vendor_tail() {
        let mut source = make_box(FourCC::new(*b"ftyp"), b"heic").unwrap();
        source.extend_from_slice(&make_box(MDAT, b"base").unwrap());
        let body_len = source.len();
        source.extend_from_slice(b"vendor-tail");
        assert_eq!(standard_heif_body(&source).unwrap().len(), body_len);
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
            effective_family: SourceFamily::X7,
            requested_input_processing_branch: InputProcessingBranch::Hybrid,
            effective_input_processing_branch: InputProcessingBranch::Hybrid,
            base_strategy: xdremux_engine::BaseStrategy::PreserveCompressed,
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
            container_writer: xdremux_engine::ContainerWriter::Rust,
            oppo_compatibility: OppoCompatibility::Off,
            oppo_camera_tail: OppoCameraTail::Off,
            tmap_format: TmapFormat::ImageIo,
            required_capabilities: Vec::new(),
        };
        publisher
            .publish_artifact(&plan, b"new".to_vec())
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        fs::remove_file(path).unwrap();
    }
}
