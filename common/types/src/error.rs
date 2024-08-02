use nym_mixnet_contract_common::ContractsCommonError;
use nym_validator_client::error::TendermintRpcError;
use nym_validator_client::nym_api::error::NymAPIError;
use nym_validator_client::{nyxd::error::NyxdError, ValidatorClientError};
use serde::{Serialize, Serializer};
use std::io;
use thiserror::Error;

// TODO: ask @MS why this even exists
#[derive(Error, Debug)]
pub enum TypesError {
    #[error(transparent)]
    ContractsCommon(#[from] ContractsCommonError),

    #[error("{source}")]
    NyxdError {
        #[from]
        source: NyxdError,
    },
    #[error("{source}")]
    CosmwasmStd {
        #[from]
        source: cosmwasm_std::StdError,
    },
    #[error("{source}")]
    TendermintRpcError {
        #[from]
        source: TendermintRpcError,
    },
    #[error("{source}")]
    ErrorReport {
        #[from]
        source: eyre::Report,
    },
    #[error("{source}")]
    NymApiError {
        #[from]
        source: NymAPIError,
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
    #[error("No nym API URL configured")]
    NoNymApiUrlConfigured,
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
    #[error("Provided event is not a delegation event")]
    NotADelegationEvent,
    #[error("Unknown network - {0}")]
    UnknownNetwork(String),
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
            ValidatorClientError::NymAPIError { source } => source.into(),
            ValidatorClientError::MalformedUrlProvided(e) => e.into(),
            ValidatorClientError::NyxdError(e) => e.into(),
            ValidatorClientError::NoAPIUrlAvailable => TypesError::NoNymApiUrlConfigured,
            ValidatorClientError::TendermintErrorRpc(err) => err.into(),
        }
    }
}
