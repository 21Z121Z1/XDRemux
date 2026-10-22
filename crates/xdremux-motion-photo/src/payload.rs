use std::collections::HashSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::{ByteRange, MotionPhotoError};

pub const DEFAULT_MAX_PAYLOAD_BYTES: u64 = 1_073_741_824;
pub const DEFAULT_COPY_BUFFER_SIZE: usize = 1_048_576;

#[derive(Debug)]
pub enum MotionPhotoCopyError {
    MotionPhoto(MotionPhotoError),
    Io(io::Error),
}

impl MotionPhotoCopyError {
    pub fn motion_photo_error(&self) -> Option<&MotionPhotoError> {
        match self {
            Self::MotionPhoto(error) => Some(error),
            Self::Io(_) => None,
        }
    }
}

impl fmt::Display for MotionPhotoCopyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MotionPhoto(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for MotionPhotoCopyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MotionPhoto(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<MotionPhotoError> for MotionPhotoCopyError {
    fn from(error: MotionPhotoError) -> Self {
        Self::MotionPhoto(error)
    }
}

impl From<io::Error> for MotionPhotoCopyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub type CopyResult<T> = std::result::Result<T, MotionPhotoCopyError>;

/// Copies a bounded byte range with the publication rules of
/// [`copy_payload_range_with_options`] and the default size limits.
pub fn copy_payload_range(
    source_path: impl AsRef<Path>,
    range: ByteRange,
    destination_path: impl AsRef<Path>,
) -> CopyResult<()> {
    copy_payload_range_with_options(
        source_path,
        range,
        destination_path,
        DEFAULT_MAX_PAYLOAD_BYTES,
        DEFAULT_COPY_BUFFER_SIZE,
    )
}

/// Copies an exact range, replacing the destination only after the copy is complete.
///
/// A failed copy preserves the source and any existing destination, removing only
/// its own staging file. Destination entries used to resolve the source are
/// rejected. Distinct hard links and safe destination symlinks are replaced as
/// entries without changing their targets. Alias checks can require directory
/// read access; a failed check leaves both entries intact. Callers must keep
/// source and destination path resolution stable during the operation.
pub fn copy_payload_range_with_options(
    source_path: impl AsRef<Path>,
    range: ByteRange,
    destination_path: impl AsRef<Path>,
    max_bytes: u64,
    buffer_size: usize,
) -> CopyResult<()> {
    if range.length() > max_bytes {
        return Err(MotionPhotoError::PayloadTooLarge.into());
    }
    if buffer_size == 0 {
        return Err(MotionPhotoError::InvalidByteRange.into());
    }

    let source_path = source_path.as_ref();
    let destination_path = destination_path.as_ref();
    let mut source = File::open(source_path)?;
    let file_size = source.metadata()?.len();
    if range.upper_bound > file_size {
        return Err(MotionPhotoError::InvalidByteRange.into());
    }

    let parent = entry_parent(destination_path);
    fs::create_dir_all(parent)?;
    if source_uses_destination_entry(source_path, destination_path)? {
        return Err(MotionPhotoError::InvalidByteRange.into());
    }
    let mut destination = tempfile::Builder::new()
        .prefix(".xdremux-payload-")
        .make_in(parent, |path| {
            OpenOptions::new().write(true).create_new(true).open(path)
        })?;
    source.seek(SeekFrom::Start(range.lower_bound))?;

    let buffer_size_u64 =
        u64::try_from(buffer_size).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
    let mut remaining = range.length();
    let mut buffer = vec![0u8; buffer_size];
    while remaining > 0 {
        let requested = usize::try_from(remaining.min(buffer_size_u64))
            .map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
        let read = source.read(&mut buffer[..requested])?;
        if read == 0 {
            return Err(MotionPhotoError::InvalidByteRange.into());
        }
        destination.write_all(&buffer[..read])?;
        let read_u64 = u64::try_from(read).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
        remaining = remaining
            .checked_sub(read_u64)
            .ok_or(MotionPhotoError::ArithmeticOverflow)?;
    }
    destination.as_file().sync_all()?;
    destination
        .persist(destination_path)
        .map_err(|error| error.error)?;
    Ok(())
}

fn entry_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn source_uses_destination_entry(source: &Path, destination: &Path) -> io::Result<bool> {
    let destination_metadata = match fs::symlink_metadata(destination) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let mut pending = vec![fs::canonicalize(source)?, source.to_path_buf()];
    let mut inspected = HashSet::new();
    while let Some(path) = pending.pop() {
        for ancestor in path.ancestors() {
            let Some(name) = ancestor.file_name() else {
                continue;
            };
            let entry = fs::canonicalize(entry_parent(ancestor))?.join(name);
            if !inspected.insert(entry.clone()) {
                continue;
            }
            if same_entry(&entry, destination, &destination_metadata)? {
                return Ok(true);
            }
            if fs::symlink_metadata(&entry)?.is_symlink() {
                pending.push(entry_parent(&entry).join(fs::read_link(&entry)?));
            }
        }
    }
    Ok(false)
}

#[cfg(unix)]
fn same_entry(left: &Path, right: &Path, right_metadata: &fs::Metadata) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let left_metadata = fs::symlink_metadata(left)?;
    if (left_metadata.dev(), left_metadata.ino()) != (right_metadata.dev(), right_metadata.ino()) {
        return Ok(false);
    }
    let left_parent = fs::metadata(entry_parent(left))?;
    let right_parent = fs::metadata(entry_parent(right))?;
    if (left_parent.dev(), left_parent.ino()) != (right_parent.dev(), right_parent.ino()) {
        return Ok(false);
    }
    if left.file_name() == right.file_name() {
        return Ok(true);
    }
    let mut left_present = false;
    let mut right_present = false;
    for entry in fs::read_dir(entry_parent(left))? {
        let name = entry?.file_name();
        left_present |= left.file_name() == Some(name.as_os_str());
        right_present |= right.file_name() == Some(name.as_os_str());
        if left_present && right_present {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(not(unix))]
fn same_entry(left: &Path, right: &Path, right_metadata: &fs::Metadata) -> io::Result<bool> {
    if fs::symlink_metadata(left)?.is_symlink() != right_metadata.is_symlink() {
        return Ok(false);
    }
    match fs::canonicalize(right) {
        Ok(right) => Ok(fs::canonicalize(left)? == right),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    struct TempDir {
        path: std::path::PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "xdremux-motion-payload-{}-{id}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn copies_exact_range_replaces_destination_and_creates_parent() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("nested/output.bin");
        fs::write(&source, (0u8..=127).collect::<Vec<_>>()).unwrap();
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, b"stale destination").unwrap();

        copy_payload_range_with_options(
            &source,
            ByteRange::new(17, 91).unwrap(),
            &destination,
            1024,
            13,
        )
        .unwrap();

        assert_eq!(
            fs::read(&destination).unwrap(),
            (17u8..91).collect::<Vec<_>>()
        );
    }

    #[test]
    fn payload_limit_failure_leaves_existing_destination_untouched() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("output.bin");
        fs::write(&source, vec![0x55; 32]).unwrap();
        fs::write(&destination, b"keep me").unwrap();

        let error = copy_payload_range_with_options(
            &source,
            ByteRange::new(0, 32).unwrap(),
            &destination,
            31,
            8,
        )
        .unwrap_err();
        assert_eq!(
            error.motion_photo_error(),
            Some(&MotionPhotoError::PayloadTooLarge)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"keep me");
    }

    #[test]
    fn zero_buffer_failure_leaves_existing_destination_untouched() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("output.bin");
        fs::write(&source, vec![0x66; 16]).unwrap();
        fs::write(&destination, b"keep me").unwrap();

        let error = copy_payload_range_with_options(
            &source,
            ByteRange::new(0, 16).unwrap(),
            &destination,
            1024,
            0,
        )
        .unwrap_err();
        assert_eq!(
            error.motion_photo_error(),
            Some(&MotionPhotoError::InvalidByteRange)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"keep me");
    }

    #[test]
    fn out_of_file_range_leaves_existing_destination_untouched() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("output.bin");
        fs::write(&source, vec![0x77; 8]).unwrap();
        fs::write(&destination, b"keep me").unwrap();

        let error = copy_payload_range_with_options(
            &source,
            ByteRange::new(0, 9).unwrap(),
            &destination,
            1024,
            8,
        )
        .unwrap_err();
        assert_eq!(
            error.motion_photo_error(),
            Some(&MotionPhotoError::InvalidByteRange)
        );
        assert_eq!(fs::read(&destination).unwrap(), b"keep me");
    }

    #[test]
    fn same_path_is_rejected_without_changing_source() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        fs::write(&source, vec![0x88; 16]).unwrap();

        assert_alias_preserves_source(&source, &source);
    }

    fn assert_alias_preserves_source(source: &Path, destination: &Path) {
        let original = fs::read(source).unwrap();
        let error = copy_payload_range_with_options(
            source,
            ByteRange::new(3, 11).unwrap(),
            destination,
            1024,
            8,
        )
        .unwrap_err();
        assert_eq!(
            error.motion_photo_error(),
            Some(&MotionPhotoError::InvalidByteRange)
        );
        assert_eq!(fs::read(source).unwrap(), original);
        assert_eq!(fs::read(destination).unwrap(), original);
    }

    #[test]
    fn parent_component_alias_is_rejected_without_changing_source() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        fs::write(&source, vec![0x89; 16]).unwrap();
        fs::create_dir(temp.path.join("nested")).unwrap();

        assert_alias_preserves_source(&source, &temp.path.join("nested/../source.bin"));
    }

    #[test]
    fn case_alias_is_rejected_on_case_insensitive_filesystems() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("SOURCE.BIN");
        fs::write(&source, vec![0x8a; 16]).unwrap();
        if !destination.exists() {
            return;
        }

        assert_alias_preserves_source(&source, &destination);
    }

    #[test]
    fn distinct_hardlink_is_replaced_without_changing_source() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("output.bin");
        let original = (0u8..16).collect::<Vec<_>>();
        fs::write(&source, &original).unwrap();
        fs::hard_link(&source, &destination).unwrap();

        copy_payload_range(&source, ByteRange::new(3, 11).unwrap(), &destination).unwrap();

        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(
            fs::read(&destination).unwrap(),
            (3u8..11).collect::<Vec<_>>()
        );
    }

    #[test]
    fn copy_creates_missing_destination_parents() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("new/nested/output.bin");
        fs::write(&source, (0u8..16).collect::<Vec<_>>()).unwrap();

        copy_payload_range(&source, ByteRange::new(3, 11).unwrap(), &destination).unwrap();

        assert_eq!(
            fs::read(&destination).unwrap(),
            (3u8..11).collect::<Vec<_>>()
        );
        assert_eq!(fs::read(&source).unwrap(), (0u8..16).collect::<Vec<_>>());
    }

    #[test]
    fn publication_failure_preserves_existing_entries_and_leaves_no_partial_file() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("output.bin");
        fs::write(&source, vec![0x8b; 16]).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("keep.bin"), b"published bytes").unwrap();

        assert!(copy_payload_range(&source, ByteRange::new(3, 11).unwrap(), &destination).is_err());

        assert_eq!(fs::read(&source).unwrap(), vec![0x8b; 16]);
        assert_eq!(
            fs::read(destination.join("keep.bin")).unwrap(),
            b"published bytes"
        );
        assert_eq!(fs::read_dir(&temp.path).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_parent_alias_is_rejected_without_changing_source() {
        let temp = TempDir::new();
        let source = temp.path.join("actual/source.bin");
        fs::create_dir(source.parent().unwrap()).unwrap();
        fs::write(&source, vec![0x8c; 16]).unwrap();
        std::os::unix::fs::symlink("actual", temp.path.join("alias")).unwrap();

        assert_alias_preserves_source(&source, &temp.path.join("alias/source.bin"));
    }

    #[cfg(unix)]
    #[test]
    fn source_symlink_target_alias_is_rejected_without_changing_source() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("target.bin");
        fs::write(&destination, vec![0x8d; 16]).unwrap();
        std::os::unix::fs::symlink("target.bin", &source).unwrap();

        assert_alias_preserves_source(&source, &destination);
        assert_eq!(fs::read_link(&source).unwrap(), Path::new("target.bin"));
    }

    #[cfg(unix)]
    #[test]
    fn same_symlink_entry_is_rejected_without_changing_source() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        fs::write(temp.path.join("target.bin"), vec![0x8e; 16]).unwrap();
        std::os::unix::fs::symlink("target.bin", &source).unwrap();

        assert_alias_preserves_source(&source, &source);
        assert_eq!(fs::read_link(&source).unwrap(), Path::new("target.bin"));
    }

    #[cfg(unix)]
    #[test]
    fn source_symlink_chain_is_rejected_without_replacing_an_intermediate_link() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("middle.bin");
        fs::write(temp.path.join("target.bin"), vec![0x8f; 16]).unwrap();
        std::os::unix::fs::symlink("middle.bin", &source).unwrap();
        std::os::unix::fs::symlink("target.bin", &destination).unwrap();

        assert_alias_preserves_source(&source, &destination);
        assert_eq!(
            fs::read_link(&destination).unwrap(),
            Path::new("target.bin")
        );
    }

    #[cfg(unix)]
    #[test]
    fn source_parent_symlink_is_rejected_without_replacing_the_directory_link() {
        let temp = TempDir::new();
        let destination = temp.path.join("alias");
        let source = destination.join("source.bin");
        fs::create_dir(temp.path.join("actual")).unwrap();
        fs::write(temp.path.join("actual/source.bin"), vec![0x90; 16]).unwrap();
        std::os::unix::fs::symlink("actual", &destination).unwrap();

        let error =
            copy_payload_range(&source, ByteRange::new(3, 11).unwrap(), &destination).unwrap_err();

        assert_eq!(
            error.motion_photo_error(),
            Some(&MotionPhotoError::InvalidByteRange)
        );
        assert_eq!(fs::read(&source).unwrap(), vec![0x90; 16]);
        assert_eq!(fs::read_link(&destination).unwrap(), Path::new("actual"));
    }

    #[cfg(unix)]
    #[test]
    fn destination_symlink_is_replaced_without_changing_its_target() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("output.bin");
        let target = temp.path.join("target.bin");
        fs::write(&source, (0u8..16).collect::<Vec<_>>()).unwrap();
        fs::write(&target, b"keep target").unwrap();
        std::os::unix::fs::symlink("target.bin", &destination).unwrap();

        copy_payload_range(&source, ByteRange::new(3, 11).unwrap(), &destination).unwrap();

        assert_eq!(
            fs::read(&destination).unwrap(),
            (3u8..11).collect::<Vec<_>>()
        );
        assert!(!fs::symlink_metadata(&destination).unwrap().is_symlink());
        assert_eq!(fs::read(&target).unwrap(), b"keep target");
        assert_eq!(fs::read(&source).unwrap(), (0u8..16).collect::<Vec<_>>());
    }

    #[cfg(unix)]
    #[test]
    fn destination_symlink_to_source_is_replaced_without_changing_source() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        let destination = temp.path.join("output.bin");
        fs::write(&source, (0u8..16).collect::<Vec<_>>()).unwrap();
        std::os::unix::fs::symlink("source.bin", &destination).unwrap();

        copy_payload_range(&source, ByteRange::new(3, 11).unwrap(), &destination).unwrap();

        assert_eq!(fs::read(&source).unwrap(), (0u8..16).collect::<Vec<_>>());
        assert_eq!(
            fs::read(&destination).unwrap(),
            (3u8..11).collect::<Vec<_>>()
        );
        assert!(!fs::symlink_metadata(&destination).unwrap().is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn read_failure_preserves_published_destination_and_removes_partial_output() {
        let temp = TempDir::new();
        let source = temp.path.join("source-directory");
        let destination = temp.path.join("output.bin");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("keep.bin"), b"source entry").unwrap();
        fs::write(&destination, b"published bytes").unwrap();

        assert!(copy_payload_range(&source, ByteRange::new(0, 1).unwrap(), &destination).is_err());

        assert_eq!(fs::read(&destination).unwrap(), b"published bytes");
        assert_eq!(fs::read(source.join("keep.bin")).unwrap(), b"source entry");
        assert_eq!(fs::read_dir(&temp.path).unwrap().count(), 2);
    }
}
