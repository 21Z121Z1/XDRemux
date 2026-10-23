use std::fs;
use std::path::PathBuf;

use xdremux_format::{
    isobmff::{parse_boxes, BoxHeader},
    jpeg_image_end,
};
use xdremux_heif::validate_gain_map_structure;
use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions, TrackKind, VendorDialect};
use xdremux_runtime::PortableRuntime;

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo")
        .join(relative)
}

fn disable_motion_flag(source: &mut [u8]) {
    let flag = b":MotionPhoto=\"1\"";
    let mut replaced = false;
    for start in 0..source.len().saturating_sub(flag.len()) + 1 {
        if source.get(start..start + flag.len()) == Some(flag) {
            source[start + flag.len() - 2] = b'0';
            replaced = true;
        }
    }
    assert!(replaced, "fixture must declare the Motion Photo flag");
}

#[test]
fn conversion_preserves_a_source_directory_with_a_temporary_looking_name() {
    let directory = tempfile::tempdir().unwrap();
    let source_directory = directory.path().join(".capture.unowned.tmp.heic");
    fs::create_dir(&source_directory).unwrap();
    let input = source_directory.join("source.heic");
    let source = fs::read(fixture("samsung/heif-ultrahdr-01.heic")).unwrap();
    fs::write(&input, &source).unwrap();
    let output = directory.path().join("capture.heic");
    let receipt = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, &output)
        .unwrap();
    assert_eq!(fs::read(&input).unwrap(), source);
    let still = fs::read(&receipt.image).unwrap();
    let movie = fs::read(&receipt.video).unwrap();
    MotionPhoto::parse(
        Input::ApplePair {
            still: &still,
            movie: &movie,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
}

#[test]
#[cfg(unix)]
fn second_member_failure_retains_partial_movie_and_complete_staged_pair() {
    use std::error::Error;
    use std::os::unix::fs::MetadataExt;
    use xdremux_motion_photo::PairPublicationError;
    let directory = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let source = fs::read(&input).unwrap();
    // On standard Unix filesystems, MOV fits NAME_MAX while HEIC exceeds it.
    // Check that boundary before relying on it to fail only the second link.
    let output = directory.path().join(format!("{}.heic", "a".repeat(251)));
    let movie = output.with_extension("mov");
    fs::write(&movie, b"legal MOV filename probe").unwrap();
    fs::remove_file(&movie).unwrap();
    assert!(fs::write(&output, b"overlong still filename probe").is_err());
    let error = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, &output)
        .unwrap_err();
    assert!(movie.is_file(), "{error}");
    let publication = error
        .source()
        .unwrap()
        .downcast_ref::<PairPublicationError>()
        .unwrap();
    assert_eq!(publication.partial_output(), Some(movie.as_path()));
    let directories = fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    assert_eq!(directories.len(), 1);
    let staging = &directories[0];
    let still = fs::read(staging.join("capture.heic")).unwrap();
    let staged_movie = fs::read(staging.join("capture.mov")).unwrap();
    let mut retained_names = fs::read_dir(staging)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    retained_names.sort();
    assert_eq!(
        retained_names,
        ["capture.heic", "capture.mov", "recovery.heic", "recovery.mov"]
            .map(str::to_owned)
    );
    let recovery_image = staging.join("recovery.heic");
    let recovery_video = staging.join("recovery.mov");
    let recovered_still = fs::read(&recovery_image).unwrap();
    let recovered_movie = fs::read(&recovery_video).unwrap();
    assert_eq!(recovered_still, still);
    assert_eq!(recovered_movie, staged_movie);
    for (staged, recovery) in [
        (staging.join("capture.heic"), &recovery_image),
        (staging.join("capture.mov"), &recovery_video),
    ] {
        let original = fs::metadata(staged).unwrap();
        let independent = fs::metadata(recovery).unwrap();
        assert_ne!(
            (original.dev(), original.ino()),
            (independent.dev(), independent.ino())
        );
    }
    MotionPhoto::parse(
        Input::ApplePair {
            still: &still,
            movie: &staged_movie,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(fs::read(&movie).unwrap(), staged_movie);
    MotionPhoto::parse(
        Input::ApplePair {
            still: &recovered_still,
            movie: &recovered_movie,
        },
        ParseOptions::strict(),
    )
    .unwrap()
    .validate()
    .unwrap();
    fs::write(&movie, b"subsequent partial publication damage").unwrap();
    assert_eq!(fs::read(&recovery_image).unwrap(), recovered_still);
    assert_eq!(fs::read(&recovery_video).unwrap(), recovered_movie);
    assert!(error
        .to_string()
        .contains("independent checked recovery pair retained"));
    assert!(
        error.to_string().contains(&staging.display().to_string()),
        "{error}"
    );
    assert!(
        error.to_string().contains(&movie.display().to_string()),
        "{error}"
    );
    assert!(error.source().and_then(Error::source).is_some());
    assert_eq!(fs::read(input).unwrap(), source);
}

fn video_media_boxes(movie: &[u8]) -> Vec<BoxHeader> {
    let moov = parse_boxes(movie, 0..movie.len())
        .unwrap()
        .into_iter()
        .find(|b| b.kind.as_bytes() == b"moov")
        .unwrap();
    let mut media_boxes = Vec::new();
    for track in parse_boxes(movie, moov.data_start..moov.data_end).unwrap() {
        if track.kind.as_bytes() != b"trak" {
            continue;
        }
        let mdia = parse_boxes(movie, track.data_start..track.data_end)
            .unwrap()
            .into_iter()
            .find(|b| b.kind.as_bytes() == b"mdia")
            .unwrap();
        if parse_boxes(movie, mdia.data_start..mdia.data_end)
            .unwrap()
            .iter()
            .any(|handler| {
                let position = handler.data_start + 8;
                handler.kind.as_bytes() == b"hdlr"
                    && movie.get(position..position + 4) == Some(b"vide")
            })
        {
            media_boxes.push(mdia);
        }
    }
    assert!(!media_boxes.is_empty());
    media_boxes
}

#[test]
fn known_timestamp_without_video_track_is_rejected_before_publication() {
    let mut source = fs::read(fixture("samsung/heif-ultrahdr-01.heic")).unwrap();
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert!(photo.presentation_time().is_some());
    let video_start = photo.motion_video().extents[0].lower_bound as usize;
    let movie = photo.motion_video_bytes().unwrap();
    let video_handlers = video_media_boxes(movie)
        .iter()
        .flat_map(|mdia| parse_boxes(movie, mdia.data_start..mdia.data_end).unwrap())
        .filter(|handler| handler.kind.as_bytes() == b"hdlr")
        .map(|handler| video_start + handler.data_start + 8)
        .collect::<Vec<_>>();
    assert!(!video_handlers.is_empty());
    for position in video_handlers {
        source[position..position + 4].copy_from_slice(b"junk");
    }
    let parsed =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert!(parsed.presentation_time().is_some());
    let primary_movie = parsed
        .asset()
        .containers
        .iter()
        .find(|container| container.resource == parsed.motion_video().id)
        .unwrap();
    assert!(primary_movie
        .tracks
        .iter()
        .all(|track| track.kind != TrackKind::Video));

    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("source.heic");
    let output = directory.path().join("capture.heic");
    fs::write(&input, &source).unwrap();
    let error = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, &output)
        .expect_err("known cover time cannot substitute for a video track");
    assert!(error.to_string().contains("video track"), "{error}");
    assert!(!output.exists());
    assert!(!output.with_extension("mov").exists());
    assert_eq!(fs::read(input).unwrap(), source);
}

#[test]
fn known_timestamp_without_video_samples_is_rejected_before_publication() {
    let mut source = fs::read(fixture("samsung/heif-ultrahdr-01.heic")).unwrap();
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert!(photo.presentation_time().is_some());
    let video_start = photo.motion_video().extents[0].lower_bound as usize;
    let movie = photo.motion_video_bytes().unwrap();
    let sample_tables = video_media_boxes(movie)
        .iter()
        .map(|mdia| {
            let minf = parse_boxes(movie, mdia.data_start..mdia.data_end)
                .unwrap()
                .into_iter()
                .find(|b| b.kind.as_bytes() == b"minf")
                .unwrap();
            parse_boxes(movie, minf.data_start..minf.data_end)
                .unwrap()
                .into_iter()
                .find(|b| b.kind.as_bytes() == b"stbl")
                .unwrap()
                .box_start
        })
        .collect::<Vec<_>>();
    for start in sample_tables {
        source[video_start + start + 4..video_start + start + 8].copy_from_slice(b"free");
    }
    let parsed =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert!(parsed.presentation_time().is_some());
    let primary_movie = parsed
        .asset()
        .containers
        .iter()
        .find(|container| container.resource == parsed.motion_video().id)
        .unwrap();
    assert!(primary_movie
        .tracks
        .iter()
        .any(|track| track.kind == TrackKind::Video));
    assert!(primary_movie
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Video)
        .all(|track| track.sample_entries.is_empty()));

    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("source.heic");
    let output = directory.path().join("capture.heic");
    fs::write(&input, &source).unwrap();
    let error = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, &output)
        .expect_err("known cover time cannot substitute for video samples");
    assert!(error.to_string().contains("stbl box"), "{error}");
    assert!(!output.exists());
    assert!(!output.with_extension("mov").exists());
    assert_eq!(fs::read(input).unwrap(), source);
}

