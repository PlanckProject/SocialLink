//! Mongo adapter for the [`super::TimeSeries`] boundary.
//!
//! All BSON documents, aggregation pipelines and Mongo-specific filter
//! construction are private to this module — nothing here leaks into
//! [`super`]'s public trait surface. Click/view counts are always computed
//! by aggregating the events collection; this adapter never writes to a
//! link (or any other) document.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bson::{Bson, Document, doc};
use chrono::{DateTime, NaiveDate, Utc};
use futures::TryStreamExt;
use mongodb::options::{ClientOptions, Credential, IndexOptions, Tls, TlsOptions};
use mongodb::{Client, Collection, Database, IndexModel};
use serde::{Deserialize, Serialize};

use super::{LinkClickSeries, RankedLinkClicks, TimeSeries};
use crate::config::TimeseriesConfig;
use crate::domain::{AnalyticsEvent, DailyClickCount, DailyCount, DateRange, EntityId, EventKind};

/// Mongo-backed implementation of [`TimeSeries`]. Cheap to clone: it only
/// holds handles into the driver's connection pool.
#[derive(Clone)]
pub struct MongoTimeSeries {
    #[allow(dead_code)]
    database: Database,
    events: Collection<EventDoc>,
}

impl MongoTimeSeries {
    /// Connects to Mongo using the given typed time-series configuration.
    ///
    /// Connection precedence: an explicit, non-empty `connection_string`
    /// is used as-is; otherwise a `mongodb://host:port` URI is built from
    /// `host`/`port` (defaulting to the standard `27017` port). Explicit
    /// `username`/`password` and `certificate` (a client certificate/key
    /// file path for mTLS), when set, are layered on top of whichever URI
    /// was resolved. The target database and collection are always taken
    /// from the explicit `db`/`collection` fields, never inferred from the
    /// connection string. Connects fail fast via a `ping` command, and
    /// nothing about the resolved URI, credentials or certificate path is
    /// ever logged.
    pub async fn connect(config: &TimeseriesConfig) -> Result<Self> {
        let uses_connection_string = non_empty(config.connection_string.as_deref()).is_some();
        let uri = resolve_uri(config)?;
        let mut options = ClientOptions::parse(&uri)
            .await
            .context("parse Mongo timeseries connection string")?;

        let db_name = if uses_connection_string {
            options
                .default_database
                .clone()
                .context("Mongo timeseries connection string must include a database")?
        } else {
            if let Some(username) = non_empty(config.username.as_deref()) {
                options.credential = Some(
                    Credential::builder()
                        .username(username.to_string())
                        .password(config.password.clone())
                        .build(),
                );
            }

            if let Some(certificate) = non_empty(config.certificate.as_deref()) {
                options.tls = Some(Tls::Enabled(
                    TlsOptions::builder()
                        .ca_file_path(PathBuf::from(certificate))
                        .build(),
                ));
            }

            non_empty(config.db.as_deref())
                .context("timeseries.config.db must be set")?
                .to_string()
        };

        let client = Client::with_options(options).context("build Mongo timeseries client")?;
        let database = client.database(&db_name);

        // Fail fast if the server is unreachable.
        database
            .run_command(doc! { "ping": 1 })
            .await
            .context("ping Mongo timeseries database")?;

        let events = database.collection::<EventDoc>(&config.collection);

        tracing::info!(
            db = db_name,
            collection = %config.collection,
            "connected to Mongo timeseries store"
        );

        Ok(Self { database, events })
    }

    /// Raw handle to the events collection as generic documents, for
    /// aggregation pipelines.
    fn raw(&self) -> Collection<Document> {
        self.events.clone_with_type()
    }

    /// Runs an aggregation pipeline against the raw events collection and
    /// collects the resulting documents. Private: pipelines never appear
    /// in this adapter's public surface.
    async fn aggregate(&self, pipeline: Vec<Document>) -> Result<Vec<Document>> {
        let cursor = self.raw().aggregate(pipeline).await?;
        let docs = cursor.try_collect().await?;
        Ok(docs)
    }
}

impl TimeSeries for MongoTimeSeries {
    async fn record_event(&self, event: &AnalyticsEvent) -> Result<()> {
        let doc = EventDoc::from_domain(event);
        self.events
            .insert_one(doc)
            .await
            .context("insert analytics event")?;
        Ok(())
    }

    async fn total_views(&self, user_id: EntityId) -> Result<i64> {
        let filter = doc! {
            "user_id": user_id.to_hyphenated_string(),
            "kind": kind_str(EventKind::View),
        };
        let count = self
            .events
            .count_documents(filter)
            .await
            .context("count page views")?;
        Ok(count as i64)
    }

