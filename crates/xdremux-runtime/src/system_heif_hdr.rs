//! Opt-in prototype: Core Image calculates the HEIF gain map.
//!
//! `XDREMUX_SYSTEM_GAIN_MAP=1` replaces the product base, after Rust has
//! reconstructed linear HDR, with a HEIF written by
//! `CIContext.writeHEIFRepresentationOfImage`. The SDR primary is Display P3
//! RGBA8. The HDR image is extended-linear Display P3 RGBAh. Core Image is
//! asked to derive the gain map through `kCIImageRepresentationHDRImage`.
//! The default Styles path is unchanged when the variable is unset.
//!
//! Guided-filter weights, radius, and epsilon are provisional. They are not
//! calibrated against private `CMIGuidedFilter`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xdremux_engine::{
    content_headroom_from_linear, encode_display_p3_rgba8, linear_difference, pack_linear_rgba_f16,
    provisional_guided_filter, unpack_linear_rgb_f16, LinearDifference, PROVISIONAL_GUIDED_EPSILON,
    PROVISIONAL_GUIDED_RADIUS, PROVISIONAL_HDR_LOCAL_WEIGHT, PROVISIONAL_SDR_LOCAL_WEIGHT,
};

use crate::apple_adapter::{AppleAdapterClient, SystemHeifAuxiliaryPlane, SystemHeifHdrFacts};
use crate::apple_styles::{scene_linear_pair, SceneLinearPair};
use crate::{AtomicFilePublisher, Result, RuntimeError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemHeifHdrReport {
    pub provisional: bool,
    pub guided_filter: GuidedFilterReport,
    pub width: u32,
    pub height: u32,
    pub content_headroom: f32,
    pub gain_map_as_rgb: bool,
    pub before_styles: SystemHeifHdrReadback,
    #[serde(default)]
    pub after_styles: Option<SystemHeifHdrReadback>,
    #[serde(default)]
    pub styles_carrier_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuidedFilterReport {
    pub radius: usize,
    pub epsilon: f32,
    pub hdr_local_weight: f32,
    pub sdr_local_weight: f32,
    pub coefficient_storage: String,
    pub tuning: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemHeifHdrReadback {
    pub apple_hdr_gain_map: AuxiliaryPlaneReport,
    pub iso_gain_map: AuxiliaryPlaneReport,
    pub content_headroom_sent: Option<f32>,
    pub decoded_content_headroom: f32,
    pub decoded_width: u32,
    pub decoded_height: u32,
    pub error_vs_submitted_f16: Option<LinearDifference>,
    pub error_vs_prequantized_f32: Option<LinearDifference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuxiliaryPlaneReport {
    pub present: bool,
    pub pixel_format: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub(crate) struct SystemHeifEncoded {
    pub bytes: Vec<u8>,
    pub report_path: PathBuf,
    pub reference_f32_path: PathBuf,
    pub reference_f16_path: PathBuf,
    pub submitted_scene: Option<SceneLinearPair>,
    pub content_headroom: f32,
}

pub(crate) fn requested() -> Result<bool> {
    match std::env::var("XDREMUX_SYSTEM_GAIN_MAP") {
        Err(std::env::VarError::NotPresent) => Ok(false),
        Ok(value) if value == "1" => Ok(true),
        Ok(value) => Err(RuntimeError::new(
            "system HEIF gain map",
            format!("XDREMUX_SYSTEM_GAIN_MAP must be unset or 1, found {value}"),
        )),
        Err(error) => Err(RuntimeError::external("system HEIF gain map", error)),
    }
}

pub(crate) fn encode_system_base(
    executable: &Path,
    product_base: &[u8],
    staging: &Path,
    output: &Path,
) -> Result<SystemHeifEncoded> {
    xdremux_heif::validate_gain_map_structure(product_base)
        .map_err(|error| RuntimeError::external("system HEIF source Gain Map", error))?;
    let geometry = xdremux_heif::primary_coded_geometry(product_base)
        .map_err(|error| RuntimeError::external("system HEIF source geometry", error))?;
    let maximum_edge = max_edge_override()?.unwrap_or(geometry.width.max(geometry.height).max(2));
    let client = AppleAdapterClient::new(executable.to_path_buf());
    let scene = scene_linear_pair(&client, product_base, staging, maximum_edge)?;
    let width = u32::try_from(scene.width)
        .map_err(|_| RuntimeError::new("system HEIF gain map", "scene width exceeds u32"))?;
    let height = u32::try_from(scene.height)
        .map_err(|_| RuntimeError::new("system HEIF gain map", "scene height exceeds u32"))?;
    let staging_dir = tempfile::Builder::new()
        .prefix(".xdremux-system-hdr-")
        .tempdir_in(staging)
        .map_err(|error| RuntimeError::external("system HEIF staging", error))?;
    let measurement = encode_linear_pair(
        &client,
        &scene.sdr,
        &scene.hdr,
        width,
        height,
        staging_dir.path(),
    )?;
    let report_path = report_path_for(output);
    write_report(&report_path, &measurement.report)?;
    ensure_round_trip(&measurement.report)?;
    let reference_f32_path = staging.join("system-hdr-reference-f32.bin");
    let reference_f16_path = staging.join("system-hdr-reference-f16.bin");
    write_f32(&reference_f32_path, &measurement.prequantized_hdr)?;
    write_f32(&reference_f16_path, &measurement.submitted_hdr)?;
    let bytes = fs::read(&measurement.output)
        .map_err(|error| RuntimeError::external("system HEIF read", error))?;
    Ok(SystemHeifEncoded {
        bytes,
        report_path,
        reference_f32_path,
        reference_f16_path,
        content_headroom: measurement.report.content_headroom,
        submitted_scene: Some(SceneLinearPair {
            width: dimension_usize(width)?,
            height: dimension_usize(height)?,
            sdr: measurement.submitted_sdr,
            hdr: measurement.submitted_hdr,
        }),
    })
}

pub fn measure_system_heif_hdr_pair(
    executable: &Path,
    sdr_linear: &[f32],
    hdr_linear: &[f32],
    width: u32,
    height: u32,
    output: &Path,
) -> Result<SystemHeifHdrReport> {
    let staging = tempfile::Builder::new()
        .prefix(".xdremux-system-hdr-")
        .tempdir()
        .map_err(|error| RuntimeError::external("system HEIF staging", error))?;
    let measurement = encode_linear_pair(
        &AppleAdapterClient::new(executable.to_path_buf()),
        sdr_linear,
        hdr_linear,
        width,
        height,
        staging.path(),
    )?;
    publish_measurement(&measurement, output)?;
    Ok(measurement.report)
}

fn publish_measurement(measurement: &EncodedMeasurement, output: &Path) -> Result<()> {
    ensure_round_trip(&measurement.report)?;
    let bytes = fs::read(&measurement.output)
        .map_err(|error| RuntimeError::external("system HEIF staged output", error))?;
    AtomicFilePublisher::new(output.to_path_buf()).publish_bytes(bytes)?;
    Ok(())
}

pub(crate) fn record_carrier_error(path: &Path, detail: &str) {
    let Ok(bytes) = fs::read(path) else {
        return;
    };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return;
    };
    let Some(object) = value.as_object_mut() else {
        return;
    };
    object.insert(
        "styles_carrier_error".to_owned(),
        serde_json::Value::String(detail.to_owned()),
    );
    if let Ok(encoded) = serde_json::to_vec_pretty(&value) {
        let _ = fs::write(path, encoded);
    }
}

pub(crate) fn record_after_styles(
    adapter: &AppleAdapterClient,
    heif: &Path,
    report_path: &Path,
    reference_f32: &Path,
    reference_f16: &Path,
) -> Result<()> {
    let mut report = read_report(report_path)?;
    let readback_path = staging_readback(heif)?;
    let facts = adapter.imageio_read_system_hdr(heif, &readback_path)?;
    let submitted = read_f32(reference_f16)?;
    let prequantized = read_f32(reference_f32)?;
    report.after_styles = Some(readback_from_file(
        &facts,
        &readback_path,
        &submitted,
        &prequantized,
    )?);
    write_report(report_path, &report)?;
    if facts.decoded_width != report.width || facts.decoded_height != report.height {
        return Err(RuntimeError::new(
            "system HEIF HDR readback",
            format!(
                "Styles file decoded as {}x{}, submitted {}x{}",
                facts.decoded_width, facts.decoded_height, report.width, report.height
            ),
        ));
    }
    Ok(())
}

struct EncodedMeasurement {
    output: PathBuf,
    report: SystemHeifHdrReport,
    prequantized_hdr: Vec<f32>,
    submitted_hdr: Vec<f32>,
    submitted_sdr: Vec<f32>,
}

fn encode_linear_pair(
    adapter: &AppleAdapterClient,
    sdr_linear: &[f32],
    hdr_linear: &[f32],
    width: u32,
    height: u32,
    staging: &Path,
) -> Result<EncodedMeasurement> {
    let width_usize = dimension_usize(width)?;
    let height_usize = dimension_usize(height)?;
    let pixel_count = width_usize
        .checked_mul(height_usize)
        .ok_or_else(|| RuntimeError::new("system HEIF gain map", "pixel count overflows"))?;
    if sdr_linear.len() != pixel_count.saturating_mul(3)
        || hdr_linear.len() != pixel_count.saturating_mul(3)
    {
        return Err(RuntimeError::new(
            "system HEIF gain map",
            "SDR and HDR rasters do not match the requested geometry",
        ));
    }
    let sdr = provisional_guided_filter(
        sdr_linear,
        width_usize,
        height_usize,
        PROVISIONAL_SDR_LOCAL_WEIGHT,
    )
    .map_err(|error| RuntimeError::external("system HEIF SDR look", error))?;
    let hdr = provisional_guided_filter(
        hdr_linear,
        width_usize,
        height_usize,
        PROVISIONAL_HDR_LOCAL_WEIGHT,
    )
    .map_err(|error| RuntimeError::external("system HEIF HDR look", error))?;
    let sdr_bytes = encode_display_p3_rgba8(&sdr)
        .map_err(|error| RuntimeError::external("system HEIF SDR", error))?;
    let hdr_bytes = pack_linear_rgba_f16(&hdr)
        .map_err(|error| RuntimeError::external("system HEIF HDR", error))?;
    let submitted = unpack_linear_rgb_f16(&hdr_bytes)
        .map_err(|error| RuntimeError::external("system HEIF HDR", error))?;
    let headroom = content_headroom_from_linear(&submitted)
        .map_err(|error| RuntimeError::external("system HEIF content headroom", error))?;
    let sdr_path = staging.join("sdr-rgba8.bin");
    let hdr_path = staging.join("hdr-rgbah.bin");
    let readback_path = staging.join("readback-rgbah.bin");
    let output = staging.join("system.heic");
    fs::write(&sdr_path, flip_rows(&sdr_bytes, width_usize * 4)?)
        .map_err(|error| RuntimeError::external("system HEIF SDR staging", error))?;
    fs::write(&hdr_path, flip_rows(&hdr_bytes, width_usize * 8)?)
        .map_err(|error| RuntimeError::external("system HEIF HDR staging", error))?;
    let facts = adapter.coreimage_write_heif_hdr(&crate::apple_adapter::SystemHeifHdrWrite {
        output: &output,
        sdr_rgba8: &sdr_path,
        hdr_rgba_f16: &hdr_path,
        readback: &readback_path,
        width,
        height,
        content_headroom: headroom,
        gain_map_as_rgb: true,
    })?;
    let readback = readback_from_file(&facts, &readback_path, &submitted, &hdr)?;
    let report = SystemHeifHdrReport {
        provisional: true,
        guided_filter: guided_filter_report(),
        width,
        height,
        content_headroom: headroom,
        gain_map_as_rgb: true,
        before_styles: readback,
        after_styles: None,
        styles_carrier_error: None,
    };
    Ok(EncodedMeasurement {
        output,
        report,
        prequantized_hdr: hdr,
        submitted_hdr: submitted,
        submitted_sdr: submitted_sdr_linear(&sdr_bytes),
    })
}

// Decode the exact RGBA8 bytes submitted to Core Image, including quantization
// and clipping. They use Display P3's IEC sRGB transfer, not a new gain map.
fn submitted_sdr_linear(rgba8: &[u8]) -> Vec<f32> {
    rgba8
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|pixel| {
            pixel[..3].iter().map(|code| {
                let value = f32::from(*code) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            })
        })
        .collect()
}

fn dimension_usize(value: u32) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| RuntimeError::new("system HEIF gain map", "dimension exceeds usize"))
}

fn guided_filter_report() -> GuidedFilterReport {
    GuidedFilterReport {
        radius: PROVISIONAL_GUIDED_RADIUS,
        epsilon: PROVISIONAL_GUIDED_EPSILON,
        hdr_local_weight: PROVISIONAL_HDR_LOCAL_WEIGHT,
        sdr_local_weight: PROVISIONAL_SDR_LOCAL_WEIGHT,
        coefficient_storage: "f16".to_owned(),
        tuning: "provisional".to_owned(),
    }
}

fn readback_from_file(
    facts: &SystemHeifHdrFacts,
    readback_path: &Path,
    submitted: &[f32],
    prequantized: &[f32],
) -> Result<SystemHeifHdrReadback> {
    let packed = fs::read(readback_path)
        .map_err(|error| RuntimeError::external("system HEIF HDR readback", error))?;
    if packed.len() != facts.readback_byte_count {
        return Err(RuntimeError::new(
            "system HEIF HDR readback",
            format!(
                "readback has {} bytes; adapter reported {}",
                packed.len(),
                facts.readback_byte_count
            ),
        ));
    }
    let row_bytes = usize::try_from(facts.decoded_width)
        .ok()
        .and_then(|width| width.checked_mul(8))
        .ok_or_else(|| {
            RuntimeError::new("system HEIF HDR readback", "readback row size overflows")
        })?;
    let top_down = flip_rows(&packed, row_bytes)?;
    let decoded = unpack_linear_rgb_f16(&top_down)
        .map_err(|error| RuntimeError::external("system HEIF HDR readback", error))?;
    comparison(facts, &decoded, submitted, prequantized)
}

fn comparison(
    facts: &SystemHeifHdrFacts,
    decoded: &[f32],
    submitted: &[f32],
    prequantized: &[f32],
) -> Result<SystemHeifHdrReadback> {
    let same_geometry = facts.decoded_width > 0
        && decoded.len() == submitted.len()
        && decoded.len() == prequantized.len();
    let (against_f16, against_f32) = if same_geometry {
        (
            Some(
                linear_difference(submitted, decoded)
                    .map_err(|error| RuntimeError::external("system HEIF HDR readback", error))?,
            ),
            Some(
                linear_difference(prequantized, decoded)
                    .map_err(|error| RuntimeError::external("system HEIF HDR readback", error))?,
            ),
        )
    } else {
        (None, None)
    };
    Ok(SystemHeifHdrReadback {
        apple_hdr_gain_map: plane_report(&facts.apple_hdr_gain_map),
        iso_gain_map: plane_report(&facts.iso_gain_map),
        content_headroom_sent: facts.content_headroom_sent,
        decoded_content_headroom: facts.decoded_content_headroom,
        decoded_width: facts.decoded_width,
        decoded_height: facts.decoded_height,
        error_vs_submitted_f16: against_f16,
        error_vs_prequantized_f32: against_f32,
    })
}

fn ensure_round_trip(report: &SystemHeifHdrReport) -> Result<()> {
    let readback = &report.before_styles;
    if readback.decoded_width != report.width || readback.decoded_height != report.height {
        return Err(RuntimeError::new(
            "system HEIF HDR readback",
            format!(
                "decoded {}x{}, submitted {}x{}",
                readback.decoded_width, readback.decoded_height, report.width, report.height
            ),
        ));
    }
    if let Some(sent) = readback.content_headroom_sent {
        if (f64::from(sent) - f64::from(report.content_headroom)).abs() > 1.0e-3 {
            return Err(RuntimeError::new(
                "system HEIF HDR encode",
                format!(
                    "adapter echoed content headroom {sent}, Rust sent {}",
                    report.content_headroom
                ),
            ));
        }
    }
    if readback.error_vs_submitted_f16.is_none() {
        return Err(RuntimeError::new(
            "system HEIF HDR readback",
            "HDR readback could not be compared with the submitted image",
        ));
    }
    Ok(())
}

fn plane_report(plane: &SystemHeifAuxiliaryPlane) -> AuxiliaryPlaneReport {
    AuxiliaryPlaneReport {
        present: plane.present,
        pixel_format: plane.pixel_format,
        width: plane.width,
        height: plane.height,
    }
}

fn report_path_for(output: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("XDREMUX_SYSTEM_GAIN_MAP_REPORT") {
        return PathBuf::from(path);
    }
    let mut name = output
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("output"))
        .to_os_string();
    name.push(".system-hdr.json");
    output.with_file_name(name)
}

