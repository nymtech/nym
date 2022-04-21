// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: There's a significant argument to pull those out of the package and make a PR on https://github.com/cosmos/cosmos-rust/

use crate::nymd::cosmwasm_client::helpers::parse_proto_coin_vec;
use crate::nymd::cosmwasm_client::logs::Log;
use crate::nymd::error::NymdError;
use cosmrs::crypto::PublicKey;
use cosmrs::proto::cosmos::auth::v1beta1::{
    BaseAccount as ProtoBaseAccount, ModuleAccount as ProtoModuleAccount,
};
use cosmrs::proto::cosmos::base::abci::v1beta1::{
    GasInfo as ProtoGasInfo, Result as ProtoAbciResult,
};
use cosmrs::proto::cosmos::tx::v1beta1::SimulateResponse as ProtoSimulateResponse;
use cosmrs::proto::cosmos::vesting::v1beta1::{
    BaseVestingAccount as ProtoBaseVestingAccount,
    ContinuousVestingAccount as ProtoContinuousVestingAccount,
    DelayedVestingAccount as ProtoDelayedVestingAccount, Period as ProtoPeriod,
    PeriodicVestingAccount as ProtoPeriodicVestingAccount,
    PermanentLockedAccount as ProtoPermanentLockedAccount,
};
use cosmrs::proto::cosmwasm::wasm::v1::{
    CodeInfoResponse, ContractCodeHistoryEntry as ProtoContractCodeHistoryEntry,
    ContractCodeHistoryOperationType, ContractInfo as ProtoContractInfo,
};
use cosmrs::tendermint::{abci, chain};
use cosmrs::tx::{AccountNumber, Gas, SequenceNumber};
use cosmrs::{tx, AccountId, Any, Coin};
use prost::Message;
use serde::Serialize;
use std::convert::{TryFrom, TryInto};

pub type ContractCodeId = u64;

#[derive(Serialize)]
pub struct EmptyMsg {}

#[derive(Debug)]
pub struct SequenceResponse {
    pub account_number: AccountNumber,
    pub sequence: SequenceNumber,
}

/// BaseAccount defines a base account type. It contains all the necessary fields
/// for basic account functionality. Any custom account type should extend this
/// type for additional functionality (e.g. vesting).
#[derive(Debug)]
pub struct BaseAccount {
    /// Bech32 account address
    pub address: AccountId,
    pub pubkey: Option<PublicKey>,
    pub account_number: AccountNumber,
    pub sequence: SequenceNumber,
}

impl TryFrom<ProtoBaseAccount> for BaseAccount {
    type Error = NymdError;

    fn try_from(value: ProtoBaseAccount) -> Result<Self, Self::Error> {
        let address: AccountId = value
            .address
            .parse()
            .map_err(|_| NymdError::MalformedAccountAddress(value.address.clone()))?;

        let pubkey = value
            .pub_key
            .map(PublicKey::try_from)
            .transpose()
            .map_err(|_| NymdError::InvalidPublicKey(address.clone()))?;

        Ok(BaseAccount {
            address,
            pubkey,
            account_number: value.account_number,
            sequence: value.sequence,
        })
    }
}

/// ModuleAccount defines an account for modules that holds coins on a pool.
#[derive(Debug)]
pub struct ModuleAccount {
    pub base_account: Option<BaseAccount>,
    pub name: String,
    pub permissions: Vec<String>,
}

impl TryFrom<ProtoModuleAccount> for ModuleAccount {
    type Error = NymdError;

    fn try_from(value: ProtoModuleAccount) -> Result<Self, Self::Error> {
        let base_account = value.base_account.map(TryFrom::try_from).transpose()?;

        Ok(ModuleAccount {
            base_account,
            name: value.name,
            permissions: value.permissions,
        })
    }
}

/// BaseVestingAccount implements the VestingAccount interface. It contains all
/// the necessary fields needed for any vesting account implementation.
#[derive(Debug)]
pub struct BaseVestingAccount {
    pub base_account: Option<BaseAccount>,
    pub original_vesting: Vec<Coin>,
    pub delegated_free: Vec<Coin>,
    pub delegated_vesting: Vec<Coin>,
    pub end_time: i64,
}

impl TryFrom<ProtoBaseVestingAccount> for BaseVestingAccount {
    type Error = NymdError;

    fn try_from(value: ProtoBaseVestingAccount) -> Result<Self, Self::Error> {
        let base_account = value.base_account.map(TryFrom::try_from).transpose()?;

        let original_vesting = parse_proto_coin_vec(value.original_vesting)?;
        let delegated_free = parse_proto_coin_vec(value.delegated_free)?;
        let delegated_vesting = parse_proto_coin_vec(value.delegated_vesting)?;

        Ok(BaseVestingAccount {
            base_account,
            original_vesting,
            delegated_free,
            delegated_vesting,
            end_time: value.end_time,
        })
    }
}

