use std::fmt::Display;

pub(crate) type HttpResult<T> = Result<T, HttpError>;

pub(crate) struct HttpError {
    message: String,
    status: axum::http::StatusCode,
}

impl HttpError {
    pub(crate) fn invalid_input(msg: impl Display) -> Self {
        Self {
            message: serde_json::json!({"message": msg.to_string()}).to_string(),
            status: axum::http::StatusCode::BAD_REQUEST,
        }
    }

    pub(crate) fn unauthorized() -> Self {
        Self {
            message: serde_json::json!({"message": "Make sure your public key is registered with NS API"}).to_string(),
            status: axum::http::StatusCode::UNAUTHORIZED,
        }
    }

    pub(crate) fn internal_with_logging(msg: impl Display) -> Self {
        tracing::error!("{}", msg.to_string());
        Self::internal()
    }

    pub(crate) fn internal() -> Self {
        Self {
            message: serde_json::json!({"message": "Internal server error"}).to_string(),
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn no_testruns_available() -> Self {
        Self {
            message: serde_json::json!({"message": "No testruns available"}).to_string(),
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
