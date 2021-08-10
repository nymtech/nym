// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::types::ContractCodeId;
use cosmos_sdk::tendermint::block;
use cosmos_sdk::{bip32, rpc, tx, AccountId};
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymdError {
    // #[error("An IO error has occured: {source}")]
    // IoError {
    //     #[from]
    //     source: io::Error,
    // },
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
    TendermintError(#[from] rpc::Error),

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
}
