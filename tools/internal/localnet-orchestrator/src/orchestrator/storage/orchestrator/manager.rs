// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::orchestrator::storage::orchestrator::models::{
    LocalnetMetadata, RawAccount, RawAuthorisedNetworkMonitor, RawAuxiliaryAccounts, RawContract,
    RawLocalnetContracts, RawNymApi, RawNymNode, RawNyxd, StoredMetadata,
};

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

#[allow(dead_code)]
impl StorageManager {
    pub(crate) fn into_connection_pool(self) -> sqlx::SqlitePool {
        self.connection_pool
    }

    pub(crate) async fn save_latest_network_id(
        &self,
        latest_network_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE metadata SET latest_network_id = ?",
            latest_network_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn save_latest_nyxd_id(&self, latest_nyxd_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE metadata SET latest_nyxd_id = ?", latest_nyxd_id)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn get_metadata(&self) -> Result<StoredMetadata, sqlx::Error> {
        sqlx::query_as("SELECT * FROM metadata")
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_localnet_metadata(&self, name: String) -> Result<i64, sqlx::Error> {
        let localnet_id = sqlx::query!("INSERT INTO localnet_metadata (name) VALUES (?)", name,)
            .execute(&self.connection_pool)
            .await?
            .last_insert_rowid();
        Ok(localnet_id)
    }

    pub(crate) async fn load_localnet_metadata(
        &self,
        localnet_id: i64,
    ) -> Result<LocalnetMetadata, sqlx::Error> {
        sqlx::query_as("SELECT * FROM localnet_metadata WHERE id = ?")
            .bind(localnet_id)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn load_localnet_metadata_by_name(
        &self,
        name: &str,
    ) -> Result<LocalnetMetadata, sqlx::Error> {
        sqlx::query_as("SELECT * FROM localnet_metadata WHERE name = ?")
            .bind(name)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_nyxd_details(
        &self,
        rpc_endpoint: String,
        master_address: String,
    ) -> Result<i64, sqlx::Error> {
        let nyxd_id = sqlx::query!(
            "INSERT INTO nyxd (rpc_endpoint, master_address) VALUES (?, ?)",
            rpc_endpoint,
            master_address
        )
        .execute(&self.connection_pool)
        .await?
        .last_insert_rowid();
        Ok(nyxd_id)
    }

    pub(crate) async fn load_nyxd_details(&self, nyxd_id: i64) -> Result<RawNyxd, sqlx::Error> {
        sqlx::query_as("SELECT * FROM nyxd WHERE id = ?")
            .bind(nyxd_id)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn load_nyxd_by_master_address(
        &self,
        address: &str,
    ) -> Result<RawNyxd, sqlx::Error> {
        sqlx::query_as("SELECT * FROM nyxd WHERE master_address = ?")
            .bind(address)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_account(
        &self,
        address: &str,
        mnemonic: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO account (address, mnemonic) VALUES (?, ?)",
            address,
            mnemonic
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn load_account(&self, address: &str) -> Result<RawAccount, sqlx::Error> {
        sqlx::query_as("SELECT * FROM account WHERE address = ?")
            .bind(address)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_contract(
        &self,
        name: &str,
        address: &str,
        admin_address: &str,
    ) -> Result<i64, sqlx::Error> {
        let id = sqlx::query!(
            "INSERT INTO contract (name, address, admin_address) VALUES (?, ?, ?)",
            name,
            address,
            admin_address
        )
        .execute(&self.connection_pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    pub(crate) async fn load_contract(&self, id: i64) -> Result<RawContract, sqlx::Error> {
        sqlx::query_as("SELECT * FROM contract WHERE id = ?")
            .bind(id)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_authorised_network_monitor(
        &self,
        network_id: i64,
        address: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO authorised_network_monitor (network_id, address) VALUES (?, ?)",
            network_id,
            address,
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn load_authorised_network_monitors(
        &self,
        network_id: i64,
    ) -> Result<Vec<RawAuthorisedNetworkMonitor>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM authorised_network_monitor WHERE network_id = ?")
            .bind(network_id)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_auxiliary_accounts(
        &self,
        network_id: i64,
        rewarder_address: &str,
        ecash_holding_account_address: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO localnet_auxiliary_accounts (network_id, rewarder_address, ecash_holding_account_address) VALUES (?, ?, ?)",
            network_id,
            rewarder_address,
            ecash_holding_account_address
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn load_auxiliary_accounts(
        &self,
        network_id: i64,
    ) -> Result<RawAuxiliaryAccounts, sqlx::Error> {
        sqlx::query_as("SELECT * FROM localnet_auxiliary_accounts WHERE network_id = ?")
            .bind(network_id)
            .fetch_one(&self.connection_pool)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn save_localnet_contracts(
        &self,
        metadata_id: i64,
        mixnet_id: i64,
        vesting_id: i64,
        ecash_id: i64,
        cw3_id: i64,
        cw4_id: i64,
        dkg_id: i64,
        performance_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO localnet_contracts (
                    metadata_id,
                    mixnet_contract_id,
                    vesting_contract_id,
                    ecash_contract_id,
                    cw3_multisig_contract_id,
                    cw4_group_contract_id,
                    dkg_contract_id,
                    performance_contract_id

                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            metadata_id,
            mixnet_id,
            vesting_id,
            ecash_id,
            cw3_id,
            cw4_id,
            dkg_id,
            performance_id,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn load_localnet_contracts(
        &self,
        id: i64,
    ) -> Result<RawLocalnetContracts, sqlx::Error> {
        sqlx::query_as("SELECT * FROM localnet_contracts WHERE metadata_id = ?")
            .bind(id)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_nym_api(
        &self,
        network_id: i64,
        endpoint: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO nym_api (network_id, endpoint) VALUES (?, ?)",
            network_id,
            endpoint
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn load_nym_api(&self, network_id: i64) -> Result<RawNymApi, sqlx::Error> {
        sqlx::query_as("SELECT * FROM nym_api WHERE network_id = ?")
            .bind(network_id)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn load_gateway_nym_nodes(
        &self,
        network_id: i64,
    ) -> Result<Vec<RawNymNode>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM nym_node WHERE network_id = ? AND gateway IS FALSE")
            .bind(network_id)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn load_mix_nym_nodes(
        &self,
        network_id: i64,
    ) -> Result<Vec<RawNymNode>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM nym_node WHERE network_id = ? AND gateway IS NOT FALSE")
            .bind(network_id)
            .fetch_all(&self.connection_pool)
            .await
    }

    pub(crate) async fn save_nym_node(
        &self,
        node_id: i64,
        identity_key: &str,
        private_identity_key: &str,
        network_id: i64,
        owner_address: &str,
        gateway: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO nym_node (node_id, identity_key, private_identity_key, network_id, owner_address, gateway) VALUES (?, ?, ?, ?, ?, ?)",
            node_id,
            identity_key,
            private_identity_key,
            network_id,
            owner_address,
            gateway,
        )
            .execute(&self.connection_pool)
            .await?;

        Ok(())
    }
}
