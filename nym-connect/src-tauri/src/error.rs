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
}

impl Serialize for BackendError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}
