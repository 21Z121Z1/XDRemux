use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use xdremux_motion_photo::{
    apple, companion_video_path, publish_live_photo_pair, Input, MediaTime, MotionPhoto,
    ParseOptions,
};

use crate::{Result, RuntimeError};

/// Immutable product-checked bytes. This does not prove native Photos acceptance.
#[derive(Debug)]
pub(super) struct CheckedPair {
    still: Vec<u8>,
    movie: Vec<u8>,
}

impl CheckedPair {
    pub(super) fn check(
        still: Vec<u8>,
        movie: Vec<u8>,
        source_movie: &[u8],
        expected_identifier: &str,
        expected_presentation: MediaTime,
    ) -> Result<Self> {
        check_pair(
            &still,
            &movie,
            source_movie,
            expected_identifier,
            expected_presentation,
        )?;
        Ok(Self { still, movie })
    }
}

pub(super) fn check_pair<'a>(
    still: &'a [u8],
    movie: &'a [u8],
    source_movie: &[u8],
    expected_identifier: &str,
    expected_presentation: MediaTime,
) -> Result<MotionPhoto<'a>> {
    let pair = MotionPhoto::parse(Input::ApplePair { still, movie }, ParseOptions::strict())
        .map_err(|error| RuntimeError::external("Live Photo pair validation", error))?;
    pair.validate()
        .map_err(|error| RuntimeError::external("Live Photo pair validation", error))?;
    if !pair
        .presentation_time()
        .is_some_and(|time| time.equivalent(expected_presentation))
    {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "exact presentation time changed",
        ));
    }
    if pair.asset().pairing.identifier.as_deref() != Some(expected_identifier) {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "pairing identifier changed",
        ));
    }
    let source_media = apple::media_payloads(source_movie)
        .map_err(|error| RuntimeError::external("source Motion Photo media validation", error))?;
    let output_media = apple::media_payloads(movie)
        .map_err(|error| RuntimeError::external("Live Photo media validation", error))?;
    if source_media != output_media {
        return Err(RuntimeError::new(
            "Live Photo media validation",
            "compressed video/audio mdat payload changed during MOV remux",
        ));
    }
    Ok(pair)
}

#[derive(Debug)]
pub(super) struct PublishedPair {
    pub(super) image: PathBuf,
    pub(super) video: PathBuf,
}

#[derive(Debug)]
enum ObservedFinalEntries {
    Movie(PathBuf),
    Pair(PublishedPair),
}

/// Observed paths for explicit recovery, never authority to unlink final entries.
#[derive(Debug)]
struct RetainedPair {
    staging: PathBuf,
    observed: ObservedFinalEntries,
    recovery: Result<()>,
}

#[derive(Debug)]
pub(super) struct PublicationFailure {
    cause: RuntimeError,
    retained: Option<Box<RetainedPair>>,
}

impl PublicationFailure {
    fn unpublished(cause: RuntimeError) -> Self {
        Self {
            cause,
            retained: None,
        }
    }

    pub(super) fn into_runtime_error(mut self) -> RuntimeError {
        if let Some(retained) = self.retained {
            let observed = match retained.observed {
                ObservedFinalEntries::Movie(movie) => movie.display().to_string(),
                ObservedFinalEntries::Pair(pair) => {
                    format!("{} and {}", pair.image.display(), pair.video.display())
                }
            };
            let recovery = match retained.recovery {
                Ok(()) => {
                    "independent checked recovery pair retained as recovery.heic and recovery.mov"
                        .to_owned()
                }
                Err(error) => format!(
                    "recovery-unverified: {error}; retained staging may share damaged final inodes"
                ),
            };
            self.cause.detail.push_str(&format!(
                "; {recovery} at {}; publication observed at {observed}; inspect both final entries before explicit recovery",
                retained.staging.display()
            ));
        }
        self.cause
    }
}

fn retain_for_recovery(
    pair: &CheckedPair,
    staging: tempfile::TempDir,
    observed: ObservedFinalEntries,
) -> Box<RetainedPair> {
    // Publication hard-links capture.* to final entries. Never repair those
    // names in place: doing so would also overwrite an observed final inode.
    // New files retain the immutable checked buffers independently, and only a
    // successful readback permits a complete-recovery claim.
    let image = staging.path().join("recovery.heic");
    let video = staging.path().join("recovery.mov");
    let recovery = super::write_synced_new(&image, &pair.still)
        .and_then(|()| super::write_synced_new(&video, &pair.movie))
        .and_then(|()| verify_bytes(&image, &pair.still))
        .and_then(|()| verify_bytes(&video, &pair.movie));
    Box::new(RetainedPair {
        staging: staging.keep(),
        observed,
        recovery,
    })
}

