use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymConfigTomlError {
    #[error(transparent)]
    FileIoFailure(#[from] io::Error),
    #[error(transparent)]
    TomlSerializeFailure(#[from] toml::ser::Error),
}
