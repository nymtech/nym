// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::errors::ClientStorageError;
use crate::storage::ClientStorage;
use async_trait::async_trait;
use nym_client_core::client::base_client::storage::gateway_details::{
    GatewayDetailsStore, PersistedGatewayDetails,
};
use nym_client_core::client::base_client::storage::MixnetClientStorage;
use nym_client_core::client::key_manager::persistence::KeyStore;
use nym_client_core::client::key_manager::KeyManager;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use wasm_utils::console_log;

// temporary until other variants are properly implemented (probably it should get changed into `ClientStorage`
// implementing all traits and everything getting combined
pub struct FullWasmClientStorage {
    pub(crate) keys_and_gateway_store: ClientStorage,
    pub(crate) reply_storage: browser_backend::Backend,
    pub(crate) credential_storage: EphemeralCredentialStorage,
}

impl MixnetClientStorage for FullWasmClientStorage {
    type KeyStore = ClientStorage;
    type ReplyStore = browser_backend::Backend;
    type CredentialStore = EphemeralCredentialStorage;

    type GatewayDetailsStore = ClientStorage;

    fn into_runtime_stores(self) -> (Self::ReplyStore, Self::CredentialStore) {
        (self.reply_storage, self.credential_storage)
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

    fn gateway_details_store(&self) -> &Self::GatewayDetailsStore {
        &self.keys_and_gateway_store
    }
}

#[async_trait(?Send)]
impl KeyStore for ClientStorage {
    type StorageError = ClientStorageError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        console_log!("attempting to load cryptographic keys...");

        // all keys implement `ZeroizeOnDrop`, so if we return an Error, whatever was already loaded will be cleared
        let identity_keypair = self.must_read_identity_keypair().await?;
        let encryption_keypair = self.must_read_encryption_keypair().await?;
        let ack_keypair = self.must_read_ack_key().await?;
        let gateway_shared_key = self.must_read_gateway_shared_key().await?;

        Ok(KeyManager::from_keys(
            identity_keypair,
            encryption_keypair,
            gateway_shared_key,
            ack_keypair,
        ))
    }

    async fn store_keys(&self, keys: &KeyManager) -> Result<(), Self::StorageError> {
        console_log!("attempting to store cryptographic keys...");

        self.store_identity_keypair(&keys.identity_keypair())
            .await?;
        self.store_encryption_keypair(&keys.encryption_keypair())
            .await?;
        self.store_ack_key(&keys.ack_key()).await?;
        self.store_gateway_shared_key(&keys.gateway_shared_key())
            .await
    }
}

#[async_trait(?Send)]
impl GatewayDetailsStore for ClientStorage {
    type StorageError = ClientStorageError;

    async fn load_gateway_details(&self) -> Result<PersistedGatewayDetails, Self::StorageError> {
        self.must_read_gateway_details().await
    }

    async fn store_gateway_details(
        &self,
        details: &PersistedGatewayDetails,
    ) -> Result<(), Self::StorageError> {
        self.store_gateway_details(details).await
    }
}
