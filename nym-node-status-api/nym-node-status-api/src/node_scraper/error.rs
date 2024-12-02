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