fn verify_bytes(path: &Path, expected: &[u8]) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| RuntimeError::external("Live Photo byte readback", error))?;
    if !metadata.file_type().is_file() {
        return Err(RuntimeError::new(
            "Live Photo byte readback",
            format!("{} is not a regular file", path.display()),
        ));
    }
    let mut file = File::open(path)
        .map_err(|error| RuntimeError::external("Live Photo byte readback", error))?;
    let mut buffer = [0_u8; 8192];
    for chunk in expected.chunks(buffer.len()) {
        file.read_exact(&mut buffer[..chunk.len()])
            .map_err(|error| RuntimeError::external("Live Photo byte readback", error))?;
        if &buffer[..chunk.len()] != chunk {
            return Err(RuntimeError::new(
                "Live Photo byte readback",
                format!("{} differs from the checked pair", path.display()),
            ));
        }
    }
    if file
        .read(&mut buffer[..1])
        .map_err(|error| RuntimeError::external("Live Photo byte readback", error))?
        != 0
    {
        return Err(RuntimeError::new(
            "Live Photo byte readback",
            format!("{} has bytes beyond the checked pair", path.display()),
        ));
    }
    Ok(())
}

pub(super) fn publish(
    pair: CheckedPair,
    destination: &Path,
) -> std::result::Result<PublishedPair, PublicationFailure> {
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let staging = tempfile::Builder::new()
        .prefix(".xdremux-live-photo-")
        .tempdir_in(parent)
        .map_err(|error| {
            PublicationFailure::unpublished(RuntimeError::external(
                "Live Photo staging directory",
                error,
            ))
        })?;
    let temporary_image = staging.path().join("capture.heic");
    let temporary_video = staging.path().join("capture.mov");
    super::write_synced_new(&temporary_image, &pair.still)
        .and_then(|()| super::write_synced_new(&temporary_video, &pair.movie))
        .and_then(|()| verify_bytes(&temporary_image, &pair.still))
        .and_then(|()| verify_bytes(&temporary_video, &pair.movie))
        .map_err(PublicationFailure::unpublished)?;
    let published = PublishedPair {
        image: destination.to_path_buf(),
        video: companion_video_path(destination),
    };
    if let Err(error) = publish_live_photo_pair(
        &temporary_image,
        &temporary_video,
        &published.image,
        &published.video,
    ) {
        let retained = error.partial_output().map(|movie| {
            retain_for_recovery(
                &pair,
                staging,
                ObservedFinalEntries::Movie(movie.to_path_buf()),
            )
        });
        return Err(PublicationFailure {
            cause: RuntimeError::external("Live Photo pair publication", error),
            retained,
        });
    }
    readback_published(pair, staging, published)
}

