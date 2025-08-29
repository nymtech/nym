// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::credentials::ticketbook::NodeId;
use crate::deposits_buffer::helpers::{BufferedDeposit, PerformedDeposits};
use crate::error::CredentialProxyError;
use crate::storage::manager::SqliteStorageManager;
use crate::storage::models::{BlindedShares, MinimalWalletShare};
use nym_compact_ecash::PublicKeyUser;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_validator_client::ecash::BlindedSignatureResponse;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::ecash_query_client::DepositId;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use sqlx::ConnectOptions;
use std::fmt::Debug;
use std::path::Path;
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tracing::log::LevelFilter;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

mod manager;
pub mod models;

#[derive(Clone)]
pub struct CredentialProxyStorage {
    pub(crate) storage_manager: SqliteStorageManager,
}

impl CredentialProxyStorage {
    #[instrument]
    pub async fn init<P: AsRef<Path> + Debug>(
        database_path: P,
    ) -> Result<Self, CredentialProxyError> {
        debug!("Attempting to connect to database");

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path)
            .create_if_missing(true)
            .log_statements(LevelFilter::Trace)
            .log_slow_statements(LevelFilter::Warn, Duration::from_millis(250));

        let pool_opts = sqlx::sqlite::SqlitePoolOptions::new()
            .min_connections(5)
            .max_connections(25)
            .acquire_timeout(Duration::from_secs(60));

