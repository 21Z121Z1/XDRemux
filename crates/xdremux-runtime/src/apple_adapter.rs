use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::{mpsc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use xdremux_engine::{
    apple_style_scene_scores_from_vision_observations, AppleAuxiliaryKind, AppleAuxiliaryPayload,
    AppleGainMapFacts, AppleImageAuxiliaryFacts, AppleL8Mask, AppleMetadataValue,
    AppleSemanticRole, AppleStyleSceneScores, AppleVisionClassificationObservation,
    OperationCapability,
};
use xdremux_format::FourCC;

use crate::{Result, RuntimeError};

const APPLE_ADAPTER_SCHEMA_VERSION: u32 = 2;
const APPLE_ADAPTER_PERSISTENT_ARGUMENT: &str = "--persistent-json-lines";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const APPLE_COMPUTE_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_APPLE_ADAPTER_FRAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_APPLE_ADAPTER_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const MAX_APPLE_L8_MASK_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AppleImageProperties {
    pub width: u32,
    pub height: u32,
    pub orientation: Option<u32>,
    pub focal_length_mm: Option<f64>,
    pub focal_length_in_35mm_film: Option<f64>,
    pub digital_zoom_ratio: Option<f64>,
    pub lens_model: Option<String>,
    pub f_number: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleStylePropertiesFacts {
    pub framework_loaded: bool,
    pub class_available: bool,
    pub parse_succeeded: bool,
    pub style_data_length: usize,
    pub readback_written: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppleVideoToolboxMain10Facts {
    pub width: u32,
    pub height: u32,
    pub annex_b_length: usize,
    pub hvcc_length: usize,
}

pub(super) struct AppleVideoToolboxMain10Encode<'a> {
    pub input: &'a Path,
    pub output_annex_b: &'a Path,
    pub output_hvcc: &'a Path,
    pub width: u32,
    pub height: u32,
    pub bytes_per_row: u32,
    pub quality: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppleAdapterCapabilities {
    photographic_styles: bool,
    portrait: bool,
}

impl AppleAdapterCapabilities {
    pub(super) fn operation_capabilities(&self) -> Vec<OperationCapability> {
        let mut capabilities = Vec::with_capacity(2);
        if self.photographic_styles {
            capabilities.push(OperationCapability::PhotographicStylesAdapter);
        }
        if self.portrait {
            capabilities.push(OperationCapability::PortraitAdapter);
        }
        capabilities
    }
}

pub(super) struct AppleAdapterClient {
    executable: PathBuf,
    timeout: Duration,
    session: Mutex<Option<AppleAdapterSession>>,
}

enum AdapterStreamEvent {
    Frame(Vec<u8>),
    Error(io::Error),
    Eof,
}

struct AppleAdapterSession {
    child: Child,
    stdin: Option<ChildStdin>,
    responses: mpsc::Receiver<AdapterStreamEvent>,
    stdout_reader: Option<JoinHandle<()>>,
    stderr_reader: Option<JoinHandle<Vec<u8>>>,
}

impl AppleAdapterSession {
    fn spawn(executable: &Path) -> Result<Self> {
        let mut child = Command::new(executable)
            .arg(APPLE_ADAPTER_PERSISTENT_ARGUMENT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| RuntimeError::external("Apple adapter launch", error))?;

        let pipes = (child.stdin.take(), child.stdout.take(), child.stderr.take());
        let (stdin, stdout, stderr) = match pipes {
            (Some(stdin), Some(stdout), Some(stderr)) => (stdin, stdout, stderr),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RuntimeError::new(
                    "Apple adapter launch",
                    "persistent helper pipes are unavailable",
                ));
            }
        };

        let (sender, responses) = mpsc::channel();
        let stdout_reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_bounded_adapter_frame(&mut reader) {
                    Ok(Some(frame)) => {
                        if sender.send(AdapterStreamEvent::Frame(frame)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = sender.send(AdapterStreamEvent::Eof);
                        break;
                    }
                    Err(error) => {
                        let _ = sender.send(AdapterStreamEvent::Error(error));
                        break;
                    }
                }
            }
        });
        let stderr_reader = thread::spawn(move || read_bounded_diagnostic(stderr));

        Ok(Self {
            child,
            stdin: Some(stdin),
            responses,
            stdout_reader: Some(stdout_reader),
            stderr_reader: Some(stderr_reader),
        })
    }

    fn is_running(&mut self) -> Result<bool> {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|error| RuntimeError::external("Apple adapter wait", error))
    }

    fn invoke(&mut self, executable: &Path, request: &[u8], timeout: Duration) -> Result<Vec<u8>> {
        if let Err(error) = self
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "helper stdin is unavailable"))
            .and_then(|stdin| {
                stdin.write_all(request)?;
                stdin.write_all(b"\n")?;
                stdin.flush()
            })
        {
            return Err(self.execution_error(format!("request write failed: {error}")));
        }

        match self.responses.recv_timeout(timeout) {
            Ok(AdapterStreamEvent::Frame(frame)) if !frame.is_empty() => Ok(frame),
            Ok(AdapterStreamEvent::Frame(_)) => {
                Err(self.execution_error("adapter returned an empty response frame".to_owned()))
            }
            Ok(AdapterStreamEvent::Error(error)) => {
                Err(self.execution_error(format!("adapter stdout framing failed: {error}")))
            }
            Ok(AdapterStreamEvent::Eof) => Err(self
                .execution_error("adapter closed stdout before returning a response".to_owned())),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.abort();
                Err(RuntimeError::new(
                    "Apple adapter timeout",
                    format!(
                        "{} exceeded {} ms",
                        executable.display(),
                        timeout.as_millis()
                    ),
                ))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(self.execution_error("adapter response channel disconnected".to_owned()))
            }
        }
    }

    fn execution_error(&mut self, fallback: String) -> RuntimeError {
        let (status, stderr) = self.abort();
        let diagnostic = String::from_utf8_lossy(&stderr).trim().to_owned();
        let mut detail = fallback;
        if let Some(status) = status {
            detail.push_str(&format!("; adapter exited with {status}"));
        }
        if !diagnostic.is_empty() {
            detail.push_str(": ");
            detail.push_str(&diagnostic);
        }
        RuntimeError::new("Apple adapter execution", detail)
    }

    fn abort(&mut self) -> (Option<ExitStatus>, Vec<u8>) {
        self.stdin.take();
        let status = match self.child.try_wait() {
            Ok(Some(status)) => Some(status),
            Ok(None) | Err(_) => {
                let _ = self.child.kill();
                self.child.wait().ok()
            }
        };
        if let Some(reader) = self.stdout_reader.take() {
            let _ = reader.join();
        }
        let stderr = self
            .stderr_reader
            .take()
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        (status, stderr)
    }
}

