use crate::socks::types::SocksProxyError;
use client_core::client::replies::reply_storage::fs_backend;
use client_core::error::ClientCoreError;
use socks5_requests::ConnectionId;

#[derive(thiserror::Error, Debug)]
pub enum Socks5ClientError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError<fs_backend::Backend>),

    #[error("SOCKS proxy error")]
    SocksProxyError(SocksProxyError),

    #[error("Failed to load config for: {0}")]
    FailedToLoadConfig(String),

    #[error("Failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("Fail to bind address")]
    FailToBindAddress,

    #[error("{connection_id}: {error}")]
    NetworkRequesterError {
        connection_id: ConnectionId,
        error: String,
    },
}