#[test]
fn sef_fallback_hdr_without_mapped_gain_map_is_rejected_before_publication() {
    let mut source = fs::read(fixture("samsung/jpeg-ultrahdr-01.jpg")).unwrap();
    disable_motion_flag(&mut source);
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert_eq!(photo.asset().dialect, VendorDialect::SamsungSef);
    assert!(!photo.resources().any(|resource| resource
        .vendor_role
        .as_deref()
        .is_some_and(|role| role.eq_ignore_ascii_case("GainMap"))));

    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("source.jpg");
    let output = directory.path().join("capture.heic");
    fs::write(&input, &source).unwrap();
    let error = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, &output)
        .expect_err("unmapped HDR must not silently become SDR");
    assert!(error.to_string().contains("GainMap resource"), "{error}");
    assert!(!output.exists());
    assert!(!output.with_extension("mov").exists());
    assert_eq!(fs::read(input).unwrap(), source);
}

#[test]
fn sef_fallback_with_real_sdr_still_and_movie_remains_supported() {
    let original = fs::read(fixture("vivo/android-v1-sdr-01.jpg")).unwrap();
    let photo =
        MotionPhoto::parse(Input::SingleFile(&original), ParseOptions::compatible()).unwrap();
    let end = jpeg_image_end(photo.still_bytes().unwrap(), 0).unwrap();
    let mut source = photo.still_bytes().unwrap()[..end].to_vec();
    disable_motion_flag(&mut source);
    let record_start = source.len();
    let tag = [0, 0, 0x30, 0x0a];
    let name = b"MotionPhoto_Data";
    source.extend_from_slice(&tag);
    source.extend_from_slice(&(name.len() as u32).to_le_bytes());
    source.extend_from_slice(name);
    source.extend_from_slice(photo.motion_video_bytes().unwrap());
    let directory_start = source.len();
    let record_length = (directory_start - record_start) as u32;
    source.extend_from_slice(b"SEFH");
    source.extend_from_slice(&107_u32.to_le_bytes());
    source.extend_from_slice(&1_u32.to_le_bytes());
    source.extend_from_slice(&tag);
    source.extend_from_slice(&record_length.to_le_bytes());
    source.extend_from_slice(&record_length.to_le_bytes());
    source.extend_from_slice(&24_u32.to_le_bytes());
    source.extend_from_slice(b"SEFT");
    let parsed =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    assert_eq!(parsed.asset().dialect, VendorDialect::SamsungSef);

    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("source.jpg");
    let output = directory.path().join("capture.heic");
    fs::write(&input, &source).unwrap();
    let receipt = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, &output)
        .expect("SEF association with a real SDR image and movie remains supported");
    assert!(!receipt.source_had_gain_map);
    assert!(validate_gain_map_structure(&fs::read(&output).unwrap()).is_err());
    let movie = fs::read(&receipt.video).unwrap();
    let still = fs::read(&receipt.image).unwrap();
    MotionPhoto::parse(
        Input::ApplePair {
            still: &still,
            movie: &movie,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(fs::read(input).unwrap(), source);
}
