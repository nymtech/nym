// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::types::ContractCodeId;
use crate::signing::direct_wallet::DirectSecp256k1HdWalletError;
use cosmrs::tendermint::Hash;
use cosmrs::{
    tendermint::{abci::Code as AbciCode, block},
    AccountId,
};
use std::{io, time::Duration};
use tendermint_rpc::endpoint::abci_query::AbciQuery;
use thiserror::Error;

pub use cosmrs::tendermint::error::Error as TendermintError;
pub use tendermint_rpc::{
    error::{Error as TendermintRpcError, ErrorDetail as TendermintRpcErrorDetail},
    response_error::{Code, ResponseError},
};

#[derive(Debug, Error)]
pub enum NyxdError {
    #[error("No contract address is available to perform the call: {0}")]
    NoContractAddressAvailable(String),

    #[error(transparent)]
    WalletError(#[from] DirectSecp256k1HdWalletError),

    #[error("There was an issue on the cosmrs side: {0}")]
    CosmrsError(#[from] cosmrs::Error),

    #[error("There was an issue on the cosmrs side: {0}")]
    CosmrsErrorReport(#[from] cosmrs::ErrorReport),

    #[error("cosmwasm event not found")]
    ComswasmEventNotFound,

    #[error("cosmwasm attribute not found")]
    ComswasmAttributeNotFound,

    #[error("Failed to derive account address")]
    AccountDerivationError,

    #[error("Address {0} was not found in the wallet")]
    SigningAccountNotFound(AccountId),

    #[error("Failed to sign raw transaction")]
    SigningFailure,

    #[error("{0} is not a valid tx hash")]
    InvalidTxHash(String),

    #[error("Tendermint RPC request failed - {0}")]
    TendermintErrorRpc(#[from] TendermintRpcError),

    #[error("tendermint library failure: {0}")]
    TendermintError(#[from] TendermintError),

    #[error("Failed when attempting to serialize data ({0})")]
    SerializationError(String),

    #[error("Failed when attempting to deserialize data ({0})")]
    DeserializationError(String),

    #[error("Failed when attempting to encode our protobuf data - {0}")]
    ProtobufEncodingError(#[from] prost::EncodeError),

    #[error("Failed to decode our protobuf data - {0}")]
    ProtobufDecodingError(#[from] prost::DecodeError),

    #[error("Account {0} does not exist on the chain")]
    NonExistentAccountError(AccountId),

    #[error("Failed on json serialization/deserialization - {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Account {0} is not a valid account address")]
    MalformedAccountAddress(String),

    #[error("Account {0} has an invalid associated public key")]
    InvalidPublicKey(AccountId),

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
    "Error when broadcasting tx {hash} at height {height:?}. Error occurred during CheckTx phase. Code: {code}; Raw log: {raw_log}"
    )]
    BroadcastTxErrorCheckTx {
        hash: Hash,
        height: Option<block::Height>,
        code: u32,
        raw_log: String,
    },

    #[error(
    "Error when broadcasting tx {hash} at height {height:?}. Error occurred during DeliverTx phase. Code: {code}; Raw log: {raw_log}"
    )]
    BroadcastTxErrorDeliverTx {
        hash: Hash,
        height: Option<block::Height>,
        code: u32,
        raw_log: String,
    },

    #[error("The provided gas price is malformed")]
    MalformedGasPrice,

    #[error("Failed to estimate gas price for the transaction")]
    GasEstimationFailure,

    #[error("Abci query failed with code {code} - {log}")]
    AbciError {
        code: u32,
        log: String,
        pretty_log: Option<String>,
    },

    #[error("Unsupported account type: {type_url}")]
    UnsupportedAccountType { type_url: String },

    #[error("{coin_representation} is not a valid Cosmos Coin")]
    MalformedCoin { coin_representation: String },

    #[error("This account does not have BaseAccount information available to it")]
    NoBaseAccountInformationAvailable,

    #[error("Transaction with ID {hash} has been submitted but not yet found on the chain. You might want to check for it later. There was a total wait of {} seconds", .timeout.as_secs())]
    BroadcastTimeout { hash: Hash, timeout: Duration },

    #[error("Cosmwasm std error: {0}")]
    CosmwasmStdError(#[from] cosmwasm_std::StdError),

    #[error("Account had an unexpected bech32 prefix. Expected: {expected}, got: {got}")]
    UnexpectedBech32Prefix { got: String, expected: String },
}

// The purpose of parsing the abci query result is that we want to generate the `pretty_log` if
// possible.
pub fn parse_abci_query_result(query_result: AbciQuery) -> Result<AbciQuery, NyxdError> {
    match query_result.code {
        AbciCode::Ok => Ok(query_result),
        AbciCode::Err(code) => Err(NyxdError::AbciError {
            code: code.into(),
            log: query_result.log.clone(),
            pretty_log: try_parse_abci_log(&query_result.log),
        }),
    }
}

// Some of the error strings returned by the query are a bit too technical to present to the
// enduser. So we special case some commonly encountered errors.
fn try_parse_abci_log(log: &str) -> Option<String> {
    if log.contains("Maximum amount of locked coins has already been pledged") {
        Some("Maximum amount of locked tokens has already been used. You can only use up to 10% of your locked tokens for bonding and delegating.".to_string())
    } else {
        None
    }
}

impl NyxdError {
    pub fn is_tendermint_response_timeout(&self) -> bool {
        match &self {
            NyxdError::TendermintErrorRpc(TendermintRpcError(
                TendermintRpcErrorDetail::Response(err),
                _,
            )) => {
                let response = &err.source;
                if response.code() == Code::InternalError {
                    // 0.34 (and earlier) versions of tendermint seemed to be using phrase "timed out waiting ..."
                    // (https://github.com/tendermint/tendermint/blob/v0.34.13/rpc/core/mempool.go#L124)
                    // while 0.35+ has "timeout waiting for ..."
                    // https://github.com/tendermint/tendermint/blob/v0.35.0-rc3/internal/rpc/core/mempool.go#L99
                    // note that as of the time of writing this comment (08.10.2021), the most recent version
                    // of cosmos-sdk (v0.44.1) uses tendermint 0.34.13
                    if let Some(data) = response.data() {
                        data.contains("timed out") || data.contains("timeout")
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn is_tendermint_response_duplicate(&self) -> bool {
        match &self {
            NyxdError::TendermintErrorRpc(TendermintRpcError(
                TendermintRpcErrorDetail::Response(err),
                _,
            )) => {
                let response = &err.source;
                if response.code() == Code::InternalError {
                    // this particular error message seems to be unchanged between 0.34 and newer versions
                    // https://github.com/tendermint/tendermint/blob/v0.34.13/mempool/errors.go#L10
                    // https://github.com/tendermint/tendermint/blob/v0.35.0-rc3/types/mempool.go#L10
                    if let Some(data) = response.data() {
                        data.contains("tx already exists in cache")
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn unavailable_contract_address<S: Into<String>>(contract_type: S) -> Self {
        NyxdError::NoContractAddressAvailable(contract_type.into())
    }
}
