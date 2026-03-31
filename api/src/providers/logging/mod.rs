mod rolling_file;

use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use tracing_appender::non_blocking::{NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use rolling_file::SizeRotatingWriter;

/// Log line rendering. `Text` is human readable; `Json` emits one JSON
/// object per event for machine consumption.
#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    Text,
    Json,
}

/// Settings for the local (file-based) logging provider.
///
/// `level` is the default/global directive (e.g. "info") and `directives`
/// are additional target-scoped overrides (e.g. "api::auth=debug"), both
/// merged into a single `EnvFilter`.
#[derive(Debug, Clone)]
pub struct LocalLoggingSettings {
    pub level: String,
    pub directives: Vec<String>,
    pub format: LogFormat,
    pub file: PathBuf,
    pub mirror_stdout: bool,
    pub max_size_bytes: u64,
    pub max_files: usize,
}

/// Must be held for the lifetime of the process: dropping it stops the
/// non-blocking writer's background worker, which flushes buffered lines
/// before shutting down. Dropping it early can silently discard logs.
pub struct LoggingGuard {
    _file_guard: WorkerGuard,
}

/// Provider-selection factory. Only the local provider is implemented
/// today; future providers (e.g. a remote/cloud sink) can be added as new
/// variants without changing call sites that match on `LogFormat`/settings.
pub enum LoggingProvider {
    Local { _guard: LoggingGuard },
}

impl LoggingProvider {
    /// Initializes and selects the local file-based logging provider.
    pub fn init_local(settings: LocalLoggingSettings) -> Result<Self> {
        init_local_logging(settings).map(|guard| Self::Local { _guard: guard })
    }
}

