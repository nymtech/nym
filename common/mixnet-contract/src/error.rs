use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MixnetContractError {
    #[error("Could not convert ration to f64: {0} / {1}")]
    InvalidRatio(u128, u128)
}