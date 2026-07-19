//! Named groups that organize an owner's [`crate::domain::link::Link`]s.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::id::EntityId;

/// How a group lays out its links. Governs card shape only (list = wide
/// rows, grid = square tiles); colors and text stay owned by the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupLayout {
    #[default]
    List,
    Grid,
}

fn default_link_radius() -> String {
    "22%".to_string()
}

fn default_icon_radius() -> String {
    "50%".to_string()
}

fn default_group_spacing() -> String {
    "12px".to_string()
}

/// Per-group appearance. The group owns how its links look — the layout, the
/// corner roundness of the link cards and their icon badges, and the spacing
/// between links — so every link in a group looks consistent, while
/// colors/text remain theme-owned. `link_radius`/`icon_radius` are
/// percent-like strings (0–50; `>=50` renders as a pill/circle) mirroring the
/// theme radii; `spacing` is any CSS length. Defaults are list + 22% + 50% +
/// 12px.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupStyle {
    #[serde(default)]
    pub layout: GroupLayout,
    #[serde(default = "default_link_radius")]
    pub link_radius: String,
    #[serde(default = "default_icon_radius")]
    pub icon_radius: String,
    #[serde(default = "default_group_spacing")]
    pub spacing: String,
}

impl Default for GroupStyle {
    fn default() -> Self {
        Self {
            layout: GroupLayout::default(),
            link_radius: default_link_radius(),
            icon_radius: default_icon_radius(),
            spacing: default_group_spacing(),
        }
    }
}

/// A named list that groups links together (e.g. "Shopping").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkGroup {
    pub id: EntityId,
    pub user_id: EntityId,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_collapsible: bool,
    pub is_active: bool,
    #[serde(default)]
    pub style: GroupStyle,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LinkGroup {
    /// Builds a new active group owned by `user_id`, appended at
    /// `sort_order`.
    pub fn new(user_id: EntityId, title: impl Into<String>, sort_order: i32) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            user_id,
            title: title.into(),
            description: None,
            sort_order,
            is_collapsible: true,
            is_active: true,
            style: GroupStyle::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Fields accepted when creating or updating a group. `is_collapsible`
/// defaults to `true` when creating and is left unchanged on update when
/// `None`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkGroupInput {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_collapsible: Option<bool>,
    #[serde(default)]
    pub style: Option<GroupStyle>,
}

/// A new ordering (and optionally re-parenting) for a set of groups.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GroupOrdering {
    pub ordered_ids: Vec<EntityId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_collapsible_and_active() {
        let owner = EntityId::new();
        let group = LinkGroup::new(owner, "Shopping", 2);
        assert_eq!(group.user_id, owner);
        assert_eq!(group.title, "Shopping");
        assert_eq!(group.sort_order, 2);
        assert!(group.is_collapsible);
        assert!(group.is_active);
        assert_eq!(group.created_at, group.updated_at);
    }

    #[test]
    fn new_defaults_style_to_list_defaults() {
        let group = LinkGroup::new(EntityId::new(), "Shopping", 0);
        assert_eq!(group.style.layout, GroupLayout::List);
        assert_eq!(group.style.link_radius, "22%");
        assert_eq!(group.style.icon_radius, "50%");
        assert_eq!(group.style.spacing, "12px");
    }

    #[test]
    fn group_style_serializes_to_snake_case() {
        let style = GroupStyle {
            layout: GroupLayout::Grid,
            link_radius: "10%".to_string(),
            icon_radius: "0%".to_string(),
            spacing: "20px".to_string(),
        };
        let json = serde_json::to_value(style).expect("serialize");
        assert_eq!(json["layout"], "grid");
        assert_eq!(json["link_radius"], "10%");
        assert_eq!(json["icon_radius"], "0%");
        assert_eq!(json["spacing"], "20px");
    }

    #[test]
    fn group_style_deserializes_partial_with_defaults() {
        let style: GroupStyle =
            serde_json::from_str(r#"{"layout":"grid"}"#).expect("deserialize");
        assert_eq!(style.layout, GroupLayout::Grid);
        assert_eq!(style.link_radius, "22%");
        assert_eq!(style.icon_radius, "50%");
        assert_eq!(style.spacing, "12px");
    }
}
