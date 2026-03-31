use axum::Json;
use axum::extract::State;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::jwt;
use crate::error::{AppError, AppResult};
use crate::io::http::handlers::common;
use crate::services::people::RegisterPerson;
use crate::state::AppState;
use crate::util::default_theme_value;

#[derive(Deserialize)]
pub struct RegisterPayload {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<RegisterPayload>,
) -> AppResult<(CookieJar, Json<Value>)> {
    if !state.config.application.mode.is_multi() {
        return Err(AppError::forbidden());
    }

    let person = state
        .services
        .register_person(
            RegisterPerson {
                username: payload.username,
                email: payload.email,
                password: payload.password,
                display_name: payload.display_name,
            },
            default_theme_value(),
        )
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
