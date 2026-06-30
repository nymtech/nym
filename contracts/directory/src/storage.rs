// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, DepsMut, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_directory_contract_common::constants::storage_keys;
use nym_directory_contract_common::msg::InitialLabel;
use nym_directory_contract_common::{
    DirectoryContractError, DirectoryEntry, EntryKey, KnownLabel, LabelConfig,
};
use nym_mixnet_contract_common::NodeId;

pub const NYM_DIRECTORY_CONTRACT_STORAGE: NymDirectoryContractStorage =
    NymDirectoryContractStorage::new();

pub struct NymDirectoryContractStorage {
    /// Admin of the contract; gates privileged operations.
    pub(crate) contract_admin: Admin,

    /// Address of the mixnet contract; used to verify a node id refers to a
    /// real, registered, and bonded node.
    pub(crate) mixnet_contract_address: Item<Addr>,

    pub(crate) sequences: Map<NodeId, u64>,

    pub(crate) allowed_storage_labels: Map<String, LabelConfig>,

    pub(crate) digest_state: Item<[u8; nym_lthash::DIGEST_LEN]>,

    pub(crate) directory_entries: Map<EntryKey, DirectoryEntry>,
}

impl NymDirectoryContractStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        NymDirectoryContractStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT_ADDRESS),
            sequences: Map::new(storage_keys::SEQUENCES),
            allowed_storage_labels: Map::new(storage_keys::ALLOWED_LABELS),
            digest_state: Item::new(storage_keys::DIGEST_STATE),
            directory_entries: Map::new(storage_keys::ENTRIES),
        }
    }

    /// One-time storage initialisation called from the contract's `instantiate`
    /// entry point. Persists the mixnet contract address  
    /// and sets `sender` as the contract admin.
    pub(crate) fn initialise(
        &self,
        deps: DepsMut,
        sender: Addr,
        mixnet_contract_address: Addr,
        initial_labels: Vec<InitialLabel>,
    ) -> Result<(), DirectoryContractError> {
        // 1. set mixnet contract address
        self.mixnet_contract_address
            .save(deps.storage, &mixnet_contract_address)?;

        // 2. save known labels
        for label in KnownLabel::ALL {
            self.allowed_storage_labels.save(
                deps.storage,
                label.as_str().to_string(),
                &label.default_config(),
            )?;
        }

        // 3. save additional, provided, labels (if applicable)
        // if there's an overlap with the known labels,
        // prefer the explicitly provided config
        for initial_label in initial_labels {
            self.allowed_storage_labels.save(
                deps.storage,
                initial_label.label.as_str().to_string(),
                &initial_label.config,
            )?;
        }

        // 4. save contract admin (consumes deps)
        self.contract_admin.set(deps, Some(sender))?;

        Ok(())
    }

    pub(crate) fn current_sequence(
        &self,
        store: &dyn Storage,
        node_id: NodeId,
    ) -> Result<u64, DirectoryContractError> {
        Ok(self.sequences.may_load(store, node_id)?.unwrap_or_default())
    }

    pub(crate) fn increment_account_sequence(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
    ) -> Result<(), DirectoryContractError> {
        let current_sequence = self.current_sequence(store, node_id)?;
        self.sequences
            .save(store, node_id, &(current_sequence + 1))?;
        Ok(())
    }
}

pub mod retrieval_limits {
    //
}
