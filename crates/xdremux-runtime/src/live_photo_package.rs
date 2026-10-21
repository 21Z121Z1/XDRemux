use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;
use xdremux_motion_photo::{AssetFormat, MotionPhoto};

use crate::{Result, RuntimeError};

const METADATA: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>PFVideoComplementMetadataVersionKey</key><string>1</string></dict></plist>
"#;
const MAX_METADATA_BYTES: u64 = 64 * 1024;

#[derive(Debug)]
pub struct LivePhotoPackagePublicationError {
    staging: PathBuf,
    destination: PathBuf,
    source: io::Error,
}

impl LivePhotoPackagePublicationError {
    /// Complete staged package retained after publication failed. This path
    /// does not authorize deletion of an entry another writer may have changed.
    pub fn staging_path(&self) -> &Path {
        &self.staging
    }
}

impl fmt::Display for LivePhotoPackagePublicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not exclusively publish PVT directory {}: {}; complete staged package retained at {}", self.destination.display(), self.source, self.staging.display())
    }
}

impl Error for LivePhotoPackagePublicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageMembers {
    pub image: PathBuf,
    pub video: PathBuf,
}

pub fn validate_still_output_path(output: &Path) -> Result<()> {
    validate_output_parent(output)?;
    if output
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case("pvt"))
    {
        return Err(RuntimeError::new(
            "PVT output",
            "a PVT destination requires Motion Photo input",
        ));
    }
    Ok(())
}

pub(crate) fn validate_output_parent(output: &Path) -> Result<()> {
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let is_package = |path: &Path| {
        path.extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pvt"))
    };
    let mut resolved = None;
    for ancestor in parent
        .ancestors()
        .filter(|path| !path.as_os_str().is_empty())
    {
        if is_package(ancestor) && fs::symlink_metadata(ancestor).is_ok() {
            return Err(RuntimeError::new(
                "PVT output",
                "refusing to write inside an existing PVT package",
            ));
        }
        if resolved.is_none() {
            resolved = fs::canonicalize(ancestor).ok();
        }
    }
    if resolved.is_some_and(|path| path.ancestors().any(is_package)) {
        return Err(RuntimeError::new(
            "PVT output",
            "refusing to write inside an existing PVT package",
        ));
    }
    Ok(())
}

pub(crate) fn planned_members(package: &Path) -> Result<PackageMembers> {
    let stem = package
        .file_stem()
        .ok_or_else(|| RuntimeError::new("PVT output", "package destination has no file stem"))?;
    let mut image = stem.to_os_string();
    image.push(".heic");
    let mut video = stem.to_os_string();
    video.push(".mov");
    Ok(PackageMembers {
        image: package.join(image),
        video: package.join(video),
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags, CWD};
    // Darwin maps NOREPLACE to RENAME_EXCL and fails if renameatx_np is absent.
    renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn rename_exclusive(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic exclusive PVT directory publication is unavailable on this platform",
    ))
}

fn publish_staging(staging: tempfile::TempDir, destination: &Path) -> Result<()> {
    // Disarm before rename: no guard may later remove the old pathname after
    // the owned directory moves, even if another writer creates an entry there.
    let staging = staging.keep();
    rename_exclusive(&staging, destination).map_err(|source| {
        RuntimeError::external(
            "PVT publication",
            LivePhotoPackagePublicationError {
                staging,
                destination: destination.to_path_buf(),
                source,
            },
        )
    })
}

