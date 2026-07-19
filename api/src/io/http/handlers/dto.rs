use serde::Serialize;
use serde_json::Value;

use crate::domain::{Branding, GroupStyle, Link, LinkGroup, Person};
use crate::util::utc_to_iso;

#[derive(Serialize)]
pub struct PublicLinkDto {
    pub id: String,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub icon_image: Option<String>,
    pub icon_font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_count: Option<i64>,
    pub expires_at: Option<String>,
}

impl PublicLinkDto {
    /// Builds a public link DTO. The click count is included only when the
    /// active theme opts in via `show_click_count`; otherwise it is omitted
    /// from the response entirely.
    pub fn from_model(l: &Link, click_count: Option<i64>) -> Self {
        Self {
            id: l.id.to_string(),
            title: l.title.clone(),
            url: l.url.clone(),
            description: l.description.clone(),
            icon: l.icon.clone(),
            icon_image: l.icon_image.clone(),
            icon_font: l.icon_font.clone(),
            click_count,
            expires_at: l.expires_at.as_ref().map(utc_to_iso),
        }
    }
}

#[derive(Serialize)]
pub struct PublicGroupDto {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub collapsible: bool,
    pub style: GroupStyle,
    pub links: Vec<PublicLinkDto>,
}

#[derive(Serialize)]
pub struct SocialDto {
    pub platform: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct ProfileDto {
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub socials: Vec<SocialDto>,
    pub avatar_url: Option<String>,
    pub cover_url: Option<String>,
}

impl ProfileDto {
    pub fn from_model(p: &Person) -> Self {
        Self {
            username: p.username.clone(),
            display_name: p.display_name.clone(),
            bio: p.bio.clone(),
            location: p.location.clone(),
            socials: p
                .socials
                .iter()
                .map(|s| SocialDto {
                    platform: s.platform.clone(),
                    url: s.url.clone(),
                })
                .collect(),
            avatar_url: p.avatar_path.clone(),
            cover_url: p.cover_path.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct StatsDto {
    pub views: i64,
}

#[derive(Serialize)]
pub struct PublicProfileResponse {
    pub profile: ProfileDto,
    pub groups: Vec<PublicGroupDto>,
    pub ungrouped: Vec<PublicLinkDto>,
    /// Insertion index for the ungrouped block among `groups` (clamped to
    /// `groups.len()`), so the client renders it at the owner-chosen position.
    pub ungrouped_position: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<StatsDto>,
    pub theme: Value,
    pub branding: Branding,
}

#[derive(Serialize)]
pub struct AdminProfileDto {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub socials: Vec<SocialDto>,
    pub avatar_url: Option<String>,
    pub cover_url: Option<String>,
    pub branding: Branding,
    pub ungrouped_position: i32,
}

impl AdminProfileDto {
    pub fn from_model(p: &Person) -> Self {
        Self {
            username: p.username.clone(),
            email: p.email.clone(),
            display_name: p.display_name.clone(),
            bio: p.bio.clone(),
            location: p.location.clone(),
            socials: p
                .socials
                .iter()
                .map(|s| SocialDto {
                    platform: s.platform.clone(),
                    url: s.url.clone(),
                })
                .collect(),
            avatar_url: p.avatar_path.clone(),
            cover_url: p.cover_path.clone(),
            branding: p.branding.clone(),
            ungrouped_position: p.ungrouped_position,
        }
    }
}

#[derive(Serialize)]
pub struct AdminGroupDto {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub collapsible: bool,
    pub is_active: bool,
    pub sort_order: i32,
    pub style: GroupStyle,
}

impl AdminGroupDto {
    pub fn from_model(g: &LinkGroup) -> Self {
        Self {
            id: g.id.to_string(),
            title: g.title.clone(),
            description: g.description.clone(),
            collapsible: g.is_collapsible,
            is_active: g.is_active,
            sort_order: g.sort_order,
            style: g.style.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct AdminLinkDto {
    pub id: String,
    pub group_id: Option<String>,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub icon_image: Option<String>,
    pub icon_font: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub expires_at: Option<String>,
    pub click_count: i64,
}

impl AdminLinkDto {
    pub fn from_model(l: &Link, click_count: i64) -> Self {
        Self {
            id: l.id.to_string(),
            group_id: l.group_id.map(|id| id.to_string()),
            title: l.title.clone(),
            url: l.url.clone(),
            description: l.description.clone(),
            icon: l.icon.clone(),
            icon_image: l.icon_image.clone(),
            icon_font: l.icon_font.clone(),
            is_active: l.is_active,
            sort_order: l.sort_order,
            expires_at: l.expires_at.as_ref().map(utc_to_iso),
            click_count,
        }
    }
}
