//! Texture wire reports and workfiles stay behind the runtime's Apple adapter.
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use xdremux_engine::{AppleTextureTuningProfile, AppleTextureValue};

use super::{
    input_path, validate_schema, AckResponse, AppleAdapterClient, APPLE_ADAPTER_SCHEMA_VERSION,
    APPLE_COMPUTE_TIMEOUT,
};
use crate::{Result, RuntimeError};

pub(crate) struct TexturePrepareRequest<'a> {
    pub input: &'a Path,
    pub source_bytes: &'a [u8],
    pub directory: &'a Path,
    pub max_dimension: u32,
}

pub(crate) struct TexturePrepared {
    directory: PathBuf,
    working: PathBuf,
    working_sha256: String,
    width: u64,
    height: u64,
}

pub(crate) struct TextureInferRequest<'a> {
    pub prepared: &'a TexturePrepared,
}

pub(crate) struct TextureInference {
    directory: PathBuf,
    working: PathBuf,
    input_sha256: String,
    people: Vec<TextureObservation>,
    resources: Vec<Value>,
    instances: Vec<Value>,
    codecs: Vec<Value>,
}

pub(crate) struct TextureObservation {
    pub instance_key: String,
    pub instance_role: String,
    pub face_probability_overlap: f64,
    native_person: Value,
}

pub(crate) struct TextureMask {
    pub auxiliary_type: String,
    pub instance_reference_key: Option<String>,
    pub codec: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum TextureEffect {
    Mattify,
    UnderEyeBrighten,
    SkinSmoothStandalone,
}

impl TextureEffect {
    fn name(self) -> &'static str {
        match self {
            Self::Mattify => "Mattify",
            Self::UnderEyeBrighten => "UnderEyeBrighten",
            Self::SkinSmoothStandalone => "SkinSmoothStandalone",
        }
    }
}

pub(crate) struct TextureStatisticsRequest<'a> {
    pub inference: &'a TextureInference,
    pub directory: &'a Path,
    pub person_index: usize,
    pub effect: TextureEffect,
    pub tuning: AppleTextureTuningProfile,
}

pub(crate) struct TextureStatistics {
    person: Option<TexturePerson>,
}

impl TextureStatistics {
    pub(crate) fn into_person(self) -> Option<TexturePerson> {
        self.person
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TexturePerson {
    geometry: Value,
    statistics: serde_json::Map<String, Value>,
}

impl TexturePerson {
    fn decode(mut value: Value) -> Result<Self> {
        let statistics = value
            .as_object_mut()
            .and_then(|fields| fields.remove("imageStats"))
            .ok_or_else(|| invalid("missing native imageStats"))?;
        let Value::Object(statistics) = statistics else {
            return Err(invalid("invalid effect imageStats"));
        };
        Ok(Self {
            geometry: value,
            statistics,
        })
    }

    pub(crate) fn merge_effect(&mut self, other: Self) -> Result<()> {
        if !same(&other.geometry, &self.geometry) {
            return Err(invalid("effects used different geometry"));
        }
        for (name, value) in other.statistics {
            if self.statistics.insert(name, value).is_some() {
                return Err(invalid("duplicate native statistics group"));
            }
        }
        Ok(())
    }

    pub(crate) fn bind_instance(&mut self, key: &str) {
        self.geometry["instanceMaskReferenceKey"] = json!(key);
    }

    pub(crate) fn property_list_value(&self) -> Result<AppleTextureValue> {
        texture_value(&self.wire_value())
    }

    fn wire_value(&self) -> Value {
        let mut value = self.geometry.clone();
        value["imageStats"] = Value::Object(self.statistics.clone());
        value
    }
}

pub(crate) struct TextureReadbackRequest<'a> {
    pub input: &'a Path,
    pub contents: &'a [u8],
    pub directory: &'a Path,
    pub expected_people: &'a [TexturePerson],
}

pub(crate) struct TextureExistingReadbackRequest<'a> {
    pub input: &'a Path,
    pub contents: &'a [u8],
    pub directory: &'a Path,
}

pub(crate) struct TextureReadback {
    people: Vec<TexturePerson>,
}

impl TextureReadback {
    pub(crate) fn people(&self) -> &[TexturePerson] {
        &self.people
    }
}

