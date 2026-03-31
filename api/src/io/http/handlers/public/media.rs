use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Response};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub async fn get_upload(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> AppResult<Response<Body>> {
    let object = state
        .services
        .media
        .load(&key)
        .await?
        .ok_or_else(|| AppError::not_found("upload not found"))?;
    let content_type = HeaderValue::from_str(&object.content_type)
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))?;

    Response::builder()
        .header(CONTENT_TYPE, content_type)
        .header(CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header("x-content-type-options", "nosniff")
        .body(Body::from(object.bytes))
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))
}
