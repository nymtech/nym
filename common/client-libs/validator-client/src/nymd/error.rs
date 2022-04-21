// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::types::ContractCodeId;
use cosmrs::tendermint::{abci, block};
use cosmrs::{bip32, tx, AccountId};
use std::io;
use thiserror::Error;

pub use cosmrs::rpc::error::{
    Error as TendermintRpcError, ErrorDetail as TendermintRpcErrorDetail,
};
pub use cosmrs::rpc::response_error::{Code, ResponseError};

#[derive(Debug, Error)]
pub enum NymdError {
    #[error("No contract address is available to perform the call")]
    NoContractAddressAvailable,

    #[error("There was an issue with bip32 - {0}")]
    Bip32Error(#[from] bip32::Error),

    #[error("There was an issue with bip32 - {0}")]
    Bip39Error(#[from] bip39::Error),

    #[error("Failed to derive account address")]
    AccountDerivationError,

    #[error("Address {0} was not found in the wallet")]
    SigningAccountNotFound(AccountId),

    #[error("Failed to sign raw transaction")]
    SigningFailure,

    #[error("{0} is not a valid tx hash")]
    InvalidTxHash(String),

    #[error("There was an issue with a tendermint RPC request - {0}")]
    TendermintError(#[from] TendermintRpcError),

    #[error("There was an issue when attempting to serialize data")]
    SerializationError(String),

    #[error("There was an issue when attempting to deserialize data")]
    DeserializationError(String),

    #[error("There was an issue when attempting to encode our protobuf data - {0}")]
    ProtobufEncodingError(#[from] prost::EncodeError),

    #[error("There was an issue when attempting to decode our protobuf data - {0}")]
    ProtobufDecodingError(#[from] prost::DecodeError),

    #[error("Account {0} does not exist on the chain")]
    NonExistentAccountError(AccountId),

    #[error("There was an issue with the serialization/deserialization - {0}")]
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

    #[error("The provided gas price is malformed")]
    MalformedGasPrice,

    #[error("Failed to estimate gas price for the transaction")]
    GasEstimationFailure,

    #[error("Abci query failed with code {0} - {1}")]
    AbciError(u32, abci::Log),

    #[error("Unsupported account type: {type_url}")]
    UnsupportedAccountType { type_url: String },

    #[error("{coin_representation} is not a valid Cosmos Coin")]
    MalformedCoin { coin_representation: String },

    #[error("This account does not have BaseAccount information available to it")]
    NoBaseAccountInformationAvailable,
}

impl NymdError {
    pub fn is_tendermint_response_timeout(&self) -> bool {
        match &self {
            NymdError::TendermintError(TendermintRpcError(
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
            NymdError::TendermintError(TendermintRpcError(
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
