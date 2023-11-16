use tonic::Status;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("daemon is offline")]
    DaemonUnavailable,
    #[error("{}", .0.message())]
    Grpc(#[from] Status),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    InvalidArgument(String),
}