/// Builds the combined `EnvFilter` from the global level plus any target
/// directives. Pure/no I/O so it can be unit tested without touching the
/// filesystem or tracing's global state.
fn build_filter(settings: &LocalLoggingSettings) -> Result<EnvFilter> {
    let directives = std::iter::once(settings.level.as_str())
        .chain(settings.directives.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(",");
    EnvFilter::try_new(directives).context("parse logging level directives")
}

/// Opens the rotating file writer and wraps it in a lossless (non-dropping)
/// non-blocking writer. The returned `WorkerGuard` must be kept alive for as
/// long as logging should keep flushing.
fn build_non_blocking_writer(
    settings: &LocalLoggingSettings,
) -> Result<(NonBlocking, WorkerGuard)> {
    let writer = SizeRotatingWriter::open(
        settings.file.clone(),
        settings.max_size_bytes,
        settings.max_files,
    )?;
    Ok(NonBlockingBuilder::default().lossy(false).finish(writer))
}

/// Converts a boxed subscriber-init error (which does not itself implement
/// `std::error::Error` in a way `anyhow::Context` can pick up generically)
/// into a plain message so `.context(..)` can attach a description.
fn init_error(error: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(error.to_string())
}

pub fn init_local_logging(settings: LocalLoggingSettings) -> Result<LoggingGuard> {
    ensure!(
        settings.max_size_bytes > 0,
        "log rotation max size must be positive"
    );
    ensure!(
        settings.max_files > 0,
        "log rotation max files must be positive"
    );

    let filter = build_filter(&settings)?;
    let (file_writer, file_guard) = build_non_blocking_writer(&settings)?;

    match (settings.format, settings.mirror_stdout) {
        (LogFormat::Text, true) => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(file_writer.and(std::io::stdout))
            .try_init()
            .map_err(init_error)
            .context("initialize local text logging")?,
        (LogFormat::Text, false) => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(file_writer)
            .try_init()
            .map_err(init_error)
            .context("initialize local text logging")?,
        (LogFormat::Json, true) => tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(file_writer.and(std::io::stdout))
            .try_init()
            .map_err(init_error)
            .context("initialize local JSON logging")?,
        (LogFormat::Json, false) => tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(file_writer)
            .try_init()
            .map_err(init_error)
            .context("initialize local JSON logging")?,
    }

    Ok(LoggingGuard {
        _file_guard: file_guard,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
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
                "social-link-logging-mod-test-{label}-{}-{nanos}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).expect("create temp test dir");
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn base_settings(dir: &TempDir, format: LogFormat) -> LocalLoggingSettings {
        LocalLoggingSettings {
            level: "info".to_string(),
            directives: vec![],
            format,
            file: dir.path("app.log"),
            mirror_stdout: false,
            max_size_bytes: 1_000_000,
            max_files: 2,
        }
    }

    // --- build_filter: pure, no I/O, no tracing global state -----------

    #[test]
    fn build_filter_merges_level_and_target_directives() {
        let dir = TempDir::new("filter-ok");
        let mut settings = base_settings(&dir, LogFormat::Text);
        settings.directives = vec!["some_module::inner=warn".to_string()];

        let filter = build_filter(&settings).expect("valid directives must parse");
        let rendered = filter.to_string();
        assert!(rendered.contains("info"), "rendered filter: {rendered}");
        assert!(
            rendered.contains("some_module::inner=warn"),
            "rendered filter: {rendered}"
        );
    }

    #[test]
    fn build_filter_rejects_invalid_directive_syntax() {
        let dir = TempDir::new("filter-invalid");
        let mut settings = base_settings(&dir, LogFormat::Text);
        settings.directives = vec!["my_target=verbose".to_string()];

        let error = build_filter(&settings).expect_err("invalid directive must fail");
        assert!(format!("{error}").contains("parse logging level directives"));
    }

    // --- settings validation: fails before any filesystem/global side
    // effects, so safe to call repeatedly ---------------------------------

    #[test]
    fn init_local_logging_rejects_zero_max_size() {
        let dir = TempDir::new("validate-max-size");
        let mut invalid = base_settings(&dir, LogFormat::Text);
        invalid.max_size_bytes = 0;

        let result = init_local_logging(invalid);
        let message = match result {
            Ok(_) => panic!("zero max size must be rejected"),
            Err(error) => error.to_string(),
        };
        assert!(message.contains("max size must be positive"));
        assert!(
            !dir.path("app.log").exists(),
            "validation must fail before any file is created"
        );
    }

    #[test]
    fn init_local_logging_rejects_zero_max_files() {
        let dir = TempDir::new("validate-max-files");
        let mut invalid = base_settings(&dir, LogFormat::Text);
        invalid.max_files = 0;

        let result = init_local_logging(invalid);
        let message = match result {
            Ok(_) => panic!("zero max files must be rejected"),
            Err(error) => error.to_string(),
        };
        assert!(message.contains("max files must be positive"));
        assert!(
            !dir.path("app.log").exists(),
            "validation must fail before any file is created"
        );
    }

    // --- format/level pipeline: built manually with `.finish()` and run
    // through `tracing::subscriber::with_default`, which scopes the
    // dispatcher to the current thread/closure only. This lets many tests
    // exercise real tracing emission without ever installing a repeated (or
    // conflicting) process-wide global default via `try_init`. ------------

    #[test]
    fn text_pipeline_respects_level_filter_and_writes_to_file() {
        let dir = TempDir::new("text-pipeline");
        let settings = base_settings(&dir, LogFormat::Text);
        let path = settings.file.clone();

        let filter = build_filter(&settings).expect("filter");
        let (writer, guard) = build_non_blocking_writer(&settings).expect("writer");
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(writer)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("hidden-by-level-filter");
            tracing::info!("visible-info-line");
        });
        drop(guard); // flush + join the non-blocking worker before reading

        let contents = std::fs::read_to_string(&path).expect("read log file");
        assert!(contents.contains("visible-info-line"));
        assert!(!contents.contains("hidden-by-level-filter"));
    }

    #[test]
    fn json_pipeline_emits_valid_json_lines() {
        let dir = TempDir::new("json-pipeline");
        let settings = base_settings(&dir, LogFormat::Json);
        let path = settings.file.clone();

        let filter = build_filter(&settings).expect("filter");
        let (writer, guard) = build_non_blocking_writer(&settings).expect("writer");
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(writer)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(order_id = 42, "json-line-marker");
        });
        drop(guard);

        let contents = std::fs::read_to_string(&path).expect("read log file");
        let line = contents
            .lines()
            .find(|line| line.contains("json-line-marker"))
            .expect("expected json line present");
        let parsed: serde_json::Value =
            serde_json::from_str(line).expect("each json log line must be valid json");
        assert_eq!(parsed["fields"]["order_id"], 42);
    }

    // --- non-blocking writer: lossless + safe under concurrent producers,
    // exercised directly against the channel without any tracing subscriber
    // (global or scoped) involved at all. ---------------------------------

    #[test]
    fn non_blocking_writer_is_lossless_under_concurrent_load() {
        let dir = TempDir::new("non-blocking-concurrency");
        let settings = base_settings(&dir, LogFormat::Text);
        let path = settings.file.clone();
        let (writer, guard) = build_non_blocking_writer(&settings).expect("writer");

        const THREADS: usize = 8;
        const LINES_PER_THREAD: usize = 500;
        let handles: Vec<_> = (0..THREADS)
            .map(|thread_idx| {
                let mut writer = writer.clone();
                std::thread::spawn(move || {
                    for line_idx in 0..LINES_PER_THREAD {
                        writer
                            .write_all(format!("t{thread_idx}-l{line_idx}\n").as_bytes())
                            .expect("lossless (non-lossy) writer must never drop a write");
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("writer thread panicked");
        }
        // Dropping the guard flushes and joins the background worker,
        // guaranteeing every message sent above has actually reached disk.
        drop(guard);

        let contents = std::fs::read_to_string(&path).expect("read log file");
        let line_count = contents.lines().count();
        assert_eq!(
            line_count,
            THREADS * LINES_PER_THREAD,
            "lossless non-blocking writer must not drop or duplicate any message"
        );
        for thread_idx in 0..THREADS {
            for line_idx in 0..LINES_PER_THREAD {
                let expected = format!("t{thread_idx}-l{line_idx}");
                assert!(
                    contents.contains(&expected),
                    "missing expected concurrent line: {expected}"
                );
            }
        }
    }

    // --- public entry point, end to end -----------------------------------
    //
    // This is the ONLY test in this module allowed to call
    // `init_local_logging`/`LoggingProvider::init_local` in a way that
    // reaches `try_init`, which installs a process-wide global default
    // dispatcher. `try_init` (unlike `init`) fails gracefully instead of
    // panicking if a global default is already set, but repeatedly relying
    // on that across tests would make coverage order-dependent, so every
    // other test above intentionally exercises the same logic (filter
    // building, writer construction, format rendering, concurrency) through
    // scoped (`with_default`) or global-state-free paths instead.
    #[test]
    fn init_local_logging_end_to_end_installs_subscriber_once() {
        let dir = TempDir::new("end-to-end-global-init");
        let settings = base_settings(&dir, LogFormat::Text);
        let path = settings.file.clone();

        let guard = init_local_logging(settings).expect("global init must succeed exactly once");
        tracing::info!("end-to-end-global-init-marker-4f9a21");
        drop(guard);

        let contents = std::fs::read_to_string(&path).expect("read log file");
        assert!(contents.contains("end-to-end-global-init-marker-4f9a21"));
    }
}
