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

    pub(crate) fn internal() -> Self {
        Self {
            message: String::from("Internal"),
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