fn max_edge_override() -> Result<Option<u32>> {
    match std::env::var("XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE") {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Ok(value) => {
            let edge = value.parse::<u32>().map_err(|_| {
                RuntimeError::new(
                    "system HEIF gain map",
                    format!("XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE is not an integer: {value}"),
                )
            })?;
            if edge < 2 {
                return Err(RuntimeError::new(
                    "system HEIF gain map",
                    "XDREMUX_SYSTEM_GAIN_MAP_MAX_EDGE must be at least 2",
                ));
            }
            Ok(Some(edge))
        }
        Err(error) => Err(RuntimeError::external("system HEIF gain map", error)),
    }
}

fn write_report(path: &Path, report: &SystemHeifHdrReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            fs::create_dir_all(parent)
                .map_err(|error| RuntimeError::external("system HEIF report", error))?;
        }
    }
    let mut encoded = serde_json::to_vec_pretty(report)
        .map_err(|error| RuntimeError::external("system HEIF report", error))?;
    encoded.push(b'\n');
    fs::write(path, encoded).map_err(|error| RuntimeError::external("system HEIF report", error))
}

fn read_report(path: &Path) -> Result<SystemHeifHdrReport> {
    let bytes =
        fs::read(path).map_err(|error| RuntimeError::external("system HEIF report", error))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| RuntimeError::external("system HEIF report", error))
}

