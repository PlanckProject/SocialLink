use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::AppResult;
use crate::io::http::handlers::common;
use crate::io::http::handlers::dto::PublicProfileResponse;
use crate::state::AppState;
use crate::util::req_meta;

/// Single-mode public profile: the seeded/primary owner.
pub async fn get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<PublicProfileResponse>> {
    let person = common::primary_user(&state).await?;
    let response = common::build_public_profile(&state, &person).await?;
    common::log_event(&state, person.id, None, "view", &req_meta(&headers)).await;
    Ok(Json(response))
}

/// Multi-mode public profile addressed by username.
pub async fn get_profile_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<PublicProfileResponse>> {
    // Tolerate a leading `@` so `/u/@name` and `/u/name` both resolve.
    let username = username.strip_prefix('@').unwrap_or(&username);
    let person = common::user_by_username(&state, username).await?;
    let response = common::build_public_profile(&state, &person).await?;
    common::log_event(&state, person.id, None, "view", &req_meta(&headers)).await;
    Ok(Json(response))
}

/// Query parameters for the username availability check (`?username=` or `?u=`).
#[derive(Deserialize)]
pub struct UsernameQuery {
    #[serde(default, alias = "u")]
    pub username: String,
}

/// Lightweight public check used by the signup and profile forms to tell,
/// before submitting, whether a handle is both well-formed and still free.
/// Applies the same normalization/validation as the write path so the answer
/// matches what a subsequent save would do.
pub async fn check_username(
    State(state): State<AppState>,
    Query(query): Query<UsernameQuery>,
) -> AppResult<Json<Value>> {
    let availability = state
        .services
        .username_availability(&query.username)
        .await?;

    Ok(Json(json!({
        "username": availability.username,
        "valid": availability.valid,
        "available": availability.available,
        "reason": availability.reason,
    })))
}