/// Package an already parsed Apple pair. No codec or remux provider crosses
/// this boundary: the resource bytes, pairing identity and time stay unchanged.
pub(crate) fn publish_pair(pair: &MotionPhoto<'_>, output: &Path) -> Result<PackageMembers> {
    if pair.format() != AssetFormat::AppleLivePhoto {
        return Err(RuntimeError::new(
            "PVT output",
            "PVT requires a validated Apple pair",
        ));
    }
    if pair.still().mime != "image/heic" {
        return Err(RuntimeError::new("PVT output", "the product PVT writer requires its validated HEIF still; JPEG packages remain readable"));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let members = planned_members(output)?;
    let staging = tempfile::Builder::new()
        .prefix(".xdremux-live-photo-pvt-")
        .suffix(".pvt")
        .tempdir_in(parent)
        .map_err(|error| RuntimeError::external("PVT staging directory", error))?;
    let image = staging.path().join(members.image.file_name().unwrap());
    let video = staging.path().join(members.video.file_name().unwrap());
    super::live_photo::write_synced_new(
        &image,
        pair.still_bytes()
            .map_err(|error| RuntimeError::external("PVT still resource", error))?,
    )?;
    super::live_photo::write_synced_new(
        &video,
        pair.motion_video_bytes()
            .map_err(|error| RuntimeError::external("PVT movie resource", error))?,
    )?;
    super::live_photo::write_synced_new(&staging.path().join("metadata.plist"), METADATA)?;
    publish_staging(staging, output)?;
    Ok(members)
}

fn invalid_package() -> RuntimeError {
    RuntimeError::new("PVT validation", "unsupported or ambiguous PVT directory; expected one regular still, one regular MOV and an XML version-1 metadata.plist")
}

fn validate_metadata(bytes: &[u8]) -> Result<()> {
    // Deliberately bounded to the observed XML recipe, not a general Apple
    // property-list implementation. No external entities are fetched.
    let expected: [&[u8]; 10] = [
        b"plist",
        b"dict",
        b"key",
        b"PFVideoComplementMetadataVersionKey",
        b"/key",
        b"string",
        b"1",
        b"/string",
        b"/dict",
        b"/plist",
    ];
    let mut position = 0;
    let mut declaration = false;
    let mut doctype = false;
    let mut reader = Reader::from_reader(bytes);
    loop {
        let token = match reader
            .read_event()
            .map_err(|error| RuntimeError::external("PVT metadata XML", error))?
        {
            Event::Start(value) => {
                let name = value.name();
                let attributes = value
                    .attributes()
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|error| RuntimeError::external("PVT metadata XML", error))?;
                if name.as_ref() == "plist" {
                    if attributes.len() != 1
                        || attributes[0].key.as_ref() != "version"
                        || attributes[0].value.as_ref() != "1.0"
                    {
                        return Err(invalid_package());
                    }
                } else if !attributes.is_empty() {
                    return Err(invalid_package());
                }
                if !matches!(position, 0 | 1 | 2 | 5) {
                    return Err(invalid_package());
                }
                name.as_ref().as_bytes().to_vec()
            }
            Event::End(value) => {
                if !matches!(position, 4 | 7 | 8 | 9) {
                    return Err(invalid_package());
                }
                [b"/".as_slice(), value.name().as_ref().as_bytes()].concat()
            }
            Event::Text(value) => {
                if value
                    .as_ref()
                    .as_bytes()
                    .iter()
                    .all(u8::is_ascii_whitespace)
                {
                    continue;
                }
                if !matches!(position, 3 | 6) {
                    return Err(invalid_package());
                }
                value.as_ref().as_bytes().to_vec()
            }
            Event::Comment(_) => continue,
            Event::Decl(_) if position == 0 && !declaration && !doctype => {
                declaration = true;
                continue;
            }
            Event::DocType(_) if position == 0 && !doctype => {
                doctype = true;
                continue;
            }
            Event::Eof => break,
            _ => return Err(invalid_package()),
        };
        if expected.get(position).copied() != Some(token.as_slice()) {
            return Err(invalid_package());
        }
        position += 1;
    }
    if position != expected.len() {
        return Err(invalid_package());
    }
    Ok(())
}

