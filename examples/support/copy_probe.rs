//! Experimental single-file copy with a cancellation boundary before publication.
//! Not integrated with FileView's handlers. No snapshot, batch, or crash recovery guarantee.

use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};

const RUNNING: u8 = 0;
const CANCELLED: u8 = 1;
const COMMITTING: u8 = 2;
const FINISHED: u8 = 3;

/// Single-use cancellation token. Accepted cancellation prevents publication.
#[derive(Default)]
pub struct CancelToken(AtomicU8);

impl CancelToken {
    /// Request cancellation. False means publication has already started or the job ended.
    pub fn request_cancel(&self) -> bool {
        matches!(
            self.0
                .compare_exchange(RUNNING, CANCELLED, Ordering::AcqRel, Ordering::Acquire),
            Ok(_) | Err(CANCELLED)
        )
    }

    fn check(&self, copied: u64) -> Result<(), CopyError> {
        match self.0.load(Ordering::Acquire) {
            RUNNING => Ok(()),
            CANCELLED => Err(CopyError::Cancelled { copied }),
            _ => Err(CopyError::InvalidToken),
        }
    }

    fn begin_commit(&self, copied: u64) -> Result<(), CopyError> {
        self.0
            .compare_exchange(RUNNING, COMMITTING, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|state| {
                if state == CANCELLED {
                    CopyError::Cancelled { copied }
                } else {
                    CopyError::InvalidToken
                }
            })
    }
}

/// Result of the experimental operation.
#[derive(Debug, thiserror::Error)]
pub enum CopyError {
    /// A cancellation accepted before publication removed the temporary file.
    #[error("copy cancelled after {copied} bytes")]
    Cancelled { copied: u64 },
    /// Only a regular, non-symlink selected source entry is supported.
    #[error("only a regular non-symlink source is supported")]
    UnsupportedSource,
    /// Metadata or identity changed during the best-effort check.
    #[error("source changed while copying")]
    SourceChanged,
    /// Existing destinations must never be overwritten.
    #[error("destination already exists")]
    DestinationExists,
    /// A token belongs to one operation and cannot be reused.
    #[error("cancellation token has already been used")]
    InvalidToken,
    /// Ordinary I/O failure, including injected disk-full failures in tests.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Monotonic progress within one operation.
#[derive(Clone, Copy, Debug, Default)]
pub struct Progress {
    /// Bytes successfully written to the temporary file.
    pub copied: u64,
    /// Source length observed before copying.
    pub total: u64,
}

struct Finish<'a>(&'a CancelToken);
impl Drop for Finish<'_> {
    fn drop(&mut self) {
        self.0 .0.store(FINISHED, Ordering::Release);
    }
}

fn same_source(a: &Metadata, b: &Metadata) -> bool {
    let same = b.is_file() && a.len() == b.len() && a.modified().ok() == b.modified().ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        same && a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    {
        same
    }
}

/// Stream a regular file into a destination-local temporary file, then publish without overwrite.
///
/// Cancellation is checked per chunk, but cannot interrupt a blocking OS read/write/sync syscall.
/// Once `request_cancel` returns false at the commit boundary, a complete output may be published.
/// Source checks are best-effort, not an atomic snapshot. Permissions, ACLs, xattrs, resource forks,
/// sparse extents, directory copying, cross-device moves and crash recovery are deliberately absent.
pub fn copy_file(
    source: &Path,
    destination: &Path,
    chunk_bytes: usize,
    cancel: &CancelToken,
    progress: impl FnMut(Progress),
) -> Result<u64, CopyError> {
    copy_with_writer(
        source,
        destination,
        chunk_bytes,
        cancel,
        progress,
        |file, bytes| file.write_all(bytes),
    )
}

