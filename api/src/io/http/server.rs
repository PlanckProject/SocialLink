use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::routing::{get, post, put};
use axum::{Json, Router, middleware};
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::io::http::handlers::{admin, public};
use crate::io::http::middleware::request::correlate_and_envelope;
use crate::io::http::response::REQUEST_ID_HEADER;
use crate::state::AppState;

/// Builds the full Axum application. Note: the Nuxt/Nitro proxy strips the
/// browser's `/api` prefix, so routes here are declared without it.
pub fn build_router(state: AppState) -> Router {
    let max_bytes = state.config.uploads.max_mb * 1024 * 1024;
    let cors = build_cors(&state);
    let upload_route = format!("{}/{{*key}}", state.config.storage.route_prefix);

    let admin_routes = Router::new()
        .route(
            "/profile",
            get(admin::profile::get_profile).put(admin::profile::update_profile),
        )
        .route("/password", put(admin::profile::change_password))
        .route("/preview", get(admin::profile::preview_profile))
        .route(
            "/profile/{kind}",
            post(admin::profile::upload_image).delete(admin::profile::delete_image),
        )
        .route("/uploads", post(admin::profile::upload_asset))
        .route("/uploads/favicon", post(admin::profile::upload_favicon))
        .route(
            "/groups",
            get(admin::groups::list_groups).post(admin::groups::create_group),
        )
        .route("/groups/reorder", post(admin::groups::reorder_groups))
        .route(
            "/groups/{id}",
            put(admin::groups::update_group).delete(admin::groups::delete_group),
        )
        .route(
            "/links",
            get(admin::links::list_links).post(admin::links::create_link),
        )
        .route("/links/reorder", post(admin::links::reorder_links))
        .route("/links/{id}/preview", get(admin::links::preview_link))
        .route(
            "/links/{id}",
            put(admin::links::update_link).delete(admin::links::delete_link),
        )
        .route(
            "/themes",
            get(admin::themes::list_themes).post(admin::themes::create_theme),
        )
        .route("/themes/import", post(admin::themes::import_theme_library))
        .route(
            "/themes/{id}",
            get(admin::themes::get_theme_one)
                .put(admin::themes::update_theme)
                .delete(admin::themes::delete_theme),
        )
        .route("/themes/{id}/activate", post(admin::themes::activate_theme))
        .route("/presets/apply", post(admin::themes::apply_preset))
        .route(
            "/themes/{id}/favorite",
            post(admin::themes::toggle_favorite),
        )
        .route("/themes/{id}/export", get(admin::themes::export_theme_one))
        .route(
            "/theme",
            get(admin::themes::get_active_theme).put(admin::themes::update_active_theme),
        )
        .route("/theme/export", get(admin::themes::export_theme))
        .route("/theme/import", post(admin::themes::import_theme))
        .route("/analytics/overview", get(admin::analytics::overview))
        .route("/analytics/links", get(admin::analytics::links));

    Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/config", get(public::config::get_config))
        .route("/theme", get(public::theme::get_theme))
        .route("/profile", get(public::profile::get_profile))
        .route(
            "/u/{username}",
            get(public::profile::get_profile_by_username),
        )
        .route("/username-available", get(public::profile::check_username))
        .route("/l/{id}", get(public::redirect::redirect_link))
        .route("/auth/login", post(public::login::login))
        .route("/auth/register", post(public::signup::register))
        .route("/auth/logout", post(public::session::logout))
        .route("/auth/me", get(public::session::me))
        .route(&upload_route, get(public::media::get_upload))
        .nest("/admin", admin_routes)
        .layer(DefaultBodyLimit::max(max_bytes))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(correlate_and_envelope))
        .with_state(state)
}

fn build_cors(state: &AppState) -> CorsLayer {
    let origins: Vec<HeaderValue> = state
        .config
        .server
        .cors_origins
        .iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION])
        .expose_headers([HeaderName::from_static(REQUEST_ID_HEADER)])
}