impl Drop for AppleAdapterSession {
    fn drop(&mut self) {
        self.abort();
    }
}

impl AppleAdapterClient {
    pub(super) fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            timeout: DEFAULT_TIMEOUT,
            session: Mutex::new(None),
        }
    }
    pub(super) fn capabilities(&self) -> Result<AppleAdapterCapabilities> {
        let output = self.invoke_request(AdapterRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "capabilities".to_owned(),
            input_path: None,
            output_path: None,
            roles: None,
            orientation: None,
            metadata_source_path: None,
            lossy_quality: None,
        })?;
        let response: CapabilitiesResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;

        let mut photographic_styles = false;
        let mut portrait = false;
        for capability in response.capabilities {
            match capability.as_str() {
                "photographic-styles" => photographic_styles = true,
                "portrait" => portrait = true,
                other => {
                    return Err(RuntimeError::new(
                        "Apple adapter protocol",
                        format!("unknown capability {other:?}"),
                    ));
                }
            }
        }
        Ok(AppleAdapterCapabilities {
            photographic_styles,
            portrait,
        })
    }

    pub(super) fn supports_photographic_styles(&self) -> Result<bool> {
        Ok(self.capabilities()?.photographic_styles)
    }

    pub(super) fn imageio_auxiliary_facts(&self, input: &Path) -> Result<AppleImageAuxiliaryFacts> {
        let output = self.invoke_request(AdapterRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "imageio-auxiliary-facts".to_owned(),
            input_path: Some(input_path(input)?),
            output_path: None,
            roles: None,
            orientation: None,
            metadata_source_path: None,
            lossy_quality: None,
        })?;
        let response: AuxiliaryResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;
        Ok(response.auxiliary.into())
    }

    pub(super) fn imageio_gain_map_facts(&self, input: &Path) -> Result<AppleGainMapFacts> {
        let output = self.invoke_request(AdapterRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "imageio-gain-map-facts".to_owned(),
            input_path: Some(input_path(input)?),
            output_path: None,
            roles: None,
            orientation: None,
            metadata_source_path: None,
            lossy_quality: None,
        })?;
        let response: GainMapResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;
        Ok(AppleGainMapFacts {
            pixel_format: FourCC::new(response.gain_map.pixel_format.to_be_bytes()),
            width: response.gain_map.width,
            height: response.gain_map.height,
        })
    }

    pub(super) fn imageio_image_properties(&self, input: &Path) -> Result<AppleImageProperties> {
        let output = self.invoke_request(AdapterRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "imageio-image-properties".to_owned(),
            input_path: Some(input_path(input)?),
            output_path: None,
            roles: None,
            orientation: None,
            metadata_source_path: None,
            lossy_quality: None,
        })?;
        let response: ImagePropertiesResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;
        if response.image_properties.width == 0 || response.image_properties.height == 0 {
            return Err(RuntimeError::new(
                "Apple adapter protocol",
                "ImageIO image properties contain zero geometry",
            ));
        }
        Ok(response.image_properties.into())
    }

    /// Ask the private Apple metadata consumer to parse a Rust-produced Styles
    /// property list and round-trip its key-1 style data. The adapter reports
    /// framework facts; Rust owns the expected bytes and acceptance policy.
    pub(super) fn semantic_style_properties_facts(
        &self,
        metadata: &[u8],
        expected_style_data: &[u8],
    ) -> Result<AppleStylePropertiesFacts> {
        if metadata.is_empty() || expected_style_data.is_empty() {
            return Err(RuntimeError::new(
                "Apple semantic Styles validation",
                "metadata and expected style data must be non-empty",
            ));
        }
        let sidecars = tempfile::tempdir().map_err(|error| {
            RuntimeError::external("Apple semantic Styles sidecar directory", error)
        })?;
        let metadata_path = sidecars.path().join("style-metadata.bplist");
        let readback_path = sidecars.path().join("style-data-readback.bin");
        fs::write(&metadata_path, metadata)
            .map_err(|error| RuntimeError::external("Apple semantic Styles metadata", error))?;
        let output = self.invoke_request(AdapterRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "semantic-style-properties-facts".to_owned(),
            input_path: Some(input_path(&metadata_path)?),
            output_path: Some(input_path(&readback_path)?),
            roles: None,
            orientation: None,
            metadata_source_path: None,
            lossy_quality: None,
        })?;
        let response: StylePropertiesResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;
        let facts: AppleStylePropertiesFacts = response.style_properties.into();
        if !facts.framework_loaded || !facts.class_available || !facts.parse_succeeded {
            return Err(RuntimeError::new(
                "Apple semantic Styles validation",
                format!("private Apple consumer did not accept Styles metadata: {facts:?}"),
            ));
        }
        if facts.style_data_length != expected_style_data.len() || !facts.readback_written {
            return Err(RuntimeError::new(
                "Apple semantic Styles validation",
                format!("private Apple consumer returned incomplete style data: {facts:?}"),
            ));
        }
        let readback = fs::read(&readback_path)
            .map_err(|error| RuntimeError::external("Apple semantic Styles readback", error))?;
        if readback != expected_style_data {
            return Err(RuntimeError::new(
                "Apple semantic Styles validation",
                "private Apple consumer changed Rust-owned key-1 style data",
            ));
        }
        Ok(facts)
    }

    /// Return raw Vision classification facts and derive the four XDRemux scene
    /// confidence buckets in Rust. Vision identifiers are platform output;
    /// identifier grouping, thresholds, priority, and scene type are product policy.
    pub(super) fn vision_scene_scores(&self, input: &Path) -> Result<AppleStyleSceneScores> {
        let output = self.invoke_request_with_timeout(
            AdapterRequest {
                schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
                operation: "vision-scene-classification".to_owned(),
                input_path: Some(input_path(input)?),
                output_path: None,
                roles: None,
                orientation: None,
                metadata_source_path: None,
                lossy_quality: None,
            },
            APPLE_COMPUTE_TIMEOUT,
        )?;
        let response: SceneClassificationResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;
        scene_scores_from_wire(response.scene_classification)
    }

    /// Ask VideoToolbox for the one Apple-specific codec primitive required by
    /// the Styles graph. Raster ownership, resource selection, and HEIF graph
    /// assembly remain in Rust; this operation only writes the framework's
    /// Annex-B and hvcC outputs to caller-provided staging paths.
    pub(super) fn videotoolbox_encode_main10(
        &self,
        encode: &AppleVideoToolboxMain10Encode<'_>,
    ) -> Result<AppleVideoToolboxMain10Facts> {
        if encode.width == 0 || encode.height == 0 {
            return Err(RuntimeError::new(
                "Apple VideoToolbox Main10 encode",
                "raster geometry must be non-zero",
            ));
        }
        let minimum_row = encode.width.checked_mul(3).ok_or_else(|| {
            RuntimeError::new(
                "Apple VideoToolbox Main10 encode",
                "RGB8 row geometry overflows",
            )
        })?;
        if encode.bytes_per_row < minimum_row {
            return Err(RuntimeError::new(
                "Apple VideoToolbox Main10 encode",
                "RGB8 row is shorter than width * 3",
            ));
        }
        if !encode.quality.is_finite() || !(0.0..=1.0).contains(&encode.quality) {
            return Err(RuntimeError::new(
                "Apple VideoToolbox Main10 encode",
                "quality must be finite and within 0 through 1",
            ));
        }
        let wire_request = VideoToolboxMain10Request {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "videotoolbox-encode-main10",
            input_path: input_path(encode.input)?,
            output_path: input_path(encode.output_annex_b)?,
            video_toolbox_main10: VideoToolboxMain10Wire {
                raw_width: encode.width,
                raw_height: encode.height,
                raw_bytes_per_row: encode.bytes_per_row,
                quality: encode.quality,
                hvcc_path: input_path(encode.output_hvcc)?,
            },
        };
        let request = serde_json::to_vec(&wire_request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        let output = self.invoke(&request, APPLE_COMPUTE_TIMEOUT)?;
        let response: VideoToolboxMain10Response = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;
        let facts = AppleVideoToolboxMain10Facts {
            width: response.video_toolbox_main10.width,
            height: response.video_toolbox_main10.height,
            annex_b_length: response.video_toolbox_main10.annex_b_length,
            hvcc_length: response.video_toolbox_main10.hvcc_length,
        };
        if facts.width != encode.width
            || facts.height != encode.height
            || facts.annex_b_length == 0
            || facts.hvcc_length == 0
        {
            return Err(RuntimeError::new(
                "Apple VideoToolbox Main10 encode",
                format!("adapter returned invalid output facts: {facts:?}"),
            ));
        }
        Ok(facts)
    }

    pub(super) fn imageio_write_auxiliary(
        &self,
        input: &Path,
        output: &Path,
        payloads: &[AppleAuxiliaryPayload],
    ) -> Result<()> {
        if payloads.is_empty() {
            return Err(RuntimeError::new(
                "Apple ImageIO auxiliary write",
                "auxiliary payload set is empty",
            ));
        }

        let sidecars = tempfile::tempdir().map_err(|error| {
            RuntimeError::external("Apple ImageIO auxiliary sidecar directory", error)
        })?;
        let mut wire_payloads = Vec::with_capacity(payloads.len());
        for (index, payload) in payloads.iter().enumerate() {
            let sidecar = sidecars.path().join(format!("auxiliary-{index}.bin"));
            fs::write(&sidecar, &payload.data).map_err(|error| {
                RuntimeError::external("Apple ImageIO auxiliary sidecar write", error)
            })?;
            wire_payloads.push(auxiliary_payload_wire(payload, &sidecar)?);
        }

        let request = WriteAuxiliaryRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "imageio-write-auxiliary",
            input_path: input_path(input)?,
            output_path: input_path(output)?,
            auxiliary_payloads: wire_payloads,
        };
        let request = serde_json::to_vec(&request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        let output = self.invoke(&request, self.timeout)?;
        let response: AckResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)
    }

    /// Ask ImageIO to encode an adjacent base/Gain Map source image as HEIF.
    ///
    /// The Rust runtime owns source selection, metadata policy, and quality.
    /// The adapter only performs the framework operation that preserves the
    /// source Gain Map while creating an HEIF carrier.
    pub(super) fn imageio_encode_source_image(
        &self,
        source_image: &Path,
        output: &Path,
        lossy_quality: f64,
    ) -> Result<()> {
        if !lossy_quality.is_finite() || !(0.0..=1.0).contains(&lossy_quality) {
            return Err(RuntimeError::new(
                "Apple ImageIO source-image encode",
                "lossy quality must be finite and within 0 through 1",
            ));
        }
        let request = EncodeSourceImageRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "imageio-encode-source-image",
            input_path: input_path(source_image)?,
            output_path: input_path(output)?,
            lossy_quality,
        };
        let request = serde_json::to_vec(&request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        let output = self.invoke(&request, self.timeout)?;
        let response: AckResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)
    }

    pub(super) fn imageio_merge_metadata(
        &self,
        input: &Path,
        metadata_source: &Path,
        output: &Path,
    ) -> Result<()> {
        let request = MergeMetadataRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "imageio-merge-metadata",
            input_path: input_path(input)?,
            output_path: input_path(output)?,
            metadata_source_path: input_path(metadata_source)?,
        };
        self.invoke_ack(request)
    }

    pub(super) fn imageio_merge_xmp_metadata(
        &self,
        input: &Path,
        xmp: &[u8],
        output: &Path,
    ) -> Result<()> {
        let sidecar = tempfile::tempdir()
            .map_err(|error| RuntimeError::external("Apple ImageIO XMP sidecar", error))?;
        let xmp_path = sidecar.path().join("primary-metadata.xmp");
        fs::write(&xmp_path, xmp)
            .map_err(|error| RuntimeError::external("Apple ImageIO XMP sidecar write", error))?;
        let request = XmpMergeRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "imageio-merge-xmp",
            input_path: input_path(input)?,
            output_path: input_path(output)?,
            primary_metadata_xmp_path: input_path(&xmp_path)?,
        };
        self.invoke_ack(request)
    }

    fn invoke_ack<T: Serialize>(&self, request: T) -> Result<()> {
        let request = serde_json::to_vec(&request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        let output = self.invoke(&request, self.timeout)?;
        let response: AckResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)
    }

    pub(super) fn coreimage_render_l8(
        &self,
        mask: &AppleL8Mask,
        target_width: u32,
        target_height: u32,
        orientation: u8,
    ) -> Result<AppleL8Mask> {
        let source_bytes =
            checked_l8_byte_count(mask.width, mask.height, "Apple CoreImage L8 render input")?;
        if mask.pixels.len() != source_bytes {
            return Err(RuntimeError::new(
                "Apple CoreImage L8 render input",
                format!(
                    "mask has {} bytes; expected {source_bytes}",
                    mask.pixels.len()
                ),
            ));
        }
        if !(1..=8).contains(&orientation) {
            return Err(RuntimeError::new(
                "Apple CoreImage L8 render",
                format!("orientation {orientation} is outside 1 through 8"),
            ));
        }
        let target_bytes = checked_l8_byte_count(
            target_width,
            target_height,
            "Apple CoreImage L8 render output",
        )?;

        let sidecars = tempfile::tempdir()
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 sidecars", error))?;
        let mask_path = sidecars.path().join("input.l8");
        let output_path = sidecars.path().join("output.l8");
        fs::write(&mask_path, &mask.pixels)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 input write", error))?;

        let request = CoreImageRenderL8Request {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "coreimage-render-l8",
            output_path: input_path(&output_path)?,
            render_l8: RenderL8Wire {
                mask_path: input_path(&mask_path)?,
                source_width: mask.width,
                source_height: mask.height,
                target_width,
                target_height,
                orientation,
            },
        };
        let request = serde_json::to_vec(&request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        let output = self.invoke(&request, APPLE_COMPUTE_TIMEOUT)?;
        let response: AckResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;

        let metadata = fs::metadata(&output_path)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 output metadata", error))?;
        if metadata.len() != u64::try_from(target_bytes).unwrap_or(u64::MAX) {
            return Err(RuntimeError::new(
                "Apple CoreImage L8 render output",
                format!(
                    "sidecar has {} bytes; expected {target_bytes}",
                    metadata.len()
                ),
            ));
        }
        let pixels = fs::read(&output_path)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 output read", error))?;
        AppleL8Mask::new(target_width, target_height, pixels)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 render output", error))
    }

    pub(super) fn coreimage_edge_preserve_upsample_l8(
        &self,
        guide: &Path,
        small_mask: &AppleL8Mask,
        target_width: u32,
        target_height: u32,
        spatial_sigma: f32,
        luma_sigma: f32,
    ) -> Result<AppleL8Mask> {
        let small_bytes = checked_l8_byte_count(
            small_mask.width,
            small_mask.height,
            "Apple CoreImage L8 input",
        )?;
        if small_mask.pixels.len() != small_bytes {
            return Err(RuntimeError::new(
                "Apple CoreImage L8 input",
                format!(
                    "mask has {} bytes; expected {small_bytes}",
                    small_mask.pixels.len()
                ),
            ));
        }
        let target_bytes =
            checked_l8_byte_count(target_width, target_height, "Apple CoreImage L8 output")?;
        if !spatial_sigma.is_finite()
            || spatial_sigma <= 0.0
            || !luma_sigma.is_finite()
            || luma_sigma <= 0.0
        {
            return Err(RuntimeError::new(
                "Apple CoreImage L8 upsample",
                "spatial and luma sigma must be finite and positive",
            ));
        }

        let sidecars = tempfile::tempdir()
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 sidecars", error))?;
        let small_path = sidecars.path().join("small.l8");
        let output_path = sidecars.path().join("output.l8");
        fs::write(&small_path, &small_mask.pixels)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 input write", error))?;

        let request = CoreImageEdgePreserveUpsampleRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "coreimage-edge-preserve-upsample-l8",
            input_path: input_path(guide)?,
            output_path: input_path(&output_path)?,
            edge_preserve_upsample: EdgePreserveUpsampleWire {
                small_mask_path: input_path(&small_path)?,
                small_width: small_mask.width,
                small_height: small_mask.height,
                target_width,
                target_height,
                spatial_sigma,
                luma_sigma,
            },
        };
        let request = serde_json::to_vec(&request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        let output = self.invoke(&request, APPLE_COMPUTE_TIMEOUT)?;
        let response: AckResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;

        let metadata = fs::metadata(&output_path)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 output metadata", error))?;
        if metadata.len() != u64::try_from(target_bytes).unwrap_or(u64::MAX) {
            return Err(RuntimeError::new(
                "Apple CoreImage L8 output",
                format!(
                    "sidecar has {} bytes; expected {target_bytes}",
                    metadata.len()
                ),
            ));
        }
        let pixels = fs::read(&output_path)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 output read", error))?;
        AppleL8Mask::new(target_width, target_height, pixels)
            .map_err(|error| RuntimeError::external("Apple CoreImage L8 output", error))
    }

    pub(super) fn vision_semantic_mattes(
        &self,
        input: &Path,
        roles: &[AppleSemanticRole],
        orientation: Option<u32>,
    ) -> Result<BTreeMap<AppleSemanticRole, AppleL8Mask>> {
        if roles.is_empty() {
            return Err(RuntimeError::new(
                "Apple Vision semantic mattes",
                "requested role set is empty",
            ));
        }
        let expected = roles.iter().copied().collect::<BTreeSet<_>>();
        if expected.len() != roles.len() {
            return Err(RuntimeError::new(
                "Apple Vision semantic mattes",
                "requested role set contains duplicates",
            ));
        }

        let output_directory = tempfile::tempdir()
            .map_err(|error| RuntimeError::external("Apple Vision temporary directory", error))?;
        let output = self.invoke_request_with_timeout(
            AdapterRequest {
                schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
                operation: "vision-semantic-mattes".to_owned(),
                input_path: Some(input_path(input)?),
                output_path: Some(input_path(output_directory.path())?),
                roles: Some(
                    roles
                        .iter()
                        .map(|role| semantic_role_wire(*role).to_owned())
                        .collect(),
                ),
                orientation,
                metadata_source_path: None,
                lossy_quality: None,
            },
            APPLE_COMPUTE_TIMEOUT,
        )?;
        let response: SemanticResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;

        let mut masks = BTreeMap::new();
        for wire in response.semantic_masks {
            let role = parse_semantic_role(&wire.role)?;
            if !expected.contains(&role) {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!("adapter returned unrequested role {:?}", role),
                ));
            }
            if FourCC::new(wire.pixel_format.to_be_bytes()) != FourCC::new(*b"L008") {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!("role {:?} is not L008", role),
                ));
            }
            if wire.width == 0 || wire.height == 0 {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!("role {:?} has zero geometry", role),
                ));
            }
            let expected_bytes = usize::try_from(wire.width)
                .ok()
                .and_then(|width| {
                    usize::try_from(wire.height)
                        .ok()
                        .and_then(|height| width.checked_mul(height))
                })
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Vision semantic mattes",
                        format!("role {:?} geometry overflows", role),
                    )
                })?;
            if expected_bytes > MAX_APPLE_L8_MASK_BYTES {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!("role {:?} exceeds semantic mask safety limit", role),
                ));
            }
            let path = output_directory
                .path()
                .join(format!("{}.l8", semantic_role_wire(role)));
            let metadata = fs::metadata(&path).map_err(|error| {
                RuntimeError::external("Apple Vision semantic mask metadata", error)
            })?;
            if metadata.len() != u64::try_from(expected_bytes).unwrap_or(u64::MAX) {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!(
                        "role {:?} has {} bytes; expected {}",
                        role,
                        metadata.len(),
                        expected_bytes
                    ),
                ));
            }
            let pixels = fs::read(&path).map_err(|error| {
                RuntimeError::external("Apple Vision semantic mask read", error)
            })?;
            let mask = AppleL8Mask::new(wire.width, wire.height, pixels)
                .map_err(|error| RuntimeError::external("Apple Vision semantic mask", error))?;
            if masks.insert(role, mask).is_some() {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!("adapter returned duplicate role {:?}", role),
                ));
            }
        }

        let observed = masks.keys().copied().collect::<BTreeSet<_>>();
        if observed != expected {
            let missing = expected.difference(&observed).copied().collect::<Vec<_>>();
            return Err(RuntimeError::new(
                "Apple Vision semantic mattes",
                format!("adapter omitted required roles: {missing:?}"),
            ));
        }
        Ok(masks)
    }

    fn invoke_request(&self, request: AdapterRequest) -> Result<Vec<u8>> {
        self.invoke_request_with_timeout(request, self.timeout)
    }

    fn invoke_request_with_timeout(
        &self,
        request: AdapterRequest,
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let request = serde_json::to_vec(&request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        self.invoke(&request, timeout)
    }

    fn invoke(&self, request: &[u8], timeout: Duration) -> Result<Vec<u8>> {
        if request.is_empty() {
            return Err(RuntimeError::new(
                "Apple adapter protocol",
                "request frame is empty",
            ));
        }
        if request.len() > MAX_APPLE_ADAPTER_FRAME_BYTES {
            return Err(RuntimeError::new(
                "Apple adapter protocol",
                format!(
                    "request frame has {} bytes; maximum is {}",
                    request.len(),
                    MAX_APPLE_ADAPTER_FRAME_BYTES
                ),
            ));
        }
        if request.iter().any(|byte| matches!(*byte, b'\n' | b'\r')) {
            return Err(RuntimeError::new(
                "Apple adapter protocol",
                "request frame contains a raw line delimiter",
            ));
        }

        let mut session = self.session.lock().map_err(|_| {
            RuntimeError::new(
                "Apple adapter session",
                "persistent session mutex is poisoned",
            )
        })?;
        let needs_restart = if let Some(current) = session.as_mut() {
            match current.is_running() {
                Ok(running) => !running,
                Err(error) => {
                    session.take();
                    return Err(error);
                }
            }
        } else {
            true
        };
        if needs_restart {
            session.take();
            *session = Some(AppleAdapterSession::spawn(&self.executable)?);
        }

        let result = session
            .as_mut()
            .expect("persistent session was initialized")
            .invoke(&self.executable, request, timeout);
        if result.is_err() {
            session.take();
        }
        result
    }
}

