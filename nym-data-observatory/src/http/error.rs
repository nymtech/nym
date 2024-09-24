pub(crate) type HttpResult<T> = Result<T, Error>;

pub(crate) struct Error {
    message: String,
    status: axum::http::StatusCode,
}

impl Error {
    pub(crate) fn not_found(message: String) -> Self {
        Self {
            message,
            status: axum::http::StatusCode::NOT_FOUND,
        }
    }

    pub(crate) fn internal() -> Self {
        Self {
            message: String::from("Internal server error"),
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
