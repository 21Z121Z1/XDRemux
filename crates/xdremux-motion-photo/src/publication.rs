use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static PUBLICATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct PairPublicationError {
    detail: String,
    source: Option<io::Error>,
}

impl PairPublicationError {
    fn invalid(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            source: None,
        }
    }

    fn io(operation: &'static str, path: &Path, source: io::Error) -> Self {
        Self {
            detail: format!("{operation} {}: {source}", path.display()),
            source: Some(source),
        }
    }
}

impl fmt::Display for PairPublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for PairPublicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|error| error as &(dyn Error + 'static))
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

fn unique_suffix() -> String {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let count = PUBLICATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{time:x}-{count:x}", std::process::id())
}

fn unlink_if_exists(path: &Path) -> PairPublicationResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(PairPublicationError::io("could not remove", path, error)),
    }
}

fn rename(source: &Path, destination: &Path) -> PairPublicationResult<()> {
    fs::rename(source, destination)
        .map_err(|error| PairPublicationError::io("could not rename", source, error))
}

fn remove_stale_artifacts(image: &Path, video: &Path) -> PairPublicationResult<()> {
    let directory = publication_parent(image);
    let image_name = image
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PairPublicationError::invalid("Live Photo image path has no UTF-8 file name"))?;
    let video_name = video
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PairPublicationError::invalid("Live Photo movie path has no UTF-8 file name"))?;
    let stem = image
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PairPublicationError::invalid("Live Photo image path has no UTF-8 stem"))?;

    let legacy = directory.join(".xdremux-live-photo-transactions");
    match fs::remove_dir_all(&legacy) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(PairPublicationError::io("could not remove", &legacy, error)),
    }
    unlink_if_exists(&directory.join(".xdremux-live-photo-transactions.lock"))?;

    let entries = fs::read_dir(directory)
        .map_err(|error| PairPublicationError::io("could not inspect", directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| PairPublicationError::io("could not inspect", directory, error))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_backup = (name.starts_with(&format!(".{image_name}."))
            || name.starts_with(&format!(".{video_name}.")))
            && name.ends_with(".backup");
        let is_temporary = name.starts_with(&format!(".{stem}."))
            && (name.ends_with(".tmp.heic")
                || name.ends_with(".tmp.heif")
                || name.ends_with(".tmp.mov"));
        if !is_backup && !is_temporary {
            continue;
        }
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| PairPublicationError::io("could not inspect", &path, error))?;
        if file_type.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|error| PairPublicationError::io("could not remove", &path, error))?;
        } else {
            unlink_if_exists(&path)?;
        }
    }
    Ok(())
}

/// Reconcile a previously interrupted Live Photo publication.
///
/// A local filesystem cannot atomically rename two independent directory entries
/// as one operation. The product contract is therefore explicit: an incomplete
/// or invalid final pair is disposable derived state and is rebuilt from the
/// immutable Motion Photo source on the next run.
pub fn reconcile_live_photo_pair<F>(
    image: &Path,
    video: &Path,
    pair_validator: F,
) -> PairPublicationResult<()>
where
    F: FnOnce(&Path, &Path) -> bool,
{
    let directory = publication_parent(image);
    if publication_parent(video) != directory {
        return Err(PairPublicationError::invalid(
            "Live Photo resources must share one destination directory",
        ));
    }
    fs::create_dir_all(directory)
        .map_err(|error| PairPublicationError::io("could not create", directory, error))?;
    remove_stale_artifacts(image, video)?;

    let image_exists = image.is_file();
    let video_exists = video.is_file();
    if !image_exists && !video_exists {
        return Ok(());
    }
    if image_exists && video_exists && pair_validator(image, video) {
        return Ok(());
    }
    unlink_if_exists(image)?;
    unlink_if_exists(video)?;
    Ok(())
}

