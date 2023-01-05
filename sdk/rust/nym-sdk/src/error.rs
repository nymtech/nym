#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("toml serialization error: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
    #[error("toml deserialization error: {0}")]
    TomlDeserializationError(#[from] toml::de::Error),
    #[error(transparent)]
    ClientCoreError(#[from] client_core::error::ClientCoreError),
    #[error("key file encountered that we don't want to overwrite")]
    DontOverwrite,
    #[error("shared gateway key file encountered that we don't want to overwrite")]
    DontOverwriteGatewayKey,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
