use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use xdremux_engine::{CapabilityInventory, OperationCapability};

use crate::{Result, RuntimeError};

pub const APPLE_ADAPTER_SCHEMA_VERSION: u32 = 1;
pub const APPLE_ADAPTER_ENV: &str = "XDREMUX_APPLE_ADAPTER";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleAdapterCapabilities {
    pub photographic_styles: bool,
    pub portrait: bool,
}

impl AppleAdapterCapabilities {
    pub fn operation_capabilities(&self) -> Vec<OperationCapability> {
        let mut capabilities = Vec::with_capacity(2);
        if self.photographic_styles {
            capabilities.push(OperationCapability::PhotographicStylesAdapter);
        }
        if self.portrait {
            capabilities.push(OperationCapability::PortraitAdapter);
        }
        capabilities
    }

    pub fn inventory(&self) -> CapabilityInventory {
        CapabilityInventory::new(self.operation_capabilities())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleAdapterClient {
    executable: PathBuf,
    timeout: Duration,
}

impl AppleAdapterClient {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn from_environment() -> Result<Option<Self>> {
        let Some(value) = std::env::var_os(APPLE_ADAPTER_ENV) else {
            return Ok(None);
        };
        if value.is_empty() {
            return Err(RuntimeError::new(
                "Apple adapter configuration",
                format!("{APPLE_ADAPTER_ENV} is set but empty"),
            ));
        }
        Ok(Some(Self::new(PathBuf::from(value))))
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn capabilities(&self) -> Result<AppleAdapterCapabilities> {
        let request = AdapterRequest {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation: "capabilities",
        };
        let request = serde_json::to_vec(&request)
            .map_err(|error| RuntimeError::external("Apple adapter request encoding", error))?;
        let output = self.invoke(&request)?;
        let response: AdapterResponse = serde_json::from_slice(&output)
            .map_err(|error| RuntimeError::external("Apple adapter response decoding", error))?;
        if response.schema_version != APPLE_ADAPTER_SCHEMA_VERSION {
            return Err(RuntimeError::new(
                "Apple adapter protocol",
                format!(
                    "unsupported schema_version {}; expected {}",
                    response.schema_version, APPLE_ADAPTER_SCHEMA_VERSION
                ),
            ));
        }

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

    fn invoke(&self, request: &[u8]) -> Result<Vec<u8>> {
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

        let deadline = Instant::now() + self.timeout;
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
                            self.timeout.as_millis()
                        ),
                    ));
                }
                None => thread::sleep(POLL_INTERVAL),
            }
        };

        // `try_wait` reaps the child on supported platforms once the status is
        // available, but `wait` remains safe and keeps lifecycle ownership here.
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

#[derive(Debug, Serialize)]
struct AdapterRequest<'a> {
    schema_version: u32,
    operation: &'a str,
}

#[derive(Debug, Deserialize)]
struct AdapterResponse {
    schema_version: u32,
    capabilities: Vec<String>,
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
        .map_err(|_| RuntimeError::new("Apple adapter output", format!("{stream} reader panicked")))?
        .map_err(|error| RuntimeError::external("Apple adapter output", error))
}
