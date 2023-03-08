// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::types::ContractCodeId;
use cosmrs::{
    bip32,
    rpc::endpoint::abci_query::AbciQuery,
    tendermint::{
        abci::{self, Code as AbciCode},
        block,
    },
    tx, AccountId,
};
use thiserror::Error;

use std::{io, time::Duration};

pub use cosmrs::rpc::{
    error::{Error as TendermintRpcError, ErrorDetail as TendermintRpcErrorDetail},
    response_error::{Code, ResponseError},
};

#[derive(Debug, Error)]
pub enum NyxdError {
    #[error("No contract address is available to perform the call")]
    NoContractAddressAvailable,

    #[error("There was an issue with bip32 - {0}")]
    Bip32Error(#[from] bip32::Error),

    #[error("There was an issue with bip39 - {0}")]
    Bip39Error(#[from] bip39::Error),

    #[error("There was an issue on the cosmrs side - {0}")]
    CosmrsError(#[from] cosmrs::Error),

    #[error("Failed to derive account address")]
    AccountDerivationError,

    #[error("Address {0} was not found in the wallet")]
    SigningAccountNotFound(AccountId),

    #[error("Failed to sign raw transaction")]
    SigningFailure,

    #[error("{0} is not a valid tx hash")]
    InvalidTxHash(String),

    #[error("Tendermint RPC request failed - {0}")]
    TendermintError(#[from] TendermintRpcError),

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
        hash: tx::Hash,
        height: Option<block::Height>,
        code: u32,
        raw_log: String,
    },

    #[error(
    "Error when broadcasting tx {hash} at height {height:?}. Error occurred during DeliverTx phase. Code: {code}; Raw log: {raw_log}"
    )]
    BroadcastTxErrorDeliverTx {
        hash: tx::Hash,
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
        log: abci::Log,
        pretty_log: Option<String>,
    },

    #[error("Unsupported account type: {type_url}")]
    UnsupportedAccountType { type_url: String },

    #[error("{coin_representation} is not a valid Cosmos Coin")]
    MalformedCoin { coin_representation: String },

    #[error("This account does not have BaseAccount information available to it")]
    NoBaseAccountInformationAvailable,

    #[error("Transaction with ID {hash} has been submitted but not yet found on the chain. You might want to check for it later. There was a total wait of {} seconds", .timeout.as_secs())]
    BroadcastTimeout { hash: tx::Hash, timeout: Duration },

    #[error("Cosmwasm std error: {0}")]
    CosmwasmStdError(#[from] cosmwasm_std::StdError),

    #[error("Coconut interface error: {0}")]
    CoconutInterfaceError(#[from] nym_coconut_interface::error::CoconutInterfaceError),

    #[error("Account had an unexpected bech32 prefix. Expected: {expected}, got: {got}")]
    UnexpectedBech32Prefix { got: String, expected: String },
}

// The purpose of parsing the abci query result is that we want to generate the `pretty_log` if
// possible.
pub fn parse_abci_query_result(query_result: AbciQuery) -> Result<AbciQuery, NyxdError> {
    match query_result.code {
        AbciCode::Ok => Ok(query_result),
        AbciCode::Err(code) => Err(NyxdError::AbciError {
            code,
            log: query_result.log.clone(),
            pretty_log: try_parse_abci_log(&query_result.log),
        }),
    }
}

// Some of the error strings returned by the query are a bit too technical to present to the
// enduser. So we special case some commonly encountered errors.
fn try_parse_abci_log(log: &abci::Log) -> Option<String> {
    if log
        .value()
        .contains("Maximum amount of locked coins has already been pledged")
    {
        Some("Maximum amount of locked tokens has alredy been used. You can only use up to 10% of your locked tokens for bonding and delegating.".to_string())
    } else {
        None
    }
}

impl NyxdError {
    pub fn is_tendermint_response_timeout(&self) -> bool {
        match &self {
            NyxdError::TendermintError(TendermintRpcError(
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
            NyxdError::TendermintError(TendermintRpcError(
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
}
