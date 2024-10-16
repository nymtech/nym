// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::credentials::ticketbook::NodeId;
use crate::error::VpnApiError;
use crate::storage::manager::SqliteStorageManager;
use crate::storage::models::{BlindedShares, MinimalWalletShare};
use nym_compact_ecash::PublicKeyUser;
use nym_credentials::ecash::bandwidth::issuance::Hash;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::ecash::BlindedSignatureResponse;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::ecash_query_client::DepositId;
use nym_validator_client::nyxd::Coin;
use sqlx::ConnectOptions;
use std::fmt::Debug;
use std::path::Path;
use time::{Date, OffsetDateTime};
use tracing::{debug, error, info, instrument};
use uuid::Uuid;
use zeroize::Zeroizing;

mod manager;
pub mod models;

#[derive(Clone)]
pub struct VpnApiStorage {
    pub(crate) storage_manager: SqliteStorageManager,
}

impl VpnApiStorage {
    #[instrument]
    pub async fn init<P: AsRef<Path> + Debug>(database_path: P) -> Result<Self, VpnApiError> {
        debug!("Attempting to connect to database");

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true)
            .disable_statement_logging();

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

        Ok(VpnApiStorage {
            storage_manager: SqliteStorageManager { connection_pool },
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn load_blinded_shares_status_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Option<BlindedShares>, VpnApiError> {
        Ok(self
            .storage_manager
            .load_blinded_shares_status_by_shares_id(id)
            .await?)
    }

    pub(crate) async fn load_wallet_shares_by_shares_id(
        &self,
        id: i64,
    ) -> Result<Vec<MinimalWalletShare>, VpnApiError> {
        Ok(self
            .storage_manager
            .load_wallet_shares_by_shares_id(id)
            .await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn load_blinded_shares_status_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Option<BlindedShares>, VpnApiError> {
        Ok(self
            .storage_manager
            .load_blinded_shares_status_by_device_and_credential_id(device_id, credential_id)
            .await?)
    }

    pub(crate) async fn load_wallet_shares_by_device_and_credential_id(
        &self,
        device_id: &str,
        credential_id: &str,
    ) -> Result<Vec<MinimalWalletShare>, VpnApiError> {
        Ok(self
            .storage_manager
            .load_wallet_shares_by_device_and_credential_id(device_id, credential_id)
            .await?)
    }

    pub(crate) async fn insert_new_pending_async_shares_request(
        &self,
        request: Uuid,
        device_id: &str,
        credential_id: &str,
    ) -> Result<BlindedShares, VpnApiError> {
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
    ) -> Result<BlindedShares, VpnApiError> {
        self.storage_manager
            .update_pending_async_blinded_shares_issued(
                available_shares as i64,
                device_id,
                credential_id,
            )
            .await
    }

    pub(crate) async fn update_pending_async_blinded_shares_error(
        &self,
        available_shares: usize,
        device_id: &str,
        credential_id: &str,
        error: &str,
    ) -> Result<BlindedShares, VpnApiError> {
        self.storage_manager
            .update_pending_async_blinded_shares_error(
                available_shares as i64,
                device_id,
                credential_id,
                error,
            )
            .await
    }

    pub(crate) async fn prune_old_blinded_shares(&self) -> Result<(), VpnApiError> {
        let max_age = OffsetDateTime::now_utc() - time::Duration::days(31);

        self.storage_manager
            .prune_old_partial_blinded_wallets(max_age)
            .await?;
        self.storage_manager
            .prune_old_partial_blinded_wallet_failures(max_age)
            .await?;
        self.storage_manager.prune_old_blinded_shares(max_age).await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_deposit_data(
        &self,
        deposit_id: DepositId,
        deposit_tx_hash: Hash,
        requested_on: OffsetDateTime,
        request: Uuid,
        deposit_amount: Coin,
        client_ecash_pubkey: &PublicKeyUser,
        ed22519_keypair: &ed25519::KeyPair,
    ) -> Result<(), VpnApiError> {
        debug!("inserting deposit data");

        let private_key_bytes = Zeroizing::new(ed22519_keypair.private_key().to_bytes());

        self.storage_manager
            .insert_deposit_data(
                deposit_id,
                deposit_tx_hash.to_string(),
                requested_on,
                request.to_string(),
                deposit_amount.to_string(),
                &client_ecash_pubkey.to_bytes(),
                private_key_bytes.as_ref(),
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn insert_partial_wallet_share(
        &self,
        deposit_id: DepositId,
        epoch_id: EpochId,
        expiration_date: Date,
        node_id: NodeId,
        res: &Result<BlindedSignatureResponse, VpnApiError>,
    ) -> Result<(), VpnApiError> {
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
    ) -> Result<Option<EpochVerificationKey>, VpnApiError> {
        let Some(raw) = self
            .storage_manager
            .get_master_verification_key(epoch_id as i64)
            .await?
        else {
            return Ok(None);
        };

        let deserialised =
            EpochVerificationKey::try_unpack(&raw.serialised_key, raw.serialization_revision)
                .map_err(|err| VpnApiError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    pub(crate) async fn insert_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), VpnApiError> {
        let packed = key.pack();
        Ok(self
            .storage_manager
            .insert_master_verification_key(packed.revision, key.epoch_id as i64, &packed.data)
            .await?)
    }

    pub(crate) async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedCoinIndicesSignatures>, VpnApiError> {
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
        .map_err(|err| VpnApiError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    pub(crate) async fn insert_master_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), VpnApiError> {
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
    ) -> Result<Option<AggregatedExpirationDateSignatures>, VpnApiError> {
        let Some(raw) = self
            .storage_manager
            .get_master_expiration_date_signatures(expiration_date)
            .await?
        else {
            return Ok(None);
        };

        let deserialised = AggregatedExpirationDateSignatures::try_unpack(
            &raw.serialised_signatures,
            raw.serialization_revision,
        )
        .map_err(|err| VpnApiError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    pub(crate) async fn insert_master_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), VpnApiError> {
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
    use rand::rngs::OsRng;
    use rand::RngCore;
    use std::ops::Deref;
    use tempfile::{NamedTempFile, TempPath};

    // create the wrapper so the underlying file gets deleted when it's no longer needed
    struct StorageTestWrapper {
        inner: VpnApiStorage,
        _path: TempPath,
    }

    impl StorageTestWrapper {
        async fn new() -> anyhow::Result<Self> {
            let file = NamedTempFile::new()?;
            let path = file.into_temp_path();

            println!("Creating database at {:?}...", path);

            Ok(StorageTestWrapper {
                inner: VpnApiStorage::init(&path).await?,
                _path: path,
            })
        }

        async fn insert_dummy_deposit(&self, uuid: Uuid) -> anyhow::Result<DepositId> {
            let mut rng = OsRng;
            let deposit_id = rng.next_u32();
            let tx_hash = Hash::Sha256(Default::default());
            let requested_on = OffsetDateTime::now_utc();
            let deposit_amount = Coin::new(1, "ufoomp");
            let client_keypair = KeyPairUser::new();
            let client_ecash_pubkey = &client_keypair.public_key();

            let deposit_keypair = ed25519::KeyPair::new(&mut rng);

            self.inner
                .insert_deposit_data(
                    deposit_id,
                    tx_hash,
                    requested_on,
                    uuid,
                    deposit_amount,
                    client_ecash_pubkey,
                    &deposit_keypair,
                )
                .await?;

            Ok(deposit_id)
        }
    }

    impl Deref for StorageTestWrapper {
        type Target = VpnApiStorage;
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

        storage.insert_dummy_deposit(dummy_uuid).await?;
        let res = storage
            .insert_new_pending_async_shares_request(dummy_uuid, "1234", "1234")
            .await;
        if let Err(e) = &res {
            println!("❌ {}", e);
        }
        assert!(res.is_ok());
        let res = res.unwrap();
        println!("res = {:?}", res);
        assert_eq!(res.status, BlindedSharesStatus::Pending);

        println!("🚀 update_pending_blinded_share_error...");
        let res = storage
            .update_pending_async_blinded_shares_error(0, "1234", "1234", "this is an error")
            .await;
        if let Err(e) = &res {
            println!("❌ {}", e);
        }
        assert!(res.is_ok());
        let res = res.unwrap();
        println!("res = {:?}", res);
        assert!(res.error_message.is_some());
        assert_eq!(res.status, BlindedSharesStatus::Error);

        println!("🚀 update_pending_blinded_share_data...");
        let res = storage
            .update_pending_async_blinded_shares_issued(42, "1234", "1234")
            .await;
        if let Err(e) = &res {
            println!("❌ {}", e);
        }
        assert!(res.is_ok());
        let res = res.unwrap();
        println!("res = {:?}", res);
        assert_eq!(res.status, BlindedSharesStatus::Issued);
        assert!(res.error_message.is_none());

        Ok(())
    }
}