        let connection_pool = match pool_opts.connect_with(opts).await {
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

        Ok(CredentialProxyStorage {
            storage_manager: SqliteStorageManager { connection_pool },
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn load_blinded_shares_status_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Option<BlindedShares>, CredentialProxyError> {
        Ok(self
            .storage_manager
            .load_blinded_shares_status_by_shares_id(id)
            .await?)
    }

    pub(crate) async fn load_wallet_shares_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Vec<MinimalWalletShare>, CredentialProxyError> {
        Ok(self
            .storage_manager
            .load_wallet_shares_by_shares_id(id)
            .await?)
    }

    pub(crate) async fn load_shares_error_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Option<String>, CredentialProxyError> {
        Ok(self
            .storage_manager
            .load_shares_error_by_device_by_shares_id(id)
            .await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn load_blinded_shares_status_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Option<BlindedShares>, CredentialProxyError> {
        Ok(self
            .storage_manager
            .load_blinded_shares_status_by_device_and_credential_id(device_id, credential_id)
            .await?)
    }

    pub(crate) async fn load_wallet_shares_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Vec<MinimalWalletShare>, CredentialProxyError> {
        Ok(self
            .storage_manager
            .load_wallet_shares_by_device_and_credential_id(device_id, credential_id)
            .await?)
    }

    pub(crate) async fn load_shares_error_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Option<String>, CredentialProxyError> {
        Ok(self
            .storage_manager
            .load_shares_error_by_device_and_credential_id(device_id, credential_id)
            .await?)
    }

    pub(crate) async fn insert_new_pending_async_shares_request(
        &self,
        request: Uuid,
        device_id: &str,
        credential_id: &str,
    ) -> Result<BlindedShares, CredentialProxyError> {
        Ok(self
            .storage_manager
            .insert_new_pending_async_shares_request(request.to_string(), device_id, credential_id)
            .await?)
    }

    pub(crate) async fn update_pending_async_blinded_shares_issued(
        &self,
        available_shares: usize,
        device_id: &str,
        credential_id: &str,
    ) -> Result<BlindedShares, CredentialProxyError> {
        Ok(self
            .storage_manager
            .update_pending_async_blinded_shares_issued(
                available_shares as i64,
                device_id,
                credential_id,
            )
            .await?)
    }

    pub(crate) async fn update_pending_async_blinded_shares_error(
        &self,
        available_shares: usize,
        device_id: &str,
        credential_id: &str,
        error: &str,
    ) -> Result<BlindedShares, CredentialProxyError> {
        Ok(self
            .storage_manager
            .update_pending_async_blinded_shares_error(
                available_shares as i64,
                device_id,
                credential_id,
                error,
            )
            .await?)
    }

    pub(crate) async fn prune_old_blinded_shares(&self) -> Result<(), CredentialProxyError> {
        let max_age = OffsetDateTime::now_utc() - time::Duration::days(31);

        self.storage_manager
            .prune_old_partial_blinded_wallets(max_age)
            .await?;
        self.storage_manager
            .prune_old_partial_blinded_wallet_failures(max_age)
            .await?;
        self.storage_manager
            .prune_old_blinded_shares(max_age)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_new_deposits(
        &self,
        deposits: &PerformedDeposits,
    ) -> Result<(), CredentialProxyError> {
        debug!("inserting {} deposits data", deposits.deposits_data.len());

        self.storage_manager
            .insert_new_deposits(deposits.to_storable())
            .await?;
        Ok(())
    }

    pub(crate) async fn load_unused_deposits(
        &self,
    ) -> Result<Vec<BufferedDeposit>, CredentialProxyError> {
        self.storage_manager
            .load_unused_deposits()
            .await?
            .into_iter()
            .map(|deposit| deposit.try_into())
            .collect()
    }

    pub(crate) async fn insert_deposit_usage(
        &self,
        deposit_id: DepositId,
        requested_on: OffsetDateTime,
        client_pubkey: PublicKeyUser,
        request_uuid: Uuid,
    ) -> Result<(), CredentialProxyError> {
        self.storage_manager
            .insert_deposit_usage(
                deposit_id,
                requested_on,
                client_pubkey.to_bytes(),
                request_uuid.to_string(),
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_deposit_usage_error(
        &self,
        deposit_id: DepositId,
        error: String,
    ) -> Result<(), CredentialProxyError> {
        self.storage_manager
            .insert_deposit_usage_error(deposit_id, error)
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_partial_wallet_share(
        &self,
        deposit_id: DepositId,
        epoch_id: EpochId,
        expiration_date: Date,
        node_id: NodeId,
        res: &Result<BlindedSignatureResponse, CredentialProxyError>,
    ) -> Result<(), CredentialProxyError> {
        debug!("inserting partial wallet share");
        let now = OffsetDateTime::now_utc();

        match res {
            Ok(share) => {
                self.storage_manager
                    .insert_partial_wallet_share(
                        deposit_id,
                        epoch_id as i64,
                        expiration_date,
                        node_id as i64,
                        now,
                        &share.blinded_signature.to_bytes(),
                    )
                    .await?;
            }
            Err(err) => {
                self.storage_manager
                    .insert_partial_wallet_issuance_failure(
                        deposit_id,
                        epoch_id as i64,
                        expiration_date,
                        node_id as i64,
                        now,
                        err.to_string(),
                    )
                    .await?
            }
        }
        Ok(())
    }

    pub(crate) async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochVerificationKey>, CredentialProxyError> {
        let Some(raw) = self
            .storage_manager
            .get_master_verification_key(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let deserialised =
            EpochVerificationKey::try_unpack(&raw.serialised_key, raw.serialization_revision)
                .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    pub(crate) async fn insert_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), CredentialProxyError> {
        let packed = key.pack();
        Ok(self
            .storage_manager
            .insert_master_verification_key(packed.revision, key.epoch_id as i64, &packed.data)
            .await?)
    }

    pub(crate) async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedCoinIndicesSignatures>, CredentialProxyError> {
        let Some(raw) = self
            .storage_manager
            .get_master_coin_index_signatures(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let deserialised = AggregatedCoinIndicesSignatures::try_unpack(
            &raw.serialised_signatures,
            raw.serialization_revision,
        )
        .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    pub(crate) async fn insert_master_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), CredentialProxyError> {
        let packed = signatures.pack();
        self.storage_manager
            .insert_master_coin_index_signatures(
                packed.revision,
                signatures.epoch_id as i64,
                &packed.data,
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedExpirationDateSignatures>, CredentialProxyError> {
        let Some(raw) = self
            .storage_manager
            .get_master_expiration_date_signatures(expiration_date, epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let deserialised = AggregatedExpirationDateSignatures::try_unpack(
            &raw.serialised_signatures,
            raw.serialization_revision,
        )
        .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    pub(crate) async fn insert_master_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), CredentialProxyError> {
        let packed = signatures.pack();
        self.storage_manager
            .insert_master_expiration_date_signatures(
                packed.revision,
                signatures.epoch_id as i64,
                signatures.expiration_date,
                &packed.data,
            )
            .await?;
        Ok(())
    }
}

#[allow(clippy::expect_used)]
#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::helpers;
    use crate::storage::models::BlindedSharesStatus;
    use nym_compact_ecash::scheme::keygen::KeyPairUser;
    use nym_crypto::asymmetric::ed25519;
    use nym_validator_client::nyxd::{Coin, Hash};
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::ops::Deref;
    use tempfile::{NamedTempFile, TempPath};

    // create the wrapper so the underlying file gets deleted when it's no longer needed
    struct StorageTestWrapper {
        inner: CredentialProxyStorage,
        _path: TempPath,
    }

    impl StorageTestWrapper {
        async fn new() -> anyhow::Result<Self> {
            let file = NamedTempFile::new()?;
            let path = file.into_temp_path();

            println!("Creating database at {path:?}...");

            Ok(StorageTestWrapper {
                inner: CredentialProxyStorage::init(&path).await?,
                _path: path,
            })
        }

        async fn insert_dummy_used_deposit(&self, uuid: Uuid) -> anyhow::Result<DepositId> {
            let mut rng = OsRng;
            let deposit_id = rng.next_u32();
            let tx_hash = Hash::Sha256(Default::default());
            let requested_on = OffsetDateTime::now_utc();
            let deposit_amount = Coin::new(1, "ufoomp");
            let client_keypair = KeyPairUser::new();
            let client_ecash_pubkey = &client_keypair.public_key();

            let deposit_key = ed25519::PrivateKey::new(&mut rng);

            self.inner
                .insert_new_deposits(&PerformedDeposits {
                    deposits_data: vec![BufferedDeposit {
                        deposit_id,
                        ed25519_private_key: deposit_key,
                    }],
                    tx_hash,
                    requested_on,
                    deposit_amount,
                })
                .await?;
            self.inner
                .insert_deposit_usage(deposit_id, requested_on, *client_ecash_pubkey, uuid)
                .await?;

            Ok(deposit_id)
        }
    }

    impl Deref for StorageTestWrapper {
        type Target = CredentialProxyStorage;
        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    async fn get_storage() -> anyhow::Result<StorageTestWrapper> {
        StorageTestWrapper::new().await
    }

    #[tokio::test]
    async fn test_creation() -> anyhow::Result<()> {
        let storage = get_storage().await;
        assert!(storage.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_add() -> anyhow::Result<()> {
        let storage = get_storage().await?;

        let dummy_uuid = helpers::random_uuid();
        println!("🚀 insert_pending_blinded_share...");

        storage.insert_dummy_used_deposit(dummy_uuid).await?;
        let res = storage
            .insert_new_pending_async_shares_request(dummy_uuid, "1234", "1234")
            .await;
        if let Err(e) = &res {
            println!("❌ {e}");
        }
        assert!(res.is_ok());
        let res = res?;
        println!("res = {res:?}");
        assert_eq!(res.status, BlindedSharesStatus::Pending);

        println!("🚀 update_pending_blinded_share_error...");
        let res = storage
            .update_pending_async_blinded_shares_error(0, "1234", "1234", "this is an error")
            .await;
        if let Err(e) = &res {
            println!("❌ {e}");
        }
        assert!(res.is_ok());
        let res = res?;
        println!("res = {res:?}");
        assert!(res.error_message.is_some());
        assert_eq!(res.status, BlindedSharesStatus::Error);

        println!("🚀 update_pending_blinded_share_data...");
        let res = storage
            .update_pending_async_blinded_shares_issued(42, "1234", "1234")
            .await;
        if let Err(e) = &res {
            println!("❌ {e}");
        }
        assert!(res.is_ok());
        let res = res?;
        println!("res = {res:?}");
        assert_eq!(res.status, BlindedSharesStatus::Issued);
        assert!(res.error_message.is_none());

        Ok(())
    }
}
