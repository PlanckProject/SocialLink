use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::providers::cache::CacheProvider;
use crate::providers::database::DatabaseProvider;
use crate::providers::storage::StorageProvider;
use crate::providers::timeseries::TimeSeriesProvider;

use super::cache::TypedCache;
use super::media::MediaService;

#[derive(Clone)]
pub struct AppServices {
    pub(crate) database: Arc<DatabaseProvider>,
    pub(crate) timeseries: Arc<TimeSeriesProvider>,
    pub(crate) cache: TypedCache<CacheProvider>,
    pub(crate) media: MediaService<StorageProvider>,
}

impl AppServices {
    pub fn new(
        database: Arc<DatabaseProvider>,
        storage: Arc<StorageProvider>,
        timeseries: Arc<TimeSeriesProvider>,
        cache: Arc<CacheProvider>,
        config: &Config,
    ) -> Result<Self> {
        let max_upload_bytes = config
            .uploads
            .max_mb
            .checked_mul(1024 * 1024)
            .context("upload size limit is too large")?;
        let cache = TypedCache::new(cache, Duration::from_secs(config.cache.ttl_seconds));
        let media = MediaService::new(
            storage,
            max_upload_bytes,
            config.storage.route_prefix.clone(),
        )
        .context("initialize media service")?;

        Ok(Self {
            database,
            timeseries,
            cache,
            media,
        })
    }
}
