use axum::Json;
use axum::extract::{Path, State};
use axum::response::Redirect;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::domain::{EntityId, LinkInput, LinkOrdering};
use crate::error::{AppError, AppResult};
use crate::io::http::handlers::dto::AdminLinkDto;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LinkPayload {
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
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
    pub expires_at: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

impl LinkPayload {
    fn into_input(self) -> AppResult<LinkInput> {
        let icon = normalize_optional(self.icon);
        let icon_image = normalize_optional(self.icon_image);
        let icon_font = normalize_optional(self.icon_font);
        let icon_count = [&icon, &icon_image, &icon_font]
            .into_iter()
            .filter(|value| value.is_some())
            .count();
        if icon_count > 1 {
            return Err(AppError::bad_request(
                "a link may have at most one icon: choose an emoji, an image, or an icon class",
            ));
        }
        Ok(LinkInput {
            group_id: parse_optional_id(self.group_id.as_deref(), "group")?,
            title: self.title,
            url: self.url,
            description: self.description,
            icon,
            icon_image,
            icon_font,
            expires_at: parse_optional_datetime(self.expires_at.as_deref())?,
            is_active: self.is_active,
        })
    }
}

pub async fn list_links(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<AdminLinkDto>>> {
    Ok(Json(state.services.admin_links(user.user_id).await?))
}

pub async fn create_link(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<LinkPayload>,
) -> AppResult<Json<AdminLinkDto>> {
    let link = state
        .services
        .create_link(user.user_id, payload.into_input()?)
        .await?;
    let response = state.services.admin_link(user.user_id, &link).await?;
    Ok(Json(response))
}

pub async fn update_link(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<LinkPayload>,
) -> AppResult<Json<AdminLinkDto>> {
    let id = parse_id(&id, "link")?;
    let link = state
        .services
        .update_link(user.user_id, id, payload.into_input()?)
        .await?;
    let response = state.services.admin_link(user.user_id, &link).await?;
    Ok(Json(response))
}

pub async fn delete_link(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let id = parse_id(&id, "link")?;
    state.services.delete_link(user.user_id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn preview_link(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Redirect> {
    let id = parse_id(&id, "link")?;
    let link = state.services.preview_link(user.user_id, id).await?;
    Ok(Redirect::to(&link.url))
}

#[derive(Deserialize)]
pub struct ReorderPayload {
    #[serde(default)]
    pub group_id: Option<String>,
    pub ordered_ids: Vec<String>,
}

pub async fn reorder_links(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<ReorderPayload>,
) -> AppResult<Json<Value>> {
    let ordering = LinkOrdering {
        group_id: parse_optional_id(payload.group_id.as_deref(), "group")?,
        ordered_ids: payload
            .ordered_ids
            .iter()
            .map(|id| parse_id(id, "link"))
            .collect::<AppResult<_>>()?,
    };
    state.services.reorder_links(user.user_id, ordering).await?;
    Ok(Json(json!({ "ok": true })))
}

fn parse_id(value: &str, entity: &str) -> AppResult<EntityId> {
    EntityId::parse(value).map_err(|_| AppError::bad_request(format!("invalid {entity} id")))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_optional_id(value: Option<&str>, entity: &str) -> AppResult<Option<EntityId>> {
    match value.map(str::trim) {
        Some(value) if !value.is_empty() => parse_id(value, entity).map(Some),
        _ => Ok(None),
    }
}

fn parse_optional_datetime(value: Option<&str>) -> AppResult<Option<DateTime<Utc>>> {
    match value.map(str::trim) {
        Some(value) if !value.is_empty() => DateTime::parse_from_rfc3339(value)
            .map(|value| Some(value.with_timezone(&Utc)))
            .map_err(|_| AppError::bad_request("invalid datetime (expected RFC 3339)")),
        _ => Ok(None),
    }
}
