// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NetworkManagerError;
use crate::helpers::{async_with_progress, default_storage_dir, wasm_code};
use crate::manager::network::LoadedNetwork;
use crate::manager::storage::NetworkManagerStorage;
use bip39::rand::prelude::SliceRandom;
use bip39::rand::thread_rng;
use indicatif::ProgressBar;
use nym_config::defaults::NymNetworkDetails;
use nym_validator_client::nyxd::cosmwasm_client::types::UploadResult;
use nym_validator_client::nyxd::Config;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, QueryHttpRpcNyxdClient};
use std::path::{Path, PathBuf};
use url::Url;
use zeroize::Zeroizing;

mod contract;
mod dkg_skip;
pub(crate) mod env;
mod local_apis;
mod local_client;
mod local_nodes;
pub(crate) mod network;
mod network_init;
pub(crate) mod storage;

pub(crate) struct NetworkManager {
    admin: Zeroizing<bip39::Mnemonic>,
    storage: NetworkManagerStorage,
    rpc_endpoint: Url,
}

impl NetworkManager {
    pub(crate) async fn new<P: AsRef<Path>>(
        database_path: P,
        mnemonic: Option<bip39::Mnemonic>,
        rpc_endpoint: Option<Url>,
    ) -> Result<Self, NetworkManagerError> {
        let storage = NetworkManagerStorage::init(database_path).await?;

        let (mnemonic, rpc_endpoint) = if !storage.metadata_set().await? {
            let mnemonic = mnemonic.ok_or(NetworkManagerError::MnemonicNotSet)?;
            let rpc_endpoint = rpc_endpoint.ok_or(NetworkManagerError::RpcEndpointNotSet)?;

            storage
                .set_initial_metadata(&mnemonic, &rpc_endpoint)
                .await?;
            (mnemonic, rpc_endpoint)
        } else {
            let mnemonic = storage
                .get_master_mnemonic()
                .await?
                .ok_or(NetworkManagerError::MnemonicNotSet)?;

            let rpc_endpoint = storage
                .get_rpc_endpoint()
                .await?
                .ok_or(NetworkManagerError::RpcEndpointNotSet)?;

            (mnemonic, rpc_endpoint)
        };

        Ok(NetworkManager {
            admin: Zeroizing::new(mnemonic),
            storage,
            rpc_endpoint,
        })
    }

    pub fn default_latest_env_file_path(&self) -> PathBuf {
        default_storage_dir().join("latest.env")
    }

    #[allow(unused)]
    pub(crate) fn query_client(
        &self,
        network: &LoadedNetwork,
    ) -> Result<QueryHttpRpcNyxdClient, NetworkManagerError> {
        let network_details = NymNetworkDetails::from(network);
        let config = Config::try_from_nym_network_details(&network_details)?;

        Ok(QueryHttpRpcNyxdClient::connect(
            config,
            self.rpc_endpoint.as_str(),
        )?)
    }

    fn get_network_name(&self, user_provided: Option<String>) -> String {
        user_provided.unwrap_or_else(|| {
            // a hack to get human-readable words without extra deps : )
            let mut rng = thread_rng();

            let words = bip39::Language::English.word_list();
            let first = words.choose(&mut rng).unwrap();
            let second = words.choose(&mut rng).unwrap();
            format!("{first}-{second}")
        })
    }

    async fn upload_contract<P: AsRef<Path>>(
        &self,
        admin: &DirectSigningHttpRpcNyxdClient,
        pb: &ProgressBar,
        path: P,
    ) -> Result<UploadResult, NetworkManagerError> {
        let wasm = wasm_code(path)?;
        let upload_future = admin.upload(wasm, "contract upload from testnet-manager", None);

        async_with_progress(upload_future, pb)
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn load_existing_network(
        &self,
        network_name: Option<String>,
    ) -> Result<LoadedNetwork, NetworkManagerError> {
        let network_name = if let Some(explicit) = network_name {
            explicit
        } else {
            self.storage.get_latest_network_name().await?
        };

        self.storage.try_load_network(&network_name).await
    }
}