    async fn click_totals(
        &self,
        user_id: EntityId,
        link_ids: &[EntityId],
    ) -> Result<HashMap<EntityId, i64>> {
        let mut totals = zeroed_totals(link_ids);
        if link_ids.is_empty() {
            return Ok(totals);
        }

        let pipeline = vec![
            doc! { "$match": {
                "user_id": user_id.to_hyphenated_string(),
                "kind": kind_str(EventKind::Click),
                "link_id": { "$in": id_strings(link_ids) },
            } },
            doc! { "$group": { "_id": "$link_id", "count": { "$sum": 1 } } },
        ];
        let docs = self
            .aggregate(pipeline)
            .await
            .context("aggregate per-link click totals")?;

        for entry in &docs {
            let Ok(id_str) = entry.get_str("_id") else {
                continue;
            };
            let Ok(link_id) = EntityId::parse(id_str) else {
                continue;
            };
            totals.insert(link_id, count_i64(entry, "count"));
        }
        Ok(totals)
    }

    async fn daily_series(&self, user_id: EntityId, range: DateRange) -> Result<Vec<DailyCount>> {
        let pipeline = vec![
            doc! { "$match": {
                "user_id": user_id.to_hyphenated_string(),
                "created_at": { "$gte": bson_dt(range.start), "$lte": bson_dt(range.end) },
            } },
            doc! { "$group": {
                "_id": {
                    "day": { "$dateToString": { "format": "%Y-%m-%d", "date": "$created_at" } },
                    "kind": "$kind",
                },
                "count": { "$sum": 1 },
            } },
        ];
        let docs = self
            .aggregate(pipeline)
            .await
            .context("aggregate daily view/click series")?;

        let mut per_day: HashMap<String, (i64, i64)> = HashMap::new();
        for entry in &docs {
            let Ok(id) = entry.get_document("_id") else {
                continue;
            };
            let day = id.get_str("day").unwrap_or_default().to_string();
            let count = count_i64(entry, "count");
            let bucket = per_day.entry(day).or_insert((0, 0));
            match id.get_str("kind").unwrap_or_default() {
                "view" => bucket.0 += count,
                "click" => bucket.1 += count,
                _ => {}
            }
        }

        Ok(each_day(&range)
            .into_iter()
            .map(|date| {
                let (views, clicks) = per_day.get(&day_key(date)).copied().unwrap_or((0, 0));
                DailyCount {
                    date,
                    views,
                    clicks,
                }
            })
            .collect())
    }

    async fn top_links(
        &self,
        user_id: EntityId,
        range: DateRange,
        limit: usize,
    ) -> Result<Vec<RankedLinkClicks>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let pipeline = vec![
            doc! { "$match": {
                "user_id": user_id.to_hyphenated_string(),
                "kind": kind_str(EventKind::Click),
                "link_id": { "$exists": true },
                "created_at": { "$gte": bson_dt(range.start), "$lte": bson_dt(range.end) },
            } },
            doc! { "$group": { "_id": "$link_id", "count": { "$sum": 1 } } },
            doc! { "$sort": { "count": -1 } },
            doc! { "$limit": limit as i64 },
        ];
        let docs = self
            .aggregate(pipeline)
            .await
            .context("aggregate top links by clicks")?;

