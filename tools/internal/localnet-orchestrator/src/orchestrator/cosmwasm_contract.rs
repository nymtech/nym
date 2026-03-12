// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::account::Account;
use anyhow::Context;
use nym_contracts_common::ContractBuildInformation;
use nym_validator_client::nyxd::cosmwasm_client::types::{
    ContractCodeId, InstantiateResult, MigrateResult, UploadResult,
};
use nym_validator_client::nyxd::{AccountId, Hash};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CosmwasmContract {
    /// Name associated with the contract, e.g. 'mixnet', 'performance', etc.
    pub(crate) name: String,

    /// n1 address of the contract
    pub(crate) address: AccountId,

    /// n1 address and mnemonic of the contract admin (i.e. wallet that is allowed to perform migrations)
    pub(crate) admin: Account,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContractBeingInitialised {
    pub(crate) name: String,
    pub(crate) wasm_path: Option<PathBuf>,
    pub(crate) upload_info: Option<MinimalUploadInfo>,
    pub(crate) admin: Option<Account>,
    pub(crate) init_info: Option<MinimalInitInfo>,
    pub(crate) migrate_info: Option<MinimalMigrateInfo>,
    pub(crate) build_info: Option<ContractBuildInformation>,
}

impl ContractBeingInitialised {
    pub(crate) fn new<S: Into<String>>(name: S) -> Self {
        ContractBeingInitialised {
            name: name.into(),
            wasm_path: None,
            upload_info: None,
            admin: None,
            init_info: None,
            migrate_info: None,
            build_info: None,
        }
    }

    pub(crate) fn wasm_path(&self) -> anyhow::Result<&PathBuf> {
        self.wasm_path.as_ref().context(format!(
            "could not find .wasm file for {} contract under the provided directory",
            self.name
        ))
    }

    pub(crate) fn upload_info(&self) -> anyhow::Result<&MinimalUploadInfo> {
        self.upload_info
            .as_ref()
            .context(format!("could not find code_id for {} contract", self.name))
    }

    pub(crate) fn code_id(&self) -> anyhow::Result<ContractCodeId> {
        Ok(self.upload_info()?.code_id)
    }

    pub(crate) fn admin(&self) -> anyhow::Result<&Account> {
        self.admin.as_ref().context(format!(
            "could not find contract admin for {} contract",
            self.name
        ))
    }

    pub(crate) fn admin_address(&self) -> anyhow::Result<AccountId> {
        Ok(self.admin()?.address.clone())
    }

    pub(crate) fn init_info(&self) -> anyhow::Result<&MinimalInitInfo> {
        self.init_info
            .as_ref()
            .context(format!("could not find address for {} contract", self.name))
    }

    #[allow(dead_code)]
    pub(crate) fn build_info(&self) -> anyhow::Result<&ContractBuildInformation> {
        self.build_info.as_ref().context(format!(
            "could not find build information for {} contract",
            self.name
        ))
    }

    pub(crate) fn address(&self) -> anyhow::Result<&AccountId> {
        self.init_info().map(|info| &info.contract_address)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MinimalUploadInfo {
    pub transaction_hash: Hash,
    pub code_id: ContractCodeId,
}

impl From<UploadResult> for MinimalUploadInfo {
    fn from(value: UploadResult) -> Self {
        MinimalUploadInfo {
            transaction_hash: value.transaction_hash,
            code_id: value.code_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MinimalInitInfo {
    pub transaction_hash: Hash,
    pub contract_address: AccountId,
}

impl From<InstantiateResult> for MinimalInitInfo {
    fn from(value: InstantiateResult) -> Self {
        MinimalInitInfo {
            transaction_hash: value.transaction_hash,
            contract_address: value.contract_address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MinimalMigrateInfo {
    pub transaction_hash: Hash,
}

impl From<MigrateResult> for MinimalMigrateInfo {
    fn from(value: MigrateResult) -> Self {
        MinimalMigrateInfo {
            transaction_hash: value.transaction_hash,
        }
    }
}
