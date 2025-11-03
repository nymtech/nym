// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::account::Account;
use crate::orchestrator::cosmwasm_contract::CosmwasmContract;
use crate::orchestrator::network::{AuxiliaryAccounts, NymContracts, NyxdDetails};
use crate::orchestrator::nym_node::LocalnetNymNode;
use crate::orchestrator::storage::orchestrator::manager::StorageManager;
use crate::orchestrator::storage::orchestrator::models::{LocalnetMetadata, StoredMetadata};
use anyhow::{Context, anyhow};
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use zeroize::Zeroizing;

pub(crate) mod manager;
pub(crate) mod models;

pub(crate) struct LocalnetOrchestratorStorage {
    _storage_path: PathBuf,
    manager: StorageManager,
}

impl LocalnetOrchestratorStorage {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> anyhow::Result<Self> {
        let database_path = database_path.as_ref();
        info!(
            "attempting to initialise storage at {}",
            database_path.display()
        );

        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path)
            .create_if_missing(true)
            .disable_statement_logging();

        let connection_pool = sqlx::SqlitePool::connect_with(opts)
            .await
            .context("db connection failure")?;

        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .context("db migrations failure")?;

        info!("Database migration finished!");

        Ok(LocalnetOrchestratorStorage {
            _storage_path: database_path.to_path_buf(),
            manager: StorageManager { connection_pool },
        })
    }

    pub(crate) async fn stop(self) -> anyhow::Result<PathBuf> {
        let pool = self.manager.into_connection_pool();
        pool.close().await;
        Ok(self._storage_path)
    }

    pub(crate) async fn get_last_created(&self) -> anyhow::Result<StoredMetadata> {
        Ok(self.manager.get_metadata().await?)
    }

    async fn save_account(&self, account: &Account) -> anyhow::Result<()> {
        let as_str = Zeroizing::new(account.mnemonic.to_string());
        Ok(self
            .manager
            .save_account(account.address.as_ref(), as_str.as_str())
            .await?)
    }

    async fn load_account(&self, address: &str) -> anyhow::Result<Account> {
        let raw_account = self.manager.load_account(address).await?;
        raw_account.try_into()
    }

    pub(crate) async fn save_new_localnet_metadata(&self, name: &str) -> anyhow::Result<()> {
        let localnet_id = self
            .manager
            .save_localnet_metadata(name.to_string())
            .await?;
        self.manager.save_latest_network_id(localnet_id).await?;
        Ok(())
    }

    pub(crate) async fn get_localnet_metadata(
        &self,
        db_id: i64,
    ) -> anyhow::Result<LocalnetMetadata> {
        Ok(self.manager.load_localnet_metadata(db_id).await?)
    }

    pub(crate) async fn get_localnet_metadata_by_name(
        &self,
        name: &str,
    ) -> anyhow::Result<LocalnetMetadata> {
        Ok(self.manager.load_localnet_metadata_by_name(name).await?)
    }

    pub(crate) async fn get_nyxd_details(&self, db_id: i64) -> anyhow::Result<NyxdDetails> {
        let raw_details = self.manager.load_nyxd_details(db_id).await?;
        let raw_account = self
            .manager
            .load_account(&raw_details.master_address)
            .await?;
        Ok(NyxdDetails {
            rpc_endpoint: raw_details.rpc_endpoint.parse()?,
            master_account: raw_account.try_into()?,
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn get_nyxd_details_by_master_address(
        &self,
        address: &str,
    ) -> anyhow::Result<NyxdDetails> {
        let raw_details = self.manager.load_nyxd_by_master_address(address).await?;
        let raw_account = self.manager.load_account(address).await?;
        Ok(NyxdDetails {
            rpc_endpoint: raw_details.rpc_endpoint.parse()?,
            master_account: raw_account.try_into()?,
        })
    }

    pub(crate) async fn save_nyxd_details(
        &self,
        nyxd_details: &NyxdDetails,
    ) -> anyhow::Result<i64> {
        // 1. save master mnemonic
        self.save_account(&nyxd_details.master_account).await?;

        // 2. save actual nyxd information
        let nyxd_id = self
            .manager
            .save_nyxd_details(
                nyxd_details.rpc_endpoint.to_string(),
                nyxd_details.master_account.address.to_string(),
            )
            .await?;

        // 3. update global metadata
        self.manager.save_latest_nyxd_id(nyxd_id).await?;
        Ok(nyxd_id)
    }

    async fn load_cosmwasm_contract(&self, id: i64) -> anyhow::Result<CosmwasmContract> {
        let raw = self.manager.load_contract(id).await?;
        let admin = self.load_account(&raw.admin_address).await?;

        Ok(CosmwasmContract {
            name: raw.name,
            address: raw
                .address
                .parse()
                .map_err(|err| anyhow!("malformed address: {err}"))?,
            admin,
        })
    }

    async fn save_cosmwasm_contract(&self, contract: &CosmwasmContract) -> anyhow::Result<i64> {
        // 1. save admin details
        self.save_account(&contract.admin).await?;

        // 2. persist actual contract information
        let contract_id = self
            .manager
            .save_contract(
                &contract.name,
                contract.address.as_ref(),
                contract.admin.address.as_ref(),
            )
            .await?;

        Ok(contract_id)
    }

    async fn save_authorised_network_monitor(
        &self,
        network_id: i64,
        account: &Account,
    ) -> anyhow::Result<()> {
        self.save_account(account).await?;
        self.manager
            .save_authorised_network_monitor(network_id, account.address.as_ref())
            .await?;
        Ok(())
    }

    pub(crate) async fn save_auxiliary_accounts(
        &self,
        localnet_human_name: &str,
        aux: &AuxiliaryAccounts,
    ) -> anyhow::Result<()> {
        // 1. retrieve associated metadata id based on the network name
        let metadata = self
            .manager
            .load_localnet_metadata_by_name(localnet_human_name)
            .await?;

        // 2. save accounts
        self.save_account(&aux.mixnet_rewarder).await?;
        self.save_account(&aux.ecash_holding_account).await?;
        for network_monitor in &aux.network_monitor {
            // 3. and network monitors
            self.save_authorised_network_monitor(metadata.id, network_monitor)
                .await?;
        }

        // 4. create the container row
        self.manager
            .save_auxiliary_accounts(
                metadata.id,
                aux.mixnet_rewarder.address.as_ref(),
                aux.ecash_holding_account.address.as_ref(),
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn load_auxiliary_accounts(
        &self,
        localnet_id: i64,
    ) -> anyhow::Result<AuxiliaryAccounts> {
        let raw = self.manager.load_auxiliary_accounts(localnet_id).await?;
        let mixnet_rewarder = self.load_account(&raw.rewarder_address).await?;
        let ecash_holding_account = self
            .load_account(&raw.ecash_holding_account_address)
            .await?;
        let raw_monitors = self
            .manager
            .load_authorised_network_monitors(localnet_id)
            .await?;
        let mut network_monitor = Vec::with_capacity(raw_monitors.len());
        for raw_monitor in raw_monitors {
            network_monitor.push(self.load_account(&raw_monitor.address).await?)
        }
        Ok(AuxiliaryAccounts {
            mixnet_rewarder,
            network_monitor,
            ecash_holding_account,
        })
    }

    pub(crate) async fn save_localnet_contracts(
        &self,
        localnet_human_name: &str,
        contracts: &NymContracts,
    ) -> anyhow::Result<()> {
        // 1. retrieve associated metadata id based on the network name
        let metadata = self
            .manager
            .load_localnet_metadata_by_name(localnet_human_name)
            .await?;

        // 2. save contracts data
        let mixnet_id = self.save_cosmwasm_contract(&contracts.mixnet).await?;
        let vesting_id = self.save_cosmwasm_contract(&contracts.vesting).await?;
        let ecash_id = self.save_cosmwasm_contract(&contracts.ecash).await?;
        let cw3_multisig_id = self.save_cosmwasm_contract(&contracts.cw3_multisig).await?;
        let cw4_group_id = self.save_cosmwasm_contract(&contracts.cw4_group).await?;
        let dkg_id = self.save_cosmwasm_contract(&contracts.dkg).await?;
        let performance_id = self.save_cosmwasm_contract(&contracts.performance).await?;

        // 3. clump it all together
        self.manager
            .save_localnet_contracts(
                metadata.id,
                mixnet_id,
                vesting_id,
                ecash_id,
                cw3_multisig_id,
                cw4_group_id,
                dkg_id,
                performance_id,
            )
            .await?;

        Ok(())
    }

    pub(crate) async fn load_localnet_contracts(
        &self,
        localnet_id: i64,
    ) -> anyhow::Result<NymContracts> {
        let raw = self.manager.load_localnet_contracts(localnet_id).await?;

        let mixnet = self.load_cosmwasm_contract(raw.mixnet_contract_id).await?;
        let vesting = self.load_cosmwasm_contract(raw.vesting_contract_id).await?;
        let ecash = self.load_cosmwasm_contract(raw.ecash_contract_id).await?;
        let cw3_multisig = self
            .load_cosmwasm_contract(raw.cw3_multisig_contract_id)
            .await?;
        let cw4_group = self
            .load_cosmwasm_contract(raw.cw4_group_contract_id)
            .await?;
        let dkg = self.load_cosmwasm_contract(raw.dkg_contract_id).await?;
        let performance = self
            .load_cosmwasm_contract(raw.performance_contract_id)
            .await?;

        Ok(NymContracts {
            mixnet,
            vesting,
            ecash,
            cw3_multisig,
            cw4_group,
            dkg,
            performance,
        })
    }

    pub(crate) async fn save_nym_api_details(
        &self,
        localnet_human_name: &str,
        nym_api_endpoint: &str,
    ) -> anyhow::Result<()> {
        // 1. retrieve associated metadata id based on the network name
        let metadata = self
            .manager
            .load_localnet_metadata_by_name(localnet_human_name)
            .await?;

        self.manager
            .save_nym_api(metadata.id, nym_api_endpoint)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_nym_api_details(&self, localnet_id: i64) -> anyhow::Result<url::Url> {
        let raw = self.manager.load_nym_api(localnet_id).await?;
        Ok(raw.endpoint.parse()?)
    }

    pub(crate) async fn save_nym_node_details(
        &self,
        localnet_human_name: &str,
        node: &LocalnetNymNode,
    ) -> anyhow::Result<()> {
        // 1. retrieve associated metadata id based on the network name
        let metadata = self
            .manager
            .load_localnet_metadata_by_name(localnet_human_name)
            .await?;

        // 2. save account
        self.save_account(&node.owner).await?;

        // 3. save node details
        self.manager
            .save_nym_node(
                node.id as i64,
                &node.identity.public_key().to_base58_string(),
                &node.identity.private_key().to_base58_string(),
                metadata.id,
                node.owner.address.as_ref(),
                node.gateway,
            )
            .await?;
        Ok(())
    }
}
