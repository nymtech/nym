// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use js_sys::Promise;
use nym_client_core::client::key_manager::{persistence::KeyStore, KeyManager};
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_client::SharedKeys;
use nym_sphinx::acknowledgements::AckKey;
use std::sync::Arc;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::storage::error::StorageError;
use wasm_utils::storage::{IdbVersionChangeEvent, WasmStorage};
use wasm_utils::{console_log, simple_js_error, PromisableResult};
use zeroize::Zeroizing;

const STORAGE_NAME_PREFIX: &str = "wasm-client-storage";
const STORAGE_VERSION: u32 = 1;

// v1 tables
mod v1 {
    // stores
    pub const KEYS_STORE: &str = "keys";

    // keys
    pub const ED25519_IDENTITY_KEYPAIR: &str = "ed25519_identity_keypair";
    pub const X25519_ENCRYPTION_KEYPAIR: &str = "x25519_encryption_key";

    // TODO: for those we could actually use the subtle crypto storage
    pub const AES128CTR_ACK_KEY: &str = "aes128ctr_ack_key";
    pub const AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS: &str = "aes128ctr_blake3_hmac_gateway_keys";
}

#[derive(Debug, Error)]
pub enum ClientStorageError {
    #[error("failed to use the storage: {source}")]
    StorageError {
        #[from]
        source: StorageError,
    },

    #[error("{typ} cryptographic key is not available in storage")]
    CryptoKeyNotInStorage { typ: String },
}

impl From<ClientStorageError> for JsValue {
    fn from(value: ClientStorageError) -> Self {
        simple_js_error(value.to_string())
    }
}

#[wasm_bindgen]
pub struct ClientStorage {
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) inner: Arc<WasmStorage>,
}

#[wasm_bindgen]
impl ClientStorage {
    fn db_name(client_id: &str) -> String {
        format!("{STORAGE_NAME_PREFIX}-{client_id}")
    }

    pub(crate) async fn new_async(
        client_id: &str,
        passphrase: Option<String>,
    ) -> Result<Self, ClientStorageError> {
        let name = Self::db_name(client_id);

        // make sure the password is zeroized when no longer used, especially if we error out.
        // special care must be taken on JS side to ensure it's correctly used there.
        let passphrase = Zeroizing::new(passphrase);

        let migrate_fn = Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
            // Even if the web-sys bindings expose the version as a f64, the IndexedDB API
            // works with an unsigned integer.
            // See <https://github.com/rustwasm/wasm-bindgen/issues/1149>
            let old_version = evt.old_version() as u32;

            if old_version < 1 {
                // migrating to version 1
                let db = evt.db();

                db.create_object_store(v1::KEYS_STORE)?;
            }

            Ok(())
        });

        let inner = WasmStorage::new(
            &name,
            STORAGE_VERSION,
            migrate_fn,
            passphrase.as_ref().map(|p| p.as_bytes()),
        )
        .await?;

        Ok(ClientStorage {
            inner: Arc::new(inner),
            name,
        })
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(client_id: String, passphrase: String) -> Promise {
        future_to_promise(async move {
            Self::new_async(&client_id, Some(passphrase))
                .await
                .into_promise_result()
        })
    }

    pub fn new_unencrypted(client_id: String) -> Promise {
        future_to_promise(async move {
            Self::new_async(&client_id, None)
                .await
                .into_promise_result()
        })
    }

    async fn may_read_identity_keypair(
        &self,
    ) -> Result<Option<identity::KeyPair>, ClientStorageError> {
        self.inner
            .read_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
            )
            .await
            .map_err(Into::into)
    }

    async fn may_read_encryption_keypair(
        &self,
    ) -> Result<Option<encryption::KeyPair>, ClientStorageError> {
        self.inner
            .read_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
            )
            .await
            .map_err(Into::into)
    }

    async fn may_read_ack_key(&self) -> Result<Option<AckKey>, ClientStorageError> {
        self.inner
            .read_value(v1::KEYS_STORE, JsValue::from_str(v1::AES128CTR_ACK_KEY))
            .await
            .map_err(Into::into)
    }

    async fn may_read_gateway_shared_key(&self) -> Result<Option<SharedKeys>, ClientStorageError> {
        self.inner
            .read_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS),
            )
            .await
            .map_err(Into::into)
    }

    async fn must_read_identity_keypair(&self) -> Result<identity::KeyPair, ClientStorageError> {
        self.may_read_identity_keypair()
            .await?
            .ok_or(ClientStorageError::CryptoKeyNotInStorage {
                typ: v1::ED25519_IDENTITY_KEYPAIR.to_string(),
            })
    }

    async fn must_read_encryption_keypair(
        &self,
    ) -> Result<encryption::KeyPair, ClientStorageError> {
        self.may_read_encryption_keypair()
            .await?
            .ok_or(ClientStorageError::CryptoKeyNotInStorage {
                typ: v1::X25519_ENCRYPTION_KEYPAIR.to_string(),
            })
    }

    async fn must_read_ack_key(&self) -> Result<AckKey, ClientStorageError> {
        self.may_read_ack_key()
            .await?
            .ok_or(ClientStorageError::CryptoKeyNotInStorage {
                typ: v1::AES128CTR_ACK_KEY.to_string(),
            })
    }

    async fn must_read_gateway_shared_key(&self) -> Result<SharedKeys, ClientStorageError> {
        self.may_read_gateway_shared_key()
            .await?
            .ok_or(ClientStorageError::CryptoKeyNotInStorage {
                typ: v1::AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS.to_string(),
            })
    }

    async fn store_identity_keypair(
        &self,
        keypair: &identity::KeyPair,
    ) -> Result<(), ClientStorageError> {
        self.inner
            .store_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
                keypair,
            )
            .await
            .map_err(Into::into)
    }

    async fn store_encryption_keypair(
        &self,
        keypair: &encryption::KeyPair,
    ) -> Result<(), ClientStorageError> {
        self.inner
            .store_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
                keypair,
            )
            .await
            .map_err(Into::into)
    }

    async fn store_ack_key(&self, key: &AckKey) -> Result<(), ClientStorageError> {
        self.inner
            .store_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::AES128CTR_ACK_KEY),
                key,
            )
            .await
            .map_err(Into::into)
    }

    async fn store_gateway_shared_key(&self, key: &SharedKeys) -> Result<(), ClientStorageError> {
        self.inner
            .store_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS),
                key,
            )
            .await
            .map_err(Into::into)
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