impl AppleAdapterClient {
    pub(crate) fn texture_prepare(
        &self,
        request: TexturePrepareRequest<'_>,
    ) -> Result<TexturePrepared> {
        #[derive(Serialize)]
        struct Options {
            max_dimension: u32,
        }
        self.invoke_texture(
            "texture-prepare",
            request.input,
            request.directory,
            Options {
                max_dimension: request.max_dimension,
            },
        )?;
        let report = read(&request.directory.join("input.json"))?;
        if report["source_sha256"] != sha(request.source_bytes) {
            return Err(invalid("analysis input is not the assembled Styles file"));
        }
        let working = request.directory.join("working.png");
        let bytes =
            fs::read(&working).map_err(|e| RuntimeError::external("Texture analysis raster", e))?;
        let working_sha256 = sha(&bytes);
        if report["working_sha256"] != working_sha256 {
            return Err(invalid("analysis raster differs from prepared input"));
        }
        Ok(TexturePrepared {
            directory: request.directory.to_owned(),
            working,
            working_sha256,
            width: unsigned(&report, "working_width")?,
            height: unsigned(&report, "working_height")?,
        })
    }

    pub(crate) fn texture_infer(
        &self,
        request: TextureInferRequest<'_>,
    ) -> Result<TextureInference> {
        let prepared = request.prepared;
        self.invoke_texture("texture-infer", &prepared.working, &prepared.directory, ())?;
        let report = read(&prepared.directory.join("carrier-production.json"))?;
        if report["stage"] != "finished"
            || report["input_sha256"] != prepared.working_sha256
            || report["width"].as_u64() != Some(prepared.width)
            || report["height"].as_u64() != Some(prepared.height)
        {
            return Err(invalid(
                "native inference is incomplete or uses a different raster",
            ));
        }
        let codecs = array(&report, "native_mask_codec_controls")?;
        if !codecs
            .iter()
            .all(|r| truth(&r["native_auxiliary_readback"]) && truth(&r["finalized"]))
        {
            return Err(invalid("ImageIO did not encode/read back every mask"));
        }
        let observations = array(&report, "people")?;
        let instances = array(&report, "instance_resources")?;
        if observations.len() != instances.len() {
            return Err(invalid("person and instance resource counts differ"));
        }
        let people = observations
            .iter()
            .map(|person| {
                Ok(TextureObservation {
                    instance_key: text(person, "instance_reference_key")?.to_owned(),
                    instance_role: text(person, "instance_role")?.to_owned(),
                    face_probability_overlap: person["face_probability_overlap"]
                        .as_f64()
                        .unwrap_or(0.0),
                    native_person: person["native_person_without_stats"].clone(),
                })
            })
            .collect::<Result<_>>()?;
        Ok(TextureInference {
            directory: prepared.directory.clone(),
            working: prepared.working.clone(),
            input_sha256: prepared.working_sha256.clone(),
            people,
            resources: array(&report, "resources")?.clone(),
            instances: instances.clone(),
            codecs: codecs.clone(),
        })
    }

