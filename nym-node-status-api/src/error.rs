use nym_explorer_client::ExplorerApiError;
use nym_validator_client::{nym_api::error::NymAPIError, ValidatorClientError};
use thiserror::Error;

pub(crate) type NodeStatusApiResult<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("Failed to start client: {0}")]
    ClientStartupError(String),
    #[error("Client connection error: {0}")]
    ClientConnectionError(String),
    #[error("DB error: {0}")]
    DatabaseError(String),
    #[error("Internal: {0}")]
    Internal(String),
}

impl From<ExplorerApiError> for Error {
    fn from(value: ExplorerApiError) -> Self {
        Self::ClientStartupError(value.to_string())
    }
}

impl From<ValidatorClientError> for Error {
    fn from(value: ValidatorClientError) -> Self {
        Self::ClientConnectionError(value.to_string())
    }
}

impl From<NymAPIError> for Error {
    fn from(value: NymAPIError) -> Self {
        Self::ClientConnectionError(value.to_string())
    }
}

impl From<sqlx::error::Error> for Error {
    fn from(value: sqlx::error::Error) -> Self {
        Self::DatabaseError(value.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value.to_string())
    }
}