/// Install an already validated HEIC/MOV pair with in-process rollback.
///
/// All four paths must be siblings so each individual rename remains an atomic,
/// same-filesystem operation. This is deliberately not described as a two-file
/// atomic transaction: a process crash between the two final renames can expose
/// a partial pair, which `reconcile_live_photo_pair` removes on the next run.
pub fn publish_live_photo_pair(
    temporary_image: &Path,
    temporary_video: &Path,
    final_image: &Path,
    final_video: &Path,
) -> PairPublicationResult<()> {
    let directory = publication_parent(final_image);
    if publication_parent(final_video) != directory
        || publication_parent(temporary_image) != directory
        || publication_parent(temporary_video) != directory
    {
        return Err(PairPublicationError::invalid(
            "Live Photo publication resources must share the destination directory/filesystem",
        ));
    }
    if !temporary_image.is_file() || !temporary_video.is_file() {
        return Err(PairPublicationError::invalid(
            "validated Live Photo temporary pair is incomplete",
        ));
    }

    let suffix = unique_suffix();
    let image_name = final_image
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PairPublicationError::invalid("Live Photo image path has no UTF-8 file name"))?;
    let video_name = final_video
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PairPublicationError::invalid("Live Photo movie path has no UTF-8 file name"))?;
    let image_backup = directory.join(format!(".{image_name}.{suffix}.backup"));
    let video_backup = directory.join(format!(".{video_name}.{suffix}.backup"));
    let had_image = final_image.exists();
    let had_video = final_video.exists();
    let mut image_installed = false;
    let mut video_installed = false;

    let publish_result = (|| {
        if had_image {
            rename(final_image, &image_backup)?;
        }
        if had_video {
            rename(final_video, &video_backup)?;
        }
        rename(temporary_image, final_image)?;
        image_installed = true;
        rename(temporary_video, final_video)?;
        video_installed = true;
        Ok(())
    })();

    if let Err(error) = publish_result {
        if image_installed {
            let _ = unlink_if_exists(final_image);
        }
        if video_installed {
            let _ = unlink_if_exists(final_video);
        }
        if had_image && image_backup.exists() {
            let _ = rename(&image_backup, final_image);
        }
        if had_video && video_backup.exists() {
            let _ = rename(&video_backup, final_video);
        }
        return Err(error);
    }

    let _ = unlink_if_exists(&image_backup);
    let _ = unlink_if_exists(&video_backup);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "xdremux-motion-photo-{name}-{}",
            unique_suffix()
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
    fn publication_installs_both_resources_and_cleans_backups() {
        let directory = test_directory("publish");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        let temporary_image = directory.join(".capture.one.tmp.heic");
        let temporary_video = directory.join(".capture.one.tmp.mov");
        fs::write(&image, b"old-image").unwrap();
        fs::write(&video, b"old-video").unwrap();
        fs::write(&temporary_image, b"new-image").unwrap();
        fs::write(&temporary_video, b"new-video").unwrap();

        publish_live_photo_pair(&temporary_image, &temporary_video, &image, &video).unwrap();

        assert_eq!(fs::read(&image).unwrap(), b"new-image");
        assert_eq!(fs::read(&video).unwrap(), b"new-video");
        assert!(!temporary_image.exists());
        assert!(!temporary_video.exists());
        assert!(fs::read_dir(&directory)
            .unwrap()
            .all(|entry| !entry.unwrap().file_name().to_string_lossy().ends_with(".backup")));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reconcile_keeps_only_a_valid_complete_pair() {
        let directory = test_directory("reconcile");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        fs::write(&image, b"image").unwrap();

        reconcile_live_photo_pair(&image, &video, |_, _| true).unwrap();
        assert!(!image.exists());
        assert!(!video.exists());

        fs::write(&image, b"image").unwrap();
        fs::write(&video, b"video").unwrap();
        reconcile_live_photo_pair(&image, &video, |_, _| true).unwrap();
        assert!(image.exists());
        assert!(video.exists());

        reconcile_live_photo_pair(&image, &video, |_, _| false).unwrap();
        assert!(!image.exists());
        assert!(!video.exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn stale_temporary_and_backup_artifacts_are_removed_before_reconcile() {
        let directory = test_directory("stale");
        let image = directory.join("capture.heic");
        let video = directory.join("capture.mov");
        let stale_image = directory.join(".capture.dead.tmp.heic");
        let stale_video = directory.join(".capture.dead.tmp.mov");
        let stale_backup = directory.join(".capture.heic.dead.backup");
        fs::write(&stale_image, b"temp").unwrap();
        fs::write(&stale_video, b"temp").unwrap();
        fs::write(&stale_backup, b"backup").unwrap();

        reconcile_live_photo_pair(&image, &video, |_, _| false).unwrap();

        assert!(!stale_image.exists());
        assert!(!stale_video.exists());
        assert!(!stale_backup.exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
