use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::domain::{EntityId, GroupOrdering, GroupStyle, LinkGroupInput};
use crate::error::{AppError, AppResult};
use crate::io::http::handlers::dto::AdminGroupDto;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct GroupPayload {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub collapsible: Option<bool>,
    #[serde(default)]
    pub style: Option<GroupStyle>,
}

impl From<GroupPayload> for LinkGroupInput {
    fn from(payload: GroupPayload) -> Self {
        Self {
            title: payload.title,
            description: payload.description,
            is_collapsible: payload.collapsible,
            style: payload.style,
        }
    }
}

#[derive(Deserialize)]
pub struct ReorderGroupsPayload {
    pub ordered_ids: Vec<String>,
}

pub async fn list_groups(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<AdminGroupDto>>> {
    let groups = state.services.list_groups(user.user_id).await?;
    Ok(Json(groups.iter().map(AdminGroupDto::from_model).collect()))
}

pub async fn create_group(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<GroupPayload>,
) -> AppResult<Json<AdminGroupDto>> {
    let group = state
        .services
        .create_group(
            user.user_id,
            payload.into(),
            state.config.content.max_groups,
        )
        .await?;
    Ok(Json(AdminGroupDto::from_model(&group)))
}

pub async fn reorder_groups(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<ReorderGroupsPayload>,
) -> AppResult<Json<Value>> {
    // The client sends the combined block order, using the literal token
    // `"ungrouped"` to mark where the synthetic ungrouped block sits. We split
    // that out: real group ids define the group ordering, and the token's index
    // becomes the persisted ungrouped position.
    let mut ordered_ids = Vec::new();
    let mut ungrouped_position = None;
    for (index, id) in payload.ordered_ids.iter().enumerate() {
        if id == "ungrouped" {
            ungrouped_position = Some(index as i32);
        } else {
            ordered_ids.push(parse_id(id, "group")?);
        }
    }
    let ordering = GroupOrdering { ordered_ids };
    state
        .services
        .reorder_groups(user.user_id, ordering, ungrouped_position)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn update_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<GroupPayload>,
) -> AppResult<Json<AdminGroupDto>> {
    let id = parse_id(&id, "group")?;
    let group = state
        .services
        .update_group(user.user_id, id, payload.into())
        .await?;
    Ok(Json(AdminGroupDto::from_model(&group)))
}

pub async fn delete_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let id = parse_id(&id, "group")?;
    state.services.delete_group(user.user_id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

fn parse_id(value: &str, entity: &str) -> AppResult<EntityId> {
    EntityId::parse(value).map_err(|_| AppError::bad_request(format!("invalid {entity} id")))
}
