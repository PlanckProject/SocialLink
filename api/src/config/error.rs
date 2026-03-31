use std::path::{Path, PathBuf};

/// Errors that can occur while loading, interpolating, parsing, or
/// validating the application configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to determine current working directory: {0}")]
    WorkingDir(#[source] std::io::Error),

    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("missing required environment variable `{0}` referenced in configuration")]
    MissingEnvVar(String),

    #[error("invalid ${{...}} interpolation syntax in {path}: {reason}")]
    Interpolation { path: PathBuf, reason: String },

    #[error("failed to parse YAML in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("invalid configuration in {path}: {reason}")]
    Validation { path: PathBuf, reason: String },
}

impl ConfigError {
    pub(super) fn validation(path: &Path, reason: impl Into<String>) -> Self {
        ConfigError::Validation {
            path: path.to_path_buf(),
            reason: reason.into(),
        }
    }
}
