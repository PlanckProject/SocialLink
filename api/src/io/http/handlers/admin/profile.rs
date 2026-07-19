use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum_extra::extract::CookieJar;
use bytes::Bytes;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::{AuthUser, jwt};
use crate::domain::{Branding, PersonImageSlot, PersonProfileUpdate, Social};
use crate::error::{AppError, AppResult};
use crate::io::http::handlers::common;
use crate::io::http::handlers::dto::{AdminProfileDto, PublicProfileResponse};
use crate::services::media::MediaKind;
use crate::state::AppState;

pub async fn get_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<AdminProfileDto>> {
    let person = state.services.person_by_id(user.user_id).await?;
    Ok(Json(AdminProfileDto::from_model(&person)))
}

pub async fn preview_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<PublicProfileResponse>> {
    let person = state.services.person_by_id(user.user_id).await?;
    Ok(Json(common::build_public_profile(&state, &person).await?))
}

#[derive(Deserialize)]
pub struct UpdateProfilePayload {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub socials: Option<Vec<Social>>,
    #[serde(default)]
    pub branding: Option<Branding>,
}

pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
    Json(payload): Json<UpdateProfilePayload>,
) -> AppResult<(CookieJar, Json<AdminProfileDto>)> {
    let person = state
        .services
        .update_person_profile(
            user.user_id,
            PersonProfileUpdate {
                username: payload.username,
                display_name: payload.display_name,
                bio: payload.bio,
                location: payload.location,
                socials: payload.socials,
                branding: payload.branding,
                ungrouped_position: None,
            },
        )
        .await?;

    let jar = if person.username != user.username {
        let token = jwt::create_token(
            &state.config.auth.jwt_secret,
            &person.id.to_string(),
            &person.username,
            state.config.auth.jwt_ttl_hours,
        )?;
        jar.add(common::build_auth_cookie(&state.config, token))
    } else {
        jar
    };

    Ok((jar, Json(AdminProfileDto::from_model(&person))))
}

#[derive(Deserialize)]
pub struct ChangePasswordPayload {
    pub current_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<ChangePasswordPayload>,
) -> AppResult<Json<Value>> {
    state
        .services
        .change_person_password(
            user.user_id,
            &payload.current_password,
            &payload.new_password,
        )
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn upload_image(
    State(state): State<AppState>,
    user: AuthUser,
    Path(kind): Path<String>,
    multipart: Multipart,
) -> AppResult<Json<AdminProfileDto>> {
    let slot = parse_image_slot(&kind)?;
    let upload = read_upload(multipart, max_upload_bytes(&state)?).await?;
    let person = state
        .services
        .store_person_image(
            user.user_id,
            slot,
            upload.bytes,
            upload.content_type.as_deref(),
            upload.file_name.as_deref(),
        )
        .await?;
    Ok(Json(AdminProfileDto::from_model(&person)))
}

pub async fn delete_image(
    State(state): State<AppState>,
    user: AuthUser,
    Path(kind): Path<String>,
) -> AppResult<Json<AdminProfileDto>> {
    let person = state
        .services
        .delete_person_image(user.user_id, parse_image_slot(&kind)?)
        .await?;
    Ok(Json(AdminProfileDto::from_model(&person)))
}

pub async fn upload_asset(
    State(state): State<AppState>,
    _user: AuthUser,
    multipart: Multipart,
) -> AppResult<Json<Value>> {
    let upload = read_upload(multipart, max_upload_bytes(&state)?).await?;
    let url = state
        .services
        .store_media(
            upload.bytes,
            upload.content_type.as_deref(),
            upload.file_name.as_deref(),
            MediaKind::LinkIcon,
        )
        .await?;
    Ok(Json(json!({ "url": url })))
}

pub async fn upload_favicon(
    State(state): State<AppState>,
    _user: AuthUser,
    multipart: Multipart,
) -> AppResult<Json<Value>> {
    let upload = read_upload(multipart, max_upload_bytes(&state)?).await?;
    let url = state
        .services
        .store_media(
            upload.bytes,
            upload.content_type.as_deref(),
            upload.file_name.as_deref(),
            MediaKind::Favicon,
        )
        .await?;
    Ok(Json(json!({ "url": url })))
}

struct Upload {
    bytes: Bytes,
    content_type: Option<String>,
    file_name: Option<String>,
}

async fn read_upload(mut multipart: Multipart, max_bytes: usize) -> AppResult<Upload> {
    while let Some(field) = multipart.next_field().await? {
        if field.name() != Some("file") {
            continue;
        }
        let file_name = field.file_name().map(str::to_string);
        let content_type = field.content_type().map(str::to_string);
        let bytes = field.bytes().await?;
        if bytes.len() > max_bytes {
            return Err(AppError::bad_request("file exceeds size limit"));
        }
        return Ok(Upload {
            bytes,
            content_type,
            file_name,
        });
    }
    Err(AppError::bad_request("missing file field"))
}

fn max_upload_bytes(state: &AppState) -> AppResult<usize> {
    state
        .config
        .uploads
        .max_mb
        .checked_mul(1024 * 1024)
        .ok_or_else(|| AppError::bad_request("configured upload limit is too large"))
}

fn parse_image_slot(kind: &str) -> AppResult<PersonImageSlot> {
    match kind {
        "avatar" => Ok(PersonImageSlot::Avatar),
        "cover" => Ok(PersonImageSlot::Cover),
        _ => Err(AppError::bad_request("invalid upload kind")),
    }
}
