// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use crate::manager::storage::models::{RawAccount, RawContract, RawNetwork};
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    pub(crate) async fn metadata_set(&self) -> Result<bool, sqlx::Error> {
        Ok(sqlx::query("SELECT id FROM metadata")
            .fetch_optional(&self.connection_pool)
            .await?
            .is_some())
    }

    pub(crate) async fn get_master_mnemonic(&self) -> Result<Option<String>, sqlx::Error> {
        sqlx::query!("SELECT master_mnemonic FROM metadata")
            .fetch_optional(&self.connection_pool)
            .await
            .map(|maybe_record| maybe_record.map(|r| r.master_mnemonic))
    }

    pub(crate) async fn get_rpc_endpoint(&self) -> Result<Option<String>, sqlx::Error> {
        sqlx::query!("SELECT rpc_endpoint FROM metadata")
            .fetch_optional(&self.connection_pool)
            .await
            .map(|maybe_record| maybe_record.map(|r| r.rpc_endpoint))
    }

    pub(crate) async fn get_latest_network_id(&self) -> Result<Option<i64>, sqlx::Error> {
        let maybe_record = sqlx::query!("SELECT latest_network_id FROM metadata")
            .fetch_optional(&self.connection_pool)
            .await?;
        Ok(maybe_record.and_then(|r| r.latest_network_id))
    }

    pub(crate) async fn get_network_name(&self, network_id: i64) -> Result<String, sqlx::Error> {
        sqlx::query!("SELECT name FROM network WHERE id = ?", network_id)
            .fetch_one(&self.connection_pool)
            .await
            .map(|record| record.name)
    }

    pub(crate) async fn set_initial_metadata(
        &self,
        master_mnemonic: &str,
        rpc_endpoint: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO metadata (id, master_mnemonic, rpc_endpoint) VALUES (0, ?, ?)",
            master_mnemonic,
            rpc_endpoint
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn save_network(
        &self,
        name: &str,
        created_at: OffsetDateTime,
        mixnet_id: i64,
        vesting_id: i64,
        ecash_id: i64,
        cw3_id: i64,
        cw4_id: i64,
        dkg_id: i64,
        rewarder_address: &str,
        ecash_holding_address: &str,
    ) -> Result<i64, sqlx::Error> {
        let network_id = sqlx::query!(
            r#"
                INSERT INTO network (
                    name,
                    created_at,
                    mixnet_contract_id,
                    vesting_contract_id,
                    ecash_contract_id,
                    cw3_multisig_contract_id,
                    cw4_group_contract_id,
                    dkg_contract_id,
                    rewarder_address,
                    ecash_holding_account_address
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            name,
            created_at,
            mixnet_id,
            vesting_id,
            ecash_id,
            cw3_id,
            cw4_id,
            dkg_id,
            rewarder_address,
            ecash_holding_address,
        )
        .execute(&self.connection_pool)
        .await?
        .last_insert_rowid();
        Ok(network_id)
    }

    pub(crate) async fn load_network(&self, name: &str) -> Result<RawNetwork, sqlx::Error> {
        sqlx::query_as("SELECT * FROM network WHERE name = ?")
            .bind(name)
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
        sqlx::query_as(
            r#"
            SELECT t1.id, t1.name, t1.address, t1.admin_address, t2.mnemonic
            FROM contract t1
            JOIN account t2 ON t1.admin_address = t2.address
            WHERE t1.id = ?"#,
        )
        .bind(id)
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

    pub(crate) async fn save_node(
        &self,
        identity_key: &str,
        network_id: i64,
        bonded_type: &str,
        owner_address: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO node(identity_key, network_id, bonded_type, owner_address)
            VALUES (?, ?, ?, ?) 
        "#,
            identity_key,
            network_id,
            bonded_type,
            owner_address
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}