fn checked_l8_byte_count(width: u32, height: u32, context: &'static str) -> Result<usize> {
    if width == 0 || height == 0 {
        return Err(RuntimeError::new(context, "mask geometry must be non-zero"));
    }
    let bytes = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| RuntimeError::new(context, "mask geometry overflows"))?;
    if bytes > MAX_APPLE_L8_MASK_BYTES {
        return Err(RuntimeError::new(
            context,
            "mask exceeds 128 MiB safety limit",
        ));
    }
    Ok(bytes)
}

fn input_path(input: &Path) -> Result<String> {
    input.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        RuntimeError::new(
            "Apple adapter protocol",
            "path is not valid UTF-8 for the JSON transport",
        )
    })
}

fn semantic_role_wire(role: AppleSemanticRole) -> &'static str {
    match role {
        AppleSemanticRole::Person => "person",
        AppleSemanticRole::Skin => "skin",
        AppleSemanticRole::Hair => "hair",
        AppleSemanticRole::Teeth => "teeth",
        AppleSemanticRole::Glasses => "glasses",
        AppleSemanticRole::Sky => "sky",
    }
}

fn parse_semantic_role(role: &str) -> Result<AppleSemanticRole> {
    match role {
        "person" => Ok(AppleSemanticRole::Person),
        "skin" => Ok(AppleSemanticRole::Skin),
        "hair" => Ok(AppleSemanticRole::Hair),
        "teeth" => Ok(AppleSemanticRole::Teeth),
        "glasses" => Ok(AppleSemanticRole::Glasses),
        "sky" => Ok(AppleSemanticRole::Sky),
        other => Err(RuntimeError::new(
            "Apple adapter protocol",
            format!("unknown semantic role {other:?}"),
        )),
    }
}

