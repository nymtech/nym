use nymvpn_migration::DbErr;

pub mod handler;
pub mod init;
pub mod name;
pub mod storage;

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("device service is unavailable")]
    DeviceServiceUnavailable,
    #[error("server error: {0}")]
    Server(#[from] tonic::Status),
    #[error("error connecting to server: {0}")]
    Connection(#[from] tonic::transport::Error),
    #[error("db error: {0}")]
    DbErr(#[from] DbErr),
    #[error("failed to initialize device: {0}")]
    InitError(String),
}
