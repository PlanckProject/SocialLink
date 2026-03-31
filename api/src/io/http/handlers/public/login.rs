use axum::Json;
use axum::extract::State;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::jwt;
use crate::error::AppResult;
use crate::io::http::handlers::common;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

/// Verifies credentials and issues the httpOnly JWT auth cookie.
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginPayload>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let person = state
        .services
        .authenticate_person(&payload.username, &payload.password)
        .await?;
    let token = jwt::create_token(
        &state.config.auth.jwt_secret,
        &person.id.to_string(),
        &person.username,
        state.config.auth.jwt_ttl_hours,
    )?;
    let jar = jar.add(common::build_auth_cookie(&state.config, token));

    Ok((
        jar,
        Json(json!({
            "user": { "username": person.username, "display_name": person.display_name }
        })),
    ))
}
