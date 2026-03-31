use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Redirect;

use crate::domain::{EntityId, EventKind};
use crate::error::{AppError, AppResult};
use crate::io::http::handlers::common;
use crate::state::AppState;
use crate::util::req_meta;

pub async fn redirect_link(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Redirect> {
    let id = EntityId::parse(&id).map_err(|_| AppError::bad_request("invalid link id"))?;
    let link = state.services.link_for_redirect(id).await?;

    let meta = req_meta(&headers);
    let event = common::analytics_event(
        link.user_id,
        Some(link.id),
        EventKind::Click,
        &meta,
        &state.config.auth.ip_hash_salt,
    );
    state.services.record_event_best_effort(event).await;

    Ok(Redirect::to(&link.url))
}