fn readback_published(
    pair: CheckedPair,
    staging: tempfile::TempDir,
    published: PublishedPair,
) -> std::result::Result<PublishedPair, PublicationFailure> {
    if let Err(cause) = verify_bytes(&published.image, &pair.still)
        .and_then(|()| verify_bytes(&published.video, &pair.movie))
    {
        return Err(PublicationFailure {
            cause,
            retained: Some(retain_for_recovery(
                &pair,
                staging,
                ObservedFinalEntries::Pair(published),
            )),
        });
    }
    Ok(published)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_motion_photo::write_live_photo_heif_still;

    const IDENTIFIER: &str = "127EE11A-A8EC-476E-A12E-E1FEF2C02EF2";

    fn fixture_pair() -> (Vec<u8>, Vec<u8>, Vec<u8>, MediaTime) {
        let source = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic"),
        )
        .unwrap();
        let photo =
            MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
        let presentation = photo.presentation_time().unwrap();
        let still = write_live_photo_heif_still(photo.still_bytes().unwrap(), IDENTIFIER).unwrap();
        let source_movie = photo.motion_video_bytes().unwrap().to_vec();
        let movie = apple::remux_movie(&source_movie, IDENTIFIER, presentation).unwrap();
        (still, movie, source_movie, presentation)
    }

    fn checked_fixture() -> CheckedPair {
        let (still, movie, source_movie, time) = fixture_pair();
        CheckedPair::check(still, movie, &source_movie, IDENTIFIER, time).unwrap()
    }

    #[test]
    fn constructor_keeps_existing_media_buffers() {
        let (still, movie, source_movie, time) = fixture_pair();
        let still_allocation = still.as_ptr();
        let movie_allocation = movie.as_ptr();
        let pair = CheckedPair::check(still, movie, &source_movie, IDENTIFIER, time).unwrap();
        assert_eq!(pair.still.as_ptr(), still_allocation);
        assert_eq!(pair.movie.as_ptr(), movie_allocation);
    }

    #[test]
    fn constructor_rejects_malformed_or_unexpected_pair_facts() {
        let (still, movie, source_movie, time) = fixture_pair();
        CheckedPair::check(vec![0], movie.clone(), &source_movie, IDENTIFIER, time).unwrap_err();
        let error = CheckedPair::check(
            still.clone(),
            movie.clone(),
            &source_movie,
            "another identifier",
            time,
        )
        .unwrap_err();
        assert!(error.to_string().contains("pairing identifier changed"));
        let error = CheckedPair::check(
            still,
            movie,
            &source_movie,
            IDENTIFIER,
            MediaTime::new(time.value + 1, time.timescale).unwrap(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("exact presentation time changed"));
    }

    #[test]
    fn constructor_rejects_changed_compressed_source_media() {
        let (still, movie, mut source_movie, time) = fixture_pair();
        let mdat = xdremux_format::isobmff::parse_boxes(&source_movie, 0..source_movie.len())
            .unwrap()
            .into_iter()
            .find(|header| header.kind.as_bytes() == b"mdat")
            .unwrap();
        source_movie[mdat.data_start] ^= 1;
        let error = CheckedPair::check(still, movie, &source_movie, IDENTIFIER, time).unwrap_err();
        assert!(error
            .to_string()
            .contains("compressed video/audio mdat payload changed"));
    }

    #[test]
    fn readback_rejects_changed_truncated_and_appended_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("member");
        let bytes = vec![0x59; 8193];
        fs::write(&path, &bytes).unwrap();
        verify_bytes(&path, &bytes).unwrap();
        for changed in [&bytes[..8192], &vec![0x59; 8194], &vec![0x58; 8193]] {
            fs::write(&path, changed).unwrap();
            verify_bytes(&path, &bytes).unwrap_err();
        }
    }

    #[test]
    fn owned_publication_preserves_existing_pair_and_removes_its_staging() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("capture.heic");
        let video = companion_video_path(&image);
        fs::write(&image, b"existing still").unwrap();
        fs::write(&video, b"existing movie").unwrap();
        let failure = publish(checked_fixture(), &image).unwrap_err();
        assert!(failure.retained.is_none());
        assert_eq!(fs::read(&image).unwrap(), b"existing still");
        assert_eq!(fs::read(&video).unwrap(), b"existing movie");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn owned_publication_preserves_exact_bytes_without_retaining_staging() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("capture.heic");
        let pair = checked_fixture();
        let expected_still = pair.still.clone();
        let expected_movie = pair.movie.clone();
        let published = publish(pair, &image).unwrap();
        assert_eq!(fs::read(published.image).unwrap(), expected_still);
        assert_eq!(fs::read(published.video).unwrap(), expected_movie);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn failed_final_readback_retains_staging_and_preserves_observed_entries() {
        let directory = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir_in(directory.path()).unwrap();
        let pair = checked_fixture();
        super::super::write_synced_new(&staging.path().join("capture.heic"), &pair.still).unwrap();
        super::super::write_synced_new(&staging.path().join("capture.mov"), &pair.movie).unwrap();
        let image = directory.path().join("capture.heic");
        let video = companion_video_path(&image);
        fs::write(&image, b"replacement still entry").unwrap();
        fs::write(&video, &pair.movie).unwrap();
        let failure = readback_published(
            pair,
            staging,
            PublishedPair {
                image: image.clone(),
                video: video.clone(),
            },
        )
        .unwrap_err();
        let retained = failure.retained.as_ref().unwrap();
        assert!(
            matches!(&retained.observed, ObservedFinalEntries::Pair(pair) if pair.image == image && pair.video == video)
        );
        assert!(retained.staging.join("capture.heic").is_file());
        assert!(retained.staging.join("capture.mov").is_file());
        failure.into_runtime_error();
        assert_eq!(fs::read(image).unwrap(), b"replacement still entry");
        assert!(video.is_file());
    }

    #[test]
    #[cfg(unix)]
    fn second_member_failure_retains_a_complete_pair_for_explicit_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join(format!("{}.heic", "a".repeat(251)));
        let video = companion_video_path(&image);
        fs::write(&video, b"MOV name boundary probe").unwrap();
        fs::remove_file(&video).unwrap();
        assert!(fs::write(&image, b"overlong still name").is_err());
        let failure = publish(checked_fixture(), &image).unwrap_err();
        let retained = failure.retained.as_ref().unwrap();
        assert!(matches!(&retained.observed, ObservedFinalEntries::Movie(path) if path == &video));
        assert!(retained.recovery.is_ok());
        let still = fs::read(retained.staging.join("recovery.heic")).unwrap();
        let movie = fs::read(retained.staging.join("recovery.mov")).unwrap();
        assert_eq!(fs::read(&video).unwrap(), movie);
        let (_, _, source_movie, time) = fixture_pair();
        let recovered = publish(
            CheckedPair::check(still, movie, &source_movie, IDENTIFIER, time).unwrap(),
            &directory.path().join("recovered.heic"),
        )
        .unwrap();
        assert!(recovered.image.is_file() && recovered.video.is_file());
        let competitor = directory.path().join("competitor.mov");
        fs::write(&competitor, b"replacement final entry").unwrap();
        fs::rename(&competitor, &video).unwrap();
        let error = failure.into_runtime_error();
        assert!(error
            .to_string()
            .contains("independent checked recovery pair retained"));
        assert_eq!(fs::read(video).unwrap(), b"replacement final entry");
    }

    #[test]
    fn in_place_hardlink_damage_retains_independent_checked_recovery_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir_in(directory.path()).unwrap();
        let pair = CheckedPair {
            still: b"synthetic checked still".to_vec(),
            movie: b"synthetic checked movie".to_vec(),
        };
        let expected_still = pair.still.clone();
        let expected_movie = pair.movie.clone();
        let staged_image = staging.path().join("capture.heic");
        let staged_video = staging.path().join("capture.mov");
        super::super::write_synced_new(&staged_image, &pair.still).unwrap();
        super::super::write_synced_new(&staged_video, &pair.movie).unwrap();
        let image = directory.path().join("capture.heic");
        let video = companion_video_path(&image);
        fs::hard_link(&staged_image, &image).unwrap();
        fs::hard_link(&staged_video, &video).unwrap();
        fs::write(&image, b"damaged still").unwrap();
        fs::write(&video, b"damaged movie").unwrap();
        assert_eq!(fs::read(&staged_image).unwrap(), b"damaged still");
        assert_eq!(fs::read(&staged_video).unwrap(), b"damaged movie");
        let failure = readback_published(
            pair,
            staging,
            PublishedPair {
                image: image.clone(),
                video: video.clone(),
            },
        )
        .unwrap_err();
        let retained = failure.retained.as_ref().unwrap();
        assert!(retained.recovery.is_ok());
        assert_eq!(
            fs::read(retained.staging.join("recovery.heic")).unwrap(),
            expected_still
        );
        assert_eq!(
            fs::read(retained.staging.join("recovery.mov")).unwrap(),
            expected_movie
        );
        // Further changes to either published inode cannot damage recovery.
        fs::write(&image, b"later still").unwrap();
        fs::write(&video, b"later movie").unwrap();
        assert_eq!(
            fs::read(retained.staging.join("recovery.heic")).unwrap(),
            expected_still
        );
        assert_eq!(
            fs::read(retained.staging.join("recovery.mov")).unwrap(),
            expected_movie
        );
        assert!(failure
            .into_runtime_error()
            .to_string()
            .contains("independent checked recovery pair"));
        assert_eq!(fs::read(image).unwrap(), b"later still");
        assert_eq!(fs::read(video).unwrap(), b"later movie");
    }

    #[test]
    fn failed_recovery_write_is_reported_unverified_without_overwriting_entries() {
        let directory = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir_in(directory.path()).unwrap();
        let pair = CheckedPair {
            still: b"still".to_vec(),
            movie: b"movie".to_vec(),
        };
        let occupied = staging.path().join("recovery.heic");
        fs::write(&occupied, b"existing recovery entry").unwrap();
        let image = directory.path().join("capture.heic");
        let video = companion_video_path(&image);
        fs::write(&image, b"changed").unwrap();
        fs::write(&video, &pair.movie).unwrap();
        let failure =
            readback_published(pair, staging, PublishedPair { image, video }).unwrap_err();
        assert!(failure.retained.as_ref().unwrap().recovery.is_err());
        let error = failure.into_runtime_error().to_string();
        assert!(error.contains("recovery-unverified"));
        assert!(!error.contains("independent checked recovery pair"));
        assert_eq!(fs::read(occupied).unwrap(), b"existing recovery entry");
    }
}