fn write_f32(path: &Path, samples: &[f32]) -> Result<()> {
    let mut bytes = Vec::with_capacity(samples.len().saturating_mul(4));
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).map_err(|error| RuntimeError::external("system HEIF reference", error))
}

fn read_f32(path: &Path) -> Result<Vec<f32>> {
    let bytes =
        fs::read(path).map_err(|error| RuntimeError::external("system HEIF reference", error))?;
    if !bytes.len().is_multiple_of(4) {
        return Err(RuntimeError::new(
            "system HEIF reference",
            "reference length is not a multiple of 4",
        ));
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect())
}

/// Core Image bitmap rows start at the bottom. The staged files use that
/// order. Readback is flipped back to the top-down scene order before the
/// error comparison.
fn flip_rows(bytes: &[u8], row_bytes: usize) -> Result<Vec<u8>> {
    if row_bytes == 0 || !bytes.len().is_multiple_of(row_bytes) {
        return Err(RuntimeError::new(
            "system HEIF raster",
            "row flip received a partial row",
        ));
    }
    let height = bytes.len() / row_bytes;
    let mut flipped = vec![0_u8; bytes.len()];
    for row in 0..height {
        let source = row * row_bytes;
        let destination = (height - 1 - row) * row_bytes;
        flipped[destination..destination + row_bytes]
            .copy_from_slice(&bytes[source..source + row_bytes]);
    }
    Ok(flipped)
}

