#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("key file encountered that we don't want to overwrite")]
    DontOverwrite,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
