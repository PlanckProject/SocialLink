use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::error::ConfigError;

// ---------------------------------------------------------------------
// Public, validated types
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSection {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

/// Supported logging providers. Unsupported values fail to deserialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogProvider {
    #[default]
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingConfig {
    pub provider: LogProvider,
    pub level: String,
    pub directives: Vec<String>,
    pub format: LogFormat,
    pub file: PathBuf,
    pub mirror_stdout: bool,
    pub rotation_max_size_mb: u64,
    pub rotation_max_files: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    pub cookie_secure: bool,
    pub ip_hash_salt: String,
}

/// Supported database/timeseries providers. Unsupported values fail to
/// deserialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DbProvider {
    #[default]
    Mongo,
}

/// Connection details for the primary database. `connection_string`, when
/// present, is expected to take precedence over the discrete
/// host/port/db/username/password fields, but resolving that precedence is
/// the responsibility of the database integration layer, not this module.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DatabaseConfig {
    pub provider: DbProvider,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub db: Option<String>,
    pub certificate: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub connection_string: Option<String>,
}

/// Connection details for the timeseries/events store. Same shape as
/// [`DatabaseConfig`] plus the target `collection` name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeseriesConfig {
    pub provider: DbProvider,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub db: Option<String>,
    pub certificate: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub connection_string: Option<String>,
    pub collection: String,
}

/// Supported storage providers. Unsupported values fail to deserialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageProvider {
    #[default]
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageConfig {
    pub provider: StorageProvider,
    pub base_path: PathBuf,
    pub route_prefix: String,
}