fn auxiliary_kind_wire(kind: AppleAuxiliaryKind) -> Result<&'static str> {
    match kind {
        AppleAuxiliaryKind::Disparity => Ok("disparity"),
        AppleAuxiliaryKind::PortraitEffectsMatte => Ok("portrait-effects-matte"),
        AppleAuxiliaryKind::SemanticSegmentation(AppleSemanticRole::Skin) => Ok("skin-matte"),
        AppleAuxiliaryKind::SemanticSegmentation(AppleSemanticRole::Hair) => Ok("hair-matte"),
        AppleAuxiliaryKind::SemanticSegmentation(AppleSemanticRole::Teeth) => Ok("teeth-matte"),
        AppleAuxiliaryKind::SemanticSegmentation(AppleSemanticRole::Glasses) => Ok("glasses-matte"),
        AppleAuxiliaryKind::SemanticSegmentation(AppleSemanticRole::Sky) => Ok("sky-matte"),
        AppleAuxiliaryKind::SemanticSegmentation(AppleSemanticRole::Person) => {
            Err(RuntimeError::new(
                "Apple adapter protocol",
                "person mask has no ImageIO semantic matte auxiliary type",
            ))
        }
    }
}

fn auxiliary_payload_wire(
    payload: &AppleAuxiliaryPayload,
    data_path: &Path,
) -> Result<AuxiliaryPayloadWire> {
    let metadata = payload
        .metadata
        .iter()
        .map(|tag| match &tag.value {
            AppleMetadataValue::Text(value) => MetadataTagWire {
                path: tag.path.to_owned(),
                text: Some(value.clone()),
                numbers: None,
            },
            AppleMetadataValue::Numbers(values) => MetadataTagWire {
                path: tag.path.to_owned(),
                text: None,
                numbers: Some(values.clone()),
            },
        })
        .collect();
    Ok(AuxiliaryPayloadWire {
        kind: auxiliary_kind_wire(payload.kind)?.to_owned(),
        data_path: input_path(data_path)?,
        width: payload.description.width,
        height: payload.description.height,
        bytes_per_row: payload.description.bytes_per_row,
        pixel_format: u32::from_be_bytes(*payload.description.pixel_format.as_bytes()),
        orientation: payload.description.orientation.map(u32::from),
        namespaces: payload
            .namespaces
            .iter()
            .map(|namespace| MetadataNamespaceWire {
                uri: namespace.uri.to_owned(),
                prefix: namespace.prefix.to_owned(),
            })
            .collect(),
        metadata,
    })
}

