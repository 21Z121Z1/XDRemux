use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use xdremux_engine::{
    AppleAuxiliaryKind, AppleAuxiliaryPayload, AppleGainMapFacts, AppleImageAuxiliaryFacts,
    AppleL8Mask, AppleMetadataValue, AppleSemanticRole, OperationCapability,
};
use xdremux_format::FourCC;

use crate::{Result, RuntimeError};

const APPLE_ADAPTER_SCHEMA_VERSION: u32 = 1;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const APPLE_COMPUTE_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppleAdapterClient {
    executable: PathBuf,
    timeout: Duration,
}

impl AppleAdapterClient {
    pub(super) fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            timeout: DEFAULT_TIMEOUT,
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
        let mut child = Command::new(&self.executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| RuntimeError::external("Apple adapter launch", error))?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            RuntimeError::new("Apple adapter launch", "child stdin pipe is unavailable")
        })?;
        stdin
            .write_all(request)
            .map_err(|error| RuntimeError::external("Apple adapter request write", error))?;
        drop(stdin);

        let stdout = child.stdout.take().ok_or_else(|| {
            RuntimeError::new("Apple adapter launch", "child stdout pipe is unavailable")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            RuntimeError::new("Apple adapter launch", "child stderr pipe is unavailable")
        })?;
        let stdout_reader = thread::spawn(move || read_all(stdout));
        let stderr_reader = thread::spawn(move || read_all(stderr));

        let deadline = Instant::now() + timeout;
        let status = loop {
            match child
                .try_wait()
                .map_err(|error| RuntimeError::external("Apple adapter wait", error))?
            {
                Some(status) => break status,
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = join_reader(stdout_reader, "stdout");
                    let _ = join_reader(stderr_reader, "stderr");
                    return Err(RuntimeError::new(
                        "Apple adapter timeout",
                        format!(
                            "{} exceeded {} ms",
                            self.executable.display(),
                            timeout.as_millis()
                        ),
                    ));
                }
                None => thread::sleep(POLL_INTERVAL),
            }
        };

        child
            .wait()
            .map_err(|error| RuntimeError::external("Apple adapter reap", error))?;
        let stdout = join_reader(stdout_reader, "stdout")?;
        let stderr = join_reader(stderr_reader, "stderr")?;

        if !status.success() {
            let diagnostic = String::from_utf8_lossy(&stderr).trim().to_owned();
            return Err(RuntimeError::new(
                "Apple adapter execution",
                if diagnostic.is_empty() {
                    format!("adapter exited with {status}")
                } else {
                    format!("adapter exited with {status}: {diagnostic}")
                },
            ));
        }
        Ok(stdout)
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
struct SemanticResponse {
    schema_version: u32,
    semantic_masks: Vec<SemanticMaskWire>,
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

fn read_all(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(
    reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stream: &'static str,
) -> Result<Vec<u8>> {
    reader
        .join()
        .map_err(|_| {
            RuntimeError::new("Apple adapter output", format!("{stream} reader panicked"))
        })?
        .map_err(|error| RuntimeError::external("Apple adapter output", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_properties_wire_is_platform_facts_only() {
        let response: ImagePropertiesResponse = serde_json::from_slice(
            br#"{"schema_version":1,"image_properties":{"width":4032,"height":3024,"orientation":6,"focal_length_mm":8.67,"focal_length_in_35mm_film":48,"digital_zoom_ratio":2,"lens_model":"OPPO camera 24mm","f_number":1.8}}"#,
        )
        .unwrap();
        assert_eq!(response.schema_version, 1);
        let properties: AppleImageProperties = response.image_properties.into();
        assert_eq!(properties.width, 4032);
        assert_eq!(properties.height, 3024);
        assert_eq!(properties.orientation, Some(6));
        assert_eq!(properties.focal_length_in_35mm_film, Some(48.0));
        assert_eq!(properties.lens_model.as_deref(), Some("OPPO camera 24mm"));
    }
}
