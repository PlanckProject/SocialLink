//! A single link shown on the public page.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::id::EntityId;

/// A single link. `group_id` is `None` when the link is ungrouped.
/// `expires_at`, when set and in the past, hides the link from the public
/// page. Click totals are *not* stored on the link itself — they are
/// derived from [`crate::domain::analytics::AnalyticsEvent`] time-series
/// data, which is the single source of truth for click counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub id: EntityId,
    pub user_id: EntityId,
    #[serde(default)]
    pub group_id: Option<EntityId>,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub icon_image: Option<String>,
    #[serde(default)]
    pub icon_font: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Link {
    /// Builds a new active link owned by `user_id`, appended at
    /// `sort_order` within `group_id` (or the ungrouped list).
    pub fn new(
        user_id: EntityId,
        group_id: Option<EntityId>,
        title: impl Into<String>,
        url: impl Into<String>,
        sort_order: i32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            user_id,
            group_id,
            title: title.into(),
            url: url.into(),
            description: None,
            icon: None,
            icon_image: None,
            icon_font: None,
            sort_order,
            is_active: true,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Fields accepted when creating or fully updating a link, mirroring the
/// admin form. `is_active` is left unchanged on update when `None`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkInput {
    #[serde(default)]
    pub group_id: Option<EntityId>,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub icon_image: Option<String>,
    #[serde(default)]
    pub icon_font: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// A new ordering (and optional re-parenting to `group_id`) for a set of
/// links.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkOrdering {
    #[serde(default)]
    pub group_id: Option<EntityId>,
    pub ordered_ids: Vec<EntityId>,
}

/// Filters links belonging to a single owner, optionally scoped to one
/// group (`Some(None)` means "ungrouped only"; `None` means "any group").
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkFilter {
    pub user_id: EntityId,
    #[serde(default)]
    pub group_id: Option<Option<EntityId>>,
}

impl LinkFilter {
    pub fn for_user(user_id: EntityId) -> Self {
        Self {
            user_id,
            group_id: None,
        }
    }

    pub fn in_group(mut self, group_id: Option<EntityId>) -> Self {
        self.group_id = Some(group_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_active_with_no_group() {
        let owner = EntityId::new();
        let link = Link::new(owner, None, "GitHub", "https://github.com", 0);
        assert_eq!(link.user_id, owner);
        assert!(link.group_id.is_none());
        assert!(link.is_active);
        assert!(link.expires_at.is_none());
        assert_eq!(link.created_at, link.updated_at);
    }

    #[test]
    fn link_filter_builder_scopes_to_group() {
        let owner = EntityId::new();
        let group = EntityId::new();
        let filter = LinkFilter::for_user(owner).in_group(Some(group));
        assert_eq!(filter.user_id, owner);
        assert_eq!(filter.group_id, Some(Some(group)));
    }
}
