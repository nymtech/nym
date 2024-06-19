// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use crate::error::NetworkManagerError;
use nym_mixnet_contract_common::ContractBuildInformation;
use nym_validator_client::nyxd::cosmwasm_client::types::{
    ContractCodeId, InstantiateResult, MigrateResult, UploadResult,
};
use nym_validator_client::nyxd::{AccountId, Hash};
use nym_validator_client::DirectSecp256k1HdWallet;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LoadedNymContracts {
    pub(crate) mixnet: LoadedContract,
    pub(crate) vesting: LoadedContract,
    pub(crate) ecash: LoadedContract,
    pub(crate) cw3_multisig: LoadedContract,
    pub(crate) cw4_group: LoadedContract,
    pub(crate) dkg: LoadedContract,
}

impl From<NymContracts> for LoadedNymContracts {
    fn from(value: NymContracts) -> Self {
        LoadedNymContracts {
            mixnet: value.mixnet.into(),
            vesting: value.vesting.into(),
            ecash: value.ecash.into(),
            cw3_multisig: value.cw3_multisig.into(),
            cw4_group: value.cw4_group.into(),
            dkg: value.dkg.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NymContracts {
    pub(crate) mixnet: Contract,
    pub(crate) vesting: Contract,
    pub(crate) ecash: Contract,
    pub(crate) cw3_multisig: Contract,
    pub(crate) cw4_group: Contract,
    pub(crate) dkg: Contract,
}

impl NymContracts {
    pub(crate) fn fake_iter(&self) -> Vec<&Contract> {
        vec![
            &self.mixnet,
            &self.vesting,
            &self.ecash,
            &self.cw3_multisig,
            &self.cw4_group,
            &self.dkg,
        ]
    }

    pub(crate) fn fake_iter_mut(&mut self) -> Vec<&mut Contract> {
        vec![
            &mut self.mixnet,
            &mut self.vesting,
            &mut self.ecash,
            &mut self.cw3_multisig,
            &mut self.cw4_group,
            &mut self.dkg,
        ]
    }

    pub(crate) fn count(&self) -> usize {
        6
    }

    pub(crate) fn discover_paths<P: AsRef<Path>>(
        &mut self,
        base_path: P,
    ) -> Result<(), NetworkManagerError> {
        // just look in the base path, don't traverse
        for entry_res in base_path.as_ref().read_dir()? {
            let entry = entry_res?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };

            if name.ends_with(".wasm") {
                if name.contains("mixnet") {
                    self.mixnet.wasm_path = Some(entry.path())
                }
                if name.contains("vesting") {
                    self.vesting.wasm_path = Some(entry.path())
                }
                if name.contains("ecash") {
                    self.ecash.wasm_path = Some(entry.path())
                }
                if name.contains("cw4") {
                    self.cw4_group.wasm_path = Some(entry.path())
                }
                if name.contains("cw3") {
                    self.cw3_multisig.wasm_path = Some(entry.path())
                }
                if name.contains("dkg") {
                    self.dkg.wasm_path = Some(entry.path())
                }
            }
        }

        if let Some(no_path) = self.fake_iter().iter().find(|c| c.wasm_path.is_none()) {
            return Err(NetworkManagerError::ContractWasmNotFound {
                name: no_path.name.clone(),
            });
        }

        Ok(())
    }
}

impl Default for NymContracts {
    fn default() -> Self {
        NymContracts {
            mixnet: Contract::new("mixnet"),
            vesting: Contract::new("vesting"),
            ecash: Contract::new("ecash"),
            cw4_group: Contract::new("cw4_group"),
            cw3_multisig: Contract::new("cw3_multisig"),
            dkg: Contract::new("dkg"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Account {
    pub(crate) address: AccountId,
    pub(crate) mnemonic: bip39::Mnemonic,
}

impl Account {
    pub(crate) fn new() -> Account {
        let mnemonic = bip39::Mnemonic::generate(24).unwrap();
        // sure, we're using hardcoded prefix, but realistically this will never change
        let wallet = DirectSecp256k1HdWallet::from_mnemonic("n", mnemonic.clone());
        let acc = wallet.try_derive_accounts().unwrap().pop().unwrap();
        Account {
            address: acc.address,
            mnemonic,
        }
    }

    pub(crate) fn address(&self) -> AccountId {
        self.address.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LoadedContract {
    pub(crate) name: String,
    pub(crate) address: AccountId,
    pub(crate) admin_address: AccountId,
    pub(crate) admin_mnemonic: bip39::Mnemonic,
}

impl From<Contract> for LoadedContract {
    fn from(value: Contract) -> Self {
        let admin = value.admin.expect("no admin set");
        LoadedContract {
            name: value.name,
            address: value.init_info.expect("uninitialised").contract_address,
            admin_address: admin.address,
            admin_mnemonic: admin.mnemonic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Contract {
    pub(crate) name: String,
    pub(crate) wasm_path: Option<PathBuf>,
    pub(crate) upload_info: Option<MinimalUploadInfo>,
    pub(crate) admin: Option<Account>,
    pub(crate) init_info: Option<MinimalInitInfo>,
    pub(crate) migrate_info: Option<MinimalMigrateInfo>,
    pub(crate) build_info: Option<ContractBuildInformation>,
}

impl Contract {
    pub(crate) fn new<S: Into<String>>(name: S) -> Self {
        Contract {
            name: name.into(),
            wasm_path: None,
            upload_info: None,
            admin: None,
            init_info: None,
            migrate_info: None,
            build_info: None,
        }
    }

    pub(crate) fn wasm_path(&self) -> Result<&PathBuf, NetworkManagerError> {
        self.wasm_path
            .as_ref()
            .ok_or_else(|| NetworkManagerError::ContractWasmNotFound {
                name: self.name.clone(),
            })
    }

    pub(crate) fn upload_info(&self) -> Result<&MinimalUploadInfo, NetworkManagerError> {
        self.upload_info
            .as_ref()
            .ok_or_else(|| NetworkManagerError::ContractNotUploaded {
                name: self.name.clone(),
            })
    }

    pub(crate) fn admin(&self) -> Result<&Account, NetworkManagerError> {
        self.admin
            .as_ref()
            .ok_or_else(|| NetworkManagerError::ContractAdminNotSet {
                name: self.name.clone(),
            })
    }

    pub(crate) fn init_info(&self) -> Result<&MinimalInitInfo, NetworkManagerError> {
        self.init_info
            .as_ref()
            .ok_or_else(|| NetworkManagerError::ContractNotInitialised {
                name: self.name.clone(),
            })
    }

    #[allow(dead_code)]
    pub(crate) fn build_info(&self) -> Result<&ContractBuildInformation, NetworkManagerError> {
        self.build_info
            .as_ref()
            .ok_or_else(|| NetworkManagerError::ContractNotQueried {
                name: self.name.clone(),
            })
    }

    pub(crate) fn address(&self) -> Result<&AccountId, NetworkManagerError> {
        self.init_info().map(|info| &info.contract_address)
    }
}