    pub(crate) fn texture_statistics(
        &self,
        request: TextureStatisticsRequest<'_>,
    ) -> Result<TextureStatistics> {
        #[derive(Serialize)]
        struct Options<'a> {
            context_path: String,
            person_index: usize,
            effect: TextureEffect,
            tuning: &'a AppleTextureTuningProfile,
        }
        let inference = request.inference;
        let person = inference
            .people
            .get(request.person_index)
            .ok_or_else(|| invalid("statistics person index is out of range"))?;
        let report_path = request
            .directory
            .join(format!("{}.json", request.effect.name()));
        self.invoke_texture(
            "texture-statistics",
            &inference.working,
            &report_path,
            Options {
                context_path: input_path(&inference.directory)?,
                person_index: request.person_index,
                effect: request.effect,
                tuning: &request.tuning,
            },
        )?;
        let report = read(&report_path)?;
        if report["input_sha256"] != inference.input_sha256
            || !same(&report["person_before_stats"], &person.native_person)
            || report["actual_input_mask_textures"]["instance"]["sha256"]
                != sha(&resource(
                    &inference.directory,
                    &json!(format!("{}.l8", person.instance_role)),
                )?)
        {
            return Err(invalid(
                "statistics inputs differ from inferred person/mask",
            ));
        }
        let person = statistics(&report, request.effect == TextureEffect::Mattify)?
            .map(TexturePerson::decode)
            .transpose()?;
        Ok(TextureStatistics { person })
    }

    pub(crate) fn texture_readback(
        &self,
        request: TextureReadbackRequest<'_>,
    ) -> Result<TextureReadback> {
        let report = self.texture_readback_report(
            request.input,
            &request.directory.join("final-native-readback.json"),
        )?;
        let expected = request
            .expected_people
            .iter()
            .map(TexturePerson::wire_value)
            .collect::<Vec<_>>();
        validate_readback(&report, &expected, request.contents)?;
        decode_readback(&report)
    }

    pub(crate) fn texture_readback_existing(
        &self,
        request: TextureExistingReadbackRequest<'_>,
    ) -> Result<TextureReadback> {
        let report = self.texture_readback_report(
            request.input,
            &request.directory.join("native-readback.json"),
        )?;
        validate_readback(
            &report,
            array(&report, "native_person_data_readback")?,
            request.contents,
        )?;
        decode_readback(&report)
    }

    fn texture_readback_report(&self, input: &Path, output: &Path) -> Result<Value> {
        self.invoke_texture("texture-readback", input, output, ())?;
        read(output)
    }

    fn invoke_texture<T: Serialize>(
        &self,
        operation: &'static str,
        input: &Path,
        output: &Path,
        texture: T,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct Request<T> {
            schema_version: u32,
            operation: &'static str,
            input_path: String,
            output_path: String,
            texture: T,
        }
        let bytes = serde_json::to_vec(&Request {
            schema_version: APPLE_ADAPTER_SCHEMA_VERSION,
            operation,
            input_path: input_path(input)?,
            output_path: input_path(output)?,
            texture,
        })
        .map_err(|e| RuntimeError::external("Texture adapter request", e))?;
        let bytes = self.invoke(&bytes, APPLE_COMPUTE_TIMEOUT)?;
        let response: AckResponse = serde_json::from_slice(&bytes)
            .map_err(|e| RuntimeError::external("Texture adapter response", e))?;
        validate_schema(response.schema_version)
    }
}

impl TextureInference {
    pub(crate) fn people(&self) -> &[TextureObservation] {
        &self.people
    }

    pub(crate) fn masks(&self) -> Result<Vec<TextureMask>> {
        if self.resources.len() != 13 {
            return Err(invalid("incomplete Texture role set"));
        }
        self.resources
            .iter()
            .chain(&self.instances)
            .map(|row| {
                let raw = resource(&self.directory, &row["file"])?;
                if row["sha256"] != sha(&raw) || row["size"].as_u64() != Some(raw.len() as u64) {
                    return Err(invalid("inferred raw mask binding failed"));
                }
                let codec = self
                    .codecs
                    .iter()
                    .find(|c| c["role"] == row["role"])
                    .ok_or_else(|| invalid("missing native mask codec"))?;
                Ok(TextureMask {
                    auxiliary_type: text(row, "uri")?.to_owned(),
                    instance_reference_key: row["reference_key"].as_str().map(str::to_owned),
                    codec: resource(&self.directory, &codec["file"])?,
                })
            })
            .collect()
    }
}

fn invalid(detail: impl Into<String>) -> RuntimeError {
    RuntimeError::new("Texture Styles generation", detail)
}
fn truth(v: &Value) -> bool {
    v.as_bool() == Some(true) || v.as_u64() == Some(1)
}
fn read(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).map_err(|e| RuntimeError::external("Texture native report", e))?;
    let report: Value = serde_json::from_slice(&bytes)
        .map_err(|e| RuntimeError::external("Texture native report", e))?;
    if !report["exception"].is_null() {
        return Err(invalid(format!("native failure: {}", report["exception"])));
    }
    Ok(report)
}
fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
fn resource(root: &Path, name: &Value) -> Result<Vec<u8>> {
    let name = name
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| invalid("missing resource filename"))?;
    if Path::new(name).components().count() != 1 || name == "." || name == ".." {
        return Err(invalid("native resource escapes staging"));
    }
    fs::read(root.join(name)).map_err(|e| RuntimeError::external("Texture native resource", e))
}
fn array<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>> {
    value[name]
        .as_array()
        .ok_or_else(|| invalid(format!("missing native {name}")))
}
fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value[name]
        .as_str()
        .ok_or_else(|| invalid(format!("missing native {name}")))
}
fn unsigned(value: &Value, name: &str) -> Result<u64> {
    value[name]
        .as_u64()
        .ok_or_else(|| invalid(format!("missing native {name}")))
}

