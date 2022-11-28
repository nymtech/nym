use client_core::{client::reply_key_storage::ReplyKeyStorageError, error::ClientCoreError};
use crypto::asymmetric::identity::Ed25519RecoveryError;
use gateway_client::error::GatewayClientError;
use validator_client::ValidatorClientError;

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
