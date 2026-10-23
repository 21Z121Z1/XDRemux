//! One image-bound Texture transaction, coordinated by the Rust Styles owner.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::apple_adapter::texture::{
    TextureEffect, TextureExistingReadbackRequest, TextureInferRequest, TexturePerson,
    TexturePrepareRequest, TextureReadbackRequest, TextureStatisticsRequest,
};
use crate::apple_adapter::AppleAdapterClient;
use crate::{Result, RuntimeError};
use xdremux_engine::{
    apple_style_maker_apple_dictionary, apple_texture_property_list, apple_texture_tuning_profile,
    APPLE_TEXTURE_ANALYSIS_MAX_DIMENSION,
};
use xdremux_heif::TextureStyleMask;

fn invalid(detail: impl Into<String>) -> RuntimeError {
    RuntimeError::new("Texture Styles generation", detail)
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
    let prepared = adapter.texture_prepare(TexturePrepareRequest {
        input,
        source_bytes: source,
        directory: &root,
        max_dimension: APPLE_TEXTURE_ANALYSIS_MAX_DIMENSION,
    })?;
    let inference = adapter.texture_infer(TextureInferRequest {
        prepared: &prepared,
    })?;
    let mut people = Vec::new();
    let mut keys = BTreeSet::new();
    let mut roles = BTreeSet::new();
    for (index, observation) in inference.people().iter().enumerate() {
        if !keys.insert(&observation.instance_key)
            || !roles.insert(&observation.instance_role)
            || observation.face_probability_overlap < 0.02
        {
            return Err(invalid("ambiguous face-to-instance assignment"));
        }
        let person_dir = root.join(format!("person-{index}"));
        fs::create_dir(&person_dir)
            .map_err(|e| RuntimeError::external("Texture person staging", e))?;
        let mut combined: Option<TexturePerson> = None;
        for effect in [
            TextureEffect::Mattify,
            TextureEffect::UnderEyeBrighten,
            TextureEffect::SkinSmoothStandalone,
        ] {
            let statistics = adapter.texture_statistics(TextureStatisticsRequest {
                inference: &inference,
                directory: &person_dir,
                person_index: index,
                effect,
                tuning: apple_texture_tuning_profile(),
            })?;
            if let Some(person) = statistics.into_person() {
                if let Some(combined) = &mut combined {
                    combined.merge_effect(person)?;
                } else {
                    combined = Some(person);
                }
            }
        }
        let mut person = combined.ok_or_else(|| invalid("no native person statistics"))?;
        person.bind_instance(&observation.instance_key);
        people.push(person);
    }
    let masks = inference.masks()?;
    let masks: Vec<_> = masks
        .iter()
        .map(|mask| TextureStyleMask {
            auxiliary_type: &mask.auxiliary_type,
            codec: &mask.codec,
            instance_reference_key: mask.instance_reference_key.as_deref(),
        })
        .collect();
    let plist = metadata(&people)?;
    let assembled = xdremux_heif::assemble_texture_styles_heif(source, &plist, &masks)
        .map_err(|e| RuntimeError::external("Rust Texture graph", e))?;
    let final_path = root.join("final.heic");
    fs::write(&final_path, &assembled)
        .map_err(|e| RuntimeError::external("Texture final staging", e))?;
    adapter.texture_readback(TextureReadbackRequest {
        input: &final_path,
        contents: &assembled,
        directory: &root,
        expected_people: &people,
    })?;
    Ok(assembled)
}

fn metadata(people: &[TexturePerson]) -> Result<Vec<u8>> {
    let people = people
        .iter()
        .map(TexturePerson::property_list_value)
        .collect::<Result<Vec<_>>>()?;
    apple_texture_property_list(&people).map_err(|e| RuntimeError::external("Texture metadata", e))
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
    let adapter = AppleAdapterClient::new(executable.to_path_buf());
    let properties = adapter.imageio_image_properties(input)?;
    crate::apple_styles::validate_maker_apple_properties(
        &apple_style_maker_apple_dictionary(),
        &properties.maker_apple_integer_dictionaries,
    )?;
    let readback = adapter.texture_readback_existing(TextureExistingReadbackRequest {
        input,
        contents: &bytes,
        directory: temporary.path(),
    })?;
    let _ = metadata(readback.people())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
