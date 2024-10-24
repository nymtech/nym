// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::BaseClientConfig;
use crate::error::WasmCoreError;
use crate::helpers::setup_reply_surb_storage_backend;
use crate::storage::wasm_client_traits::WasmClientStorage;
use crate::storage::ClientStorage;
use async_trait::async_trait;
use nym_client_core::client::base_client::storage::{
    gateways_storage::{ActiveGateway, GatewayRegistration, GatewaysDetailsStore},
    MixnetClientStorage,
};
use nym_client_core::client::key_manager::persistence::KeyStore;
use nym_client_core::client::key_manager::ClientKeys;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_gateway_client::SharedSymmetricKey;
use wasm_utils::console_log;

// temporary until other variants are properly implemented (probably it should get changed into `ClientStorage`
// implementing all traits and everything getting combined
pub struct FullWasmClientStorage {
    pub(crate) keys_and_gateway_store: ClientStorage,
    pub(crate) reply_storage: browser_backend::Backend,
    pub(crate) credential_storage: EphemeralCredentialStorage,
}

impl FullWasmClientStorage {
    // TODO: I dont like that base_config type, it should be something wasm-specific.
    pub fn new(base_config: &BaseClientConfig, base_storage: ClientStorage) -> Self {
        FullWasmClientStorage {
            keys_and_gateway_store: base_storage,
            reply_storage: setup_reply_surb_storage_backend(base_config.debug.reply_surbs),
            credential_storage: EphemeralCredentialStorage::default(),
        }
    }
}

impl MixnetClientStorage for FullWasmClientStorage {
    type KeyStore = ClientStorage;
    type ReplyStore = browser_backend::Backend;
    type CredentialStore = EphemeralCredentialStorage;

    type GatewaysDetailsStore = ClientStorage;

    fn into_runtime_stores(
        self,
    ) -> (
        Self::ReplyStore,
        Self::CredentialStore,
        Self::GatewaysDetailsStore,
    ) {
        (
            self.reply_storage,
            self.credential_storage,
            self.keys_and_gateway_store,
        )
    }

    fn key_store(&self) -> &Self::KeyStore {
        &self.keys_and_gateway_store
    }

    fn reply_store(&self) -> &Self::ReplyStore {
        &self.reply_storage
    }

    fn credential_store(&self) -> &Self::CredentialStore {
        &self.credential_storage
    }

    fn gateway_details_store(&self) -> &Self::GatewaysDetailsStore {
        &self.keys_and_gateway_store
    }
}

#[async_trait(?Send)]
impl KeyStore for ClientStorage {
    type StorageError = WasmCoreError;

    async fn load_keys(&self) -> Result<ClientKeys, Self::StorageError> {
        console_log!("attempting to load cryptographic keys...");

        // all keys implement `ZeroizeOnDrop`, so if we return an Error, whatever was already loaded will be cleared
        let identity_keypair = self.must_read_identity_keypair().await?;
        let encryption_keypair = self.must_read_encryption_keypair().await?;
        let ack_keypair = self.must_read_ack_key().await?;

        Ok(ClientKeys::from_keys(
            identity_keypair,
            encryption_keypair,
            ack_keypair,
        ))
    }

    async fn store_keys(&self, keys: &ClientKeys) -> Result<(), Self::StorageError> {
        console_log!("attempting to store cryptographic keys...");

        self.store_identity_keypair(&keys.identity_keypair())
            .await?;
        self.store_encryption_keypair(&keys.encryption_keypair())
            .await?;
        self.store_ack_key(&keys.ack_key()).await?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl GatewaysDetailsStore for ClientStorage {
    type StorageError = WasmCoreError;

    async fn active_gateway(&self) -> Result<ActiveGateway, Self::StorageError> {
        let raw_active = self.get_active_gateway_id().await?;
        let registration = match raw_active.active_gateway_id_bs58 {
            None => None,
            Some(gateway_id) => Some(self.load_gateway_details(&gateway_id).await?),
        };

        Ok(ActiveGateway { registration })
    }

    async fn set_active_gateway(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        <Self as WasmClientStorage>::set_active_gateway(self, Some(gateway_id)).await?;
        Ok(())
    }

    async fn all_gateways(&self) -> Result<Vec<GatewayRegistration>, Self::StorageError> {
        todo!()
        // let identities = self.all
    }

    async fn has_gateway_details(&self, gateway_id: &str) -> Result<bool, Self::StorageError> {
        self.has_registered_gateway(gateway_id).await
    }

    async fn load_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<GatewayRegistration, Self::StorageError> {
        Ok(self
            .must_get_registered_gateway(gateway_id)
            .await?
            .try_into()?)
    }

    async fn store_gateway_details(
        &self,
        details: &GatewayRegistration,
    ) -> Result<(), Self::StorageError> {
        let raw_registration = details.into();
        self.store_registered_gateway(&raw_registration).await
    }

    async fn upgrade_stored_remote_gateway_key(
        &self,
        gateway_id: PublicKey,
        updated_key: &SharedSymmetricKey,
    ) -> Result<(), Self::StorageError> {
        self.update_remote_gateway_key(
            &gateway_id.to_base58_string(),
            None,
            Some(updated_key.as_bytes()),
        )
        .await
    }

    async fn remove_gateway_details(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        self.remove_registered_gateway(gateway_id).await
    }
}
