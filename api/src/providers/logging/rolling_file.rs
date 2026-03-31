use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// A size-based rotating file writer: once appending the next write would
/// exceed `max_size` bytes, the active file is rotated out (`path`,
/// `path.1`, `path.2`, ... up to `path.<max_files>`) before the write is
/// performed, and exactly `max_files` rotated generations are retained.
#[derive(Debug)]
pub struct SizeRotatingWriter {
    path: PathBuf,
    file: Option<File>,
    current_size: u64,
    max_size: u64,
    max_files: usize,
}

impl SizeRotatingWriter {
    /// Opens (creating the parent directory and the file if needed) the
    /// active log file in append mode. If the file already exists and is
    /// already at or over `max_size` (e.g. left over from a previous run),
    /// it is rotated immediately so subsequent writes start from a fresh
    /// file.
    pub fn open(path: PathBuf, max_size: u64, max_files: usize) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create log directory {}", parent.display()))?;
        }
        let file = open_append(&path)?;
        let current_size = file
            .metadata()
            .with_context(|| format!("read log metadata {}", path.display()))?
            .len();
        let mut writer = Self {
            path,
            file: Some(file),
            current_size,
            max_size,
            max_files,
        };
        if writer.current_size >= writer.max_size {
            writer.rotate()?;
        }
        Ok(writer)
    }

    /// Shifts rotated generations up by one slot and moves the active file
    /// into slot 1, then reopens a fresh, empty active file.
    ///
    /// Windows does not allow `rename` to replace an existing destination.
    /// The oldest generation is therefore staged one slot beyond retention
    /// and removed only after the active file has been rotated successfully.
    fn rotate(&mut self) -> Result<()> {
        // Close the active file handle before touching the filesystem: on
        // Windows a rename fails while the source (or destination) file is
        // still open, so the handle must be dropped first.
        if let Some(mut file) = self.file.take() {
            file.flush()
                .with_context(|| format!("flush log file {}", self.path.display()))?;
        }

        let oldest = generation_path(&self.path, self.max_files);
        let staged_oldest = generation_path(&self.path, self.max_files + 1);
        if staged_oldest.exists() {
            if oldest.exists() {
                remove_if_exists(&staged_oldest)?;
            } else {
                fs::rename(&staged_oldest, &oldest).with_context(|| {
                    format!(
                        "recover staged log {} to {}",
                        staged_oldest.display(),
                        oldest.display()
                    )
                })?;
            }
        }
        if oldest.exists() {
            fs::rename(&oldest, &staged_oldest).with_context(|| {
                format!(
                    "stage oldest log {} as {}",
                    oldest.display(),
                    staged_oldest.display()
                )
            })?;
        }

        for generation in (1..self.max_files).rev() {
            let source = generation_path(&self.path, generation);
            if source.exists() {
                let destination = generation_path(&self.path, generation + 1);
                fs::rename(&source, &destination).with_context(|| {
                    format!(
                        "rotate log {} to {}",
                        source.display(),
                        destination.display()
                    )
                })?;
            }
        }
        if self.path.exists() {
            let first = generation_path(&self.path, 1);
            fs::rename(&self.path, &first).with_context(|| {
                format!(
                    "rotate active log {} to {}",
                    self.path.display(),
                    first.display()
                )
            })?;
        }

        self.file = Some(open_truncate(&self.path)?);
        self.current_size = 0;
        remove_if_exists(&staged_oldest)?;
        Ok(())
    }

    fn file(&mut self) -> io::Result<&mut File> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("log file is not open"))
    }
}

impl Write for SizeRotatingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        // Rotate *before* writing once this write would push the file past
        // `max_size` (writing exactly up to `max_size` is fine and does not
        // rotate). Skipped when the active file is currently empty so a
        // single write larger than `max_size` still lands somewhere instead
        // of spinning on an empty file forever; the next write will then
        // rotate it out.
        if self.current_size > 0
            && self.current_size.saturating_add(buffer.len() as u64) > self.max_size
        {
            self.rotate().map_err(io::Error::other)?;
        }
        let written = self.file()?.write(buffer)?;
        self.current_size = self.current_size.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file()?.flush()
    }
}

