use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::header;
use axum::response::Response;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::domain::{Theme, ThemeSource, ThemeUpdate};
use crate::error::{AppError, AppResult};
use crate::services::themes::ThemeDownload;
use crate::state::AppState;
use crate::util::{parse_entity_id, utc_to_iso};

#[derive(Serialize)]
pub struct ThemeResponse {
    id: String,
    name: String,
    owner: Option<String>,
    is_active: bool,
    is_favorite: bool,
    is_public: bool,
    description: Option<String>,
    tags: Vec<String>,
    source: &'static str,
    download_count: i64,
    swatch: ThemeSwatch,
    created_at: String,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<Value>,
}

#[derive(Serialize)]
pub struct ThemeSwatch {
    background: String,
    primary: String,
    accent: String,
}

impl ThemeResponse {
    fn from_model(theme: &Theme) -> Self {
        Self {
            id: theme.id.to_string(),
            name: theme.name.clone(),
            owner: theme.owner.clone(),
            is_active: theme.is_active,
            is_favorite: theme.is_favorite,
            is_public: theme.is_public,
            description: theme.description.clone(),
            tags: theme.tags.clone(),
            source: source_name(theme.source),
            download_count: theme.download_count,
            swatch: swatch_of(&theme.config),
            created_at: utc_to_iso(&theme.created_at),
            updated_at: utc_to_iso(&theme.updated_at),
            config: None,
        }
    }

    fn detail_from_model(theme: &Theme) -> Self {
        let mut response = Self::from_model(theme);
        response.config = Some(theme.config.clone());
        response
    }
}

fn source_name(source: ThemeSource) -> &'static str {
    match source {
        ThemeSource::Custom => "custom",
        ThemeSource::Imported => "imported",
        ThemeSource::Preset => "preset",
        ThemeSource::Marketplace => "marketplace",
    }
}

fn swatch_of(config: &Value) -> ThemeSwatch {
    let at = |pointer: &str| {
        config
            .pointer(pointer)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let background = if at("/background/type") == "gradient" {
        let gradient = at("/background/gradient");
        if gradient.is_empty() {
            at("/background/value")
        } else {
            gradient
        }
    } else {
        at("/background/value")
    };
    ThemeSwatch {
        background,
        primary: at("/colors/primary"),
        accent: at("/colors/accent"),
    }
}

#[derive(Deserialize)]
pub struct ListThemesQuery {
    #[serde(default)]
    pub favorite: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

pub async fn list_themes(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListThemesQuery>,
) -> AppResult<Json<Vec<ThemeResponse>>> {
    let favorite = match query.favorite.as_deref() {
        Some("true") | Some("1") => Some(true),
        Some("false") | Some("0") => Some(false),
        _ => None,
    };
    let source = query.source.filter(|source| !source.is_empty());
    let themes = state
        .services
        .list_themes(user.user_id, favorite, source.as_deref())
        .await?;
    Ok(Json(themes.iter().map(ThemeResponse::from_model).collect()))
}

#[derive(Deserialize)]
pub struct CreateThemePayload {
    #[serde(default)]
    pub name: Option<String>,
    pub config: Value,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub activate: bool,
}

pub async fn create_theme(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateThemePayload>,
) -> AppResult<Json<ThemeResponse>> {
    let theme = state
        .services
        .create_theme(
            user.user_id,
            &user.username,
            payload.name,
            payload.config,
            payload.description,
            payload.tags,
            payload.activate,
            state.config.themes.max_custom_per_user,
        )
        .await?;
    Ok(Json(ThemeResponse::from_model(&theme)))
}

pub async fn get_theme_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<ThemeResponse>> {
    let theme = state
        .services
        .get_theme(parse_entity_id(&id)?, user.user_id)
        .await?;
    Ok(Json(ThemeResponse::detail_from_model(&theme)))
}

pub async fn delete_theme(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    state
        .services
        .delete_theme(parse_entity_id(&id)?, user.user_id)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn activate_theme(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let config = state
        .services
        .activate_theme(parse_entity_id(&id)?, user.user_id)
        .await?;
    Ok(Json(config))
}

#[derive(Deserialize)]
pub struct UpdateActivePayload {
    pub config: Value,
}

pub async fn get_active_theme(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Value>> {
    Ok(Json(
        state.services.active_theme_config(user.user_id).await?,
    ))
}

pub async fn apply_preset(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateThemePayload>,
) -> AppResult<Json<Value>> {
    let config = state
        .services
        .apply_preset(
            user.user_id,
            &user.username,
            payload.name,
            payload.config,
            state.config.themes.max_preset_per_user,
        )
        .await?;
    Ok(Json(config))
}

pub async fn update_active_theme(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<UpdateActivePayload>,
) -> AppResult<Json<Value>> {
    let config = state
        .services
        .update_active_theme(
            user.user_id,
            &user.username,
            payload.config,
            state.config.themes.max_custom_per_user,
        )
        .await?;
    Ok(Json(config))
}

pub async fn export_theme(State(state): State<AppState>, user: AuthUser) -> AppResult<Response> {
    let download = state
        .services
        .download_active_theme(user.user_id, &user.username)
        .await?;
    download_response(download)
}

#[derive(Deserialize)]
pub struct UpdateThemePayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub config: Option<Value>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_public: Option<bool>,
}

pub async fn update_theme(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateThemePayload>,
) -> AppResult<Json<ThemeResponse>> {
    let theme = state
        .services
        .update_theme(
            parse_entity_id(&id)?,
            user.user_id,
            ThemeUpdate {
                name: payload.name,
                description: payload.description,
                tags: payload.tags,
                is_public: payload.is_public,
                config: payload.config,
                ..ThemeUpdate::default()
            },
        )
        .await?;
    Ok(Json(ThemeResponse::from_model(&theme)))
}

pub async fn toggle_favorite(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<ThemeResponse>> {
    let theme = state
        .services
        .toggle_theme_favorite(parse_entity_id(&id)?, user.user_id)
        .await?;
    Ok(Json(ThemeResponse::from_model(&theme)))
}

pub async fn export_theme_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let download = state
        .services
        .download_theme(parse_entity_id(&id)?, user.user_id, &user.username)
        .await?;
    download_response(download)
}

pub async fn import_theme(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<ThemeResponse>> {
    let bytes = multipart_file(&mut multipart, false).await?.0;
    let theme = state
        .services
        .import_theme(
            user.user_id,
            &user.username,
            &bytes,
            true,
            state.config.themes.max_custom_per_user,
        )
        .await?;
    Ok(Json(ThemeResponse::from_model(&theme)))
}

pub async fn import_theme_library(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<ThemeResponse>> {
    let (bytes, activate) = multipart_file(&mut multipart, true).await?;
    let theme = state
        .services
        .import_theme(
            user.user_id,
            &user.username,
            &bytes,
            activate,
            state.config.themes.max_custom_per_user,
        )
        .await?;
    Ok(Json(ThemeResponse::from_model(&theme)))
}

async fn multipart_file(
    multipart: &mut Multipart,
    read_activate: bool,
) -> AppResult<(Vec<u8>, bool)> {
    let mut bytes = None;
    let mut activate = false;
    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("file") => bytes = Some(field.bytes().await?.to_vec()),
            Some("activate") if read_activate => {
                activate = matches!(field.text().await?.trim(), "true" | "1");
            }
            _ => {}
        }
    }
    let bytes = bytes.ok_or_else(|| AppError::bad_request("missing file field"))?;
    Ok((bytes, activate))
}

fn download_response(download: ThemeDownload) -> AppResult<Response> {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", download.filename),
        )
        .body(Body::from(download.bytes))
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))
}