/// ContinuousVestingAccount implements the VestingAccount interface. It
/// continuously vests by unlocking coins linearly with respect to time.
#[derive(Debug)]
pub struct ContinuousVestingAccount {
    pub base_vesting_account: Option<BaseVestingAccount>,
    pub start_time: i64,
}

impl TryFrom<ProtoContinuousVestingAccount> for ContinuousVestingAccount {
    type Error = NymdError;

    fn try_from(value: ProtoContinuousVestingAccount) -> Result<Self, Self::Error> {
        let base_vesting_account = value
            .base_vesting_account
            .map(TryFrom::try_from)
            .transpose()?;

        Ok(ContinuousVestingAccount {
            base_vesting_account,
            start_time: value.start_time,
        })
    }
}

/// DelayedVestingAccount implements the VestingAccount interface. It vests all
/// coins after a specific time, but non prior. In other words, it keeps them
/// locked until a specified time.
#[derive(Debug)]
pub struct DelayedVestingAccount {
    pub base_vesting_account: Option<BaseVestingAccount>,
}

impl TryFrom<ProtoDelayedVestingAccount> for DelayedVestingAccount {
    type Error = NymdError;

    fn try_from(value: ProtoDelayedVestingAccount) -> Result<Self, Self::Error> {
        let base_vesting_account = value
            .base_vesting_account
            .map(TryFrom::try_from)
            .transpose()?;

        Ok(DelayedVestingAccount {
            base_vesting_account,
        })
    }
}

/// Period defines a length of time and amount of coins that will vest.
#[derive(Debug)]
pub struct Period {
    pub length: i64,
    pub amount: Vec<Coin>,
}

impl TryFrom<ProtoPeriod> for Period {
    type Error = NymdError;

    fn try_from(value: ProtoPeriod) -> Result<Self, Self::Error> {
        Ok(Period {
            length: value.length,
            amount: parse_proto_coin_vec(value.amount)?,
        })
    }
}

/// PeriodicVestingAccount implements the VestingAccount interface. It
/// periodically vests by unlocking coins during each specified period.
#[derive(Debug)]
pub struct PeriodicVestingAccount {
    pub base_vesting_account: Option<BaseVestingAccount>,
    pub start_time: i64,
    pub vesting_periods: Vec<Period>,
}

impl TryFrom<ProtoPeriodicVestingAccount> for PeriodicVestingAccount {
    type Error = NymdError;

    fn try_from(value: ProtoPeriodicVestingAccount) -> Result<Self, Self::Error> {
        let base_vesting_account = value
            .base_vesting_account
            .map(TryFrom::try_from)
            .transpose()?;

        let vesting_periods = value
            .vesting_periods
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Result<_, _>>()?;

        Ok(PeriodicVestingAccount {
            base_vesting_account,
            start_time: value.start_time,
            vesting_periods,
        })
    }
}

/// PermanentLockedAccount implements the VestingAccount interface. It does
/// not ever release coins, locking them indefinitely. Coins in this account can
/// still be used for delegating and for governance votes even while locked.
#[derive(Debug)]
pub struct PermanentLockedAccount {
    pub base_vesting_account: Option<BaseVestingAccount>,
}

impl TryFrom<ProtoPermanentLockedAccount> for PermanentLockedAccount {
    type Error = NymdError;

    fn try_from(value: ProtoPermanentLockedAccount) -> Result<Self, Self::Error> {
        let base_vesting_account = value
            .base_vesting_account
            .map(TryFrom::try_from)
            .transpose()?;

        Ok(PermanentLockedAccount {
            base_vesting_account,
        })
    }
}

#[derive(Debug)]
pub enum Account {
    Base(BaseAccount),
    Module(ModuleAccount),
    BaseVesting(BaseVestingAccount),
    ContinuousVesting(ContinuousVestingAccount),
    DelayedVesting(DelayedVestingAccount),
    PeriodicVesting(PeriodicVestingAccount),
    PermanentLockedVesting(PermanentLockedAccount),
}

impl Account {
    pub fn try_get_base_account(&self) -> Result<&BaseAccount, NymdError> {
        match self {
            Account::Base(acc) => Ok(acc),
            Account::Module(acc) => acc
                .base_account
                .as_ref()
                .ok_or(NymdError::NoBaseAccountInformationAvailable),
            Account::BaseVesting(acc) => acc
                .base_account
                .as_ref()
                .ok_or(NymdError::NoBaseAccountInformationAvailable),
            Account::ContinuousVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NymdError::NoBaseAccountInformationAvailable),
            Account::DelayedVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NymdError::NoBaseAccountInformationAvailable),
            Account::PeriodicVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NymdError::NoBaseAccountInformationAvailable),
            Account::PermanentLockedVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NymdError::NoBaseAccountInformationAvailable),
        }
    }
}

impl TryFrom<Any> for Account {
    type Error = NymdError;

