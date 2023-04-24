use thiserror::Error;

#[derive(Debug, Error)]
pub enum MixFetchError {
    #[error("invalid Socks5 response")]
    InvalidSocks5Response,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
