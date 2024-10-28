use std::fmt::Display;

pub(crate) type HttpResult<T> = Result<T, HttpError>;

pub(crate) struct HttpError {
    message: String,
    status: axum::http::StatusCode,
}

impl HttpError {
    pub(crate) fn invalid_input(message: String) -> Self {
        Self {
            message,
            status: axum::http::StatusCode::BAD_REQUEST,
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

    pub(crate) fn not_found(msg: impl Display) -> Self {
        Self {
            message: serde_json::json!({"message": msg.to_string()}).to_string(),
            status: axum::http::StatusCode::NOT_FOUND,
        }
    }
}

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
