use serde::{Serialize, Serializer};
use thiserror::Error;

#[allow(unused)]
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("{source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    #[error("String formatting error: {source}")]
    FmtError {
        #[from]
        source: std::fmt::Error,
    },
    #[error("Tauri error: {source}")]
    TauriError {
        #[from]
        source: tauri::Error,
    },
    #[error("{source}")]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
    },

    #[error("State error")]
    StateError,
    #[error("Could not connect")]
    CouldNotConnect,
    #[error("Could not disconnect")]
    CouldNotDisconnect,
    #[error("Could not send disconnect signal to the SOCKS5 client")]
    CoundNotSendDisconnectSignal,
    #[error("No service provider set")]
    NoServiceProviderSet,
    #[error("No gateway provider set")]
    NoGatewaySet,
    #[error("Initialization failed with a panic")]
    InitializationPanic,
    #[error("Could not get config id before gateway is set")]
    CouldNotGetIdWithoutGateway,
    #[error("Could initialize without gateway set")]
    CouldNotInitWithoutGateway,
    #[error("Could initialize without service provider set")]
    CouldNotInitWithoutServiceProvider,
    #[error("Could not get file name")]
    CouldNotGetFilename,
}

impl Serialize for BackendError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

// Local crate level Result alias
pub(crate) type Result<T, E = BackendError> = std::result::Result<T, E>;
