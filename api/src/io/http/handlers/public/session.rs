use axum::Json;
use axum::extract::State;
use axum_extra::extract::CookieJar;
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::io::http::handlers::common;
use crate::state::AppState;

/// Clears the auth cookie.
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> (CookieJar, Json<Value>) {
    let jar = jar.add(common::clear_auth_cookie(&state.config));
    (jar, Json(json!({ "ok": true })))
}

/// Returns the currently authenticated user.
pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let person = state
        .services
        .find_person_by_id(user.user_id)
        .await?
        .ok_or_else(AppError::unauthorized)?;

    Ok(Json(json!({
        "user": { "username": person.username, "display_name": person.display_name }
    })))
}
