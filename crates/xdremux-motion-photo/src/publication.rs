use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PairPublicationError {
    detail: String,
    source: Option<io::Error>,
    partial_output: Option<PathBuf>,
}

impl PairPublicationError {
    fn invalid(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            source: None,
            partial_output: None,
        }
    }

    fn io(operation: &'static str, path: &Path, source: io::Error) -> Self {
        Self {
            detail: format!("{operation} {}: {source}", path.display()),
            source: Some(source),
            partial_output: None,
        }
    }

    /// An entry created by this attempt before its second publication failed.
    /// Another writer may have changed it since; this is not deletion authority.
    pub fn partial_output(&self) -> Option<&Path> {
        self.partial_output.as_deref()
    }
}

impl fmt::Display for PairPublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for PairPublicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|error| error as &(dyn Error + 'static))
    }
}

pub type PairPublicationResult<T> = std::result::Result<T, PairPublicationError>;

pub fn companion_video_path(image: &Path) -> PathBuf {
    image.with_extension("mov")
}

fn publication_parent(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn entry_exists(path: &Path) -> PairPublicationResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(PairPublicationError::io("could not inspect", path, error)),
    }
}

/// Check a destination without deleting final entries, sidecars or directories.
///
/// Empty destinations and complete validated pairs are accepted. Partial or
/// invalid pairs require explicit recovery; their names do not prove ownership.
pub fn reconcile_live_photo_pair<F>(
    image: &Path,
    video: &Path,
    pair_validator: F,
) -> PairPublicationResult<()>
where
    F: FnOnce(&Path, &Path) -> bool,
{
    let directory = publication_parent(image);
    if image == video || publication_parent(video) != directory {
        return Err(PairPublicationError::invalid(
            "Live Photo final resources must be distinct paths in one destination directory",
        ));
    }
    fs::create_dir_all(directory)
        .map_err(|error| PairPublicationError::io("could not create", directory, error))?;
    let image_exists = entry_exists(image)?;
    let video_exists = entry_exists(video)?;
    if !image_exists && !video_exists {
        return Ok(());
    }
    if image_exists
        && video_exists
        && image.is_file()
        && video.is_file()
        && pair_validator(image, video)
    {
        return Ok(());
    }
    Err(PairPublicationError::invalid(
        "existing Live Photo pair is incomplete or invalid; preserving all entries for explicit recovery",
    ))
}

