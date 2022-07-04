use serde::{Serialize, Serializer};
use thiserror::Error;

#[allow(unused)]
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("State error")]
    StateError,
    #[error("Could not connect")]
    CouldNotConnect,
    #[error("Could not disconnect")]
    CouldNotDisconnect,
    #[error("No service provider set")]
    NoServiceProviderSet,
    #[error("No gateway provider set")]
    NoGatewaySet,
    #[error("{source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error("Initialization failed with a panic")]
    InitializationPanic,
}

impl Serialize for BackendError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}
