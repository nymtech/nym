use std::path::PathBuf;

/// Top-level Error enum for the mixnet client and its relevant types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("toml serialization error: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
    #[error("toml deserialization error: {0}")]
    TomlDeserializationError(#[from] toml::de::Error),
    #[error("Ed25519 error: {0}")]
    Ed25519RecoveryError(#[from] crypto::asymmetric::identity::Ed25519RecoveryError),

    #[error(transparent)]
    ClientCoreError(#[from] client_core::error::ClientCoreError),

    #[error("key file encountered that we don't want to overwrite: {0}")]
    DontOverwrite(PathBuf),
    #[error("shared gateway key file encountered that we don't want to overwrite: {0}")]
    DontOverwriteGatewayKey(PathBuf),
    #[error("no gateway config available for writing")]
    GatewayNotAvailableForWriting,
    #[error("expected to received a directory, received: {0}")]
    ExpectedDirectory(PathBuf),

    #[error("failed to transition to registered state before connection to mixnet")]
    FailedToTransitionToRegisteredState,
    #[error(
        "registering with gateway when the client is already in a registered state is not \
         supported, and likely and user mistake"
    )]
    ReregisteringGatewayNotSupported,

}

pub type Result<T, E = Error> = std::result::Result<T, E>;
