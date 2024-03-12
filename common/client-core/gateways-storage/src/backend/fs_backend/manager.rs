// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backend::fs_backend::error::StorageError,
    types::{
        RawActiveGateway, RawCustomGatewayDetails, RawRegisteredGateway, RawRemoteGatewayDetails,
    },
};
use log::{error, info};
use sqlx::ConnectOptions;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct StorageManager {
    pub connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        // ensure the whole directory structure exists
        if let Some(parent_dir) = database_path.as_ref().parent() {
            std::fs::create_dir_all(parent_dir).map_err(|source| {
                StorageError::DatabasePathUnableToCreateParentDirectory {
                    provided_path: database_path.as_ref().to_path_buf(),
                    source,
                }
            })?;
        }

        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        opts.disable_statement_logging();

        let connection_pool = sqlx::SqlitePool::connect_with(opts)
            .await
            .map_err(|source| {
                error!("Failed to connect to SQLx database: {source}");
                StorageError::DatabaseConnectionError { source }
            })?;

        sqlx::migrate!("./fs_gateways_migrations")
            .run(&connection_pool)
            .await
            .inspect_err(|err| {
                error!("Failed to initialize SQLx database: {err}");
            })?;

        info!("Database migration finished!");
        Ok(StorageManager { connection_pool })
    }

    pub(crate) async fn get_active_gateway(&self) -> Result<RawActiveGateway, sqlx::Error> {
        sqlx::query_as!(
            RawActiveGateway,
            "SELECT active_gateway_id_bs58 FROM active_gateway"
        )
        .fetch_one(&self.connection_pool)
        .await
    }

    pub(crate) async fn set_active_gateway(
        &self,
        gateway_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE active_gateway SET active_gateway_id_bs58 = ?",
            gateway_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn has_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query!("SELECT EXISTS (SELECT 1 FROM registered_gateway WHERE gateway_id_bs58 = ?) AS 'exists'", gateway_id)
            .fetch_one(&self.connection_pool)
            .await
            .map(|result| result.exists == 1)
    }

    pub(crate) async fn maybe_get_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<Option<RawRegisteredGateway>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM registered_gateway WHERE gateway_id_bs58 = ?")
            .bind(gateway_id)
            .fetch_optional(&self.connection_pool)
            .await
    }

    pub(crate) async fn must_get_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<RawRegisteredGateway, sqlx::Error> {
        sqlx::query_as("SELECT * FROM registered_gateway WHERE gateway_id_bs58 = ?")
            .bind(gateway_id)
            .fetch_one(&self.connection_pool)
            .await
    }

    pub(crate) async fn set_registered_gateway(
        &self,
        registered_gateway: &RawRegisteredGateway,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO registered_gateway(gateway_id_bs58, registration_timestamp, gateway_type) 
                VALUES (?, ?, ?)
            "#,
            registered_gateway.gateway_id_bs58,
            registered_gateway.registration_timestamp,
            registered_gateway.gateway_type,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn remove_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM registered_gateway WHERE gateway_id_bs58 = ?",
            gateway_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_remote_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<RawRemoteGatewayDetails, sqlx::Error> {
        sqlx::query_as!(
            RawRemoteGatewayDetails,
            "SELECT * FROM remote_gateway_details WHERE gateway_id_bs58 = ?",
            gateway_id
        )
        .fetch_one(&self.connection_pool)
        .await
    }

    pub(crate) async fn set_remote_gateway_details(
        &self,
        remote: &RawRemoteGatewayDetails,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO remote_gateway_details(gateway_id_bs58, derived_aes128_ctr_blake3_hmac_keys_bs58, gateway_owner_address, gateway_listener, wg_tun_address) 
                VALUES (?, ?, ?, ?, ?)
            "#,
            remote.gateway_id_bs58,
            remote.derived_aes128_ctr_blake3_hmac_keys_bs58,
            remote.gateway_owner_address,
            remote.gateway_listener,
            remote.wg_tun_address,
        )
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn remove_remote_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM remote_gateway_details WHERE gateway_id_bs58 = ?",
            gateway_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_custom_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<RawCustomGatewayDetails, sqlx::Error> {
        sqlx::query_as!(
            RawCustomGatewayDetails,
            "SELECT * FROM custom_gateway_details WHERE gateway_id_bs58 = ?",
            gateway_id
        )
        .fetch_one(&self.connection_pool)
        .await
    }

    pub(crate) async fn set_custom_gateway_details(
        &self,
        custom: &RawCustomGatewayDetails,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO custom_gateway_details(gateway_id_bs58, data) 
                VALUES (?, ?)
            "#,
            custom.gateway_id_bs58,
            custom.data,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn remove_custom_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM custom_gateway_details WHERE gateway_id_bs58 = ?",
            gateway_id
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn registered_gateways(&self) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query!("SELECT gateway_id_bs58 FROM registered_gateway")
            .fetch_all(&self.connection_pool)
            .await
            .map(|records| records.into_iter().map(|r| r.gateway_id_bs58).collect())
    }
}