#[derive(Debug, Serialize)]
struct AdapterRequest {
    schema_version: u32,
    operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    orientation: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lossy_quality: Option<f64>,
}

#[derive(Debug, Serialize)]
struct VideoToolboxMain10Request {
    schema_version: u32,
    operation: &'static str,
    input_path: String,
    output_path: String,
    video_toolbox_main10: VideoToolboxMain10Wire,
}

#[derive(Debug, Serialize)]
struct VideoToolboxMain10Wire {
    raw_width: u32,
    raw_height: u32,
    raw_bytes_per_row: u32,
    quality: f64,
    hvcc_path: String,
}

#[derive(Debug, Serialize)]
struct EncodeSourceImageRequest {
    schema_version: u32,
    operation: &'static str,
    input_path: String,
    output_path: String,
    lossy_quality: f64,
}

#[derive(Debug, Serialize)]
struct WriteAuxiliaryRequest {
    schema_version: u32,
    operation: &'static str,
    input_path: String,
    output_path: String,
    auxiliary_payloads: Vec<AuxiliaryPayloadWire>,
}

#[derive(Debug, Serialize)]
struct MergeMetadataRequest {
    schema_version: u32,
    operation: &'static str,
    input_path: String,
    output_path: String,
    metadata_source_path: String,
}