        Ok(docs
            .iter()
            .filter_map(|entry| {
                let link_id = EntityId::parse(entry.get_str("_id").ok()?).ok()?;
                Some(RankedLinkClicks {
                    link_id,
                    clicks: count_i64(entry, "count"),
                })
            })
            .collect())
    }

    async fn link_series(
        &self,
        user_id: EntityId,
        link_ids: &[EntityId],
        range: DateRange,
    ) -> Result<HashMap<EntityId, LinkClickSeries>> {
        let days = each_day(&range);
        let empty_series = || LinkClickSeries {
            total_clicks: 0,
            daily: days
                .iter()
                .map(|date| DailyClickCount {
                    date: *date,
                    clicks: 0,
                })
                .collect(),
        };
        let mut result: HashMap<EntityId, LinkClickSeries> =
            link_ids.iter().map(|id| (*id, empty_series())).collect();
        if link_ids.is_empty() {
            return Ok(result);
        }

        let pipeline = vec![
            doc! { "$match": {
                "user_id": user_id.to_hyphenated_string(),
                "kind": kind_str(EventKind::Click),
                "link_id": { "$in": id_strings(link_ids) },
                "created_at": { "$gte": bson_dt(range.start), "$lte": bson_dt(range.end) },
            } },
            doc! { "$group": {
                "_id": {
                    "link": "$link_id",
                    "day": { "$dateToString": { "format": "%Y-%m-%d", "date": "$created_at" } },
                },
                "count": { "$sum": 1 },
            } },
        ];
        let docs = self
            .aggregate(pipeline)
            .await
            .context("aggregate per-link daily analytics")?;

        for entry in &docs {
            let Ok(id) = entry.get_document("_id") else {
                continue;
            };
            let Some(link_id) = id
                .get_str("link")
                .ok()
                .and_then(|s| EntityId::parse(s).ok())
            else {
                continue;
            };
            let day = id.get_str("day").unwrap_or_default();
            let count = count_i64(entry, "count");
            if let Some(series) = result.get_mut(&link_id) {
                series.total_clicks += count;
                if let Some(bucket) = series
                    .daily
                    .iter_mut()
                    .find(|bucket| day_key(bucket.date) == day)
                {
                    bucket.clicks += count;
                }
            }
        }
        Ok(result)
    }

    async fn bootstrap(&self) -> Result<()> {
        self.events
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "created_at": 1 })
                    .build(),
            )
            .await
            .context("create timeseries index on user_id+created_at")?;

        self.events
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "kind": 1, "created_at": 1 })
                    .build(),
            )
            .await
            .context("create timeseries index on user_id+kind+created_at")?;

        self.events
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "link_id": 1, "kind": 1, "created_at": 1 })
                    .options(IndexOptions::builder().sparse(true).build())
                    .build(),
            )
            .await
            .context("create timeseries index on link_id+kind+created_at")?;

        self.events
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "ip_hash": 1 })
                    .options(IndexOptions::builder().sparse(true).build())
                    .build(),
            )
            .await
            .context("create timeseries index on user_id+ip_hash")?;

        self.events
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "referrer": 1 })
                    .options(IndexOptions::builder().sparse(true).build())
                    .build(),
            )
            .await
            .context("create timeseries index on user_id+referrer")?;

        Ok(())
    }
}

// ---------------------------------------------------------------------
// Connection string resolution
// ---------------------------------------------------------------------

/// Resolves the base Mongo connection URI: an explicit, non-empty
/// `connection_string` always takes precedence; otherwise a
/// `mongodb://host:port` URI is built from the discrete fields (defaulting
/// the port to `27017`).
fn resolve_uri(config: &TimeseriesConfig) -> Result<String> {
    if let Some(connection_string) = non_empty(config.connection_string.as_deref()) {
        return Ok(connection_string.to_string());
    }
    let host = non_empty(config.host.as_deref())
        .context("timeseries.config.host or connection_string must be set")?;
    let port = config.port.unwrap_or(27017);
    Ok(format!("mongodb://{host}:{port}"))
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

// ---------------------------------------------------------------------
// BSON document mapping (private to this adapter)
// ---------------------------------------------------------------------

/// Wire-format mirror of [`AnalyticsEvent`]. `link_id` is a string UUID
/// (matching [`EntityId`]'s own serde representation) rather than an
/// `ObjectId`, so it stays a plain, portable value in storage.
#[derive(Debug, Serialize, Deserialize)]
struct EventDoc {
    #[serde(rename = "_id")]
    id: String,
    user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    link_id: Option<String>,
    kind: String,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    referrer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ip_hash: Option<String>,
}

impl EventDoc {
    fn from_domain(event: &AnalyticsEvent) -> Self {
        Self {
            id: event.id.to_hyphenated_string(),
            user_id: event.user_id.to_hyphenated_string(),
            link_id: event.link_id.map(|id| id.to_hyphenated_string()),
            kind: kind_str(event.kind).to_string(),
            created_at: event.created_at,
            referrer: event.referrer.clone(),
            user_agent: event.user_agent.clone(),
            ip_hash: event.ip_hash.clone(),
        }
    }
}

fn kind_str(kind: EventKind) -> &'static str {
    match kind {
        EventKind::View => "view",
        EventKind::Click => "click",
    }
}

fn bson_dt(dt: DateTime<Utc>) -> Bson {
    Bson::DateTime(bson::DateTime::from_chrono(dt))
}

fn id_strings(ids: &[EntityId]) -> Vec<String> {
    ids.iter().map(|id| id.to_hyphenated_string()).collect()
}

fn zeroed_totals(link_ids: &[EntityId]) -> HashMap<EntityId, i64> {
    link_ids.iter().map(|id| (*id, 0)).collect()
}

/// Extracts an aggregation `$sum`/`$count` result as `i64`, whether Mongo
/// returned it as a 32- or 64-bit integer.
fn count_i64(doc: &Document, field: &str) -> i64 {
    doc.get_i64(field)
        .or_else(|_| doc.get_i32(field).map(i64::from))
        .unwrap_or(0)
}

