use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::Value;

use crate::error::AppResult;
use crate::io::http::handlers::common;
use crate::state::AppState;

#[derive(Debug, Default, Deserialize)]
pub struct ThemeQuery {
    #[serde(default, alias = "u")]
    pub username: Option<String>,
}

pub async fn get_theme(
    State(state): State<AppState>,
    Query(query): Query<ThemeQuery>,
) -> AppResult<Json<Value>> {
    let person = match query.username {
        Some(username) if !username.trim().is_empty() => {
            let username = username.trim().trim_start_matches('@');
            common::user_by_username(&state, username).await?
        }
        _ => common::primary_user(&state).await?,
    };
    let theme = common::active_theme_config(&state, person.id).await?;
    Ok(Json(theme))
}