#[derive(Debug, Serialize)]
struct XmpMergeRequest {
    schema_version: u32,
    operation: &'static str,
    input_path: String,
    output_path: String,
    primary_metadata_xmp_path: String,
}

#[derive(Debug, Serialize)]
struct CoreImageRenderL8Request {
    schema_version: u32,
    operation: &'static str,
    output_path: String,
    render_l8: RenderL8Wire,
}

#[derive(Debug, Serialize)]
struct RenderL8Wire {
    mask_path: String,
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
    orientation: u8,
}

#[derive(Debug, Serialize)]
struct CoreImageEdgePreserveUpsampleRequest {
    schema_version: u32,
    operation: &'static str,
    input_path: String,
    output_path: String,
    edge_preserve_upsample: EdgePreserveUpsampleWire,
}

#[derive(Debug, Serialize)]
struct EdgePreserveUpsampleWire {
    small_mask_path: String,
    small_width: u32,
    small_height: u32,
    target_width: u32,
    target_height: u32,
    spatial_sigma: f32,
    luma_sigma: f32,
}

#[derive(Debug, Serialize)]
struct AuxiliaryPayloadWire {
    kind: String,
    data_path: String,
    width: u32,
    height: u32,
    bytes_per_row: u32,
    pixel_format: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    orientation: Option<u32>,
    namespaces: Vec<MetadataNamespaceWire>,
    metadata: Vec<MetadataTagWire>,
}