fn day_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Every calendar day in `range`, inclusive of both endpoints.
fn each_day(range: &DateRange) -> Vec<NaiveDate> {
    let mut days = Vec::new();
    let mut cursor = range.start_date();
    let end = range.end_date();
    while cursor <= end {
        days.push(cursor);
        cursor += chrono::Duration::days(1);
    }
    days
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DbProvider;
    use crate::domain::RequestMetadata;

    fn timeseries_config() -> TimeseriesConfig {
        TimeseriesConfig {
            provider: DbProvider::Mongo,
            host: Some("mongo".to_string()),
            port: Some(27017),
            db: Some("social-link".to_string()),
            certificate: None,
            username: None,
            password: None,
            connection_string: None,
            collection: "events".to_string(),
        }
    }

    #[test]
    fn resolve_uri_prefers_connection_string() {
        let mut config = timeseries_config();
        config.connection_string = Some("mongodb+srv://user:pass@cluster/db".to_string());
        assert_eq!(
            resolve_uri(&config).unwrap(),
            "mongodb+srv://user:pass@cluster/db"
        );
    }

    #[test]
    fn resolve_uri_builds_from_host_and_port() {
        let config = timeseries_config();
        assert_eq!(resolve_uri(&config).unwrap(), "mongodb://mongo:27017");
    }

    #[test]
    fn resolve_uri_defaults_port_when_missing() {
        let mut config = timeseries_config();
        config.port = None;
        assert_eq!(resolve_uri(&config).unwrap(), "mongodb://mongo:27017");
    }

    #[test]
    fn resolve_uri_ignores_blank_connection_string() {
        let mut config = timeseries_config();
        config.connection_string = Some("   ".to_string());
        assert_eq!(resolve_uri(&config).unwrap(), "mongodb://mongo:27017");
    }

    #[test]
    fn resolve_uri_requires_host_or_connection_string() {
        let mut config = timeseries_config();
        config.host = None;
        assert!(resolve_uri(&config).is_err());
    }

    #[test]
    fn non_empty_trims_and_rejects_blank() {
        assert_eq!(non_empty(Some("  value  ")), Some("value"));
        assert_eq!(non_empty(Some("   ")), None);
        assert_eq!(non_empty(None), None);
    }

    #[test]
    fn event_doc_round_trips_domain_fields() {
        let user_id = EntityId::new();
        let link_id = EntityId::new();
        let meta = RequestMetadata {
            referrer: Some("https://example.com".into()),
            user_agent: Some("test-agent".into()),
            ip_hash: Some("abc123".into()),
        };
        let event = AnalyticsEvent::new(user_id, Some(link_id), EventKind::Click, meta);
        let doc = EventDoc::from_domain(&event);

        assert_eq!(doc.id, event.id.to_hyphenated_string());
        assert_eq!(doc.user_id, user_id.to_hyphenated_string());
        assert_eq!(doc.link_id, Some(link_id.to_hyphenated_string()));
        assert_eq!(doc.kind, "click");
        assert_eq!(doc.created_at, event.created_at);
    }

    #[test]
    fn event_doc_omits_link_id_for_views() {
        let event = AnalyticsEvent::new(
            EntityId::new(),
            None,
            EventKind::View,
            RequestMetadata::default(),
        );
        let doc = EventDoc::from_domain(&event);
        assert_eq!(doc.kind, "view");
        assert!(doc.link_id.is_none());
    }

    #[test]
    fn kind_str_maps_both_variants() {
        assert_eq!(kind_str(EventKind::View), "view");
        assert_eq!(kind_str(EventKind::Click), "click");
    }

    #[test]
    fn each_day_covers_inclusive_span() {
        let range = DateRange::last_days(7);
        let days = each_day(&range);
        assert_eq!(days.len(), 7);
        assert_eq!(days.first().copied(), Some(range.start_date()));
        assert_eq!(days.last().copied(), Some(range.end_date()));
    }

    #[test]
    fn day_key_formats_as_iso_date() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 4).unwrap();
        assert_eq!(day_key(date), "2026-07-04");
    }

    #[test]
    fn count_i64_reads_both_int_widths() {
        let doc32 = doc! { "count": 5_i32 };
        let doc64 = doc! { "count": 9_i64 };
        let missing = doc! {};
        assert_eq!(count_i64(&doc32, "count"), 5);
        assert_eq!(count_i64(&doc64, "count"), 9);
        assert_eq!(count_i64(&missing, "count"), 0);
    }

    #[test]
    fn zeroed_totals_covers_every_requested_id() {
        let a = EntityId::new();
        let b = EntityId::new();
        let totals = zeroed_totals(&[a, b]);
        assert_eq!(totals.get(&a), Some(&0));
        assert_eq!(totals.get(&b), Some(&0));
        assert_eq!(totals.len(), 2);
    }

    #[test]
    fn id_strings_matches_entity_id_display() {
        let a = EntityId::new();
        let b = EntityId::new();
        assert_eq!(id_strings(&[a, b]), vec![a.to_string(), b.to_string()]);
    }
}
