use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use serde_json::{Value, json};
use tracing::Instrument;

use crate::io::http::response::{ApiResponse, REQUEST_ID_HEADER, RequestId, ResponseMessage};

const MAX_JSON_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

pub async fn correlate_and_envelope(mut request: Request, next: Next) -> Response {
    let request_id = RequestId::new();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    request.extensions_mut().insert(request_id.clone());

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id.as_str(),
        method = %method,
        path = %path,
    );
    let response = next.run(request).instrument(span).await;
    finalize_response(response, request_id).await
}

async fn finalize_response(response: Response, request_id: RequestId) -> Response {
    let is_json = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    let is_attachment = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.trim_start().starts_with("attachment"));

    let should_envelope = !is_attachment
        && (is_json || response.status().is_client_error() || response.status().is_server_error());
    let mut response = if should_envelope {
        envelope_response(response, &request_id, is_json).await
    } else {
        response
    };

    response.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(request_id.as_str()).expect("UUID request IDs are valid headers"),
    );
    response
}

async fn envelope_response(
    response: Response,
    request_id: &RequestId,
    original_is_json: bool,
) -> Response {
    let status = response.status();
    let message_override = response.extensions().get::<ResponseMessage>().cloned();
    let (mut parts, body) = response.into_parts();

    let body = match to_bytes(body, MAX_JSON_RESPONSE_BYTES).await {
        Ok(body) => body,
        Err(error) => {
            tracing::error!(%error, "failed to read JSON response body");
            return envelope_failure(parts, request_id);
        }
    };

    let original = if original_is_json {
        match serde_json::from_slice::<Value>(&body) {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(%error, "failed to parse JSON response body");
                return envelope_failure(parts, request_id);
            }
        }
    } else {
        Value::Null
    };

    let success = status.is_success();
    let message = message_override
        .map(|value| value.0)
        .or_else(|| {
            (!success).then(|| {
                if original_is_json {
                    error_message(&original)
                } else {
                    text_error_message(&body, status)
                }
            })
        })
        .unwrap_or_else(|| "success".to_string());
    let data = if success { original } else { Value::Null };
    let envelope = ApiResponse {
        request_id: request_id.as_str().to_string(),
        status: status.as_u16(),
        message,
        success,
        data,
    };

    let bytes = match serde_json::to_vec(&envelope) {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::error!(%error, "failed to serialize JSON response envelope");
            return envelope_failure(parts, request_id);
        }
    };

    parts.headers.remove(CONTENT_LENGTH);
    parts
        .headers
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Response::from_parts(parts, Body::from(bytes))
}

fn envelope_failure(mut parts: axum::http::response::Parts, request_id: &RequestId) -> Response {
    parts.status = StatusCode::INTERNAL_SERVER_ERROR;
    parts.headers.remove(CONTENT_LENGTH);
    parts
        .headers
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let body = json!({
        "request_id": request_id.as_str(),
        "status": StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        "message": "internal server error",
        "success": false,
        "data": Value::Null,
    });
    Response::from_parts(parts, Body::from(body.to_string()))
}

fn error_message(value: &Value) -> String {
    value
        .get("message")
        .or_else(|| value.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("request failed")
        .to_string()
}

fn text_error_message(body: &[u8], status: StatusCode) -> String {
    std::str::from_utf8(body)
        .ok()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_string)
        .or_else(|| status.canonical_reason().map(str::to_ascii_lowercase))
        .unwrap_or_else(|| "request failed".to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::header::{CONTENT_DISPOSITION, LOCATION};

    use super::*;

    #[tokio::test]
    async fn wraps_json_and_matches_header_request_id() {
        let request_id = RequestId::new();
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"value":42}"#))
            .expect("response");

        let response = finalize_response(response, request_id.clone()).await;
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER),
            Some(
                &HeaderValue::from_str(request_id.as_str()).expect("request ID is a valid header")
            )
        );
        let body = to_bytes(response.into_body(), MAX_JSON_RESPONSE_BYTES)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("JSON");
        assert_eq!(body["request_id"], request_id.as_str());
        assert_eq!(body["status"], 200);
        assert_eq!(body["message"], "success");
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["value"], 42);
        assert_eq!(body.as_object().expect("object").len(), 5);
    }

    #[tokio::test]
    async fn preserves_redirect_body_semantics() {
        let request_id = RequestId::new();
        let response = Response::builder()
            .status(StatusCode::TEMPORARY_REDIRECT)
            .header(LOCATION, "https://example.com")
            .body(Body::empty())
            .expect("response");

        let response = finalize_response(response, request_id.clone()).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            response.headers().get(LOCATION),
            Some(&HeaderValue::from_static("https://example.com"))
        );
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER),
            Some(
                &HeaderValue::from_str(request_id.as_str()).expect("request ID is a valid header")
            )
        );
        assert!(
            to_bytes(response.into_body(), MAX_JSON_RESPONSE_BYTES)
                .await
                .expect("body")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn preserves_binary_response_body() {
        let request_id = RequestId::new();
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "image/png")
            .body(Body::from("png-bytes"))
            .expect("response");

        let response = finalize_response(response, request_id.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER),
            Some(
                &HeaderValue::from_str(request_id.as_str()).expect("request ID is a valid header")
            )
        );
        let body = to_bytes(response.into_body(), MAX_JSON_RESPONSE_BYTES)
            .await
            .expect("body");
        assert_eq!(&body[..], b"png-bytes");
    }

    #[tokio::test]
    async fn preserves_json_attachments_without_enveloping() {
        let request_id = RequestId::new();
        let jsonc = b"// exported theme\n{\"name\":\"test\"}\n";
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_DISPOSITION, "attachment; filename=\"theme.jsonc\"")
            .body(Body::from(&jsonc[..]))
            .expect("response");

        let response = finalize_response(response, request_id.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER),
            Some(
                &HeaderValue::from_str(request_id.as_str()).expect("request ID is a valid header")
            )
        );
        let body = to_bytes(response.into_body(), MAX_JSON_RESPONSE_BYTES)
            .await
            .expect("body");
        assert_eq!(&body[..], jsonc);
    }

    #[tokio::test]
    async fn converts_plain_router_errors_to_json_envelopes() {
        let request_id = RequestId::new();
        let response = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(CONTENT_TYPE, "text/plain")
            .body(Body::from("route not found"))
            .expect("response");

        let response = finalize_response(response, request_id.clone()).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(CONTENT_TYPE),
            Some(&HeaderValue::from_static("application/json"))
        );
        let body = to_bytes(response.into_body(), MAX_JSON_RESPONSE_BYTES)
            .await
            .expect("body");
        let body: Value = serde_json::from_slice(&body).expect("JSON");
        assert_eq!(body["request_id"], request_id.as_str());
        assert_eq!(body["status"], 404);
        assert_eq!(body["message"], "route not found");
        assert_eq!(body["success"], false);
        assert!(body["data"].is_null());
    }
}