fn copy_with_writer(
    source: &Path,
    destination: &Path,
    chunk_bytes: usize,
    cancel: &CancelToken,
    mut progress: impl FnMut(Progress),
    mut write: impl FnMut(&mut File, &[u8]) -> io::Result<()>,
) -> Result<u64, CopyError> {
    let _finish = Finish(cancel);
    cancel.check(0)?;
    if chunk_bytes == 0 || chunk_bytes > 16 * 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "chunk must be 1..=16 MiB").into());
    }
    let before = source.symlink_metadata()?;
    if !before.is_file() || before.file_type().is_symlink() {
        return Err(CopyError::UnsupportedSource);
    }
    match destination.symlink_metadata() {
        Ok(_) => return Err(CopyError::DestinationExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut input = options.open(source)?;
    if !same_source(&before, &input.metadata()?) {
        return Err(CopyError::SourceChanged);
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".copy-probe-")
        .tempfile_in(parent)?;
    let mut buffer = vec![0; chunk_bytes];
    let mut copied = 0;
    progress(Progress {
        copied,
        total: before.len(),
    });
    while copied < before.len() {
        cancel.check(copied)?;
        let count = (before.len() - copied).min(chunk_bytes as u64) as usize;
        if let Err(error) = input.read_exact(&mut buffer[..count]) {
            return Err(if error.kind() == io::ErrorKind::UnexpectedEof {
                CopyError::SourceChanged
            } else {
                error.into()
            });
        }
        cancel.check(copied)?;
        write(temporary.as_file_mut(), &buffer[..count])?;
        copied += count as u64;
        progress(Progress {
            copied,
            total: before.len(),
        });
    }
    cancel.check(copied)?;
    temporary.as_file().sync_all()?;
    if !same_source(&before, &input.metadata()?)
        || !same_source(&before, &source.symlink_metadata()?)
    {
        return Err(CopyError::SourceChanged);
    }
    cancel.begin_commit(copied)?;
    temporary.persist_noclobber(destination).map_err(|error| {
        if error.error.kind() == io::ErrorKind::AlreadyExists {
            CopyError::DestinationExists
        } else {
            CopyError::Io(error.error)
        }
    })?;
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        Vec<u8>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let target_dir = dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        let bytes: Vec<_> = (0..256 * 1024).map(|i| (i % 251) as u8).collect();
        fs::write(&source, &bytes).unwrap();
        (dir, source, target_dir.join("copy"), bytes)
    }

    fn cleaned(destination: &Path) {
        assert!(!destination.exists());
        assert_eq!(
            fs::read_dir(destination.parent().unwrap()).unwrap().count(),
            0
        );
    }

    #[test]
    fn successful_copy_and_monotonic_progress() {
        let (_dir, source, destination, bytes) = setup();
        let token = CancelToken::default();
        let mut previous = 0;
        assert_eq!(
            copy_file(&source, &destination, 16 * 1024, &token, |p| {
                assert!(p.copied >= previous && p.copied <= p.total);
                previous = p.copied;
            })
            .unwrap(),
            bytes.len() as u64
        );
        assert_eq!(fs::read(&source).unwrap(), bytes);
        assert_eq!(fs::read(&destination).unwrap(), bytes);
        assert!(!token.request_cancel());
        assert_eq!(
            fs::read_dir(destination.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[test]
    fn cancellation_removes_partial_output_and_keeps_source() {
        let (_dir, source, destination, bytes) = setup();
        let token = CancelToken::default();
        let result = copy_file(&source, &destination, 16 * 1024, &token, |p| {
            if p.copied == 32 * 1024 {
                assert!(token.request_cancel());
            }
        });
        assert!(matches!(
            result,
            Err(CopyError::Cancelled { copied: 32768 })
        ));
        assert_eq!(fs::read(source).unwrap(), bytes);
        cleaned(&destination);
    }

    #[test]
    fn partial_write_disk_full_is_cleaned() {
        let (_dir, source, destination, bytes) = setup();
        let mut writes = 0;
        let result = copy_with_writer(
            &source,
            &destination,
            16 * 1024,
            &CancelToken::default(),
            |_| {},
            |file, data| {
                writes += 1;
                if writes == 2 {
                    file.write_all(&data[..123])?;
                    return Err(io::Error::from(io::ErrorKind::StorageFull));
                }
                file.write_all(data)
            },
        );
        assert!(matches!(result, Err(CopyError::Io(e)) if e.kind() == io::ErrorKind::StorageFull));
        assert_eq!(fs::read(source).unwrap(), bytes);
        cleaned(&destination);
    }

    #[test]
    fn existing_and_racing_destinations_are_never_overwritten() {
        for late in [false, true] {
            let (_dir, source, destination, bytes) = setup();
            if !late {
                fs::write(&destination, b"existing").unwrap();
            }
            let result = copy_file(
                &source,
                &destination,
                16 * 1024,
                &CancelToken::default(),
                |p| {
                    if late && p.copied == p.total {
                        fs::write(&destination, b"existing").unwrap();
                    }
                },
            );
            assert!(matches!(result, Err(CopyError::DestinationExists)));
            assert_eq!(fs::read(&destination).unwrap(), b"existing");
            assert_eq!(fs::read(source).unwrap(), bytes);
            assert_eq!(
                fs::read_dir(destination.parent().unwrap()).unwrap().count(),
                1
            );
        }
    }

    #[test]
    fn source_changes_are_rejected_without_publishing() {
        let (_dir, source, destination, _) = setup();
        let result = copy_file(
            &source,
            &destination,
            16 * 1024,
            &CancelToken::default(),
            |p| {
                if p.copied == 16 * 1024 {
                    OpenOptions::new()
                        .append(true)
                        .open(&source)
                        .unwrap()
                        .write_all(b"external change")
                        .unwrap();
                }
            },
        );
        assert!(matches!(result, Err(CopyError::SourceChanged)));
        cleaned(&destination);
    }

    #[test]
    fn cancellation_before_start_and_commit_boundary() {
        let (_dir, source, destination, _) = setup();
        let token = CancelToken::default();
        assert!(token.request_cancel());
        assert!(matches!(
            copy_file(&source, &destination, 1, &token, |_| {}),
            Err(CopyError::Cancelled { copied: 0 })
        ));
        cleaned(&destination);
        let token = CancelToken::default();
        token.begin_commit(0).unwrap();
        assert!(!token.request_cancel());
    }

    #[test]
    fn accepted_cancellation_after_last_write_still_prevents_publication() {
        let (_dir, source, destination, bytes) = setup();
        let token = CancelToken::default();
        let result = copy_file(&source, &destination, 16 * 1024, &token, |p| {
            if p.copied == p.total {
                assert!(token.request_cancel());
            }
        });
        assert!(matches!(
            result,
            Err(CopyError::Cancelled { copied: 262144 })
        ));
        assert_eq!(fs::read(source).unwrap(), bytes);
        cleaned(&destination);
    }

    #[test]
    fn cancellation_and_commit_have_exactly_one_winner() {
        use std::sync::{Arc, Barrier};
        for _ in 0..64 {
            let token = Arc::new(CancelToken::default());
            let barrier = Arc::new(Barrier::new(3));
            let a = Arc::clone(&token);
            let ab = Arc::clone(&barrier);
            let cancelling = std::thread::spawn(move || {
                ab.wait();
                a.request_cancel()
            });
            let b = Arc::clone(&token);
            let bb = Arc::clone(&barrier);
            let committing = std::thread::spawn(move || {
                bb.wait();
                b.begin_commit(0).is_ok()
            });
            barrier.wait();
            assert_ne!(cancelling.join().unwrap(), committing.join().unwrap());
        }
    }

    #[cfg(unix)]
    #[test]
    fn replacing_source_with_same_size_and_mtime_is_detected_by_identity() {
        let (dir, source, destination, bytes) = setup();
        let modified = source.metadata().unwrap().modified().unwrap();
        let backup = dir.path().join("original-inode");
        let result = copy_file(
            &source,
            &destination,
            16 * 1024,
            &CancelToken::default(),
            |p| {
                if p.copied == 16 * 1024 {
                    fs::rename(&source, &backup).unwrap();
                    fs::write(&source, vec![42; bytes.len()]).unwrap();
                    File::open(&source)
                        .unwrap()
                        .set_times(std::fs::FileTimes::new().set_modified(modified))
                        .unwrap();
                }
            },
        );
        assert!(matches!(result, Err(CopyError::SourceChanged)));
        assert_eq!(fs::read(backup).unwrap(), bytes);
        assert_eq!(fs::read(source).unwrap(), vec![42; bytes.len()]);
        cleaned(&destination);
    }

    #[test]
    fn empty_regular_file_and_invalid_chunk() {
        let (_dir, source, destination, _) = setup();
        fs::write(&source, b"").unwrap();
        assert!(copy_file(&source, &destination, 0, &CancelToken::default(), |_| {}).is_err());
        cleaned(&destination);
        assert_eq!(
            copy_file(&source, &destination, 4096, &CancelToken::default(), |_| {}).unwrap(),
            0
        );
        assert_eq!(fs::metadata(destination).unwrap().len(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_source_and_dangling_destination_are_rejected() {
        use std::os::unix::fs::symlink;
        let (dir, source, destination, bytes) = setup();
        let link = dir.path().join("source-link");
        symlink(&source, &link).unwrap();
        assert!(matches!(
            copy_file(&link, &destination, 4096, &CancelToken::default(), |_| {}),
            Err(CopyError::UnsupportedSource)
        ));
        cleaned(&destination);
        symlink(dir.path().join("missing"), &destination).unwrap();
        assert!(matches!(
            copy_file(&source, &destination, 4096, &CancelToken::default(), |_| {}),
            Err(CopyError::DestinationExists)
        ));
        assert!(destination
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read(source).unwrap(), bytes);
    }
}
