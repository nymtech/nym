use nym_client_core::error::ClientCoreError;
use nym_credential_storage::error::StorageError;
use time::OffsetDateTime;

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    ClientCoreError(#[from] ClientCoreError),

    #[error("Failed to load config for: {0}")]
    FailedToLoadConfig(String),

    // TODO: add more details here
    #[error("Failed to validate the loaded config")]
    ConfigValidationFailure,

    #[error("Failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("Attempted to start the client in invalid socket mode")]
    InvalidSocketMode,

    #[error("failed to store credential: {source}")]
    CredentialStorageFailure {
        #[from]
        source: StorageError,
    },

    #[error(
        "failed to deserialize provided credential using revision {storage_revision}: {source}"
    )]
    CredentialDeserializationFailure {
        storage_revision: u8,
        #[source]
        source: nym_credentials::error::Error,
    },

    #[error("attempted to import an expired credential (it expired on {expiration})")]
    ExpiredCredentialImport { expiration: OffsetDateTime },
}
