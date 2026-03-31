//! Application configuration.
//!
//! Configuration is split across two YAML files selected by the
//! `SOCIAL_LINK_ENV` environment variable (default `local`):
//!
//! - `<application root>/config/<env>/server.yaml` — server, logging, auth,
//!   database, timeseries, storage, and cache settings.
//! - `<application root>/config/<env>/social-link.yaml` — application mode,
//!   admin bootstrap account, uploads, and themes settings.
//!
//! `${ENV_VAR}` (and `${ENV_VAR:-default}`) placeholders in YAML string
//! values are interpolated from the process environment after parsing, so
//! secret characters cannot alter the YAML structure. A bare `${ENV_VAR}`
//! with no default is a hard, clearly reported error when unset; the
//! `${ENV_VAR:-default}` form falls back to `default` when the variable is
//! unset or empty. Both files use strict
//! `deny_unknown_fields` schemas so typos and stray keys are caught eagerly,
//! and values are validated (nonzero ports/TTLs/limits, `/`-prefixed route
//! prefixes, etc.) before [`Config`] is constructed.

mod error;
mod interpolate;
mod server;
mod social_link;

#[cfg(test)]
mod tests;

use std::path::Path;

use serde::Deserialize;

pub use error::ConfigError;
pub use server::{
    AuthConfig, CacheConfig, CacheProviderConfig, DatabaseConfig, DbProvider, LogFormat,
    LogProvider, LoggingConfig, ServerSection, StorageConfig, StorageProvider, TimeseriesConfig,
    TlsConfig,
};
pub use social_link::{AdminConfig, ApplicationConfig, ThemesConfig, UploadsConfig};

use server::ServerFile;
use social_link::SocialLinkFile;

/// Environment variable used to select the config environment directory
/// (`config/<env>/...`). Defaults to [`DEFAULT_ENV`] when unset.
pub const ENV_VAR_NAME: &str = "SOCIAL_LINK_ENV";

/// Default config environment used when `SOCIAL_LINK_ENV` is unset.
pub const DEFAULT_ENV: &str = "local";

/// Application mode. `single` runs one owner; `multi` enables signups and
/// per-username public pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    Single,
    Multi,
}

impl AppMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppMode::Single => "single",
            AppMode::Multi => "multi",
        }
    }

    pub fn is_multi(&self) -> bool {
        matches!(self, AppMode::Multi)
    }
}

/// Fully validated, strongly typed application configuration assembled from
/// `config/<env>/server.yaml` and `config/<env>/social-link.yaml`.
#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerSection,
    pub logging: LoggingConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub timeseries: TimeseriesConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
    /// UI HTTPS / HTTP-2 termination settings. Parsed and validated here as
    /// canonical, schema-checked documentation, but the API never terminates
    /// TLS — the UI (`ui/serve.mjs`) is the real consumer via the `UI_TLS_*`
    /// env vars. Nothing in the non-test binary reads this field (only the
    /// `#[cfg(test)]` config tests do), so `allow(dead_code)` keeps this
    /// intentional write-only field from tripping the dead-code lint.
    #[allow(dead_code)]
    pub tls: TlsConfig,

    pub application: ApplicationConfig,
    pub admin: AdminConfig,
    pub uploads: UploadsConfig,
    pub themes: ThemesConfig,
}

impl Config {
    /// Loads configuration for the environment named by `SOCIAL_LINK_ENV`
    /// (default `local`), resolving `config/<env>/{server,social-link}.yaml`
    /// relative to the current working directory. In the container image the
    /// process runs with `/app` as its working directory, so `/app/config`
    /// is where the environment-specific files are expected to live.
    pub fn load() -> Result<Self, ConfigError> {
        let root = std::env::current_dir().map_err(ConfigError::WorkingDir)?;
        let env = std::env::var(ENV_VAR_NAME).unwrap_or_else(|_| DEFAULT_ENV.to_string());
        Self::load_from_root(&root, &env)
    }

    /// Loads configuration from an explicit application root and environment
    /// name. This is the injection point used by tests (and any other
    /// caller that wants to avoid depending on the real working directory)
    /// so they can point at a temporary directory instead.
    pub fn load_from_root(root: &Path, env: &str) -> Result<Self, ConfigError> {
        let config_dir = root.join("config").join(env);

        let server_path = config_dir.join("server.yaml");
        let social_link_path = config_dir.join("social-link.yaml");

        let server_file: ServerFile = read_yaml(&server_path)?;
        let social_link_file: SocialLinkFile = read_yaml(&social_link_path)?;

        let validated_server = server_file.into_validated(&server_path)?;
        let validated_social = social_link_file.into_validated(&social_link_path)?;

        Ok(Config {
            server: validated_server.server,
            logging: validated_server.logging,
            auth: validated_server.auth,
            database: validated_server.database,
            timeseries: validated_server.timeseries,
            storage: validated_server.storage,
            cache: validated_server.cache,
            tls: validated_server.tls,

            application: validated_social.application,
            admin: validated_social.admin,
            uploads: validated_social.uploads,
            themes: validated_social.themes,
        })
    }
}

fn read_yaml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ConfigError> {
    let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut value = serde_yaml::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    interpolate::interpolate(&mut value, path)?;
    serde_yaml::from_value(value).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}
