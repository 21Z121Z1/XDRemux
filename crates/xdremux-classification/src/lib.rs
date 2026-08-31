#![forbid(unsafe_code)]

mod capabilities;
mod model;
mod oppo;

pub use capabilities::detect_capabilities;
pub use model::{
    CameraVendor, OppoCaptureMode, OppoFlagEvidence, OppoPhotoClassification,
    OppoPhotoClassificationStatus, PhotoAsset, PhotoAssetType, PhotoCapability, PhotoClassification,
    PhotoClassificationContract, PhotoMetadataReadStatus, PhotoResource, PhotoResourceRole,
    CLASSIFICATION_LAYOUT_VERSION, UNCLASSIFIED_FOLDER_NAME,
};
pub use oppo::{
    classification_contract, classify_user_comment, classify_user_comment_with_context, parse_flags,
};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::Value;

    use super::*;

    #[test]
    fn matches_shared_swift_python_golden_contract() {
        let raw = include_str!("../../../Tests/fixtures/photo_classification_cases.json");
        let Value::Array(cases) = serde_json::from_str::<Value>(raw).unwrap() else {
            panic!("classification fixture must be a JSON array");
        };

        for mut item in cases {
            let object = item.as_object_mut().expect("fixture entry must be an object");
            let name = object
                .remove("name")
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .expect("fixture entry must have a name");
            let user_comment = object.remove("user_comment").and_then(|value| match value {
                Value::Null => None,
                Value::String(value) => Some(value),
                other => panic!("{name}: invalid user_comment {other}"),
            });
            let asset_type = match object
                .get("asset_type")
                .and_then(Value::as_str)
                .expect("fixture entry must have asset_type")
            {
                "static-photo" => PhotoAssetType::StaticPhoto,
                "live-photo" => PhotoAssetType::LivePhoto,
                value => panic!("{name}: unsupported asset type {value}"),
            };
            let classification = classify_user_comment_with_context(
                user_comment.as_deref(),
                asset_type,
                Default::default(),
            );
            let actual = serde_json::to_value(classification_contract(&classification)).unwrap();
            assert_eq!(actual, Value::Object(object.clone()), "{name}");
        }
    }

    #[test]
    fn photo_asset_keeps_live_photo_resources_together() {
        let asset = PhotoAsset::live_photo("IMG.heic", "IMG.mov", "asset-id");
        assert_eq!(asset.asset_type, PhotoAssetType::LivePhoto);
        assert_eq!(asset.primary_image(), Some(PathBuf::from("IMG.heic").as_path()));
        assert_eq!(
            asset
                .resources
                .iter()
                .map(|resource| resource.role)
                .collect::<Vec<_>>(),
            vec![
                PhotoResourceRole::PrimaryImage,
                PhotoResourceRole::PairedVideo,
            ]
        );
    }
}
