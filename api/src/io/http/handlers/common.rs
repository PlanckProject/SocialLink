use axum_extra::extract::cookie::{Cookie, SameSite};
use serde_json::Value;
use time::Duration as TimeDuration;

use crate::auth::AUTH_COOKIE;
use crate::config::Config;
use crate::domain::{AnalyticsEvent, EntityId, EventKind, Person, RequestMetadata};
use crate::error::AppResult;
use crate::state::AppState;
use crate::util::{ReqMeta, hash_ip};

use super::dto::{ProfileDto, PublicGroupDto, PublicLinkDto, PublicProfileResponse, StatsDto};

pub async fn primary_user(state: &AppState) -> AppResult<Person> {
    Ok(state
        .services
        .primary_person(&state.config.admin.username)
        .await?)
}

pub async fn user_by_username(state: &AppState, username: &str) -> AppResult<Person> {
    Ok(state.services.person_by_username(username).await?)
}

pub async fn active_theme_config(state: &AppState, user_id: EntityId) -> AppResult<Value> {
    Ok(state.services.active_theme_config(user_id).await?)
}

pub fn build_auth_cookie(config: &Config, token: String) -> Cookie<'static> {
    Cookie::build((AUTH_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(config.auth.cookie_secure)
        .path("/")
        .max_age(TimeDuration::hours(config.auth.jwt_ttl_hours))
        .build()
}

pub fn clear_auth_cookie(config: &Config) -> Cookie<'static> {
    Cookie::build((AUTH_COOKIE, String::new()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(config.auth.cookie_secure)
        .path("/")
        .max_age(TimeDuration::seconds(0))
        .build()
}

pub fn analytics_event(
    user_id: EntityId,
    link_id: Option<EntityId>,
    kind: EventKind,
    meta: &ReqMeta,
    ip_hash_salt: &str,
) -> AnalyticsEvent {
    AnalyticsEvent::new(
        user_id,
        link_id,
        kind,
        RequestMetadata {
            referrer: meta.referrer.clone(),
            user_agent: meta.user_agent.clone(),
            ip_hash: Some(hash_ip(ip_hash_salt, &meta.ip)),
        },
    )
}

pub async fn log_event(
    state: &AppState,
    user_id: EntityId,
    link_id: Option<EntityId>,
    kind: &str,
    meta: &ReqMeta,
) {
    let kind = match kind {
        "click" => EventKind::Click,
        "view" => EventKind::View,
        other => {
            tracing::warn!(event_kind = other, "ignored unknown analytics event kind");
            return;
        }
    };
    let event = analytics_event(
        user_id,
        link_id,
        kind,
        meta,
        &state.config.auth.ip_hash_salt,
    );
    state.services.record_event_best_effort(event).await;
}

pub async fn build_public_profile(
    state: &AppState,
    person: &Person,
) -> AppResult<PublicProfileResponse> {
    let public = state.services.public_profile(person).await?;
    let click_totals = public.click_totals.as_ref();

    let groups = public
        .base
        .groups
        .iter()
        .map(|group| PublicGroupDto {
            id: group.group.id.to_string(),
            title: group.group.title.clone(),
            description: group.group.description.clone(),
            collapsible: group.group.is_collapsible,
            style: group.group.style.clone(),
            links: group
                .links
                .iter()
                .map(|link| {
                    PublicLinkDto::from_model(
                        link,
                        click_totals.map(|totals| totals.get(&link.id).copied().unwrap_or(0)),
                    )
                })
                .collect(),
        })
        .collect();
    let ungrouped = public
        .base
        .ungrouped
        .iter()
        .map(|link| {
            PublicLinkDto::from_model(
                link,
                click_totals.map(|totals| totals.get(&link.id).copied().unwrap_or(0)),
            )
        })
        .collect();

    Ok(PublicProfileResponse {
        profile: ProfileDto::from_model(&public.base.person),
        groups,
        ungrouped,
        stats: public.views.map(|views| StatsDto { views }),
        theme: public.base.theme,
    })
}