#[derive(Debug, Serialize)]
struct MetadataNamespaceWire {
    uri: String,
    prefix: String,
}

#[derive(Debug, Serialize)]
struct MetadataTagWire {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    numbers: Option<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
struct AckResponse {
    schema_version: u32,
}

#[derive(Debug, Deserialize)]
struct CapabilitiesResponse {
    schema_version: u32,
    capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AuxiliaryResponse {
    schema_version: u32,
    auxiliary: AuxiliaryWire,
}

#[derive(Debug, Deserialize)]
struct GainMapResponse {
    schema_version: u32,
    gain_map: GainMapWire,
}

#[derive(Debug, Deserialize)]
struct ImagePropertiesResponse {
    schema_version: u32,
    image_properties: ImagePropertiesWire,
}

#[derive(Debug, Deserialize)]
struct StylePropertiesResponse {
    schema_version: u32,
    style_properties: StylePropertiesWire,
}

#[derive(Debug, Deserialize)]
struct VideoToolboxMain10Response {
    schema_version: u32,
    video_toolbox_main10: VideoToolboxMain10WireResponse,
}

#[derive(Debug, Deserialize)]
struct VideoToolboxMain10WireResponse {
    width: u32,
    height: u32,
    annex_b_length: usize,
    hvcc_length: usize,
}

#[derive(Debug, Deserialize)]
struct StylePropertiesWire {
    framework_loaded: bool,
    class_available: bool,
    parse_succeeded: bool,
    style_data_length: usize,
    readback_written: bool,
}

impl From<StylePropertiesWire> for AppleStylePropertiesFacts {
    fn from(value: StylePropertiesWire) -> Self {
        Self {
            framework_loaded: value.framework_loaded,
            class_available: value.class_available,
            parse_succeeded: value.parse_succeeded,
            style_data_length: value.style_data_length,
            readback_written: value.readback_written,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SemanticResponse {
    schema_version: u32,
    semantic_masks: Vec<SemanticMaskWire>,
}

#[derive(Debug, Deserialize)]
struct SceneClassificationResponse {
    schema_version: u32,
    scene_classification: SceneClassificationWire,
}

#[derive(Debug, Deserialize)]
struct SceneClassificationWire {
    observations: Vec<SceneObservationWire>,
    request_class: String,
    revision: u32,
}

#[derive(Debug, Deserialize)]
struct SceneObservationWire {
    identifier: String,
    confidence: f64,
}

fn scene_scores_from_wire(wire: SceneClassificationWire) -> Result<AppleStyleSceneScores> {
    if wire.request_class != "VNClassifyImageRequest" {
        return Err(RuntimeError::new(
            "Apple adapter protocol",
            format!("unexpected Vision request class {:?}", wire.request_class),
        ));
    }
    if wire.revision == 0 {
        return Err(RuntimeError::new(
            "Apple adapter protocol",
            "Vision request revision must be non-zero",
        ));
    }
    let mut observations = Vec::with_capacity(wire.observations.len());
    for observation in wire.observations {
        if observation.identifier.is_empty() {
            return Err(RuntimeError::new(
                "Apple adapter protocol",
                "Vision classification returned an empty identifier",
            ));
        }
        if !observation.confidence.is_finite() || !(0.0..=1.0).contains(&observation.confidence) {
            return Err(RuntimeError::new(
                "Apple Vision scene classification",
                format!(
                    "confidence for {:?} is outside 0 through 1: {}",
                    observation.identifier, observation.confidence
                ),
            ));
        }
        observations.push(AppleVisionClassificationObservation::new(
            observation.identifier,
            observation.confidence,
        ));
    }
    apple_style_scene_scores_from_vision_observations(&observations)
        .map_err(|error| RuntimeError::new("Apple Vision scene policy", error.to_string()))
}

#[derive(Debug, Deserialize)]
struct GainMapWire {
    pixel_format: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize)]
struct ImagePropertiesWire {
    width: u32,
    height: u32,
    orientation: Option<u32>,
    focal_length_mm: Option<f64>,
    focal_length_in_35mm_film: Option<f64>,
    digital_zoom_ratio: Option<f64>,
    lens_model: Option<String>,
    f_number: Option<f64>,
}

impl From<ImagePropertiesWire> for AppleImageProperties {
    fn from(value: ImagePropertiesWire) -> Self {
        Self {
            width: value.width,
            height: value.height,
            orientation: value.orientation,
            focal_length_mm: value.focal_length_mm,
            focal_length_in_35mm_film: value.focal_length_in_35mm_film,
            digital_zoom_ratio: value.digital_zoom_ratio,
            lens_model: value.lens_model,
            f_number: value.f_number,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SemanticMaskWire {
    role: String,
    width: u32,
    height: u32,
    pixel_format: u32,
}

#[derive(Debug, Deserialize)]
struct AuxiliaryWire {
    iso_gain_map: bool,
    disparity: bool,
    portrait_effects_matte: bool,
    skin_matte: bool,
    hair_matte: bool,
    teeth_matte: bool,
    glasses_matte: bool,
    focus_metadata: bool,
}

impl From<AuxiliaryWire> for AppleImageAuxiliaryFacts {
    fn from(value: AuxiliaryWire) -> Self {
        Self {
            iso_gain_map: value.iso_gain_map,
            disparity: value.disparity,
            portrait_effects_matte: value.portrait_effects_matte,
            skin_matte: value.skin_matte,
            hair_matte: value.hair_matte,
            teeth_matte: value.teeth_matte,
            glasses_matte: value.glasses_matte,
            focus_metadata: value.focus_metadata,
        }
    }
}

fn validate_schema(schema_version: u32) -> Result<()> {
    if schema_version != APPLE_ADAPTER_SCHEMA_VERSION {
        return Err(RuntimeError::new(
            "Apple adapter protocol",
            format!(
                "unsupported schema_version {schema_version}; expected {APPLE_ADAPTER_SCHEMA_VERSION}"
            ),
        ));
    }
    Ok(())
}

fn read_bounded_adapter_frame(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if frame.is_empty() {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Apple adapter response ended without a newline",
            ));
        }

        if let Some(newline) = available.iter().position(|byte| *byte == b'\n') {
            if frame.len().saturating_add(newline) > MAX_APPLE_ADAPTER_FRAME_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Apple adapter response frame exceeds safety limit",
                ));
            }
            frame.extend_from_slice(&available[..newline]);
            reader.consume(newline + 1);
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            return Ok(Some(frame));
        }

