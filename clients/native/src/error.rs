use client_core::error::ClientCoreError;

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
