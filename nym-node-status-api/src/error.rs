use thiserror::Error;

pub(crate) type NodeStatusApiResult<T> = Result<T, Error>;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("Failed to initialize service")]
    InitFailed,
}