    fn try_from(raw_account: Any) -> Result<Self, Self::Error> {
        match raw_account.type_url.as_ref() {
            "/cosmos.auth.v1beta1.BaseAccount" => Ok(Account::Base(
                ProtoBaseAccount::decode(raw_account.value.as_ref())?.try_into()?,
            )),
            "/cosmos.auth.v1beta1.ModuleAccount" => Ok(Account::Module(
                ProtoModuleAccount::decode(raw_account.value.as_ref())?.try_into()?,
            )),
            "/cosmos.vesting.v1beta1.BaseVestingAccount" => Ok(Account::BaseVesting(
                ProtoBaseVestingAccount::decode(raw_account.value.as_ref())?.try_into()?,
            )),
            "/cosmos.vesting.v1beta1.ContinuousVestingAccount" => Ok(Account::ContinuousVesting(
                ProtoContinuousVestingAccount::decode(raw_account.value.as_ref())?.try_into()?,
            )),
            "/cosmos.vesting.v1beta1.DelayedVestingAccount" => Ok(Account::DelayedVesting(
                ProtoDelayedVestingAccount::decode(raw_account.value.as_ref())?.try_into()?,
            )),
            "/cosmos.vesting.v1beta1.PeriodicVestingAccount" => Ok(Account::PeriodicVesting(
                ProtoPeriodicVestingAccount::decode(raw_account.value.as_ref())?.try_into()?,
            )),
            "/cosmos.vesting.v1beta1.PermanentLockedAccount" => {
                Ok(Account::PermanentLockedVesting(
                    ProtoPermanentLockedAccount::decode(raw_account.value.as_ref())?.try_into()?,
                ))
            }
            _ => Err(NymdError::UnsupportedAccountType {
                type_url: raw_account.type_url,
            }),
        }
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

#[derive(Debug)]
pub struct GasInfo {
    /// GasWanted is the maximum units of work we allow this tx to perform.
    pub gas_wanted: Gas,

    /// GasUsed is the amount of gas actually consumed.
    pub gas_used: Gas,
}

impl From<ProtoGasInfo> for GasInfo {
    fn from(value: ProtoGasInfo) -> Self {
        GasInfo {
            gas_wanted: value.gas_wanted.into(),
            gas_used: value.gas_used.into(),
        }
    }
}

impl GasInfo {
    pub fn new(gas_wanted: Gas, gas_used: Gas) -> Self {
        GasInfo {
            gas_wanted,
            gas_used,
        }
    }
}

#[derive(Debug)]
pub struct AbciResult {
    /// Data is any data returned from message or handler execution. It MUST be
    /// length prefixed in order to separate data from multiple message executions.
    pub data: Vec<u8>,

    /// Log contains the log information from message or handler execution.
    // todo: try to parse into Log?
    pub log: String,

    /// Events contains a slice of Event objects that were emitted during message
    /// or handler execution.
    pub events: Vec<abci::Event>,
}

impl TryFrom<ProtoAbciResult> for AbciResult {
    type Error = NymdError;

    fn try_from(value: ProtoAbciResult) -> Result<Self, Self::Error> {
        let mut events = Vec::with_capacity(value.events.len());

        for proto_event in value.events.into_iter() {
            let type_str = proto_event.r#type;

            let mut attributes = Vec::with_capacity(proto_event.attributes.len());
            for proto_attribute in proto_event.attributes.into_iter() {
                let stringified_ked = String::from_utf8(proto_attribute.key)
                    .map_err(|_| NymdError::DeserializationError("EventAttributeKey".to_owned()))?;
                let stringified_value = String::from_utf8(proto_attribute.value)
                    .map_err(|_| NymdError::DeserializationError("EventAttributeKey".to_owned()))?;

                attributes.push(abci::tag::Tag {
                    key: stringified_ked.parse().unwrap(),
                    value: stringified_value.parse().unwrap(),
                })
            }

            events.push(abci::Event {
                type_str,
                attributes,
            })
        }

        Ok(AbciResult {
            data: value.data,
            log: value.log,
            events,
        })
    }
}

#[derive(Debug)]
pub struct SimulateResponse {
    pub gas_info: Option<GasInfo>,
    pub result: Option<AbciResult>,
}

impl TryFrom<ProtoSimulateResponse> for SimulateResponse {
    type Error = NymdError;

    fn try_from(value: ProtoSimulateResponse) -> Result<Self, Self::Error> {
        Ok(SimulateResponse {
            gas_info: value.gas_info.map(|gas_info| gas_info.into()),
            result: value.result.map(|result| result.try_into()).transpose()?,
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

    pub gas_info: GasInfo,
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

    pub gas_info: GasInfo,
}

#[derive(Debug)]
pub struct ChangeAdminResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,

    pub gas_info: GasInfo,
}

#[derive(Debug)]
pub struct MigrateResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,

    pub gas_info: GasInfo,
}

#[derive(Debug)]
pub struct ExecuteResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: tx::Hash,

    pub gas_info: GasInfo,
}
