//! Provider-neutral time-series analytics boundary.

pub mod mongo;

use std::collections::HashMap;

use anyhow::Result;

pub use mongo::MongoTimeSeries;

use crate::config::TimeseriesConfig;
use crate::domain::{AnalyticsEvent, DailyClickCount, DailyCount, DateRange, EntityId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankedLinkClicks {
    pub link_id: EntityId,
    pub clicks: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkClickSeries {
    pub total_clicks: i64,
    pub daily: Vec<DailyClickCount>,
}

pub trait TimeSeries: Send + Sync {
    async fn record_event(&self, event: &AnalyticsEvent) -> Result<()>;

    async fn total_views(&self, user_id: EntityId) -> Result<i64>;

    async fn click_totals(
        &self,
        user_id: EntityId,
        link_ids: &[EntityId],
    ) -> Result<HashMap<EntityId, i64>>;

    async fn daily_series(&self, user_id: EntityId, range: DateRange) -> Result<Vec<DailyCount>>;

    async fn top_links(
        &self,
        user_id: EntityId,
        range: DateRange,
        limit: usize,
    ) -> Result<Vec<RankedLinkClicks>>;

    async fn link_series(
        &self,
        user_id: EntityId,
        link_ids: &[EntityId],
        range: DateRange,
    ) -> Result<HashMap<EntityId, LinkClickSeries>>;

    async fn bootstrap(&self) -> Result<()>;
}

#[derive(Clone)]
pub enum TimeSeriesProvider {
    Mongo(MongoTimeSeries),
}

impl TimeSeriesProvider {
    pub async fn connect(config: &TimeseriesConfig) -> Result<Self> {
        use crate::config::DbProvider;

        match config.provider {
            DbProvider::Mongo => Ok(Self::Mongo(MongoTimeSeries::connect(config).await?)),
        }
    }
}

impl TimeSeries for TimeSeriesProvider {
    async fn record_event(&self, event: &AnalyticsEvent) -> Result<()> {
        match self {
            Self::Mongo(provider) => provider.record_event(event).await,
        }
    }

    async fn total_views(&self, user_id: EntityId) -> Result<i64> {
        match self {
            Self::Mongo(provider) => provider.total_views(user_id).await,
        }
    }

    async fn click_totals(
        &self,
        user_id: EntityId,
        link_ids: &[EntityId],
    ) -> Result<HashMap<EntityId, i64>> {
        match self {
            Self::Mongo(provider) => provider.click_totals(user_id, link_ids).await,
        }
    }

    async fn daily_series(&self, user_id: EntityId, range: DateRange) -> Result<Vec<DailyCount>> {
        match self {
            Self::Mongo(provider) => provider.daily_series(user_id, range).await,
        }
    }

    async fn top_links(
        &self,
        user_id: EntityId,
        range: DateRange,
        limit: usize,
    ) -> Result<Vec<RankedLinkClicks>> {
        match self {
            Self::Mongo(provider) => provider.top_links(user_id, range, limit).await,
        }
    }

    async fn link_series(
        &self,
        user_id: EntityId,
        link_ids: &[EntityId],
        range: DateRange,
    ) -> Result<HashMap<EntityId, LinkClickSeries>> {
        match self {
            Self::Mongo(provider) => provider.link_series(user_id, link_ids, range).await,
        }
    }

    async fn bootstrap(&self) -> Result<()> {
        match self {
            Self::Mongo(provider) => provider.bootstrap().await,
        }
    }
}
