use std::fs;
// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    error::NetworkManagerError,
    manager::{
        contract::{Account, Contract, LoadedNymContracts},
        network::{LoadedNetwork, Network, SpecialAddresses},
        node::NymNode,
        storage::manager::StorageManager,
    },
};
use sqlx::ConnectOptions;
use std::path::Path;
use tracing::{error, info};
use url::Url;
use zeroize::Zeroizing;

mod manager;
mod models;

#[derive(Clone)]
pub(crate) struct NetworkManagerStorage {
    manager: StorageManager,
}

impl NetworkManagerStorage {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, NetworkManagerError> {
        let database_path = database_path.as_ref();
        info!(
            "attempting to initialise storage at {}",
            database_path.display()
        );

        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // TODO: we can inject here more stuff based on our nym-api global config
        // struct. Maybe different pool size or timeout intervals?
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true)
            .disable_statement_logging();

        // TODO: do we want auto_vacuum ?

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");

        let storage = NetworkManagerStorage {
            manager: StorageManager { connection_pool },
        };

        Ok(storage)
    }

    pub(crate) async fn metadata_set(&self) -> Result<bool, NetworkManagerError> {
        Ok(self.manager.metadata_set().await?)
    }

    pub(crate) async fn get_master_mnemonic(
        &self,
    ) -> Result<Option<bip39::Mnemonic>, NetworkManagerError> {
        Ok(self
            .manager
            .get_master_mnemonic()
            .await?
            .map(|m| m.parse())
            .transpose()?)
    }

    pub(crate) async fn get_rpc_endpoint(&self) -> Result<Option<Url>, NetworkManagerError> {
        Ok(self
            .manager
            .get_rpc_endpoint()
            .await?
            .map(|m| m.parse())
            .transpose()?)
    }

    pub(crate) async fn get_latest_network_name(&self) -> Result<String, NetworkManagerError> {
        let Some(id) = self.manager.get_latest_network_id().await? else {
            return Err(NetworkManagerError::NoNetworksInitialised);
        };
        Ok(self.manager.get_network_name(id).await?)
    }

    pub(crate) async fn set_initial_metadata(
        &self,
        master_mnemonic: &bip39::Mnemonic,
        rpc_endpoint: &Url,
    ) -> Result<(), NetworkManagerError> {
        let master = Zeroizing::new(master_mnemonic.to_string());
        Ok(self
            .manager
            .set_initial_metadata(master.as_str(), rpc_endpoint.as_str())
            .await?)
    }

    async fn persist_contract(&self, contract: &Contract) -> Result<i64, NetworkManagerError> {
        Ok(self
            .manager
            .save_contract(
                &contract.name,
                contract.init_info()?.contract_address.as_ref(),
                contract.admin()?.address.as_ref(),
            )
            .await?)
    }

    async fn persist_mixnode(
        &self,
        node: &NymNode,
        network_id: i64,
    ) -> Result<(), NetworkManagerError> {
        Ok(self
            .manager
            .save_node(
                &node.identity_key,
                network_id,
                "mixnode",
                node.owner.address.as_ref(),
            )
            .await?)
    }

    async fn persist_gateway(
        &self,
        node: &NymNode,
        network_id: i64,
    ) -> Result<(), NetworkManagerError> {
        Ok(self
            .manager
            .save_node(
                &node.identity_key,
                network_id,
                "gateway",
                node.owner.address.as_ref(),
            )
            .await?)
    }

    async fn persist_account(&self, account: &Account) -> Result<(), NetworkManagerError> {
        let as_str = Zeroizing::new(account.mnemonic.to_string());
        Ok(self
            .manager
            .save_account(account.address.as_ref(), as_str.as_str())
            .await?)
    }

    pub(crate) async fn persist_mixnodes(
        &self,
        nodes: &[NymNode],
        network_id: i64,
    ) -> Result<(), NetworkManagerError> {
        for node in nodes {
            self.persist_account(&node.owner).await?;
            self.persist_mixnode(node, network_id).await?;
        }
        Ok(())
    }

    pub(crate) async fn persist_gateways(
        &self,
        nodes: &[NymNode],
        network_id: i64,
    ) -> Result<(), NetworkManagerError> {
        for node in nodes {
            self.persist_account(&node.owner).await?;
            self.persist_gateway(node, network_id).await?;
        }
        Ok(())
    }

    pub(crate) async fn persist_network(
        &self,
        network: &Network,
    ) -> Result<(), NetworkManagerError> {
        self.persist_account(network.contracts.mixnet.admin()?)
            .await?;
        self.persist_account(network.contracts.vesting.admin()?)
            .await?;
        self.persist_account(network.contracts.ecash.admin()?)
            .await?;
        self.persist_account(network.contracts.cw3_multisig.admin()?)
            .await?;
        self.persist_account(network.contracts.cw4_group.admin()?)
            .await?;
        self.persist_account(network.contracts.dkg.admin()?).await?;

        self.persist_account(&network.auxiliary_addresses.mixnet_rewarder)
            .await?;
        self.persist_account(&network.auxiliary_addresses.ecash_holding_account)
            .await?;

        let mixnet_id = self.persist_contract(&network.contracts.mixnet).await?;
        let vesting_id = self.persist_contract(&network.contracts.vesting).await?;
        let ecash_id = self.persist_contract(&network.contracts.ecash).await?;
        let cw3_multisig_id = self
            .persist_contract(&network.contracts.cw3_multisig)
            .await?;
        let cw4_group_id = self.persist_contract(&network.contracts.cw4_group).await?;
        let dkg_id = self.persist_contract(&network.contracts.dkg).await?;

        let network_id = self
            .manager
            .save_network(
                &network.name,
                network.created_at,
                mixnet_id,
                vesting_id,
                ecash_id,
                cw3_multisig_id,
                cw4_group_id,
                dkg_id,
                network.auxiliary_addresses.mixnet_rewarder.address.as_ref(),
                network
                    .auxiliary_addresses
                    .ecash_holding_account
                    .address
                    .as_ref(),
            )
            .await?;

        self.manager.save_latest_network_id(network_id).await?;

        Ok(())
    }

    pub(crate) async fn try_load_network(
        &self,
        name: &str,
    ) -> Result<LoadedNetwork, NetworkManagerError> {
        let base_network = self.manager.load_network(name).await?;
        let rpc_endpoint = self
            .get_rpc_endpoint()
            .await?
            .ok_or_else(|| NetworkManagerError::RpcEndpointNotSet)?;

        Ok(LoadedNetwork {
            id: base_network.id,
            name: base_network.name,
            rpc_endpoint,
            created_at: base_network.created_at,
            contracts: LoadedNymContracts {
                mixnet: self
                    .manager
                    .load_contract(base_network.mixnet_contract_id)
                    .await?
                    .try_into()?,
                vesting: self
                    .manager
                    .load_contract(base_network.vesting_contract_id)
                    .await?
                    .try_into()?,
                ecash: self
                    .manager
                    .load_contract(base_network.ecash_contract_id)
                    .await?
                    .try_into()?,
                cw3_multisig: self
                    .manager
                    .load_contract(base_network.cw3_multisig_contract_id)
                    .await?
                    .try_into()?,
                cw4_group: self
                    .manager
                    .load_contract(base_network.cw4_group_contract_id)
                    .await?
                    .try_into()?,
                dkg: self
                    .manager
                    .load_contract(base_network.dkg_contract_id)
                    .await?
                    .try_into()?,
            },
            auxiliary_addresses: SpecialAddresses {
                ecash_holding_account: self
                    .manager
                    .load_account(&base_network.ecash_holding_account_address)
                    .await?
                    .try_into()?,
                mixnet_rewarder: self
                    .manager
                    .load_account(&base_network.rewarder_address)
                    .await?
                    .try_into()?,
            },
        })
    }
}
