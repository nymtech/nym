// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: There's a significant argument to pull those out of the package and make a PR on https://github.com/cosmos/cosmos-rust/

use crate::nymd::cosmwasm_client::logs::Log;
use crate::nymd::error::NymdError;
use cosmrs::crypto::PublicKey;
use cosmrs::proto::cosmos::auth::v1beta1::BaseAccount;
use cosmrs::proto::cosmwasm::wasm::v1::{
    CodeInfoResponse, ContractCodeHistoryEntry as ProtoContractCodeHistoryEntry,
    ContractCodeHistoryOperationType, ContractInfo as ProtoContractInfo,
};
use cosmrs::tendermint::chain;
use cosmrs::tx::{AccountNumber, SequenceNumber};
use cosmrs::{tx, AccountId, Coin};
use serde::Serialize;
use std::convert::TryFrom;

pub type ContractCodeId = u64;

#[derive(Serialize)]
pub struct EmptyMsg {}

#[derive(Debug)]
pub struct SequenceResponse {
    pub account_number: AccountNumber,
    pub sequence: SequenceNumber,
}

#[derive(Debug)]
pub struct Account {
    /// Bech32 account address
    pub address: AccountId,
    pub pubkey: Option<PublicKey>,
    pub account_number: AccountNumber,
    pub sequence: SequenceNumber,
}

impl TryFrom<BaseAccount> for Account {
    type Error = NymdError;

    fn try_from(value: BaseAccount) -> Result<Self, Self::Error> {
        let address: AccountId = value
            .address
            .parse()
            .map_err(|_| NymdError::MalformedAccountAddress(value.address.clone()))?;

        let pubkey = value
            .pub_key
            .map(PublicKey::try_from)
            .transpose()
            .map_err(|_| NymdError::InvalidPublicKey(address.clone()))?;

        Ok(Account {
            address,
            pubkey,
            account_number: value.account_number,
            sequence: value.sequence,
        })
    }
}

#[derive(Debug)]
pub struct Code {
    pub code_id: ContractCodeId,

    /// Bech32 account address
    pub creator: AccountId,

    /// sha256 hash of the code stored
    pub data_hash: Vec<u8>,
}

impl TryFrom<CodeInfoResponse> for Code {
    type Error = NymdError;

    fn try_from(value: CodeInfoResponse) -> Result<Self, Self::Error> {
        let CodeInfoResponse {
            code_id,
            creator,
            data_hash,
        } = value;

        let creator = creator
            .parse()
            .map_err(|_| NymdError::MalformedAccountAddress(creator))?;

        Ok(Code {
            code_id,
            creator,
            data_hash,
        })
    }
}

#[derive(Debug)]
pub struct CodeDetails {
    pub code_info: Code,

    /// The original wasm bytes
    pub data: Vec<u8>,
}

impl CodeDetails {
    pub fn new(code_info: Code, data: Vec<u8>) -> Self {
        CodeDetails { code_info, data }
    }
}

#[derive(Debug)]
pub(crate) struct ContractInfo {
    code_id: ContractCodeId,
    creator: AccountId,
    admin: Option<AccountId>,
    label: String,
}

impl TryFrom<ProtoContractInfo> for ContractInfo {
    type Error = NymdError;

    fn try_from(value: ProtoContractInfo) -> Result<Self, Self::Error> {
        let ProtoContractInfo {
            code_id,
            creator,
            admin,
            label,
            ..
        } = value;

        let admin = if admin.is_empty() {
            None
        } else {
            Some(
                admin
                    .parse()
                    .map_err(|_| NymdError::MalformedAccountAddress(admin))?,
            )
        };

        Ok(ContractInfo {
            code_id,
            creator: creator
                .parse()
                .map_err(|_| NymdError::MalformedAccountAddress(creator))?,
            admin,
            label,
        })
    }
}

#[derive(Debug)]
pub struct Contract {
    pub address: AccountId,

    pub code_id: ContractCodeId,

    /// Bech32 account address
    pub creator: AccountId,

    /// Bech32-encoded admin address
    pub admin: Option<AccountId>,

    pub label: String,
}

impl Contract {
    pub(crate) fn new(address: AccountId, contract_info: ContractInfo) -> Self {
        Contract {
            address,
            code_id: contract_info.code_id,
            creator: contract_info.creator,
            admin: contract_info.admin,
            label: contract_info.label,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ContractCodeHistoryEntryOperation {
    Init,
    Genesis,
    Migrate,
}

#[derive(Debug)]
pub struct ContractCodeHistoryEntry {
    /// The source of this history entry
    pub operation: ContractCodeHistoryEntryOperation,
    pub code_id: ContractCodeId,
    pub msg_json: String,
}

impl TryFrom<ProtoContractCodeHistoryEntry> for ContractCodeHistoryEntry {
    type Error = NymdError;

    fn try_from(value: ProtoContractCodeHistoryEntry) -> Result<Self, Self::Error> {
        let operation = match ContractCodeHistoryOperationType::from_i32(value.operation)
            .ok_or(NymdError::InvalidContractHistoryOperation)?
        {
            ContractCodeHistoryOperationType::Unspecified => {
                return Err(NymdError::InvalidContractHistoryOperation)
            }
            ContractCodeHistoryOperationType::Init => ContractCodeHistoryEntryOperation::Init,
            ContractCodeHistoryOperationType::Genesis => ContractCodeHistoryEntryOperation::Genesis,
            ContractCodeHistoryOperationType::Migrate => ContractCodeHistoryEntryOperation::Migrate,
        };

        Ok(ContractCodeHistoryEntry {
            operation,
            code_id: value.code_id,
            msg_json: String::from_utf8(value.msg)
                .map_err(|_| NymdError::DeserializationError("Contract history msg".to_owned()))?,
        })
    }
}

// ##############################################################################
// types specific to the signing client (perhaps they should go to separate file)
// ##############################################################################

/// Signing information for a single signer that is not included in the transaction.
#[derive(Debug)]
pub struct SignerData {
    pub account_number: AccountNumber,
    pub sequence: SequenceNumber,
    pub chain_id: chain::Id,
}

#[derive(Debug)]
pub struct UploadResult {
    /// Size of the original wasm code in bytes
    pub original_size: usize,

    /// A hex encoded sha256 checksum of the original wasm code (that is stored on chain)
    pub original_checksum: Vec<u8>,

    /// Size of the compressed wasm code in bytes
    pub compressed_size: usize,

    /// A sha256 checksum of the compressed wasm code (that is stored in the transaction)
    pub compressed_checksum: Vec<u8>,

    /// The ID of the code assigned by the chain
    pub code_id: ContractCodeId,

    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,
}

#[derive(Debug)]
pub struct InstantiateOptions {
    /// The funds that are transferred from the sender to the newly created contract.
    /// The funds are transferred as part of the message execution after the contract address is
    /// created and before the instantiation message is executed by the contract.
    ///
    /// Only native tokens are supported.
    pub funds: Vec<Coin>,

    /// A bech32 encoded address of an admin account.
    /// Caution: an admin has the privilege to upgrade a contract.
    /// If this is not desired, do not set this value.
    pub admin: Option<AccountId>,
}

#[derive(Debug)]
pub struct InstantiateResult {
    /// The address of the newly instantiated contract
    pub contract_address: AccountId,

    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,
}

#[derive(Debug)]
pub struct ChangeAdminResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,
}

#[derive(Debug)]
pub struct MigrateResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,
}

#[derive(Debug)]
pub struct ExecuteResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,
}
