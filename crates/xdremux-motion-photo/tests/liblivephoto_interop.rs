//! Exercise the independent resource facade through the consumer's public boundary.
use std::{fs, path::PathBuf};
use xdremux_motion_photo::{
    apple, Disposition, Input, MotionPhoto, PairingPolicy, ParseOptions, ResourceRole,
    TargetProfile, UnmappedPolicy, VendorDialect, WritePolicy,
};

#[test]
fn coloros16_public_writer_retains_unmapped_resources_as_explicit_sidecars() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg");
    let source = fs::read(path).unwrap();
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert_eq!(photo.asset().dialect, VendorDialect::Oplus);
    assert!(photo
        .resources()
        .any(|r| r.role == ResourceRole::VendorAuxiliaryVideo));
    let pairing = PairingPolicy::Set("PUBLIC-CONSUMER-CONTRACT".into());
    assert!(!photo
        .plan(
            TargetProfile::ApplePair,
            WritePolicy {
                pairing: pairing.clone(),
                ..WritePolicy::default()
            }
        )
        .can_execute());
    let plan = photo.plan(
        TargetProfile::ApplePair,
        WritePolicy {
            pairing,
            unmapped: UnmappedPolicy::PreserveSidecars,
        },
    );
    let output = plan.execute().unwrap();
    for row in plan
        .report()
        .resources
        .iter()
        .filter(|r| r.disposition == Disposition::Unmapped)
    {
        let id = row.resource.unwrap();
        let resource = photo.resources().find(|r| r.id == id).unwrap();
        let mut bytes = Vec::new();
        photo.extract(resource, &mut bytes).unwrap();
        assert_eq!(
            output
                .files
                .iter()
                .find(|f| f.sidecar_for == Some(id))
                .unwrap()
                .bytes,
            bytes
        );
    }
    let pair = MotionPhoto::parse(
        Input::ApplePair {
            still: &output.files[0].bytes,
            movie: &output.files[1].bytes,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(pair
        .presentation_time()
        .unwrap()
        .equivalent(photo.presentation_time().unwrap()));
    assert_eq!(
        apple::media_payloads(pair.motion_video_bytes().unwrap()).unwrap(),
        apple::media_payloads(photo.motion_video_bytes().unwrap()).unwrap()
    );
    let truncated = &output.files[1].bytes[..output.files[1].bytes.len() / 2];
    assert!(MotionPhoto::parse(
        Input::ApplePair {
            still: &output.files[0].bytes,
            movie: truncated,
        },
        ParseOptions::strict()
    )
    .is_err());
}
