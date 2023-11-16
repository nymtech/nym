use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "type")]
pub enum Error {
    #[error("daemon is offline")]
    DaemonIsOffline,
    #[error("Grpc code: {code}, message: {message}")]
    Grpc { code: u8, message: String },
}

impl From<tonic::Status> for Error {
    fn from(value: tonic::Status) -> Self {
        Error::Grpc {
            code: value.code() as u8,
            message: value.message().to_owned(),
        }
    }
}
