//! One image-bound Texture transaction, coordinated by the Rust Styles owner.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::apple_adapter::AppleAdapterClient;
use crate::{Result, RuntimeError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use xdremux_engine::{
    apple_style_maker_apple_dictionary, apple_texture_property_list, apple_texture_tuning_profile,
    AppleTextureValue, APPLE_TEXTURE_ANALYSIS_MAX_DIMENSION,
};
use xdremux_heif::TextureStyleMask;

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

/// Run the same producers as the verified research path, but import their
/// resources into the full-resolution HDR Styles file instead of its SDR copy.
pub(super) fn generate(
    adapter: &AppleAdapterClient,
    input: &Path,
    source: &[u8],
    staging: &Path,
) -> Result<Vec<u8>> {
    let root = staging.join("texture");
    fs::create_dir(&root).map_err(|e| RuntimeError::external("Texture staging", e))?;
    adapter.texture_primitive(
        "texture-prepare",
        input,
        &root,
        json!({"max_dimension":APPLE_TEXTURE_ANALYSIS_MAX_DIMENSION}),
    )?;
    let prepared = read(&root.join("input.json"))?;
    if prepared["source_sha256"] != sha(source) {
        return Err(invalid("analysis input is not the assembled Styles file"));
    }
    let working = root.join("working.png");
    let working_bytes =
        fs::read(&working).map_err(|e| RuntimeError::external("Texture analysis raster", e))?;
    adapter.texture_primitive("texture-infer", &working, &root, Value::Null)?;
    let production = read(&root.join("carrier-production.json"))?;
    if production["stage"] != "finished"
        || production["input_sha256"] != sha(&working_bytes)
        || production["input_sha256"] != prepared["working_sha256"]
        || production["width"] != prepared["working_width"]
        || production["height"] != prepared["working_height"]
    {
        return Err(invalid(
            "native inference is incomplete or uses a different raster",
        ));
    }
    let controls = array(&production, "native_mask_codec_controls")?;
    if !controls
        .iter()
        .all(|r| truth(&r["native_auxiliary_readback"]) && truth(&r["finalized"]))
    {
        return Err(invalid("ImageIO did not encode/read back every mask"));
    }
    let observations = array(&production, "people")?;
    let instances = array(&production, "instance_resources")?;
    if instances.len() != observations.len() {
        return Err(invalid("person and instance resource counts differ"));
    }
    let mut people = Vec::new();
    let mut keys = BTreeSet::new();
    let mut roles = BTreeSet::new();
    for (index, person) in observations.iter().enumerate() {
        let key = person["instance_reference_key"]
            .as_str()
            .ok_or_else(|| invalid("person has no instance key"))?;
        let role = person["instance_role"]
            .as_str()
            .ok_or_else(|| invalid("person has no instance mask"))?;
        if !keys.insert(key)
            || !roles.insert(role)
            || person["face_probability_overlap"].as_f64().unwrap_or(0.0) < 0.02
        {
            return Err(invalid("ambiguous face-to-instance assignment"));
        }
        let person_dir = root.join(format!("person-{index}"));
        fs::create_dir(&person_dir)
            .map_err(|e| RuntimeError::external("Texture person staging", e))?;
        let mut combined: Option<Value> = None;
        for effect in ["Mattify", "UnderEyeBrighten", "SkinSmoothStandalone"] {
            let report_path = person_dir.join(format!("{effect}.json"));
            adapter.texture_primitive("texture-statistics", &working, &report_path, json!({
                "context_path":root,"person_index":index,"effect":effect,"tuning":apple_texture_tuning_profile()}))?;
            let report = read(&report_path)?;
            if report["input_sha256"] != production["input_sha256"]
                || !same(
                    &report["person_before_stats"],
                    &person["native_person_without_stats"],
                )
                || report["actual_input_mask_textures"]["instance"]["sha256"]
                    != sha(&resource(&root, &json!(format!("{role}.l8")))?)
            {
                return Err(invalid(
                    "statistics inputs differ from inferred person/mask",
                ));
            }
            let normalized = statistics(&report, effect == "Mattify")?;
            if let Some(mut normalized) = normalized {
                if let Some(combined) = &mut combined {
                    let stats = normalized
                        .as_object_mut()
                        .and_then(|p| p.remove("imageStats"))
                        .ok_or_else(|| invalid("missing native imageStats"))?;
                    let mut geometry = combined.clone();
                    geometry.as_object_mut().unwrap().remove("imageStats");
                    if !same(&normalized, &geometry) {
                        return Err(invalid("effects used different geometry"));
                    }
                    let target = combined["imageStats"]
                        .as_object_mut()
                        .ok_or_else(|| invalid("missing Mattify imageStats"))?;
                    for (name, value) in stats
                        .as_object()
                        .ok_or_else(|| invalid("invalid effect imageStats"))?
                    {
                        if target.insert(name.clone(), value.clone()).is_some() {
                            return Err(invalid("duplicate native statistics group"));
                        }
                    }
                } else {
                    combined = Some(normalized);
                }
            }
        }
        let mut person = combined.ok_or_else(|| invalid("no native person statistics"))?;
        person["instanceMaskReferenceKey"] = json!(key);
        people.push(person);
    }
    let resources = array(&production, "resources")?;
    if resources.len() != 13 {
        return Err(invalid("incomplete Texture role set"));
    }
    let mut codecs = Vec::new();
    let mut descriptors = Vec::new();
    for row in resources.iter().chain(instances) {
        let raw = resource(&root, &row["file"])?;
        if row["sha256"] != sha(&raw) || row["size"].as_u64() != Some(raw.len() as u64) {
            return Err(invalid("inferred raw mask binding failed"));
        }
        let codec = controls
            .iter()
            .find(|c| c["role"] == row["role"])
            .ok_or_else(|| invalid("missing native mask codec"))?;
        codecs.push(resource(&root, &codec["file"])?);
        let uri = row["uri"]
            .as_str()
            .ok_or_else(|| invalid("mask has no auxiliary type"))?;
        descriptors.push((uri, row["reference_key"].as_str()));
    }
    let masks: Vec<_> = descriptors
        .iter()
        .zip(&codecs)
        .map(|((uri, key), codec)| TextureStyleMask {
            auxiliary_type: uri,
            codec,
            instance_reference_key: *key,
        })
        .collect();
    let plist = metadata(&people)?;
    let assembled = xdremux_heif::assemble_texture_styles_heif(source, &plist, &masks)
        .map_err(|e| RuntimeError::external("Rust Texture graph", e))?;
    let final_path = root.join("final.heic");
    fs::write(&final_path, &assembled)
        .map_err(|e| RuntimeError::external("Texture final staging", e))?;
    let report_path = root.join("final-native-readback.json");
    adapter.texture_primitive("texture-readback", &final_path, &report_path, Value::Null)?;
    let report = read(&report_path)?;
    validate_readback(&report, &people, &assembled)?;
    Ok(assembled)
}

fn metadata(people: &[Value]) -> Result<Vec<u8>> {
    let people = people
        .iter()
        .map(texture_value)
        .collect::<Result<Vec<_>>>()?;
    apple_texture_property_list(&people).map_err(|e| RuntimeError::external("Texture metadata", e))
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

/// The app must not reuse a plain HDR output for the combined Styles intent.
pub(super) fn validate_existing(executable: &Path, input: &Path) -> Result<()> {
    let bytes =
        fs::read(input).map_err(|e| RuntimeError::external("Styles validation input", e))?;
    if !xdremux_heif::has_texture_style_metadata(&bytes)
        .map_err(|e| RuntimeError::external("Styles Texture validation", e))?
    {
        return Err(invalid("output has no generated Texture metadata"));
    }
    let temporary =
        tempfile::tempdir().map_err(|e| RuntimeError::external("Styles validation staging", e))?;
    let path = temporary.path().join("native-readback.json");
    let adapter = AppleAdapterClient::new(executable.to_path_buf());
    let properties = adapter.imageio_image_properties(input)?;
    crate::apple_styles::validate_maker_apple_properties(
        &apple_style_maker_apple_dictionary(),
        &properties.maker_apple_integer_dictionaries,
    )?;
    adapter.texture_primitive("texture-readback", input, &path, Value::Null)?;
    let report = read(&path)?;
    let people = array(&report, "native_person_data_readback")?;
    let _ = metadata(people)?;
    validate_readback(&report, people, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn texture_validation_fixture() -> Vec<u8> {
        use xdremux_format::isobmff::{
            make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
            make_pitm_box, IlocEntry, IlocExtent, FTYP, IDAT, IPCO, IPRP, META,
        };
        use xdremux_format::FourCC;

        let locations = [IlocEntry {
            item_id: 1,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0,
                length: 4,
            }],
        }];
        let children = [
            make_pitm_box(0, 1).unwrap(),
            make_iinf_box(0, &[make_infe_box(1, FourCC::new(*b"hvc1"), 0).unwrap()]).unwrap(),
            make_iloc_box(1, 4, 4, 0, 0, &locations).unwrap(),
            make_box(
                IPRP,
                &[
                    make_box(IPCO, &[]).unwrap(),
                    make_ipma_box(0, 0, &[]).unwrap(),
                ]
                .concat(),
            )
            .unwrap(),
            make_box(IDAT, b"base").unwrap(),
        ]
        .concat();
        let source = [
            make_box(FTYP, b"mif1\0\0\0\0mif1heic").unwrap(),
            make_full_box(META, 0, 0, &children).unwrap(),
        ]
        .concat();
        xdremux_heif::assemble_texture_styles_heif(&source, &metadata(&[]).unwrap(), &[]).unwrap()
    }

    #[test]
    fn existing_styles_require_edit_state_even_when_texture_parser_would_admit() {
        use std::collections::BTreeMap;
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().unwrap();
        let helper = temporary.path().join("fake-apple-adapter.py");
        fs::write(
            &helper,
            r#"#!/usr/bin/env python3
import hashlib, json, pathlib, sys
assert sys.argv[1:] == ['--persistent-json-lines']
root = pathlib.Path(sys.argv[0])
with open(str(root) + '.launches', 'a') as log:
    log.write('launch\n')
for line in sys.stdin:
    request = json.loads(line)
    with open(str(root) + '.requests', 'a') as log:
        log.write(request['operation'] + '\n')
    if request['operation'] == 'imageio-image-properties':
        properties = json.loads(pathlib.Path(str(root) + '.properties.json').read_text())
        response = {'schema_version': 2, 'image_properties': properties}
    elif request['operation'] == 'texture-readback':
        data = pathlib.Path(request['input_path']).read_bytes()
        facts = {'input_sha256': hashlib.sha256(data).hexdigest(), 'ImageIO_decoded': True,
                 'asset_loaded': True, 'texture_properties_present': True,
                 'semantic_properties_present': True, 'texture_person_count': 0,
                 'native_person_data_readback': []}
        pathlib.Path(request['output_path']).write_text(json.dumps(facts))
        response = {'schema_version': 2}
    else:
        raise AssertionError(request['operation'])
    print(json.dumps(response), flush=True)
"#,
        )
        .unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
        let input = temporary.path().join("existing.heic");
        let bytes = texture_validation_fixture();
        fs::write(&input, &bytes).unwrap();
        let expected = apple_style_maker_apple_dictionary();
        let mut missing_field = expected.clone();
        missing_field.get_mut("84").unwrap().remove("8");
        let mut wrong_field = expected.clone();
        wrong_field.get_mut("84").unwrap().insert("8".to_owned(), 4);
        let properties_path = helper.with_extension("py.properties.json");
        for fields in [BTreeMap::new(), missing_field, wrong_field] {
            fs::write(
                &properties_path,
                serde_json::to_vec(&json!({
                    "width": 1, "height": 1, "maker_apple_integer_dictionaries": fields
                }))
                .unwrap(),
            )
            .unwrap();
            let error = crate::validate_apple_styles_file(&helper, &input).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("ImageIO did not retain MakerApple"),
                "{error}"
            );
        }
        fs::write(
            &properties_path,
            serde_json::to_vec(&json!({
                "width": 1, "height": 1, "maker_apple_integer_dictionaries": expected
            }))
            .unwrap(),
        )
        .unwrap();
        crate::validate_apple_styles_file(&helper, &input).unwrap();
        let requests = fs::read_to_string(helper.with_extension("py.requests")).unwrap();
        assert_eq!(
            requests.lines().collect::<Vec<_>>(),
            [
                "imageio-image-properties",
                "imageio-image-properties",
                "imageio-image-properties",
                "imageio-image-properties",
                "texture-readback"
            ]
        );
        let launches = fs::read_to_string(helper.with_extension("py.launches")).unwrap();
        assert_eq!(
            launches.lines().count(),
            4,
            "the successful check must reuse one adapter process"
        );
        assert_eq!(fs::read(&input).unwrap(), bytes);
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