/// Cache provider selector. `None` (the default) disables caching entirely.
/// Unsupported values fail to deserialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CacheProvider {
    #[default]
    None,
    InProcess,
    Redis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InProcessCacheConfig {
    pub max_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisCacheConfig {
    pub connection_string: String,
    pub key_prefix: String,
}

/// Provider-specific cache configuration. Only the variant matching
/// [`CacheConfig::provider`] is populated; each variant's shape is strictly
/// deserialized (e.g. `none` accepts only `{}`) once the provider is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheProviderConfig {
    None,
    InProcess(InProcessCacheConfig),
    Redis(RedisCacheConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheConfig {
    pub ttl_seconds: u64,
    pub provider: CacheProviderConfig,
}

/// TLS / HTTPS termination for the public UI (the Nuxt SSR node server).
///
/// The Rust API itself is only reached internally (it is proxied by the UI) and
/// does not terminate TLS, but the certificate mount path and the HTTP/2 / 3
/// switches are recorded here as the canonical, validated configuration
/// surface requested for `server.yaml`. The UI container consumes the matching
/// `UI_TLS_*` / `UI_HTTP*` environment variables (see `docker-compose.yml`),
/// which these values mirror via `${VAR:-default}` interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub http2: bool,
    pub http3: bool,
}

// ---------------------------------------------------------------------
// Raw (wire-format) structures mirroring server.yaml exactly. Every
// section uses `deny_unknown_fields` so typos or stray keys fail loudly.
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ServerFile {
    server: ServerSectionRaw,
    logging: LoggingSectionRaw,
    auth: AuthSectionRaw,
    database: DatabaseSectionRaw,
    timeseries: TimeseriesSectionRaw,
    storage: StorageSectionRaw,
    #[serde(default)]
    cache: CacheSectionRaw,
    #[serde(default)]
    tls: TlsSectionRaw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerSectionRaw {
    host: String,
    port: u16,
    #[serde(default)]
    cors_origins: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoggingSectionRaw {
    #[serde(default)]
    provider: LogProvider,
    level: String,
    #[serde(default)]
    directives: Vec<String>,
    #[serde(default = "default_log_format")]
    format: LogFormat,
    config: LoggingConfigRaw,
}

fn default_log_format() -> LogFormat {
    LogFormat::Text
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoggingConfigRaw {
    file: String,
    #[serde(default = "default_true")]
    mirror_stdout: bool,
    rotation: RotationConfigRaw,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RotationConfigRaw {
    max_size_mb: u64,
    max_files: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthSectionRaw {
    jwt_secret: String,
    #[serde(deserialize_with = "deserialize_ttl_hours")]
    jwt_ttl_hours: i64,
    #[serde(default)]
    cookie_secure: bool,
    ip_hash_salt: String,
}

/// Deserializes `auth.jwt_ttl_hours` from either a YAML integer or a string
/// containing an integer. Env-var interpolation (`${JWT_TTL_HOURS:-12}`)
/// always yields a string, so accepting both keeps the field configurable via
/// an environment variable while still allowing a plain literal number.
fn deserialize_ttl_hours<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct TtlHours;

    impl serde::de::Visitor<'_> for TtlHours {
        type Value = i64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer number of hours or a string containing one")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            i64::try_from(value).map_err(|_| E::custom("jwt_ttl_hours is out of range"))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            value
                .trim()
                .parse::<i64>()
                .map_err(|_| E::custom(format!("jwt_ttl_hours must be an integer, got `{value}`")))
        }
    }

    deserializer.deserialize_any(TtlHours)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatabaseSectionRaw {
    #[serde(default)]
    provider: DbProvider,
    config: ConnectionConfigRaw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeseriesSectionRaw {
    #[serde(default)]
    provider: DbProvider,
    config: TimeseriesConnectionConfigRaw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionConfigRaw {
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    db: Option<String>,
    #[serde(default)]
    certificate: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    connection_string: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeseriesConnectionConfigRaw {
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    db: Option<String>,
    #[serde(default)]
    certificate: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    connection_string: Option<String>,
    collection: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageSectionRaw {
    #[serde(default)]
    provider: StorageProvider,
    config: StorageConfigRaw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageConfigRaw {
    base_path: String,
    route_prefix: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CacheSectionRaw {
    #[serde(default)]
    provider: CacheProvider,
    #[serde(default = "default_cache_ttl")]
    ttl_seconds: u64,
    #[serde(default)]
    config: serde_yaml::Value,
}

fn default_cache_ttl() -> u64 {
    300
}

/// Optional `tls` section. Absent by default; when present every field is
/// optional so the whole block can be driven by `${UI_TLS_*}` interpolation.
/// Booleans accept real YAML bools *or* strings ("true"/"false"/"1"/"0"/…),
/// because env-var interpolation always yields strings.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct TlsSectionRaw {
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    enabled: Option<bool>,
    #[serde(default)]
    cert_path: Option<String>,
    #[serde(default)]
    key_path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    http2: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    http3: Option<bool>,
}

/// Deserializes an optional boolean from either a real YAML bool or a string.
/// Env-var interpolation (`${UI_TLS_ENABLED:-false}`) always yields a string,
/// so accepting both keeps the `tls` switches configurable via the environment
/// while still allowing plain literals. An empty string (e.g. an unset
/// `${VAR:-}`) is treated as absent so the field's default applies.
fn deserialize_flexible_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct FlexibleBool;

    impl serde::de::Visitor<'_> for FlexibleBool {
        type Value = Option<bool>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or a string containing one")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value.trim().to_ascii_lowercase().as_str() {
                "" => Ok(None),
                "true" | "1" | "yes" | "on" => Ok(Some(true)),
                "false" | "0" | "no" | "off" => Ok(Some(false)),
                other => Err(E::custom(format!("expected a boolean, got `{other}`"))),
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(FlexibleBool)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheNoneConfigRaw {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InProcessCacheConfigRaw {
    #[serde(default = "default_max_entries")]
    max_entries: usize,
}

fn default_max_entries() -> usize {
    10_000
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RedisCacheConfigRaw {
    connection_string: String,
    key_prefix: String,
}

/// Provider-specific `cache.config` is validated as an empty mapping when
/// omitted, so `none` still strictly rejects any unexpected keys instead of
/// silently accepting them.
fn normalize_cache_config(value: serde_yaml::Value) -> serde_yaml::Value {
    if value.is_null() {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    } else {
        value
    }
}

// ---------------------------------------------------------------------
// Validation: raw -> public typed sections
// ---------------------------------------------------------------------

pub(super) struct ValidatedServer {
    pub(super) server: ServerSection,
    pub(super) logging: LoggingConfig,
    pub(super) auth: AuthConfig,
    pub(super) database: DatabaseConfig,
    pub(super) timeseries: TimeseriesConfig,
    pub(super) storage: StorageConfig,
    pub(super) cache: CacheConfig,
    pub(super) tls: TlsConfig,
}

fn validate_port(port: Option<u16>, path: &Path, field: &str) -> Result<(), ConfigError> {
    if port == Some(0) {
        return Err(ConfigError::validation(
            path,
            format!("{field} must be nonzero"),
        ));
    }
    Ok(())
}

impl ServerFile {
    pub(super) fn into_validated(self, path: &Path) -> Result<ValidatedServer, ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::validation(path, "server.port must be nonzero"));
        }
        let server = ServerSection {
            host: self.server.host,
            port: self.server.port,
            cors_origins: self.server.cors_origins,
        };

        if self.logging.level.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "logging.level must not be empty",
            ));
        }
        if self.logging.config.file.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "logging.config.file must not be empty",
            ));
        }
        if self.logging.config.rotation.max_size_mb == 0 {
            return Err(ConfigError::validation(
                path,
                "logging.config.rotation.max_size_mb must be nonzero",
            ));
        }
        if self.logging.config.rotation.max_files == 0 {
            return Err(ConfigError::validation(
                path,
                "logging.config.rotation.max_files must be nonzero",
            ));
        }
        let logging = LoggingConfig {
            provider: self.logging.provider,
            level: self.logging.level,
            directives: self.logging.directives,
            format: self.logging.format,
            file: PathBuf::from(self.logging.config.file),
            mirror_stdout: self.logging.config.mirror_stdout,
            rotation_max_size_mb: self.logging.config.rotation.max_size_mb,
            rotation_max_files: self.logging.config.rotation.max_files,
        };

        if self.auth.jwt_secret.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "auth.jwt_secret must not be empty",
            ));
        }
        if self.auth.jwt_ttl_hours <= 0 {
            return Err(ConfigError::validation(
                path,
                "auth.jwt_ttl_hours must be a positive, nonzero number",
            ));
        }
        if self.auth.ip_hash_salt.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "auth.ip_hash_salt must not be empty",
            ));
        }
        let auth = AuthConfig {
            jwt_secret: self.auth.jwt_secret,
            jwt_ttl_hours: self.auth.jwt_ttl_hours,
            cookie_secure: self.auth.cookie_secure,
            ip_hash_salt: self.auth.ip_hash_salt,
        };

        validate_port(self.database.config.port, path, "database.config.port")?;
        let database = DatabaseConfig {
            provider: self.database.provider,
            host: self.database.config.host,
            port: self.database.config.port,
            db: self.database.config.db,
            certificate: self.database.config.certificate,
            username: self.database.config.username,
            password: self.database.config.password,
            connection_string: self.database.config.connection_string,
        };

        validate_port(self.timeseries.config.port, path, "timeseries.config.port")?;
        if self.timeseries.config.collection.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "timeseries.config.collection must not be empty",
            ));
        }
        let timeseries = TimeseriesConfig {
            provider: self.timeseries.provider,
            host: self.timeseries.config.host,
            port: self.timeseries.config.port,
            db: self.timeseries.config.db,
            certificate: self.timeseries.config.certificate,
            username: self.timeseries.config.username,
            password: self.timeseries.config.password,
            connection_string: self.timeseries.config.connection_string,
            collection: self.timeseries.config.collection,
        };

        let route_prefix = self
            .storage
            .config
            .route_prefix
            .trim_end_matches('/')
            .to_string();
        if route_prefix != "/uploads" {
            return Err(ConfigError::validation(
                path,
                "storage.config.route_prefix must resolve to /uploads because the UI proxies that application-owned route",
            ));
        }
        if self.storage.config.base_path.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "storage.config.base_path must not be empty",
            ));
        }
        let storage = StorageConfig {
            provider: self.storage.provider,
            base_path: PathBuf::from(self.storage.config.base_path),
            route_prefix,
        };

        if self.cache.ttl_seconds == 0 {
            return Err(ConfigError::validation(
                path,
                "cache.ttl_seconds must be nonzero",
            ));
        }
        let cache_config_value = normalize_cache_config(self.cache.config);
        let provider =
            match self.cache.provider {
                CacheProvider::None => {
                    serde_yaml::from_value::<CacheNoneConfigRaw>(cache_config_value).map_err(
                        |source| ConfigError::Parse {
                            path: path.to_path_buf(),
                            source,
                        },
                    )?;
                    CacheProviderConfig::None
                }
                CacheProvider::InProcess => {
                    let raw: InProcessCacheConfigRaw = serde_yaml::from_value(cache_config_value)
                        .map_err(|source| ConfigError::Parse {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    if raw.max_entries == 0 {
                        return Err(ConfigError::validation(
                            path,
                            "cache.config.max_entries must be nonzero",
                        ));
                    }
                    CacheProviderConfig::InProcess(InProcessCacheConfig {
                        max_entries: raw.max_entries,
                    })
                }
                CacheProvider::Redis => {
                    let raw: RedisCacheConfigRaw = serde_yaml::from_value(cache_config_value)
                        .map_err(|source| ConfigError::Parse {
                            path: path.to_path_buf(),
                            source,
                        })?;
                    if raw.connection_string.trim().is_empty() {
                        return Err(ConfigError::validation(
                            path,
                            "cache.config.connection_string must not be empty",
                        ));
                    }
                    if raw.key_prefix.trim().is_empty() {
                        return Err(ConfigError::validation(
                            path,
                            "cache.config.key_prefix must not be empty",
                        ));
                    }
                    CacheProviderConfig::Redis(RedisCacheConfig {
                        connection_string: raw.connection_string,
                        key_prefix: raw.key_prefix,
                    })
                }
            };
        let cache = CacheConfig {
            ttl_seconds: self.cache.ttl_seconds,
            provider,
        };

        let tls_enabled = self.tls.enabled.unwrap_or(false);
        let tls_cert_path = self.tls.cert_path.filter(|value| !value.trim().is_empty());
        let tls_key_path = self.tls.key_path.filter(|value| !value.trim().is_empty());
        if tls_enabled && (tls_cert_path.is_none() || tls_key_path.is_none()) {
            return Err(ConfigError::validation(
                path,
                "tls.cert_path and tls.key_path are required when tls.enabled is true",
            ));
        }
        let tls = TlsConfig {
            enabled: tls_enabled,
            cert_path: tls_cert_path,
            key_path: tls_key_path,
            http2: self.tls.http2.unwrap_or(true),
            http3: self.tls.http3.unwrap_or(false),
        };

        Ok(ValidatedServer {
            server,
            logging,
            auth,
            database,
            timeseries,
            storage,
            cache,
            tls,
        })
    }
}
