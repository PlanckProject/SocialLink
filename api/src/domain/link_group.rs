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

/// Corner treatment applied to a group's link cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupCorners {
    #[default]
    Rounded,
    Sharp,
}

/// Shape of a group's link icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupIconShape {
    #[default]
    Round,
    Square,
}

/// Per-group shape styling. The group owns the *shape* of its links so all
/// links in a group look consistent, while colors/text remain theme-owned.
/// Defaults are list + rounded + round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GroupStyle {
    #[serde(default)]
    pub layout: GroupLayout,
    #[serde(default)]
    pub corners: GroupCorners,
    #[serde(default)]
    pub icon: GroupIconShape,
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
    fn new_defaults_style_to_list_rounded_round() {
        let group = LinkGroup::new(EntityId::new(), "Shopping", 0);
        assert_eq!(group.style.layout, GroupLayout::List);
        assert_eq!(group.style.corners, GroupCorners::Rounded);
        assert_eq!(group.style.icon, GroupIconShape::Round);
    }

    #[test]
    fn group_style_serializes_to_snake_case() {
        let style = GroupStyle {
            layout: GroupLayout::Grid,
            corners: GroupCorners::Sharp,
            icon: GroupIconShape::Square,
        };
        let json = serde_json::to_value(style).expect("serialize");
        assert_eq!(json["layout"], "grid");
        assert_eq!(json["corners"], "sharp");
        assert_eq!(json["icon"], "square");
    }

    #[test]
    fn group_style_deserializes_partial_with_defaults() {
        let style: GroupStyle =
            serde_json::from_str(r#"{"layout":"grid"}"#).expect("deserialize");
        assert_eq!(style.layout, GroupLayout::Grid);
        assert_eq!(style.corners, GroupCorners::Rounded);
        assert_eq!(style.icon, GroupIconShape::Round);
    }
}
