use client_core::error::ClientCoreError;
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
    #[error("string formatting error: {source}")]
    FmtError {
        #[from]
        source: std::fmt::Error,
    },
    #[error("tauri error: {source}")]
    TauriError {
        #[from]
        source: tauri::Error,
    },
    #[error("{source}")]
    TauriApiError {
        #[from]
        source: tauri::api::Error,
    },
    #[error("{source}")]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
    },
    #[error("{source}")]
    ClientCoreError {
        #[from]
        source: ClientCoreError,
    },
    #[error("{source}")]
    ApiClientError {
        #[from]
        source: crate::operations::growth::api_client::ApiClientError,
    },

    #[error("could not send disconnect signal to the SOCKS5 client")]
    CoundNotSendDisconnectSignal,
    #[error("no service provider set")]
    NoServiceProviderSet,
    #[error("no gateway provider set")]
    NoGatewaySet,
    #[error("initialization failed with a panic")]
    InitializationPanic,
    #[error("could not get config id before gateway is set")]
    CouldNotGetIdWithoutGateway,
    #[error("could not initialize without gateway set")]
    CouldNotInitWithoutGateway,
    #[error("could not initialize without service provider set")]
    CouldNotInitWithoutServiceProvider,
    #[error("could not get file name")]
    CouldNotGetFilename,
    #[error("could not get config file location")]
    CouldNotGetConfigFilename,
    #[error("could not load existing gateway configuration")]
    CouldNotLoadExistingGatewayConfiguration(std::io::Error),
    #[error("unable to open a new window")]
    NewWindowError,
    #[error("unable to parse the specified gateway")]
    UnableToParseGateway,

    #[error("HTTP get request failed: {status_code}")]
    RequestFail {
        url: reqwest::Url,
        status_code: reqwest::StatusCode,
    },
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
