pub(crate) type HttpResult<T> = Result<T, Error>;

pub(crate) struct Error {
    message: String,
    status: axum::http::StatusCode,
}

impl Error {
    pub(crate) fn unknown_gateway(message: String) -> Self {
        Self {
            message,
            status: axum::http::StatusCode::BAD_REQUEST,
        }
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