fn staging_readback(heif: &Path) -> Result<PathBuf> {
    let mut path = heif.as_os_str().to_os_string();
    path.push(".readback-rgbah.bin");
    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_readback_preserves_an_existing_measurement_output() {
        let temporary = tempfile::tempdir().unwrap();
        let output = temporary.path().join("existing.heic");
        let candidate = temporary.path().join("candidate.heic");
        fs::write(&output, b"existing published media").unwrap();
        fs::write(&candidate, b"unvalidated candidate").unwrap();
        let plane = || AuxiliaryPlaneReport {
            present: false,
            pixel_format: None,
            width: None,
            height: None,
        };
        let measurement = EncodedMeasurement {
            output: candidate,
            report: SystemHeifHdrReport {
                provisional: true,
                guided_filter: guided_filter_report(),
                width: 2,
                height: 2,
                content_headroom: 1.0,
                gain_map_as_rgb: true,
                before_styles: SystemHeifHdrReadback {
                    apple_hdr_gain_map: plane(),
                    iso_gain_map: plane(),
                    content_headroom_sent: Some(1.0),
                    decoded_content_headroom: 1.0,
                    decoded_width: 1,
                    decoded_height: 2,
                    error_vs_submitted_f16: None,
                    error_vs_prequantized_f32: None,
                },
                after_styles: None,
                styles_carrier_error: None,
            },
            prequantized_hdr: Vec::new(),
            submitted_hdr: Vec::new(),
            submitted_sdr: Vec::new(),
        };
        assert!(publish_measurement(&measurement, &output).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"existing published media");
    }

    #[test]
    fn styles_sdr_reference_uses_the_quantized_submitted_codes() {
        let original = [1.2, -0.2, 0.25];
        let packed = encode_display_p3_rgba8(&original).unwrap();
        let submitted = submitted_sdr_linear(&packed);
        assert_eq!(submitted[0], 1.0);
        assert_eq!(submitted[1], 0.0);
        let expected = ((f32::from(packed[2]) / 255.0 + 0.055) / 1.055).powf(2.4);
        assert_eq!(submitted[2], expected);
        assert_ne!(submitted[2], original[2]);
    }
}
