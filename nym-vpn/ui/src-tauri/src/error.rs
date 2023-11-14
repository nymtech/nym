use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CommandError {
    #[error("internal error: `{0}`")]
    InternalError(String),
    #[error("caller error: `{0}`")]
    CallerError(String),
    #[error("unknown error")]
    Unknown,
}
