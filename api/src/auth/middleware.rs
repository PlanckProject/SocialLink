use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;

use crate::auth::jwt;
use crate::domain::EntityId;
use crate::error::AppError;
use crate::state::AppState;

pub const AUTH_COOKIE: &str = "sl_auth";

/// Authenticated caller, extracted from the signed JWT auth cookie. Any handler
/// that takes `AuthUser` is automatically protected.
pub struct AuthUser {
    pub user_id: EntityId,
    pub username: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::unauthorized())?;

        let token = jar
            .get(AUTH_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or_else(AppError::unauthorized)?;

        let claims = jwt::verify_token(&state.config.auth.jwt_secret, &token)
            .map_err(|_| AppError::unauthorized())?;

        let user_id = EntityId::parse(&claims.sub).map_err(|_| AppError::unauthorized())?;
        let person = state
            .services
            .find_person_by_id(user_id)
            .await?
            .ok_or_else(AppError::unauthorized)?;

        Ok(AuthUser {
            user_id,
            username: person.username,
        })
    }
}
