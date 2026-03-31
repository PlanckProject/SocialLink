//! Saved appearance themes, including the shared theme library/marketplace
//! metadata (favorites, public sharing, download counts).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::id::EntityId;

/// Origin of a theme document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeSource {
    #[default]
    Custom,
    Imported,
    Preset,
    Marketplace,
}

/// A saved, named theme. `config` holds the parsed theme JSON so the whole
/// appearance is runtime-editable. One theme per owner is active at a time.
/// The extra metadata (`owner`, `is_public`, `source`, `download_count`, ...)
/// makes the collection a per-user theme library and is forward-compatible
/// with a future theme marketplace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub id: EntityId,
    pub user_id: EntityId,
    /// Username of the theme's author (denormalized for sharing/marketplace).
    #[serde(default)]
    pub owner: Option<String>,
    pub name: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: ThemeSource,
    #[serde(default)]
    pub download_count: i64,
    pub config: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Theme {
    /// Builds a new theme document with library defaults. Centralizing
    /// construction keeps every call site in sync as fields are added.
    pub fn new(
        user_id: EntityId,
        owner: Option<String>,
        name: impl Into<String>,
        config: Value,
        source: ThemeSource,
        is_active: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            user_id,
            owner,
            name: name.into(),
            is_active,
            is_favorite: false,
            is_public: false,
            description: None,
            tags: Vec::new(),
            source,
            download_count: 0,
            config,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Partial update applied to an existing theme. `None` fields are left
/// unchanged.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ThemeUpdate {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub is_public: Option<bool>,
    #[serde(default)]
    pub config: Option<Value>,
}

/// Filters the theme library for one owner.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ThemeFilter {
    pub user_id: EntityId,
    #[serde(default)]
    pub favorite: Option<bool>,
    #[serde(default)]
    pub source: Option<ThemeSource>,
}

impl ThemeFilter {
    pub fn for_user(user_id: EntityId) -> Self {
        Self {
            user_id,
            favorite: None,
            source: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_library_defaults() {
        let owner = EntityId::new();
        let theme = Theme::new(
            owner,
            Some("alice".into()),
            "Midnight",
            serde_json::json!({}),
            ThemeSource::Custom,
            true,
        );
        assert_eq!(theme.user_id, owner);
        assert_eq!(theme.owner.as_deref(), Some("alice"));
        assert!(theme.is_active);
        assert!(!theme.is_favorite);
        assert!(!theme.is_public);
        assert_eq!(theme.download_count, 0);
        assert_eq!(theme.source, ThemeSource::Custom);
        assert_eq!(theme.created_at, theme.updated_at);
    }

    #[test]
    fn theme_source_serializes_snake_case() {
        let json = serde_json::to_string(&ThemeSource::Marketplace).unwrap();
        assert_eq!(json, "\"marketplace\"");
    }
}
