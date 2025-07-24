use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use time::OffsetDateTime;

use crate::http::state::AppState;

pub async fn add_response_headers(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Add timestamp header (RFC3339 format)
    let timestamp = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    if let Ok(value) = HeaderValue::from_str(&timestamp) {
        headers.insert(HeaderName::from_static("x-response-timestamp"), value);
    }

    // Add instance ID header
    if let Ok(value) = HeaderValue::from_str(state.instance_id()) {
        headers.insert(HeaderName::from_static("x-instance-id"), value);
    }

    response
}
