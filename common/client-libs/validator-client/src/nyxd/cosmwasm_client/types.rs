// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: There's a significant argument to pull those out of the package and make a PR on https://github.com/cosmos/cosmos-rust/

use crate::nyxd::cosmwasm_client::logs::Log;
use crate::nyxd::error::NyxdError;
use cosmrs::auth::{BaseAccount, ModuleAccount};
use cosmrs::cosmwasm::{CodeInfoResponse, ContractInfo};
use cosmrs::proto::cosmos::auth::v1beta1::{
    BaseAccount as ProtoBaseAccount, ModuleAccount as ProtoModuleAccount,
};
use cosmrs::proto::cosmos::base::abci::v1beta1::Result as ProtoAbciResult;
use cosmrs::proto::cosmos::tx::v1beta1::SimulateResponse as ProtoSimulateResponse;
use cosmrs::proto::cosmos::vesting::v1beta1::{
    BaseVestingAccount as ProtoBaseVestingAccount,
    ContinuousVestingAccount as ProtoContinuousVestingAccount,
    DelayedVestingAccount as ProtoDelayedVestingAccount,
    PeriodicVestingAccount as ProtoPeriodicVestingAccount,
    PermanentLockedAccount as ProtoPermanentLockedAccount,
};
use cosmrs::tendermint::{abci, Hash};
use cosmrs::tx::{AccountNumber, SequenceNumber};
use cosmrs::vesting::{
    BaseVestingAccount, ContinuousVestingAccount, DelayedVestingAccount, PeriodicVestingAccount,
    PermanentLockedAccount,
};
use cosmrs::{AccountId, Any, Coin as CosmosCoin};
use prost::Message;
use serde::Serialize;

pub use cosmrs::abci::GasInfo;
pub use cosmrs::abci::MsgResponse;

pub type ContractCodeId = u64;

#[derive(Serialize)]
pub struct EmptyMsg {}

#[derive(Debug)]
pub struct SequenceResponse {
    pub account_number: AccountNumber,
    pub sequence: SequenceNumber,
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
    pub fn try_get_base_account(&self) -> Result<&BaseAccount, NyxdError> {
        match self {
            Account::Base(acc) => Ok(acc),
            Account::Module(acc) => acc
                .base_account
                .as_ref()
                .ok_or(NyxdError::NoBaseAccountInformationAvailable),
            Account::BaseVesting(acc) => acc
                .base_account
                .as_ref()
                .ok_or(NyxdError::NoBaseAccountInformationAvailable),
            Account::ContinuousVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NyxdError::NoBaseAccountInformationAvailable),
            Account::DelayedVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NyxdError::NoBaseAccountInformationAvailable),
            Account::PeriodicVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NyxdError::NoBaseAccountInformationAvailable),
            Account::PermanentLockedVesting(acc) => acc
                .base_vesting_account
                .as_ref()
                .and_then(|vesting_acc| vesting_acc.base_account.as_ref())
                .ok_or(NyxdError::NoBaseAccountInformationAvailable),
        }
    }
}

impl TryFrom<Any> for Account {
    type Error = NyxdError;

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
            _ => Err(NyxdError::UnsupportedAccountType {
                type_url: raw_account.type_url,
            }),
        }
    }
}

#[derive(Debug)]
pub struct CodeDetails {
    pub code_info: CodeInfoResponse,

    /// The original wasm bytes
    pub data: Vec<u8>,
}

impl CodeDetails {
    pub fn new(code_info: CodeInfoResponse, data: Vec<u8>) -> Self {
        CodeDetails { code_info, data }
    }
}

#[derive(Debug)]
pub struct Contract {
    pub address: AccountId,

    pub contract_info: ContractInfo,
}

impl Contract {
    pub(crate) fn new(address: AccountId, contract_info: ContractInfo) -> Self {
        Contract {
            address,
            contract_info,
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
    type Error = NyxdError;

    fn try_from(value: ProtoAbciResult) -> Result<Self, Self::Error> {
        let events = value
            .events
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        #[allow(deprecated)]
        Ok(AbciResult {
            // TODO: make sure this actually works since technically we're converting from 0.37 protobuf definition as opposed to 0.34...
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
    type Error = NyxdError;

    fn try_from(value: ProtoSimulateResponse) -> Result<Self, Self::Error> {
        Ok(SimulateResponse {
            gas_info: value
                .gas_info
                .map(|gas_info| gas_info.try_into())
                .transpose()?,
            result: value.result.map(|result| result.try_into()).transpose()?,
        })
    }
}

// ##############################################################################
// types specific to the signing client (perhaps they should go to separate file)
// ##############################################################################

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
    pub transaction_hash: Hash,

    pub gas_info: GasInfo,
}

#[derive(Debug)]
pub struct InstantiateOptions {
    /// The funds that are transferred from the sender to the newly created contract.
    /// The funds are transferred as part of the message execution after the contract address is
    /// created and before the instantiation message is executed by the contract.
    ///
    /// Only native tokens are supported.
    pub funds: Vec<CosmosCoin>,

    /// A bech32 encoded address of an admin account.
    /// Caution: an admin has the privilege to upgrade a contract.
    /// If this is not desired, do not set this value.
    pub admin: Option<AccountId>,
}

impl InstantiateOptions {
    pub fn new<T: Into<CosmosCoin>>(funds: Vec<T>, admin: Option<AccountId>) -> Self {
        InstantiateOptions {
            funds: funds.into_iter().map(Into::into).collect(),
            admin,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InstantiateResult {
    /// The address of the newly instantiated contract
    pub contract_address: AccountId,

    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: Hash,

    pub gas_info: GasInfo,
}

#[derive(Debug, Serialize)]
pub struct ChangeAdminResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: Hash,

    pub gas_info: GasInfo,
}

#[derive(Debug, Serialize)]
pub struct MigrateResult {
    pub logs: Vec<Log>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: Hash,

    pub gas_info: GasInfo,
}

#[derive(Debug, Serialize)]
pub struct ExecuteResult {
    pub logs: Vec<Log>,

    pub msg_responses: Vec<MsgResponse>,

    /// Transaction hash (might be used as transaction ID)
    pub transaction_hash: Hash,

    pub gas_info: GasInfo,
}
