use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use xdremux_engine::{
    AppleGainMapFacts, AppleImageAuxiliaryFacts, AppleSemanticRole, OperationCapability,
};
use xdremux_format::FourCC;

use crate::{Result, RuntimeError};

const APPLE_ADAPTER_SCHEMA_VERSION: u32 = 1;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const VISION_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_SEMANTIC_MASK_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleSemanticMask {
    pub role: AppleSemanticRole,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
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

    pub(super) fn vision_semantic_mattes(
        &self,
        input: &Path,
        roles: &[AppleSemanticRole],
        orientation: Option<u32>,
    ) -> Result<Vec<AppleSemanticMask>> {
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
            },
            VISION_TIMEOUT,
        )?;
        let response: SemanticResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        validate_schema(response.schema_version)?;

        let mut observed = BTreeSet::new();
        let mut masks = Vec::with_capacity(response.semantic_masks.len());
        for wire in response.semantic_masks {
            let role = parse_semantic_role(&wire.role)?;
            if !expected.contains(&role) {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!("adapter returned unrequested role {:?}", role),
                ));
            }
            if !observed.insert(role) {
                return Err(RuntimeError::new(
                    "Apple Vision semantic mattes",
                    format!("adapter returned duplicate role {:?}", role),
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
            if expected_bytes > MAX_SEMANTIC_MASK_BYTES {
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
            masks.push(AppleSemanticMask {
                role,
                width: wire.width,
                height: wire.height,
                pixels,
            });
        }

        if observed != expected {
            let missing = expected.difference(&observed).copied().collect::<Vec<_>>();
            return Err(RuntimeError::new(
                "Apple Vision semantic mattes",
                format!("adapter omitted required roles: {missing:?}"),
            ));
        }
        masks.sort_by_key(|mask| mask.role);
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
