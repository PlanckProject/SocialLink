use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::{
    AnalyticsEvent, AnalyticsOverview, DateRange, EntityId, LinkAnalytics, TopLink,
};
use crate::error::ErrorKind;
use crate::providers::database::{Database, LinkRepository};
use crate::providers::timeseries::TimeSeries;

use super::AppServices;

const TOP_LINK_LIMIT: usize = 5;

/// Resolves the requested analytics window into a concrete [`DateRange`].
///
/// Presets `7d`/`30d`/`90d`/`all` map to fixed windows; `custom` requires
/// inclusive `start` and `end` dates in `YYYY-MM-DD` format.
fn resolve_date_range(
    range: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<DateRange> {
    match range.unwrap_or("30d") {
        "7d" => Ok(DateRange::last_days(7)),
        "30d" => Ok(DateRange::last_days(30)),
        "90d" => Ok(DateRange::last_days(90)),
        "all" => Ok(DateRange {
            start: DateTime::<Utc>::UNIX_EPOCH,
            end: Utc::now(),
        }),
        "custom" => resolve_custom_range(start, end),
        _ => Err(ErrorKind::BadRequest(
            "range must be one of 7d, 30d, 90d, all, or custom".to_string(),
        )
        .into()),
    }
}

fn resolve_custom_range(start: Option<&str>, end: Option<&str>) -> Result<DateRange> {
    let start = start
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ErrorKind::BadRequest("custom range requires a start date".to_string()))?;
    let end = end
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ErrorKind::BadRequest("custom range requires an end date".to_string()))?;
    let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|_| {
        ErrorKind::BadRequest("start must be a valid date in YYYY-MM-DD format".to_string())
    })?;
    let end_date = NaiveDate::parse_from_str(end, "%Y-%m-%d").map_err(|_| {
        ErrorKind::BadRequest("end must be a valid date in YYYY-MM-DD format".to_string())
    })?;
    if end_date < start_date {
        return Err(
            ErrorKind::BadRequest("end date must not be before start date".to_string()).into(),
        );
    }
    let start = start_date
        .and_hms_opt(0, 0, 0)
        .expect("midnight is always valid")
        .and_utc();
    let end = end_date
        .and_hms_opt(23, 59, 59)
        .expect("end of day is always valid")
        .and_utc();
    Ok(DateRange { start, end })
}

impl AppServices {
    /// Records analytics without allowing telemetry failures to affect the
    /// user-facing request.
    pub async fn record_event_best_effort(&self, event: AnalyticsEvent) {
        if let Err(error) = self
            .timeseries
            .record_event(&event)
            .await
            .context("record analytics event")
        {
            tracing::warn!(
                error = %format_args!("{error:#}"),
                event_kind = ?event.kind,
                "analytics event recording failed"
            );
        }
    }

    /// Returns the uncached analytics overview for an owner.
    pub async fn analytics_overview(
        &self,
        user_id: EntityId,
        range: Option<&str>,
        start: Option<&str>,
        end: Option<&str>,
    ) -> Result<AnalyticsOverview> {
        let date_range = resolve_date_range(range, start, end)?;

        let links = self
            .database
            .links()
            .list(user_id)
            .await
            .context("list links for analytics overview")?;
        let titles: HashMap<_, _> = links
            .into_iter()
            .map(|link| (link.id, link.title))
            .collect();

        let daily = self
            .timeseries
            .daily_series(user_id, date_range)
            .await
            .context("query daily analytics overview")?;
        let ranked_links = self
            .timeseries
            .top_links(user_id, date_range, TOP_LINK_LIMIT)
            .await
            .context("query top links for analytics overview")?;

        let (total_views, total_clicks) = daily.iter().fold((0, 0), |(views, clicks), bucket| {
            (views + bucket.views, clicks + bucket.clicks)
        });

        let top_links = ranked_links
            .into_iter()
            .map(|ranked| TopLink {
                link_id: ranked.link_id,
                title: titles.get(&ranked.link_id).cloned().unwrap_or_default(),
                clicks: ranked.clicks,
            })
            .collect();

        Ok(AnalyticsOverview {
            total_views,
            total_clicks,
            daily,
            top_links,
        })
    }

    /// Returns analytics for every owned link, ranked by click count.
    pub async fn links_analytics(
        &self,
        user_id: EntityId,
        range: Option<&str>,
        start: Option<&str>,
        end: Option<&str>,
    ) -> Result<Vec<LinkAnalytics>> {
        let date_range = resolve_date_range(range, start, end)?;
        let links = self
            .database
            .links()
            .list(user_id)
            .await
            .context("list owned links for analytics")?;
        let link_ids: Vec<_> = links.iter().map(|link| link.id).collect();

        let mut series_by_link = self
            .timeseries
            .link_series(user_id, &link_ids, date_range)
            .await
            .context("query analytics for owned links")?;

        let mut analytics = links
            .into_iter()
            .map(|link| {
                let series = series_by_link
                    .remove(&link.id)
                    .context("analytics provider omitted an owned link")?;
                Ok(LinkAnalytics {
                    link_id: link.id,
                    title: link.title,
                    total_clicks: series.total_clicks,
                    daily: series.daily,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        analytics.sort_by_key(|entry| std::cmp::Reverse(entry.total_clicks));
        Ok(analytics)
    }
}
