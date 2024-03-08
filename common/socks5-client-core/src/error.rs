use crate::socks::types::SocksProxyError;
use nym_client_core::error::ClientCoreError;
use nym_socks5_requests::{ConnectionError, ConnectionId};
use nym_task::manager::TaskStatusTrait;

#[derive(thiserror::Error, Debug)]
pub enum Socks5ClientCoreError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("SOCKS proxy error")]
    SocksProxyError(SocksProxyError),

    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    #[error("Network requester: connection id {connection_id}: {error}")]
    NetworkRequesterError {
        connection_id: ConnectionId,
        error: String,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum Socks5ClientCoreStatusMessage {
    #[error(transparent)]
    Socks5Error(#[from] Socks5ClientCoreError),
}

// impl TaskStatusTrait for Socks5ClientCoreStatusMessage {}

impl From<ConnectionError> for Socks5ClientCoreError {
    fn from(value: ConnectionError) -> Self {
        Socks5ClientCoreError::NetworkRequesterError {
            connection_id: value.connection_id,
            error: value.network_requester_error,
        }
    }
}