fn open_append(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open log file {}", path.display()))
}

fn open_truncate(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("create rotated log file {}", path.display()))
}

fn generation_path(path: &Path, generation: usize) -> PathBuf {
    PathBuf::from(format!("{}.{}", path.display(), generation))
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove old log {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    static UNIQUE: AtomicU64 = AtomicU64::new(0);

    /// A unique scratch directory per test, removed on drop (even if the
    /// test panics on an assertion) so runs don't leak files.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let unique = UNIQUE.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "social-link-log-test-{label}-{}-{nanos}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).expect("create temp test dir");
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn read_to_string(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    }

    #[test]
    fn rotates_by_size_and_bounds_generations() {
        let dir = TempDir::new("rotate-bounds");
        let path = dir.path("app.log");
        let mut writer = SizeRotatingWriter::open(path.clone(), 8, 2).expect("writer");
        writer.write_all(b"12345678").expect("first write");
        writer.write_all(b"abcdefgh").expect("second write");
        writer.write_all(b"ABCDEFGH").expect("third write");
        writer.flush().expect("flush");

        assert_eq!(read_to_string(&path), "ABCDEFGH");
        assert_eq!(read_to_string(&generation_path(&path, 1)), "abcdefgh");
        assert_eq!(read_to_string(&generation_path(&path, 2)), "12345678");
        assert!(!generation_path(&path, 3).exists());
    }

    #[test]
    fn write_exactly_at_max_size_does_not_rotate() {
        let dir = TempDir::new("boundary-exact");
        let path = dir.path("app.log");
        let mut writer = SizeRotatingWriter::open(path.clone(), 8, 3).expect("writer");

        // Writing exactly up to max_size must not rotate.
        writer.write_all(b"12345678").expect("write exactly max");
        writer.flush().expect("flush");
        assert!(!generation_path(&path, 1).exists());
        assert_eq!(read_to_string(&path), "12345678");

        // Any further write now would exceed max_size, so it must rotate
        // before landing in a fresh file.
        writer.write_all(b"x").expect("write triggers rotation");
        writer.flush().expect("flush");
        assert_eq!(read_to_string(&generation_path(&path, 1)), "12345678");
        assert_eq!(read_to_string(&path), "x");
    }

    #[test]
    fn oversized_single_write_to_empty_file_is_not_split() {
        let dir = TempDir::new("oversized-single-write");
        let path = dir.path("app.log");
        let mut writer = SizeRotatingWriter::open(path.clone(), 4, 2).expect("writer");

        // Larger than max_size but the active file is currently empty:
        // written in full rather than being unwritable.
        writer.write_all(b"0123456789").expect("oversized write");
        writer.flush().expect("flush");
        assert_eq!(read_to_string(&path), "0123456789");
        assert!(!generation_path(&path, 1).exists());

        // The next write must rotate the now-oversized active file out.
        writer.write_all(b"next").expect("next write rotates");
        writer.flush().expect("flush");
        assert_eq!(read_to_string(&generation_path(&path, 1)), "0123456789");
        assert_eq!(read_to_string(&path), "next");
    }

    #[test]
    fn retention_keeps_exactly_max_files_and_evicts_oldest() {
        let dir = TempDir::new("retention");
        let path = dir.path("app.log");
        let mut writer = SizeRotatingWriter::open(path.clone(), 4, 3).expect("writer");

        // Six 4-byte writes against a 4-byte max triggers a rotation before
        // every write after the first, for 5 rotations total.
        for chunk in ["aaaa", "bbbb", "cccc", "dddd", "eeee", "ffff"] {
            writer.write_all(chunk.as_bytes()).expect("write");
        }
        writer.flush().expect("flush");

        assert_eq!(read_to_string(&path), "ffff");
        assert_eq!(read_to_string(&generation_path(&path, 1)), "eeee");
        assert_eq!(read_to_string(&generation_path(&path, 2)), "dddd");
        assert_eq!(read_to_string(&generation_path(&path, 3)), "cccc");
        assert!(
            !generation_path(&path, 4).exists(),
            "only max_files generations may be retained"
        );
    }

    #[test]
    fn existing_oversized_file_is_rotated_on_open() {
        let dir = TempDir::new("existing-oversized");
        let path = dir.path("app.log");
        fs::write(&path, b"already-too-big-for-the-configured-limit").expect("seed oversized file");

        let mut writer = SizeRotatingWriter::open(path.clone(), 8, 2).expect("writer");

        // Rotation must have happened immediately as part of opening.
        assert_eq!(
            read_to_string(&generation_path(&path, 1)),
            "already-too-big-for-the-configured-limit"
        );
        assert_eq!(read_to_string(&path), "");

        writer.write_all(b"fresh").expect("write after rotation");
        writer.flush().expect("flush");
        assert_eq!(read_to_string(&path), "fresh");
    }

    #[test]
    fn reopen_appends_instead_of_truncating() {
        let dir = TempDir::new("reopen-append");
        let path = dir.path("app.log");
        {
            let mut writer = SizeRotatingWriter::open(path.clone(), 100, 2).expect("writer");
            writer.write_all(b"first-session-").expect("write");
            writer.flush().expect("flush");
        } // writer (and its file handle) dropped here, simulating a restart

        {
            let mut writer = SizeRotatingWriter::open(path.clone(), 100, 2).expect("reopen writer");
            writer.write_all(b"second-session").expect("append write");
            writer.flush().expect("flush");
        }

        assert_eq!(read_to_string(&path), "first-session-second-session");
    }

    #[test]
    fn reopen_accounts_for_existing_size_toward_rotation() {
        let dir = TempDir::new("reopen-size-accounting");
        let path = dir.path("app.log");
        {
            let mut writer = SizeRotatingWriter::open(path.clone(), 10, 2).expect("writer");
            writer.write_all(b"12345678").expect("write 8 bytes"); // under max_size, no rotation
        }

        // Reopening must pick current_size back up from the existing
        // file's length (8 bytes), so a write that would push past
        // max_size (10) rotates rather than growing the file unbounded.
        let mut writer = SizeRotatingWriter::open(path.clone(), 10, 2).expect("reopen writer");
        writer.write_all(b"abc").expect("write triggers rotation");
        writer.flush().expect("flush");

        assert_eq!(read_to_string(&generation_path(&path, 1)), "12345678");
        assert_eq!(read_to_string(&path), "abc");
    }

    #[test]
    fn creates_missing_parent_directory() {
        let dir = TempDir::new("create-parent-dir");
        let path = dir.path("nested").join("deeper").join("app.log");
        assert!(!path.parent().unwrap().exists());

        let mut writer = SizeRotatingWriter::open(path.clone(), 100, 2).expect("writer");
        writer.write_all(b"hello").expect("write");
        writer.flush().expect("flush");

        assert_eq!(read_to_string(&path), "hello");
    }

    #[test]
    fn fails_clearly_when_path_is_unwritable() {
        let dir = TempDir::new("unwritable-path");
        // Create a regular file, then try to open a log path that would
        // require treating that file as a directory component. This can
        // never succeed and must fail with a clear, contextualized error
        // instead of panicking.
        let blocking_file = dir.path("not-a-directory");
        fs::write(&blocking_file, b"i am a file, not a directory").expect("seed blocking file");
        let path = blocking_file.join("app.log");

        let error = SizeRotatingWriter::open(path, 100, 2).expect_err("must fail clearly");
        let message = format!("{error}");
        assert!(
            message.contains("create log directory"),
            "error should explain the failing step, got: {message}"
        );
    }
}
