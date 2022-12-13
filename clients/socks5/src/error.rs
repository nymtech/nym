use client_core::error::ClientCoreError;

use crate::socks::types::SocksProxyError;

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
}
