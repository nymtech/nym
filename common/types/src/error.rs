use serde::{Serialize, Serializer};
use std::io;
use thiserror::Error;
use validator_client::validator_api::error::ValidatorAPIError;
use validator_client::{nymd::error::NymdError, ValidatorClientError};

// TODO: ask @MS why this even exists
#[derive(Error, Debug)]
pub enum TypesError {
    #[error("{source}")]
    NymdError {
        #[from]
        source: NymdError,
    },
    #[error("{source}")]
    CosmwasmStd {
        #[from]
        source: cosmwasm_std::StdError,
    },
    #[error("{source}")]
    ErrorReport {
        #[from]
        source: eyre::Report,
    },
    #[error("{source}")]
    ValidatorApiError {
        #[from]
        source: ValidatorAPIError,
    },
    #[error("{source}")]
    IOError {
        #[from]
        source: io::Error,
    },
    #[error("{source}")]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
    },
    #[error("{source}")]
    MalformedUrlProvided {
        #[from]
        source: url::ParseError,
    },
    #[error("{source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error("{source}")]
    DecimalRangeExceeded {
        #[from]
        source: cosmwasm_std::DecimalRangeExceeded,
    },
    #[error("{0} is not a valid amount string")]
    InvalidAmount(String),
    #[error("{0} is not a valid denomination string")]
    InvalidDenom(String),
    #[error("Mixnode not found")]
    MixnodeNotFound(),
    #[error("Gateway bond is not valid")]
    InvalidGatewayBond(),
    #[error("Invalid delegations")]
    DelegationsInvalid,
    #[error("Attempted to use too huge currency exponent ({0})")]
    UnsupportedExponent(u32),
    #[error("Attempted to convert coin that would have resulted in loss of precision")]
    LossyCoinConversion,
    #[error("The provided coin has an unknown denomination - {0}")]
    UnknownCoinDenom(String),
}

impl Serialize for TypesError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl From<ValidatorClientError> for TypesError {
    fn from(e: ValidatorClientError) -> Self {
        match e {
            ValidatorClientError::ValidatorAPIError { source } => source.into(),
            ValidatorClientError::MalformedUrlProvided(e) => e.into(),
            ValidatorClientError::NymdError(e) => e.into(),
        }
    }
}
