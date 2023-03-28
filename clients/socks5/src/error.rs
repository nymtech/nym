use crate::socks::types::SocksProxyError;
use client_core::error::ClientCoreError;
use nym_socks5_requests::{ConnectionError, ConnectionId};

#[derive(thiserror::Error, Debug)]
pub enum Socks5ClientError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    #[error("SOCKS proxy error")]
    SocksProxyError(SocksProxyError),

    #[error("Failed to load config for: {0}")]
    FailedToLoadConfig(String),

    #[error("Failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("Fail to bind address")]
    FailToBindAddress,

    #[error("Network requester: connection id {connection_id}: {error}")]
    NetworkRequesterError {
        connection_id: ConnectionId,
        error: String,
    },
}

impl From<ConnectionError> for Socks5ClientError {
    fn from(value: ConnectionError) -> Self {
        Socks5ClientError::NetworkRequesterError {
            connection_id: value.connection_id,
            error: value.network_requester_error,
        }
    }
}