/// Publish an already validated pair with per-entry atomic no-clobber links.
///
/// Finals must be distinct siblings. Caller-owned staging may be in a private
/// directory on the same filesystem and is retained on success and failure.
/// The MOV entry is claimed first. Failure of the still link preserves that
/// entry and reports its path; callers must retain staging for explicit recovery.
/// No final entry is replaced or unlinked. Two paths are not a system-atomic
/// transaction, and another writer can still actively replace them itself.
/// The destination filesystem must support hard links; unsupported filesystems
/// fail closed with the original I/O error rather than falling back to overwrite.
pub fn publish_live_photo_pair(
    temporary_image: &Path,
    temporary_video: &Path,
    final_image: &Path,
    final_video: &Path,
) -> PairPublicationResult<()> {
    let directory = publication_parent(final_image);
    if final_image == final_video || publication_parent(final_video) != directory {
        return Err(PairPublicationError::invalid(
            "Live Photo final resources must be distinct paths in one destination directory",
        ));
    }
    if !temporary_image.is_file() || !temporary_video.is_file() {
        return Err(PairPublicationError::invalid(
            "validated Live Photo temporary pair is incomplete",
        ));
    }

    fs::hard_link(temporary_video, final_video).map_err(|error| {
        PairPublicationError::io(
            "could not publish movie with a no-clobber hard link (filesystem hard-link support required)",
            final_video,
            error,
        )
    })?;
    if let Err(source) = fs::hard_link(temporary_image, final_image) {
        let mut error = PairPublicationError::io(
            "could not publish still with a no-clobber hard link (filesystem hard-link support required)",
            final_image,
            source,
        );
        error.detail.push_str(&format!(
            "; MOV was created at {}; final entries preserved",
            final_video.display()
        ));
        error.partial_output = Some(final_video.to_path_buf());
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static PUBLICATION_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_directory(name: &str) -> PathBuf {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let count = PUBLICATION_COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "xdremux-motion-photo-{name}-{}-{time:x}-{count:x}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn companion_movie_uses_same_stem() {
        assert_eq!(
            companion_video_path(Path::new("capture.heic")),
            PathBuf::from("capture.mov")
        );
    }

    #[test]
    fn publication_refuses_existing_resources_and_preserves_staging() {
        let directory = test_directory("publish");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        let temporary_image = directory.join(".capture.one.tmp.heic");
        let temporary_video = directory.join(".capture.one.tmp.mov");
        fs::write(&image, b"old-image").unwrap();
        fs::write(&video, b"old-video").unwrap();
        fs::write(&temporary_image, b"new-image").unwrap();
        fs::write(&temporary_video, b"new-video").unwrap();

        let error = publish_live_photo_pair(&temporary_image, &temporary_video, &image, &video)
            .unwrap_err();
        assert!(error.partial_output().is_none());
        assert_eq!(
            error.source.as_ref().unwrap().kind(),
            io::ErrorKind::AlreadyExists
        );

        assert_eq!(fs::read(&image).unwrap(), b"old-image");
        assert_eq!(fs::read(&video).unwrap(), b"old-video");
        assert_eq!(fs::read(&temporary_image).unwrap(), b"new-image");
        assert_eq!(fs::read(&temporary_video).unwrap(), b"new-video");
        assert!(fs::read_dir(&directory).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".backup")
        }));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reconcile_preserves_partial_invalid_and_valid_pairs() {
        let directory = test_directory("reconcile");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        fs::write(&image, b"image").unwrap();

        reconcile_live_photo_pair(&image, &video, |_, _| true).unwrap_err();
        assert_eq!(fs::read(&image).unwrap(), b"image");
        assert!(!video.exists());

        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        reconcile_live_photo_pair(&image, &video, |_, _| true).unwrap();
        assert!(image.exists());
        assert!(video.exists());

        reconcile_live_photo_pair(&image, &video, |_, _| false).unwrap_err();
        assert_eq!(fs::read(&image).unwrap(), b"image");
        assert_eq!(fs::read(&video).unwrap(), b"video");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reconcile_preserves_unowned_artifacts_and_directories() {
        let directory = test_directory("stale");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        let stale_image = directory.join(".capture.dead.tmp.heic");
        let stale_video = directory.join(".capture.dead.tmp.mov");
        let stale_backup = directory.join(".capture.heic.dead.backup");
        fs::write(&stale_image, b"temp").unwrap();
        fs::write(&stale_video, b"temp").unwrap();
        fs::write(&stale_backup, b"backup").unwrap();
        let source_directory = directory.join(".capture.unowned.tmp.heic");
        fs::create_dir(&source_directory).unwrap();
        fs::write(source_directory.join("source.heic"), b"source").unwrap();
        let legacy = directory.join(".xdremux-live-photo-transactions");
        fs::create_dir(&legacy).unwrap();
        fs::write(legacy.join("retained"), b"legacy bytes").unwrap();
        let lock = directory.join(".xdremux-live-photo-transactions.lock");
        fs::write(&lock, b"unowned lock").unwrap();

        reconcile_live_photo_pair(&image, &video, |_, _| false).unwrap();

        assert_eq!(fs::read(stale_image).unwrap(), b"temp");
        assert_eq!(fs::read(stale_video).unwrap(), b"temp");
        assert_eq!(fs::read(stale_backup).unwrap(), b"backup");
        assert_eq!(
            fs::read(source_directory.join("source.heic")).unwrap(),
            b"source"
        );
        assert_eq!(fs::read(legacy.join("retained")).unwrap(), b"legacy bytes");
        assert_eq!(fs::read(lock).unwrap(), b"unowned lock");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn publication_retains_a_recoverable_partial_pair_after_second_link_failure() {
        let directory = test_directory("partial");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        let temporary_image = directory.join("staged.heic");
        let temporary_video = directory.join("staged.mov");
        fs::write(&temporary_image, b"new-image").unwrap();
        fs::write(&temporary_video, b"new-video").unwrap();
        fs::write(&image, b"competitor-image").unwrap();
        let error = publish_live_photo_pair(&temporary_image, &temporary_video, &image, &video)
            .unwrap_err();
        assert!(error.source().is_some());
        assert_eq!(error.partial_output(), Some(video.as_path()));
        assert_eq!(
            error.source.as_ref().unwrap().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&image).unwrap(), b"competitor-image");
        assert_eq!(fs::read(video).unwrap(), b"new-video");
        assert_eq!(fs::read(temporary_image).unwrap(), b"new-image");
        assert_eq!(fs::read(temporary_video).unwrap(), b"new-video");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn publication_accepts_owned_staging_under_destination_directory() {
        let directory = test_directory("owned-staging");
        let staging = directory.join("owned");
        fs::create_dir(&staging).unwrap();
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        let temporary_image = staging.join("still.heic");
        let temporary_video = staging.join("motion.mov");
        fs::write(&temporary_image, b"image").unwrap();
        fs::write(&temporary_video, b"video").unwrap();
        publish_live_photo_pair(&temporary_image, &temporary_video, &image, &video).unwrap();
        assert_eq!(fs::read(image).unwrap(), b"image");
        assert_eq!(fs::read(video).unwrap(), b"video");
        assert_eq!(fs::read(temporary_image).unwrap(), b"image");
        assert_eq!(fs::read(temporary_video).unwrap(), b"video");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn publication_rejects_identical_final_paths_without_effects() {
        let directory = test_directory("identical-finals");
        let image = directory.join("capture.heic");
        let temporary_image = directory.join("staged.heic");
        let temporary_video = directory.join("staged.mov");
        fs::write(&temporary_image, b"image").unwrap();
        fs::write(&temporary_video, b"video").unwrap();
        publish_live_photo_pair(&temporary_image, &temporary_video, &image, &image).unwrap_err();
        reconcile_live_photo_pair(&image, &image, |_, _| true).unwrap_err();
        assert!(!image.exists());
        assert_eq!(fs::read(temporary_image).unwrap(), b"image");
        assert_eq!(fs::read(temporary_video).unwrap(), b"video");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn publication_claims_shared_movie_before_either_still_extension() {
        let directory = test_directory("shared-movie");
        let image = directory.join("capture.heic");
        let alternate_image = directory.join("capture.heif");
        let video = companion_video_path(&image);
        assert_eq!(video, companion_video_path(&alternate_image));
        let temporary_image = directory.join("staged.heic");
        let temporary_video = directory.join("staged.mov");
        fs::write(&temporary_image, b"first-image").unwrap();
        fs::write(&temporary_video, b"first-video").unwrap();
        publish_live_photo_pair(&temporary_image, &temporary_video, &image, &video).unwrap();
        let alternate_staged_image = directory.join("second.heif");
        let alternate_staged_video = directory.join("second.mov");
        fs::write(&alternate_staged_image, b"second-image").unwrap();
        fs::write(&alternate_staged_video, b"second-video").unwrap();
        let error = publish_live_photo_pair(
            &alternate_staged_image,
            &alternate_staged_video,
            &alternate_image,
            &video,
        )
        .unwrap_err();
        assert!(error.partial_output().is_none());
        assert_eq!(
            error.source.as_ref().unwrap().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(!alternate_image.exists());
        assert_eq!(fs::read(image).unwrap(), b"first-image");
        assert_eq!(fs::read(video).unwrap(), b"first-video");
        assert_eq!(fs::read(alternate_staged_image).unwrap(), b"second-image");
        assert_eq!(fs::read(alternate_staged_video).unwrap(), b"second-video");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn publication_preserves_a_dangling_competitor_symlink() {
        use std::os::unix::fs::symlink;
        let directory = test_directory("dangling-final");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        let target = directory.join("missing-competitor-target");
        symlink(&target, &video).unwrap();
        let temporary_image = directory.join("staged.heic");
        let temporary_video = directory.join("staged.mov");
        fs::write(&temporary_image, b"image").unwrap();
        fs::write(&temporary_video, b"video").unwrap();
        publish_live_photo_pair(&temporary_image, &temporary_video, &image, &video).unwrap_err();
        assert_eq!(fs::read_link(video).unwrap(), target);
        assert!(!image.exists());
        assert_eq!(fs::read(temporary_image).unwrap(), b"image");
        assert_eq!(fs::read(temporary_video).unwrap(), b"video");
        fs::remove_dir_all(directory).unwrap();
    }
}
