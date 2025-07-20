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
            message: format!("No delegation data for node_id={node_id}"),
            status: axum::http::StatusCode::NOT_FOUND,
        }
    }
}

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use axum::http::StatusCode;
    
    #[test]
    fn test_invalid_input_error() {
        let error = HttpError::invalid_input("Invalid request data");
        assert_eq!(error.message, "Invalid request data");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        
        // Test with different input types
        let error2 = HttpError::invalid_input(42);
        assert_eq!(error2.message, "42");
        
        let error3 = HttpError::invalid_input(String::from("Dynamic string"));
        assert_eq!(error3.message, "Dynamic string");
    }
    
    #[test]
    fn test_unauthorized_error() {
        let error = HttpError::unauthorized();
        assert_eq!(error.message, "Make sure your public key is registered with NS API");
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
    }
    
    #[test]
    fn test_internal_error() {
        let error = HttpError::internal();
        assert_eq!(error.message, "Internal server error");
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    #[test]
    fn test_internal_with_logging() {
        // This would log to error but we can still test the result
        let error = HttpError::internal_with_logging("Database connection failed");
        assert_eq!(error.message, "Internal server error");
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    #[test]
    fn test_no_testruns_available() {
        let error = HttpError::no_testruns_available();
        assert_eq!(error.message, "No testruns available");
        assert_eq!(error.status, StatusCode::SERVICE_UNAVAILABLE);
    }
    
    #[test]
    fn test_no_delegations_for_node() {
        let node_id: NodeId = 42;
        let error = HttpError::no_delegations_for_node(node_id);
        assert_eq!(error.message, "No delegation data for node_id=42");
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        
        let node_id_2: NodeId = 999;
        let error2 = HttpError::no_delegations_for_node(node_id_2);
        assert_eq!(error2.message, "No delegation data for node_id=999");
    }
    
    #[test]
    fn test_into_response() {
        let error = HttpError::invalid_input("Test error");
        let response = error.into_response();
        
        // Extract status from response
        let status = response.status();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    
    #[test]
    fn test_different_error_types_into_response() {
        // Test each error type converts to response properly
        let errors = vec![
            HttpError::invalid_input("test"),
            HttpError::unauthorized(),
            HttpError::internal(),
            HttpError::no_testruns_available(),
            HttpError::no_delegations_for_node(1),
        ];
        
        let expected_statuses = vec![
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::NOT_FOUND,
        ];
        
        for (error, expected_status) in errors.into_iter().zip(expected_statuses) {
            let response = error.into_response();
            assert_eq!(response.status(), expected_status);
        }
    }
}
