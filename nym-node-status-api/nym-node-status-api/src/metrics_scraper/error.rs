use nym_network_defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_node_requests::api::client::NymNodeApiClientError;
use nym_validator_client::client::NodeId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeScraperError {
    #[error("node {node_id} has provided malformed host information ({host}: {source}")]
    MalformedHost {
        host: String,
        node_id: NodeId,
        #[source]
        source: NymNodeApiClientError,
    },

    #[error("node {node_id} with host '{host}' doesn't seem to expose its declared http port nor any of the standard API ports, i.e.: 80, 443 or {}", DEFAULT_NYM_NODE_HTTP_PORT)]
    NoHttpPortsAvailable { host: String, node_id: NodeId },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_malformed_host_error() {
        // Create a generic error to test with
        let source_error = NymNodeApiClientError::GenericRequestFailure("Invalid URL".to_string());
        let error = NodeScraperError::MalformedHost {
            host: "invalid-host:abc".to_string(),
            node_id: 42,
            source: source_error,
        };

        // Test error message formatting
        let error_msg = error.to_string();
        assert!(error_msg.contains("node 42"));
        assert!(error_msg.contains("invalid-host:abc"));
        assert!(error_msg.contains("malformed host information"));

        // Test that source error is accessible
        assert!(error.source().is_some());
    }

    #[test]
    fn test_malformed_host_error_edge_cases() {
        // Test with empty host
        let error = NodeScraperError::MalformedHost {
            host: "".to_string(),
            node_id: 0,
            source: NymNodeApiClientError::NotFound,
        };

        let error_msg = error.to_string();
        assert!(error_msg.contains("node 0"));

        // Test with very long host
        let long_host = "x".repeat(1000);
        let error = NodeScraperError::MalformedHost {
            host: long_host.clone(),
            node_id: u32::MAX,
            source: NymNodeApiClientError::GenericRequestFailure("Too long".to_string()),
        };

        let error_msg = error.to_string();
        assert!(error_msg.contains(&format!("node {}", u32::MAX)));
        assert!(error_msg.contains(&long_host));
    }

    #[test]
    fn test_no_http_ports_available_error() {
        let error = NodeScraperError::NoHttpPortsAvailable {
            host: "example.com".to_string(),
            node_id: 123,
        };

        let error_msg = error.to_string();
        assert!(error_msg.contains("node 123"));
        assert!(error_msg.contains("example.com"));
        assert!(error_msg.contains("doesn't seem to expose its declared http port"));
        assert!(error_msg.contains("80, 443 or"));
        assert!(error_msg.contains(&DEFAULT_NYM_NODE_HTTP_PORT.to_string()));

        // This error type has no source
        assert!(error.source().is_none());
    }

    #[test]
    fn test_no_http_ports_special_characters() {
        // Test with host containing special characters
        let error = NodeScraperError::NoHttpPortsAvailable {
            host: "test-node_123.example.com:8080".to_string(),
            node_id: 999,
        };

        let error_msg = error.to_string();
        assert!(error_msg.contains("test-node_123.example.com:8080"));
    }

    #[test]
    fn test_error_different_sources() {
        // Test with different NymNodeApiClientError variants
        let not_found_error = NymNodeApiClientError::NotFound;
        let error1 = NodeScraperError::MalformedHost {
            host: "host1".to_string(),
            node_id: 1,
            source: not_found_error,
        };

        let generic_error = NymNodeApiClientError::GenericRequestFailure("404 error".to_string());
        let error2 = NodeScraperError::MalformedHost {
            host: "host2".to_string(),
            node_id: 2,
            source: generic_error,
        };

        // Both should format differently based on their source
        assert!(error1.to_string().contains("host1"));
        assert!(error2.to_string().contains("host2"));
    }

    #[test]
    fn test_error_trait_implementation() {
        // Test that NodeScraperError implements std::error::Error properly
        let error = NodeScraperError::NoHttpPortsAvailable {
            host: "test.com".to_string(),
            node_id: 42,
        };

        // Can be used as dyn Error
        let _error_ref: &dyn std::error::Error = &error;

        // Display trait is implemented
        let _display = format!("{error}");

        // Debug trait is implemented
        let _debug = format!("{error:?}");
    }
}
