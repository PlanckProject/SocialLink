pub mod cache;
pub mod database;
pub mod logging;
pub mod storage;
pub mod timeseries;

use anyhow::{Context, Result};

use crate::config::{
    CacheProviderConfig, DatabaseConfig, LogFormat as ConfigLogFormat,
    LogProvider as ConfigLogProvider, LoggingConfig, StorageConfig,
    StorageProvider as ConfigStorageProvider, TimeseriesConfig,
};

use self::cache::{CacheProvider, InProcessCache, NoCache, RedisCache};
use self::database::DatabaseProvider;
use self::logging::{LocalLoggingSettings, LogFormat, LoggingProvider};
use self::storage::StorageProvider;
use self::timeseries::TimeSeriesProvider;

pub fn get_logging_provider(config: &LoggingConfig) -> Result<LoggingProvider> {
    match config.provider {
        ConfigLogProvider::Local => {
            let max_size_bytes = config
                .rotation_max_size_mb
                .checked_mul(1024 * 1024)
                .context("logging rotation size is too large")?;
            LoggingProvider::init_local(LocalLoggingSettings {
                level: config.level.clone(),
                directives: config.directives.clone(),
                format: match config.format {
                    ConfigLogFormat::Text => LogFormat::Text,
                    ConfigLogFormat::Json => LogFormat::Json,
                },
                file: config.file.clone(),
                mirror_stdout: config.mirror_stdout,
                max_size_bytes,
                max_files: config.rotation_max_files as usize,
            })
            .context("initialize configured logging provider")
        }
    }
}

pub async fn get_database_provider(config: &DatabaseConfig) -> Result<DatabaseProvider> {
    DatabaseProvider::connect(config)
        .await
        .context("initialize configured database provider")
}

pub async fn get_storage_provider(config: &StorageConfig) -> Result<StorageProvider> {
    match config.provider {
        ConfigStorageProvider::Local => StorageProvider::local(config.base_path.clone())
            .await
            .context("initialize configured storage provider"),
    }
}

pub async fn get_timeseries_provider(config: &TimeseriesConfig) -> Result<TimeSeriesProvider> {
    TimeSeriesProvider::connect(config)
        .await
        .context("initialize configured time-series provider")
}

pub async fn get_cache_provider(config: &crate::config::CacheConfig) -> Result<CacheProvider> {
    match &config.provider {
        CacheProviderConfig::None => Ok(CacheProvider::None(NoCache)),
        CacheProviderConfig::InProcess(config) => Ok(CacheProvider::InProcess(
            InProcessCache::new(config.max_entries)
                .context("initialize in-process cache provider")?,
        )),
        CacheProviderConfig::Redis(config) => Ok(CacheProvider::Redis(Box::new(
            RedisCache::connect(&config.connection_string, config.key_prefix.clone())
                .await
                .context("initialize Redis cache provider")?,
        ))),
    }
}
