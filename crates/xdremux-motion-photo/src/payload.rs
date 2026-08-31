use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::MotionPhotoError;
use crate::model::ByteRange;

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
    let file_size = fs::metadata(source_path)?.len();
    if range.upper_bound > file_size {
        return Err(MotionPhotoError::InvalidByteRange.into());
    }

    if let Some(parent) = destination_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    if destination_path.exists() {
        fs::remove_file(destination_path)?;
    }

    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination_path)?;

    let result = (|| -> CopyResult<()> {
        // Open the source only after the destination has been created to mirror the current
        // Swift transaction ordering. This also ensures a same-path misuse fails closed and the
        // partial destination is removed below.
        let mut source = File::open(source_path)?;
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
        destination.sync_all()?;
        Ok(())
    })();

    if result.is_err() {
        drop(destination);
        let _ = fs::remove_file(destination_path);
    }
    result
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
    fn same_path_misuse_fails_closed_and_removes_partial_destination() {
        let temp = TempDir::new();
        let source = temp.path.join("source.bin");
        fs::write(&source, vec![0x88; 16]).unwrap();

        let error = copy_payload_range_with_options(
            &source,
            ByteRange::new(0, 16).unwrap(),
            &source,
            1024,
            8,
        )
        .unwrap_err();
        assert_eq!(
            error.motion_photo_error(),
            Some(&MotionPhotoError::InvalidByteRange)
        );
        assert!(!source.exists());
    }
}
