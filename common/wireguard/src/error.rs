use thiserror::Error;

#[derive(Error, Debug)]
pub enum WgError {
    #[error("unable to get tunnel")]
    UnableToGetTunnel,
}
