use crate::validator_api;

use crate::nymd::cosmwasm_client::types::ContractCodeId;
use cosmos_sdk::tendermint::abci::Code;
use cosmos_sdk::tendermint::block;
use cosmos_sdk::{bip32, rpc, tx, AccountId};
use serde::Deserialize;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorClientError {
    #[error("There was an issue with the REST request - {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },
    #[error("There was an issue with the validator-api request - {source}")]
    ValidatorAPIError {
        #[from]
        source: validator_api::error::ValidatorAPIClientError,
    },
    #[error("An IO error has occured: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    #[error("There was an issue with the validator client - {0}")]
    ValidatorError(String),

    #[error("There was an issue with bip32 - {0}")]
    Bip32Error(bip32::Error),

    #[error("There was an issue with bip32 - {0}")]
    Bip39Error(bip39::Error),

    #[error("Failed to derive account address")]
    AccountDerivationError,

    #[error("Address {0} was not found in the wallet")]
    SigningAccountNotFound(AccountId),

    #[error("Failed to sign raw transaction")]
    SigningFailure,

    #[error("{0} is not a valid tx hash")]
    InvalidTxHash(String),

    #[error("There was an issue with a tendermint RPC request - {0}")]
    TendermintError(rpc::Error),

    #[error("There was an issue when attempting to serialize data")]
    SerializationError(String),

    #[error("There was an issue when attempting to deserialize data")]
    DeserializationError(String),

    #[error("There was an issue when attempting to encode our protobuf data - {0}")]
    ProtobufEncodingError(prost::EncodeError),

    #[error("There was an issue when attempting to decode our protobuf data - {0}")]
    ProtobufDecodingError(prost::DecodeError),

    #[error("Account {0} does not exist on the chain")]
    NonExistentAccountError(AccountId),

    #[error("There was an issue with the serialization/deserialization - {0}")]
    SerdeJsonError(serde_json::Error),

    #[error("Account {0} is not a valid account address")]
    MalformedAccountAddress(String),

    #[error("Queried contract (code_id: {0}) did not have any code information attached")]
    NoCodeInformation(ContractCodeId),

    #[error("Queried contract (address: {0}) did not have any contract information attached")]
    NoContractInformation(AccountId),

    #[error("Contract contains invalid operations in its history")]
    InvalidContractHistoryOperation,

    #[error("Block has an invalid height (either negative or larger than i64::MAX")]
    InvalidHeight,

    #[error("Failed to compress provided wasm code - {0}")]
    WasmCompressionError(io::Error),

    #[error("Logs returned from the validator were malformed")]
    MalformedLogString,

    #[error(
        "Error when broadcasting tx {hash} at height {height}. Error occurred during CheckTx phase. Code: {code}; Raw log: {raw_log}"
    )]
    BroadcastTxErrorCheckTx {
        hash: tx::Hash,
        height: block::Height,
        code: u32,
        raw_log: String,
    },

    #[error(
        "Error when broadcasting tx {hash} at height {height}. Error occurred during DeliverTx phase. Code: {code}; Raw log: {raw_log}"
    )]
    BroadcastTxErrorDeliverTx {
        hash: tx::Hash,
        height: block::Height,
        code: u32,
        raw_log: String,
    },
}

impl From<bip32::Error> for ValidatorClientError {
    fn from(err: bip32::Error) -> Self {
        ValidatorClientError::Bip32Error(err)
    }
}

impl From<bip39::Error> for ValidatorClientError {
    fn from(err: bip39::Error) -> Self {
        ValidatorClientError::Bip39Error(err)
    }
}

impl From<rpc::Error> for ValidatorClientError {
    fn from(err: rpc::Error) -> Self {
        ValidatorClientError::TendermintError(err)
    }
}

impl From<prost::EncodeError> for ValidatorClientError {
    fn from(err: prost::EncodeError) -> Self {
        ValidatorClientError::ProtobufEncodingError(err)
    }
}

impl From<prost::DecodeError> for ValidatorClientError {
    fn from(err: prost::DecodeError) -> Self {
        ValidatorClientError::ProtobufDecodingError(err)
    }
}

impl From<serde_json::Error> for ValidatorClientError {
    fn from(err: serde_json::Error) -> Self {
        ValidatorClientError::SerdeJsonError(err)
    }
}

// this is the case of message like
/*
{
  "code": 12,
  "message": "Not Implemented",
  "details": [
  ]
}
 */
// I didn't manage to find where it exactly originates, nor what the correct types should be
// so all of those are some educated guesses

#[derive(Error, Debug, Deserialize)]
#[error("code: {code} - {message}")]
pub(super) struct CodedError {
    code: u32,
    message: String,
    details: Vec<(String, String)>,
}

#[derive(Error, Deserialize, Debug)]
#[error("{error}")]
pub(super) struct SmartQueryError {
    pub(super) error: String,
}
