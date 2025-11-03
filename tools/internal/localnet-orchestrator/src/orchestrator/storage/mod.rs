// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::storage::cache::LocalnetCache;
use crate::orchestrator::storage::orchestrator::LocalnetOrchestratorStorage;
use nym_config::{NYM_DIR, must_get_home};
use nym_mixnet_contract_common::NodeId;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) mod cache;
pub(crate) mod orchestrator;

const NYXD_CONTAINER_DATA_DIR: &str = "nyxd";
const NYM_API_CONTAINER_DATA_DIR: &str = "nym-api";
const NYM_NODE_CONTAINER_DATA_DIR_PREFIX: &str = "nym-node";

const COSMWASM_CONTRACTS_DIR: &str = "contracts";

pub(crate) fn default_storage_dir() -> PathBuf {
    must_get_home().join(NYM_DIR).join("localnet-orchestrator")
}

pub(crate) fn default_cache_dir() -> PathBuf {
    default_storage_dir().join(".cache")
}

pub(crate) fn default_orchestrator_db_file() -> PathBuf {
    default_storage_dir().join("network-data.sqlite")
}

pub(crate) struct LocalnetStorage {
    // db with mnemonics and whatnot (to be copied from testnet manager)
    // you may ask wtf is it a sqlite db, isn't it an overkill?
    // in a way yes, but I needed some way to persist a bunch of data - mnemonics, addresses, ids, etc.
    // and shuffling multiple files around turned to be very annoying, very quickly,
    // so instead I grouped it in a single sqlite db file
    orchestrator_data: LocalnetOrchestratorStorage,

    data_cache: LocalnetCache,

    localnet_directory: PathBuf,
}

impl LocalnetStorage {
    pub fn new(
        localnet_directory: impl AsRef<Path>,
        cache_dir: impl AsRef<Path>,
        orchestrator_data: LocalnetOrchestratorStorage,
    ) -> anyhow::Result<Self> {
        let localnet_directory = localnet_directory.as_ref();
        let cache_dir = cache_dir.as_ref();

        fs::create_dir_all(localnet_directory)?;

        Ok(LocalnetStorage {
            orchestrator_data,
            data_cache: LocalnetCache::new(cache_dir)?,
            localnet_directory: localnet_directory.to_path_buf(),
        })
    }

    pub(crate) fn cosmwasm_contracts_directory(&self) -> PathBuf {
        self.localnet_directory.join(COSMWASM_CONTRACTS_DIR)
    }

    pub(crate) fn nyxd_container_data_directory(&self) -> PathBuf {
        self.localnet_directory.join(NYXD_CONTAINER_DATA_DIR)
    }

    pub(crate) fn nym_api_container_data_directory(&self) -> PathBuf {
        self.localnet_directory.join(NYM_API_CONTAINER_DATA_DIR)
    }

    pub(crate) fn global_env_file(&self) -> PathBuf {
        self.localnet_directory.join("localnet.env")
    }

    pub(crate) fn nym_node_container_data_directory(&self, id: NodeId) -> PathBuf {
        self.localnet_directory
            .join(format!("{NYM_NODE_CONTAINER_DATA_DIR_PREFIX}-{id}"))
    }

    pub(crate) fn nym_node_ed25519_private_key_path(&self, id: NodeId) -> PathBuf {
        self.nym_node_container_data_directory(id)
            .join("data")
            .join("ed25519_identity")
    }

    fn nym_api_data_directory(&self) -> PathBuf {
        self.nym_api_container_data_directory().join("data")
    }

    pub(crate) fn nym_api_ecash_key(&self) -> PathBuf {
        self.nym_api_data_directory().join("coconut.pem")
    }

    pub(crate) fn nym_api_ed25519_private_key(&self) -> PathBuf {
        self.nym_api_data_directory().join("private_identity.pem")
    }

    pub(crate) fn nym_api_ed25519_public_key(&self) -> PathBuf {
        self.nym_api_data_directory().join("public_identity.pem")
    }

    pub(crate) fn orchestrator(&self) -> &LocalnetOrchestratorStorage {
        &self.orchestrator_data
    }

    pub(crate) fn data_cache(&self) -> &LocalnetCache {
        &self.data_cache
    }

    pub(crate) fn localnet_directory(&self) -> &Path {
        &self.localnet_directory
    }

    pub(crate) fn into_orchestrator_storage(self) -> LocalnetOrchestratorStorage {
        self.orchestrator_data
    }
}
