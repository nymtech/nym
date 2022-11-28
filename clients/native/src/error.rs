use client_core::{client::reply_key_storage::ReplyKeyStorageError, error::ClientCoreError};
use crypto::asymmetric::identity::Ed25519RecoveryError;
use gateway_client::error::GatewayClientError;
use validator_client::ValidatorClientError;

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    #[error("Failed to load config for: {0}")]
    FailedToLoadConfig(String),
    #[error("Failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("Attempted to start the client in invalid socket mode")]
    InvalidSocketMode,
}