fn statistics(report: &Value, required: bool) -> Result<Option<Value>> {
    for stage in [
        "setup",
        "prewarm",
        "prepareToProcess:",
        "calculateStats",
        "process",
        "finishProcessing",
        "parameter_validation",
    ] {
        if report[stage].as_i64() != Some(0) {
            return Err(invalid(format!("native {stage} did not succeed")));
        }
    }
    let gpu = &report["native_GPU_execution"];
    if !gpu["failures"].as_array().is_some_and(Vec::is_empty)
        || gpu["status"].as_i64().is_none()
        || gpu["completed_command_buffers"].as_u64().is_none()
    {
        return Err(invalid(
            "native GPU execution failed or its receipt is incomplete",
        ));
    }
    if !truth(&report["native_statistics_operations_completed"])
        || report["terminal_queue_fence_status"].as_u64() != Some(4)
        || report["image_upload_status"].as_u64() != Some(4)
        || report["calculateStats_command"]["status"].as_u64() != Some(4)
        || report["process_command"]["status"].as_u64() != Some(4)
        || report["image_pixels"]["nonfinite"].as_u64() != Some(0)
        || report["native_call_order"]
            != json!([
                "setup",
                "prewarm",
                "prepareToProcess:",
                "calculateStats",
                "process",
                "finishProcessing"
            ])
    {
        return Err(invalid(
            "native GPU work or terminal lifecycle did not complete",
        ));
    }
    if truth(&report["native_returned_empty_statistics"]) {
        if required {
            return Err(invalid("native Mattify statistics are empty"));
        }
        return Ok(None);
    }
    if !truth(&report["statistics_generation_completed"]) {
        return Err(invalid("missing measured statistics"));
    }
    Ok(Some(report["normalized_person_with_stats"].clone()))
}

fn texture_value(value: &Value) -> Result<AppleTextureValue> {
    Ok(match value {
        Value::Bool(v) => AppleTextureValue::Bool(*v),
        Value::Number(v) => {
            if let Some(v) = v.as_u64() {
                AppleTextureValue::Integer(v)
            } else {
                AppleTextureValue::Real(
                    v.as_f64()
                        .filter(|v| v.is_finite())
                        .ok_or_else(|| invalid("non-finite native value"))?,
                )
            }
        }
        Value::String(v) => AppleTextureValue::String(v.clone()),
        Value::Array(v) => {
            AppleTextureValue::Array(v.iter().map(texture_value).collect::<Result<_>>()?)
        }
        Value::Object(v) => AppleTextureValue::Dictionary(
            v.iter()
                .map(|(k, v)| Ok((k.clone(), texture_value(v)?)))
                .collect::<Result<_>>()?,
        ),
        Value::Null => return Err(invalid("native metadata contains a null value")),
    })
}

