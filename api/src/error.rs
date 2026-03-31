use anyhow::Error;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("{0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
}

impl ErrorKind {
    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
        }
    }
}

#[derive(Debug)]
pub struct AppError(Error);

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:#}", self.0)
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self(ErrorKind::BadRequest(message.into()).into())
    }

    pub fn unauthorized() -> Self {
        Self(ErrorKind::Unauthorized.into())
    }

    pub fn forbidden() -> Self {
        Self(ErrorKind::Forbidden.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self(ErrorKind::NotFound(message.into()).into())
    }

    pub fn internal(error: impl Into<Error>) -> Self {
        Self(error.into())
    }

    pub fn into_inner(self) -> Error {
        self.0
    }

    fn response_parts(&self) -> (StatusCode, String) {
        let kind = self
            .0
            .chain()
            .find_map(|cause| cause.downcast_ref::<ErrorKind>());

        match kind {
            Some(kind) => (kind.status(), kind.to_string()),
            None => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
        }
    }
}

impl From<Error> for AppError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

macro_rules! app_error_from {
    ($($error:ty),+ $(,)?) => {
        $(
            impl From<$error> for AppError {
                fn from(error: $error) -> Self {
                    Self(Error::new(error))
                }
            }
        )+
    };
}

app_error_from!(
    jsonwebtoken::errors::Error,
    std::io::Error,
    axum::extract::multipart::MultipartError,
    serde_json::Error,
);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = self.response_parts();
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %format_args!("{:#}", self.0), "request failed");
        }
        (status, Json(json!({ "message": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;

    #[test]
    fn finds_status_marker_through_anyhow_context() {
        let error = anyhow::Result::<()>::Err(ErrorKind::NotFound("missing".into()).into())
            .context("load profile")
            .expect_err("error");
        let app_error = AppError::from(error);
        assert_eq!(
            app_error.response_parts(),
            (StatusCode::NOT_FOUND, "missing".to_string())
        );
    }

    #[test]
    fn unknown_errors_are_internal() {
        let app_error = AppError::from(anyhow::anyhow!("database failed"));
        assert_eq!(
            app_error.response_parts(),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string()
            )
        );
    }
}
