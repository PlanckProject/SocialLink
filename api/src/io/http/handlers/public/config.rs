use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::error::AppResult;
use crate::io::http::handlers::common;
use crate::state::AppState;

/// Public bootstrap payload the SSR frontend fetches on every request: the app
/// mode, feature flags and the active theme JSON (merged over static defaults).
pub async fn get_config(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let user = common::primary_user(&state).await?;
    let theme = common::active_theme_config(&state, user.id).await?;

    Ok(Json(json!({
        "mode": state.config.application.mode.as_str(),
        "features": { "registration_enabled": state.config.application.mode.is_multi() },
        "limits": {
            "max_preset_themes": state.config.themes.max_preset_per_user,
            "max_custom_themes": state.config.themes.max_custom_per_user,
        },
        "theme": theme,
    })))
}
