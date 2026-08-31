use std::collections::BTreeSet;

use serde_json::Value;

use crate::model::{
    CameraVendor, OppoCaptureMode, OppoFlagEvidence, OppoPhotoClassification, PhotoAssetType,
    PhotoCapability, PhotoClassification, PhotoClassificationContract, PhotoMetadataReadStatus,
};

const KNOWN_FLAGS_MASK: u64 = 0x4000_0003_ffff_ffff;
const MAPPED_FLAGS_MASK: u64 = 0x1_8040_fb1e;

fn parse_json_flags(raw: &str) -> Option<u64> {
    let parsed = serde_json::from_str::<Value>(raw).ok()?;
    let object = parsed.as_object()?;
    let value = object
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("oplustag"))?
        .1;
    if let Some(number) = value.as_u64() {
        return Some(number);
    }
    value.as_str()?.trim().parse().ok()
}

fn parse_prefixed_flags(raw: &str) -> Option<u64> {
    let normalized = raw.replace('\0', "");
    let lower = normalized.to_ascii_lowercase();
    for prefix in ["oplus_", "oppo_"] {
        let Some(start) = lower.find(prefix) else {
            continue;
        };
        let digits_start = start.checked_add(prefix.len())?;
        let suffix = normalized.get(digits_start..)?;
        let digit_count = suffix.bytes().take_while(u8::is_ascii_digit).count();
        if digit_count == 0 {
            continue;
        }
        return suffix.get(..digit_count)?.parse().ok();
    }
    None
}

pub fn parse_flags(raw: &str) -> Option<u64> {
    parse_json_flags(raw).or_else(|| parse_prefixed_flags(raw))
}

pub fn classify_user_comment(raw: Option<&str>) -> PhotoClassification {
    classify_user_comment_with_context(raw, PhotoAssetType::StaticPhoto, BTreeSet::new())
}

pub fn classify_user_comment_with_context(
    raw: Option<&str>,
    asset_type: PhotoAssetType,
    capabilities: BTreeSet<PhotoCapability>,
) -> PhotoClassification {
    let Some(trimmed) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        let evidence = OppoPhotoClassification {
            raw_user_comment: raw.map(ToOwned::to_owned),
            flag_evidence: None,
            capture_modes: BTreeSet::new(),
            metadata_status: PhotoMetadataReadStatus::MissingUserComment,
        };
        return PhotoClassification {
            asset_type,
            capture_modes: BTreeSet::new(),
            capabilities,
            vendor: None,
            evidence,
        };
    };

    let Some(flags) = parse_flags(trimmed) else {
        let evidence = OppoPhotoClassification {
            raw_user_comment: Some(trimmed.to_owned()),
            flag_evidence: None,
            capture_modes: BTreeSet::new(),
            metadata_status: PhotoMetadataReadStatus::MalformedUserComment,
        };
        return PhotoClassification {
            asset_type,
            capture_modes: BTreeSet::new(),
            capabilities,
            vendor: None,
            evidence,
        };
    };

    let recognized_flags = flags & MAPPED_FLAGS_MASK;
    let known_unmapped_flags = flags & KNOWN_FLAGS_MASK & !MAPPED_FLAGS_MASK;
    let unknown_flags = flags & !KNOWN_FLAGS_MASK;
    let capture_modes: BTreeSet<_> = OppoCaptureMode::FOLDER_PROJECTION_PRIORITY
        .into_iter()
        .filter(|mode| flags & mode.bit() != 0)
        .collect();
    let evidence = OppoPhotoClassification {
        raw_user_comment: Some(trimmed.to_owned()),
        flag_evidence: Some(OppoFlagEvidence {
            raw_flags: flags,
            recognized_flags,
            known_unmapped_flags,
            unknown_flags,
        }),
        capture_modes: capture_modes.clone(),
        metadata_status: PhotoMetadataReadStatus::Ok,
    };
    PhotoClassification {
        asset_type,
        capture_modes,
        capabilities,
        vendor: Some(CameraVendor::Oppo),
        evidence,
    }
}

pub fn classification_contract(classification: &PhotoClassification) -> PhotoClassificationContract {
    let primary = classification.primary_capture_mode();
    PhotoClassificationContract {
        asset_type: classification.asset_type.as_str().to_owned(),
        capture_modes: OppoCaptureMode::FOLDER_PROJECTION_PRIORITY
            .into_iter()
            .filter(|mode| classification.capture_modes.contains(mode))
            .map(|mode| mode.as_str().to_owned())
            .collect(),
        primary_capture_mode: primary.map(|mode| mode.as_str().to_owned()),
        folder: classification.folder_name().to_owned(),
        metadata_status: classification.evidence.metadata_status.as_str().to_owned(),
        recognized_flags: classification.evidence.recognized_flags(),
        known_unmapped_flags: classification.evidence.known_unmapped_flags(),
        unknown_flags: classification.evidence.unknown_flags(),
        tags: classification.tags(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_and_embedded_prefix_forms() {
        assert_eq!(parse_flags(r#"{"oplustag":4194304}"#), None);
        assert_eq!(parse_flags(r#"{"OplusTag":"18"}"#), None);
        assert_eq!(parse_flags("{\"oplustag\":4194304}"), Some(4_194_304));
        assert_eq!(parse_flags("{\"OplusTag\":\"18\"}"), Some(18));
        assert_eq!(parse_flags("ASCIIOplus_4096"), Some(4096));
        assert_eq!(parse_flags("junk\0OPPO_2048tail"), Some(2048));
    }

    #[test]
    fn keeps_all_activated_modes_but_projects_one_folder() {
        let classification = classify_user_comment(Some("oplus_18"));
        assert_eq!(
            classification.capture_modes,
            BTreeSet::from([OppoCaptureMode::Portrait, OppoCaptureMode::Beauty])
        );
        assert_eq!(
            classification.primary_capture_mode(),
            Some(OppoCaptureMode::Portrait)
        );
        assert!(!classification.tags().contains(&"capture.normal".to_owned()));
    }

    #[test]
    fn separates_known_unmapped_and_unknown_flags() {
        let known = classify_user_comment(Some("oplus_262144"));
        assert_eq!(known.evidence.known_unmapped_flags(), 262_144);
        assert_eq!(known.evidence.unknown_flags(), 0);
        assert_eq!(known.primary_capture_mode(), Some(OppoCaptureMode::Normal));

        let unknown = classify_user_comment(Some("oplus_17179869184"));
        assert_eq!(unknown.evidence.known_unmapped_flags(), 0);
        assert_eq!(unknown.evidence.unknown_flags(), 17_179_869_184);
        assert_eq!(unknown.primary_capture_mode(), None);
    }
}
