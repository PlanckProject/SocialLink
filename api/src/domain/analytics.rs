//! Analytics events and provider-neutral read models.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::id::EntityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    View,
    Click,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestMetadata {
    #[serde(default)]
    pub referrer: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub ip_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub id: EntityId,
    pub user_id: EntityId,
    #[serde(default)]
    pub link_id: Option<EntityId>,
    pub kind: EventKind,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub referrer: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub ip_hash: Option<String>,
}

impl AnalyticsEvent {
    pub fn new(
        user_id: EntityId,
        link_id: Option<EntityId>,
        kind: EventKind,
        meta: RequestMetadata,
    ) -> Self {
        Self {
            id: EntityId::new(),
            user_id,
            link_id,
            kind,
            created_at: Utc::now(),
            referrer: meta.referrer,
            user_agent: meta.user_agent,
            ip_hash: meta.ip_hash,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl DateRange {
    pub fn last_days(days: i64) -> Self {
        let today = Utc::now().date_naive();
        let start_date = today - chrono::Duration::days(days.max(1) - 1);
        let start = start_date
            .and_hms_opt(0, 0, 0)
            .expect("midnight is always valid")
            .and_utc();
        Self {
            start,
            end: Utc::now(),
        }
    }

    pub fn start_date(&self) -> NaiveDate {
        self.start.date_naive()
    }

    pub fn end_date(&self) -> NaiveDate {
        self.end.date_naive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyCount {
    pub date: NaiveDate,
    #[serde(default)]
    pub views: i64,
    #[serde(default)]
    pub clicks: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyClickCount {
    pub date: NaiveDate,
    pub clicks: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopLink {
    pub link_id: EntityId,
    pub title: String,
    pub clicks: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsOverview {
    pub total_views: i64,
    pub total_clicks: i64,
    pub daily: Vec<DailyCount>,
    pub top_links: Vec<TopLink>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkAnalytics {
    pub link_id: EntityId,
    pub title: String,
    pub total_clicks: i64,
    pub daily: Vec<DailyClickCount>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_event_stamps_id_and_timestamp() {
        let user_id = EntityId::new();
        let meta = RequestMetadata {
            referrer: Some("https://example.com".into()),
            user_agent: Some("test-agent".into()),
            ip_hash: Some("abc123".into()),
        };
        let event = AnalyticsEvent::new(user_id, None, EventKind::View, meta.clone());
        assert_eq!(event.user_id, user_id);
        assert!(event.link_id.is_none());
        assert_eq!(event.kind, EventKind::View);
        assert_eq!(event.referrer, meta.referrer);
        assert_eq!(event.user_agent, meta.user_agent);
        assert_eq!(event.ip_hash, meta.ip_hash);
    }

    #[test]
    fn event_kind_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&EventKind::View).unwrap(), "\"view\"");
        assert_eq!(
            serde_json::to_string(&EventKind::Click).unwrap(),
            "\"click\""
        );
    }

    #[test]
    fn date_range_last_days_covers_expected_span() {
        let range = DateRange::last_days(7);
        assert_eq!((range.end_date() - range.start_date()).num_days(), 6);
    }
}