fn same(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(a), Value::Object(b)) => {
            b.iter().all(|(k, v)| a.get(k).is_some_and(|a| same(a, v)))
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
        }
        (Value::Number(a), Value::Number(b)) => a
            .as_f64()
            .zip(b.as_f64())
            .is_some_and(|(a, b)| (a - b).abs() <= 1e-5 * (1.0 + b.abs())),
        _ => actual == expected,
    }
}
fn validate_readback(report: &Value, people: &[Value], bytes: &[u8]) -> Result<()> {
    if report["input_sha256"] != sha(bytes)
        || !truth(&report["ImageIO_decoded"])
        || !truth(&report["asset_loaded"])
        || !truth(&report["texture_properties_present"])
        || !truth(&report["semantic_properties_present"])
        || report["texture_person_count"].as_u64() != Some(people.len() as u64)
    {
        return Err(invalid("final file failed original Apple parser admission"));
    }
    let stored = array(report, "native_person_data_readback")?;
    if stored.len() != people.len() {
        return Err(invalid("native parser dropped measured people"));
    }
    for person in people {
        let native = stored
            .iter()
            .find(|v| v["instanceMaskReferenceKey"] == person["instanceMaskReferenceKey"])
            .ok_or_else(|| invalid("native parser lost instance binding"))?;
        if !same(native, person) {
            return Err(invalid(
                "final native person readback differs from measured data",
            ));
        }
    }
    Ok(())
}
fn decode_readback(report: &Value) -> Result<TextureReadback> {
    let people = array(report, "native_person_data_readback")?
        .iter()
        .cloned()
        .map(TexturePerson::decode)
        .collect::<Result<_>>()?;
    Ok(TextureReadback { people })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_ack_cannot_replace_required_texture_reports() {
        use std::os::unix::fs::PermissionsExt;
        use xdremux_engine::apple_texture_tuning_profile;

        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let helper = root.join("ack-only.sh");
        fs::write(&helper, "#!/bin/sh\nwhile IFS= read -r request; do\n printf '%s\\n' '{\"schema_version\":2}'\ndone\n").unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
        let adapter = AppleAdapterClient::new(&helper);
        let source = b"assembled Styles file";
        let input = root.join("input.heic");
        fs::write(&input, source).unwrap();
        let prepare = || {
            adapter.texture_prepare(TexturePrepareRequest {
                input: &input,
                source_bytes: source,
                directory: root,
                max_dimension: 1024,
            })
        };
        assert!(
            prepare().is_err(),
            "helper acknowledgement has no analysis report"
        );
        fs::write(root.join("input.json"), b"{").unwrap();
        assert!(prepare().is_err(), "malformed report must fail");
        fs::write(root.join("working.png"), b"upright raster").unwrap();
        fs::write(
            root.join("input.json"),
            serde_json::to_vec(&json!({
                "source_sha256": sha(source), "working_sha256": sha(b"upright raster"),
                "working_width": 16, "working_height": 12
            }))
            .unwrap(),
        )
        .unwrap();
        let prepared = prepare().unwrap();
        assert!(adapter
            .texture_infer(TextureInferRequest {
                prepared: &prepared
            })
            .is_err());
        let geometry = json!({"facePitch": -2.5, "faceYaw": 1, "faceRoll": 0});
        fs::write(
            root.join("carrier-production.json"),
            serde_json::to_vec(&json!({
                "stage": "finished", "input_sha256": sha(b"upright raster"),
                "width": 16, "height": 12, "native_mask_codec_controls": [],
                "people": [{"instance_reference_key": "one", "instance_role": "instance-1",
                    "face_probability_overlap": 0.5, "native_person_without_stats": geometry}],
                "instance_resources": [{}], "resources": []
            }))
            .unwrap(),
        )
        .unwrap();
        let inference = adapter
            .texture_infer(TextureInferRequest {
                prepared: &prepared,
            })
            .unwrap();
        fs::write(root.join("instance-1.l8"), b"instance pixels").unwrap();
        let measure = |effect| {
            adapter.texture_statistics(TextureStatisticsRequest {
                inference: &inference,
                directory: root,
                person_index: 0,
                effect,
                tuning: apple_texture_tuning_profile(),
            })
        };
        assert!(measure(TextureEffect::Mattify).is_err());
        let mut report = completed_statistics_report();
        report["input_sha256"] = json!(sha(b"upright raster"));
        report["person_before_stats"] = geometry.clone();
        report["actual_input_mask_textures"] =
            json!({"instance": {"sha256": sha(b"instance pixels")}});
        report["normalized_person_with_stats"] = geometry;
        report["normalized_person_with_stats"]["imageStats"] =
            json!({"Mattify": {"SkipPerson": false}});
        let report_path = root.join("Mattify.json");
        fs::write(&report_path, serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(measure(TextureEffect::Mattify)
            .unwrap()
            .into_person()
            .is_some());
        for field in [
            "input_sha256",
            "person_before_stats",
            "actual_input_mask_textures",
            "native_GPU_execution",
        ] {
            let mut incomplete = report.clone();
            incomplete.as_object_mut().unwrap().remove(field);
            fs::write(&report_path, serde_json::to_vec(&incomplete).unwrap()).unwrap();
            assert!(measure(TextureEffect::Mattify).is_err(), "missing {field}");
        }
        report["native_returned_empty_statistics"] = json!(true);
        fs::write(
            root.join("UnderEyeBrighten.json"),
            serde_json::to_vec(&report).unwrap(),
        )
        .unwrap();
        assert!(measure(TextureEffect::UnderEyeBrighten)
            .unwrap()
            .into_person()
            .is_none());
        let readback = || {
            adapter.texture_readback(TextureReadbackRequest {
                input: &input,
                contents: source,
                directory: root,
                expected_people: &[],
            })
        };
        assert!(readback().is_err());
        fs::write(root.join("final-native-readback.json"), b"{}").unwrap();
        assert!(
            readback().is_err(),
            "ack plus an empty report is not final-file admission"
        );
    }

    #[test]
    fn measured_effects_keep_geometry_and_distinct_statistics_groups() {
        let person = |pitch: f64, group: &str| {
            TexturePerson::decode(json!({
                "facePitch": pitch, "faceYaw": 0, "faceRoll": 0,
                "imageStats": {group: {"SkipPerson": false}}
            }))
            .unwrap()
        };
        let mut combined = person(-2.5, "Mattify");
        combined
            .merge_effect(person(-2.5, "UnderEyeBrighten"))
            .unwrap();
        combined.bind_instance("one");
        assert_eq!(combined.wire_value()["instanceMaskReferenceKey"], "one");
        assert_eq!(combined.statistics.len(), 2);
        assert!(person(-2.5, "Mattify")
            .merge_effect(person(12.0, "UnderEyeBrighten"))
            .is_err());
        assert!(person(-2.5, "Mattify")
            .merge_effect(person(-2.5, "Mattify"))
            .is_err());
    }

    fn completed_statistics_report() -> Value {
        json!({
            "setup": 0,
            "prewarm": 0,
            "prepareToProcess:": 0,
            "calculateStats": 0,
            "process": 0,
            "finishProcessing": 0,
            "parameter_validation": 0,
            "native_statistics_operations_completed": true,
            "terminal_queue_fence_status": 4,
            "image_upload_status": 4,
            "calculateStats_command": {"status": 4},
            "process_command": {"status": 4},
            "image_pixels": {"nonfinite": 0},
            "native_GPU_execution": {
                "status": 0,
                "completed_command_buffers": 0,
                "failures": []
            },
            "native_call_order": ["setup", "prewarm", "prepareToProcess:",
                "calculateStats", "process", "finishProcessing"],
            "statistics_generation_completed": true,
            "normalized_person_with_stats": {"imageStats": {"Mattify": {"SkipPerson": false}}}
        })
    }

    #[test]
    fn internal_gpu_failure_rejects_even_when_explicit_commands_completed() {
        let mut report = completed_statistics_report();
        assert_eq!(
            statistics(&report, true).unwrap(),
            Some(report["normalized_person_with_stats"].clone())
        );
        report["native_GPU_execution"]["failures"] =
            json!([{"status": 5, "error": "native internal command failed"}]);
        assert!(statistics(&report, true).is_err());
        report["native_returned_empty_statistics"] = json!(true);
        assert!(statistics(&report, false).is_err());
    }

    #[test]
    fn missing_or_malformed_gpu_receipts_cannot_admit_optional_empty_statistics() {
        let mut complete = completed_statistics_report();
        complete["native_returned_empty_statistics"] = json!(true);
        assert_eq!(statistics(&complete, false).unwrap(), None);
        assert!(statistics(&complete, true).is_err());
        for field in ["failures", "status", "completed_command_buffers"] {
            let mut report = complete.clone();
            report["native_GPU_execution"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(statistics(&report, false).is_err(), "missing {field}");
        }
        for receipt in [
            Value::Null,
            json!({"failures": false, "status": 0,
            "completed_command_buffers": 0}),
        ] {
            let mut report = complete.clone();
            report["native_GPU_execution"] = receipt;
            assert!(statistics(&report, false).is_err());
        }
    }

    #[test]
    fn missing_texture_and_partial_gpu_work_fail_before_publication() {
        assert!(statistics(&json!({"statistics_generation_completed":true}), true).is_err());
        let data = b"existing output";
        let report = json!({"input_sha256":sha(data),"ImageIO_decoded":true,"asset_loaded":true,
            "semantic_properties_present":true,"texture_properties_present":false,"texture_person_count":0,
            "native_person_data_readback":[]});
        assert!(validate_readback(&report, &[], data).is_err());
    }
    #[test]
    fn empty_people_context_admits_but_a_dropped_instance_fails() {
        let data = b"final file";
        let report = json!({"input_sha256":sha(data),"ImageIO_decoded":1,"asset_loaded":1,
            "semantic_properties_present":1,"texture_properties_present":1,"texture_person_count":0,
            "native_person_data_readback":[]});
        validate_readback(&report, &[], data).unwrap();
        assert!(
            validate_readback(&report, &[json!({"instanceMaskReferenceKey":"one"})], data).is_err()
        );
    }
}
