use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::domain::{AnalyticsOverview, DailyClickCount, DailyCount, LinkAnalytics, TopLink};
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RangeQuery {
    #[serde(default)]
    pub range: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

#[derive(Serialize)]
pub struct AnalyticsOverviewResponse {
    totals: AnalyticsTotalsResponse,
    series: Vec<DailyCount>,
    top_links: Vec<TopLinkResponse>,
}

#[derive(Serialize)]
struct AnalyticsTotalsResponse {
    views: i64,
    clicks: i64,
}

#[derive(Serialize)]
struct TopLinkResponse {
    id: String,
    title: String,
    clicks: i64,
}

impl From<AnalyticsOverview> for AnalyticsOverviewResponse {
    fn from(overview: AnalyticsOverview) -> Self {
        Self {
            totals: AnalyticsTotalsResponse {
                views: overview.total_views,
                clicks: overview.total_clicks,
            },
            series: overview.daily,
            top_links: overview
                .top_links
                .into_iter()
                .map(TopLinkResponse::from)
                .collect(),
        }
    }
}

impl From<TopLink> for TopLinkResponse {
    fn from(link: TopLink) -> Self {
        Self {
            id: link.link_id.to_string(),
            title: link.title,
            clicks: link.clicks,
        }
    }
}

#[derive(Serialize)]
pub struct LinkAnalyticsResponse {
    link_id: String,
    title: String,
    clicks: i64,
    series: Vec<DailyClickCount>,
}

impl From<LinkAnalytics> for LinkAnalyticsResponse {
    fn from(analytics: LinkAnalytics) -> Self {
        Self {
            link_id: analytics.link_id.to_string(),
            title: analytics.title,
            clicks: analytics.total_clicks,
            series: analytics.daily,
        }
    }
}

/// Aggregated views/clicks over a date range, plus the top links.
pub async fn overview(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<RangeQuery>,
) -> AppResult<Json<AnalyticsOverviewResponse>> {
    let overview = state
        .services
        .analytics_overview(
            user.user_id,
            query.range.as_deref(),
            query.start.as_deref(),
            query.end.as_deref(),
        )
        .await?;
    Ok(Json(overview.into()))
}

/// Per-link click totals with a daily series over the range.
pub async fn links(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<RangeQuery>,
) -> AppResult<Json<Vec<LinkAnalyticsResponse>>> {
    let analytics = state
        .services
        .links_analytics(
            user.user_id,
            query.range.as_deref(),
            query.start.as_deref(),
            query.end.as_deref(),
        )
        .await?;
    Ok(Json(
        analytics
            .into_iter()
            .map(LinkAnalyticsResponse::from)
            .collect(),
    ))
}