        let chunk_len = available.len();
        if frame.len().saturating_add(chunk_len) > MAX_APPLE_ADAPTER_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Apple adapter response frame exceeds safety limit",
            ));
        }
        frame.extend_from_slice(available);
        reader.consume(chunk_len);
    }
}

fn read_bounded_diagnostic(mut reader: impl Read) -> Vec<u8> {
    let mut diagnostic = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                let remaining = MAX_APPLE_ADAPTER_DIAGNOSTIC_BYTES.saturating_sub(diagnostic.len());
                let retained = count.min(remaining);
                diagnostic.extend_from_slice(&buffer[..retained]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    diagnostic
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_properties_wire_is_platform_facts_only() {
        let response: ImagePropertiesResponse = serde_json::from_slice(
            br#"{"schema_version":2,"image_properties":{"width":4032,"height":3024,"orientation":6,"focal_length_mm":8.67,"focal_length_in_35mm_film":48,"digital_zoom_ratio":2,"lens_model":"OPPO camera 24mm","f_number":1.8}}"#,
        )
        .unwrap();
        assert_eq!(response.schema_version, 2);
        let properties: AppleImageProperties = response.image_properties.into();
        assert_eq!(properties.width, 4032);
        assert_eq!(properties.height, 3024);
        assert_eq!(properties.orientation, Some(6));
        assert_eq!(properties.focal_length_in_35mm_film, Some(48.0));
        assert_eq!(properties.lens_model.as_deref(), Some("OPPO camera 24mm"));
    }

    #[test]
    fn scene_classification_wire_is_raw_and_rust_owns_bucketing() {
        let response: SceneClassificationResponse = serde_json::from_slice(
            br#"{"schema_version":2,"scene_classification":{"observations":[{"identifier":"meal","confidence":0.31},{"identifier":"dish","confidence":0.74},{"identifier":"sunrise","confidence":0.42},{"identifier":"room","confidence":0.55},{"identifier":"outdoor","confidence":0.63},{"identifier":"cat","confidence":0.99}],"request_class":"VNClassifyImageRequest","revision":3}}"#,
        )
        .unwrap();
        let scores = scene_scores_from_wire(response.scene_classification).unwrap();
        assert_eq!(scores.food, 0.74);
        assert_eq!(scores.sunset, 0.42);
        assert_eq!(scores.indoor, 0.55);
        assert_eq!(scores.outdoor, 0.63);
    }

    #[test]
    fn scene_classification_wire_rejects_invalid_platform_confidence() {
        let wire = SceneClassificationWire {
            observations: vec![SceneObservationWire {
                identifier: "food".to_owned(),
                confidence: 1.5,
            }],
            request_class: "VNClassifyImageRequest".to_owned(),
            revision: 3,
        };
        let error = scene_scores_from_wire(wire).unwrap_err();
        assert!(error.to_string().contains("outside 0 through 1"));
    }

    #[test]
    fn persistent_session_reuses_process_and_recovers_after_crash() {
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().expect("create fake Apple adapter directory");
        let helper = temporary.path().join("fake-apple-adapter.sh");
        std::fs::write(
            &helper,
            r#"#!/bin/sh
set -eu
[ "${1:-}" = "--persistent-json-lines" ] || exit 9
printf 'launch\n' >> "$0.log"
count=0
while IFS= read -r request; do
    count=$((count + 1))
    if [ "$count" -eq 2 ]; then
        printf 'synthetic crash\n' >&2
        exit 7
    fi
    printf '%s\n' '{"schema_version":2,"capabilities":[]}'
done
"#,
        )
        .expect("write fake Apple adapter");
        let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&helper, permissions).unwrap();

        let client = AppleAdapterClient::new(&helper);
        client.capabilities().expect("first request must succeed");
        let error = client
            .capabilities()
            .expect_err("second request must observe helper crash");
        assert!(error.to_string().contains("synthetic crash"), "{error}");
        client
            .capabilities()
            .expect("client must restart helper after crash");

        let launch_log = PathBuf::from(format!("{}.log", helper.display()));
        let launches = std::fs::read_to_string(launch_log).unwrap();
        assert_eq!(launches.lines().count(), 2);
    }

    #[test]
    fn persistent_response_reader_rejects_oversized_frame() {
        let mut payload = vec![b'x'; MAX_APPLE_ADAPTER_FRAME_BYTES + 1];
        payload.push(b'\n');
        let mut reader = BufReader::new(std::io::Cursor::new(payload));
        let error = read_bounded_adapter_frame(&mut reader).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
