use thiserror::Error;

#[derive(Error, Debug)]
pub enum KcpError {
    #[error("Invalid KCP command value: {0}")]
    InvalidCommand(u8),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