pub(crate) fn read_members(package: &Path) -> Result<PackageMembers> {
    if !fs::symlink_metadata(package)
        .map_err(|error| RuntimeError::external("PVT directory read", error))?
        .is_dir()
    {
        return Err(invalid_package());
    }
    let mut image = None;
    let mut video = None;
    let mut metadata = None;
    let entries = fs::read_dir(package)
        .map_err(|error| RuntimeError::external("PVT directory read", error))?;
    let mut count = 0;
    for entry in entries {
        let entry = entry.map_err(|error| RuntimeError::external("PVT directory read", error))?;
        count += 1;
        if count > 3
            || !entry
                .file_type()
                .map_err(|error| RuntimeError::external("PVT member type", error))?
                .is_file()
        {
            return Err(invalid_package());
        }
        let path = entry.path();
        if entry.file_name() == "metadata.plist" {
            metadata = Some(path);
        } else {
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            if ["heic", "heif", "jpg", "jpeg"]
                .iter()
                .any(|e| extension.eq_ignore_ascii_case(e))
            {
                if image.replace(path).is_some() {
                    return Err(invalid_package());
                }
            } else if extension.eq_ignore_ascii_case("mov") {
                if video.replace(path).is_some() {
                    return Err(invalid_package());
                }
            } else {
                return Err(invalid_package());
            }
        }
    }
    let (Some(image), Some(video), Some(metadata)) = (image, video, metadata) else {
        return Err(invalid_package());
    };
    let mut bytes = Vec::new();
    fs::File::open(metadata)
        .map_err(|error| RuntimeError::external("PVT metadata read", error))?
        .take(MAX_METADATA_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| RuntimeError::external("PVT metadata read", error))?;
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err(invalid_package());
    }
    validate_metadata(&bytes)?;
    Ok(PackageMembers { image, video })
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_motion_photo::{
        apple, Input, PairingPolicy, ParseOptions, TargetProfile, UnmappedPolicy, WritePolicy,
    };

    fn jpeg_pair(identifier: &str) -> (Vec<u8>, Vec<u8>) {
        let source = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"),
        )
        .unwrap();
        let photo =
            MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
        let composed = photo
            .plan(
                TargetProfile::ApplePair,
                WritePolicy {
                    unmapped: UnmappedPolicy::DropExplicitly,
                    pairing: PairingPolicy::Set(identifier.into()),
                },
            )
            .execute()
            .unwrap();
        (
            composed.files[0].bytes.clone(),
            composed.files[1].bytes.clone(),
        )
    }

    #[test]
    fn metadata_requires_the_observed_string_version_and_one_complete_document() {
        validate_metadata(METADATA).unwrap();
        let text = String::from_utf8(METADATA.to_vec()).unwrap();
        for malformed in [
            text.replace("<string>1</string>", "<integer>1</integer>"),
            text.replace("<string>1</string>", "<true/>"),
            text.replace("<string>1</string>", "<string>2</string>"),
            text.replace("PFVideoComplementMetadataVersionKey", "UnknownKey"),
            text.replace(
                "</dict>",
                "<key>PFVideoComplementMetadataVersionKey</key><string>1</string></dict>",
            ),
            format!("{text}{text}"),
            format!("<?xml version=\"1.0\"?>{text}"),
            text.replace("</plist>", ""),
        ] {
            assert!(validate_metadata(malformed.as_bytes()).is_err());
        }
    }

    #[test]
    fn jpeg_package_reader_uses_real_pair_metadata_without_filename_pairing() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("photo.pvt");
        fs::create_dir(&package).unwrap();
        let (still, movie) = jpeg_pair("PVT-JPEG-READER");
        assert_eq!(
            apple::read_still_identifier(&still).unwrap().as_deref(),
            Some("PVT-JPEG-READER")
        );
        fs::write(package.join("still.jpeg"), &still).unwrap();
        fs::write(package.join("different.mov"), &movie).unwrap();
        fs::write(package.join("metadata.plist"), METADATA).unwrap();
        let report = crate::validate_media_file(&package).unwrap();
        let crate::ValidationReport::LivePhoto(report) = report else {
            panic!("pair required");
        };
        assert_eq!(report.content_identifier, "PVT-JPEG-READER");
        assert_eq!(report.image.file_name().unwrap(), "still.jpeg");
        assert_eq!(report.video.file_name().unwrap(), "different.mov");
        assert!(
            fs::read(&report.image).unwrap() == still,
            "JPEG bytes changed"
        );
        assert!(
            fs::read(&report.video).unwrap() == movie,
            "MOV bytes changed"
        );
        let pair = MotionPhoto::parse(
            Input::ApplePair {
                still: &still,
                movie: &movie,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        assert!(publish_pair(&pair, &root.path().join("must-not-disguise-jpeg.pvt")).is_err());
        assert!(!root.path().join("must-not-disguise-jpeg.pvt").exists());
    }

    #[test]
    fn package_reader_rejects_mismatched_truncated_missing_and_ambiguous_members() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("photo.pvt");
        fs::create_dir(&package).unwrap();
        let (still, movie) = jpeg_pair("FIRST-PVT");
        let (_, other_movie) = jpeg_pair("SECOND-PVT");
        let image = package.join("capture.jpg");
        let video = package.join("capture.mov");
        fs::write(&image, &still).unwrap();
        fs::write(package.join("metadata.plist"), METADATA).unwrap();
        fs::write(&video, &other_movie).unwrap();
        assert!(crate::validate_media_file(&package).is_err());
        fs::write(&video, &movie[..movie.len() / 2]).unwrap();
        assert!(crate::validate_media_file(&package).is_err());
        fs::write(&video, &movie).unwrap();
        fs::write(package.join("duplicate.JPG"), &still).unwrap();
        assert!(crate::validate_media_file(&package).is_err());
        fs::remove_file(package.join("duplicate.JPG")).unwrap();
        fs::remove_file(&image).unwrap();
        assert!(crate::validate_media_file(&package).is_err());
        assert!(
            fs::read(&video).unwrap() == movie,
            "validation changed movie bytes"
        );
        let archive = root.path().join("archive.pvt");
        fs::write(&archive, b"PK\x03\x04not-a-directory").unwrap();
        assert!(crate::validate_media_file(&archive).is_err());
        assert_eq!(fs::read(archive).unwrap(), b"PK\x03\x04not-a-directory");
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn packaging_keeps_every_validated_heif_pair_byte_identity_time_and_auxiliary_resource() {
        let root = tempfile::tempdir().unwrap();
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic");
        let source = fs::read(&input).unwrap();
        let receipt = crate::PortableRuntime::new()
            .convert_motion_photo_file(&source, &input, root.path().join("original.heic"))
            .unwrap();
        let still = fs::read(&receipt.image).unwrap();
        let movie = fs::read(&receipt.video).unwrap();
        let pair = MotionPhoto::parse(
            Input::ApplePair {
                still: &still,
                movie: &movie,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        let output = root.path().join("capture.pvt");
        let paths = publish_pair(&pair, &output).unwrap();
        let packaged_still = fs::read(&paths.image).unwrap();
        let packaged_movie = fs::read(&paths.video).unwrap();
        assert!(
            packaged_still == still,
            "packaging changed a still/auxiliary byte"
        );
        assert!(
            packaged_movie == movie,
            "packaging changed a movie/sample/metadata byte"
        );
        let packaged = MotionPhoto::parse(
            Input::ApplePair {
                still: &packaged_still,
                movie: &packaged_movie,
            },
            ParseOptions::strict(),
        )
        .unwrap();
        assert_eq!(packaged.presentation_time(), pair.presentation_time());
        assert_eq!(
            packaged.asset().pairing.identifier,
            pair.asset().pairing.identifier
        );
        assert_eq!(packaged.asset().auxiliary, pair.asset().auxiliary);
        assert!(
            apple::media_payloads(&packaged_movie).unwrap()
                == apple::media_payloads(&movie).unwrap(),
            "encoded media changed"
        );
        crate::validate_media_file(&output).unwrap();
        assert!(fs::read(input).unwrap() == source, "source bytes changed");
    }

    fn staging(parent: &Path, marker: &[u8]) -> tempfile::TempDir {
        let staging = tempfile::tempdir_in(parent).unwrap();
        for name in ["capture.heic", "capture.mov", "metadata.plist"] {
            fs::write(staging.path().join(name), marker).unwrap();
        }
        staging
    }

    #[test]
    fn failed_publication_keeps_staging_and_original_error_without_replacing_destination() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("capture.pvt");
        fs::create_dir(&destination).unwrap();
        let owned = staging(root.path(), b"new bytes");
        let owned_path = owned.path().to_path_buf();
        let error = publish_staging(owned, &destination).unwrap_err();
        let source = error
            .source()
            .unwrap()
            .downcast_ref::<LivePhotoPackagePublicationError>()
            .unwrap();
        assert_eq!(source.staging_path(), owned_path);
        let io = source
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap();
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert_eq!(io.kind(), io::ErrorKind::AlreadyExists);
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        assert_eq!(io.kind(), io::ErrorKind::Unsupported);
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&owned_path).unwrap().count(), 3);
        assert!(error
            .to_string()
            .contains(&owned_path.display().to_string()));
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn concurrent_package_claims_publish_one_whole_directory_and_preserve_loser_staging() {
        use std::sync::{Arc, Barrier};
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("capture.pvt");
        let barrier = Arc::new(Barrier::new(2));
        let mut workers = Vec::new();
        let mut staged = Vec::new();
        for marker in [b"first".as_slice(), b"second"] {
            let owned = staging(root.path(), marker);
            staged.push(owned.path().to_path_buf());
            let destination = destination.clone();
            let barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                publish_staging(owned, &destination)
            }));
        }
        let outcomes = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        let winner = outcomes.iter().position(|outcome| outcome.is_ok()).unwrap();
        let marker = fs::read(destination.join("capture.heic")).unwrap();
        assert_eq!(fs::read(destination.join("capture.mov")).unwrap(), marker);
        assert_eq!(
            fs::read(destination.join("metadata.plist")).unwrap(),
            marker
        );
        assert!(staged[1 - winner].is_dir());
        // Reuse of the moved staging name is unowned. No surviving guard may
        // remove it after the directory was published.
        fs::create_dir(&staged[winner]).unwrap();
        fs::write(staged[winner].join("unowned"), b"keep").unwrap();
        drop(outcomes);
        assert_eq!(fs::read(staged[winner].join("unowned")).unwrap(), b"keep");
    }

    #[test]
    #[cfg(unix)]
    fn package_reader_and_output_parent_checks_reject_symlink_and_nested_package_mutation() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("existing.pvt");
        fs::create_dir(&package).unwrap();
        fs::write(package.join("keep"), b"original").unwrap();
        assert!(validate_output_parent(&package.join("nested/capture.pvt")).is_err());
        let alias = root.path().join("alias");
        symlink(&package, &alias).unwrap();
        assert!(validate_output_parent(&alias.join("nested/capture.heic")).is_err());
        assert!(read_members(&alias).is_err());
        let staged = staging(root.path(), b"new");
        let dangling = root.path().join("dangling.pvt");
        symlink(root.path().join("absent"), &dangling).unwrap();
        assert!(publish_staging(staged, &dangling).is_err());
        assert!(dangling.is_symlink());
        assert_eq!(fs::read(package.join("keep")).unwrap(), b"original");
    }

    #[test]
    fn direct_batch_and_checkpoint_writers_cannot_create_members_in_an_existing_package() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("unknown.pvt");
        fs::create_dir(&package).unwrap();
        fs::write(package.join("keep"), b"original").unwrap();
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic");
        let receipt = crate::PortableRuntime::new().convert_batch_with_options(
            vec![crate::BatchItem::new(
                &input,
                package.join("nested/capture.heic"),
            )],
            xdremux_engine::ConversionRequest::default(),
            &crate::BatchExecutionOptions::default(),
        );
        assert_eq!(receipt.failed(), 1);
        assert!(receipt.failures[0].error.contains("inside an existing PVT"));
        assert!(crate::batch_checkpoint::MotionPhotoCheckpointWriter::open(
            &package.join("checkpoint.jsonl")
        )
        .is_err());
        assert!(crate::PortableRuntime::new()
            .categorize_files(&[input], &package, false,)
            .is_err());
        assert_eq!(fs::read_dir(&package).unwrap().count(), 1);
        assert_eq!(fs::read(package.join("keep")).unwrap(), b"original");
    }
}
