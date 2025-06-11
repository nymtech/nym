use nym_mixnet_contract_common::NodeId;
use std::fmt::Display;

pub(crate) type HttpResult<T> = Result<T, HttpError>;

pub(crate) struct HttpError {
    message: String,
    status: axum::http::StatusCode,
}

impl HttpError {
    pub(crate) fn invalid_input(msg: impl Display) -> Self {
        Self {
            message: msg.to_string(),
            status: axum::http::StatusCode::BAD_REQUEST,
        }
    }

    pub(crate) fn unauthorized() -> Self {
        Self {
            message: String::from("Make sure your public key is registered with NS API"),
            status: axum::http::StatusCode::UNAUTHORIZED,
        }
    }

    pub(crate) fn internal_with_logging(msg: impl Display) -> Self {
        tracing::error!("{}", msg.to_string());
        Self::internal()
    }

    pub(crate) fn internal() -> Self {
        Self {
            message: String::from("Internal server error"),
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn no_testruns_available() -> Self {
        Self {
            message: String::from("No testruns available"),
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub(crate) fn no_delegations_for_node(node_id: NodeId) -> Self {
        Self {
            message: format!("No delegation data for node_id={}", node_id),
            status: axum::http::StatusCode::NOT_FOUND,
        }
    }
}

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
