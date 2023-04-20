// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use indexed_db_futures::prelude::*;
use indexed_db_futures::web_sys::DomException;
use js_sys::Promise;
use nym_client_core::client::key_manager::{KeyManager, KeyStore};
use nym_crypto::asymmetric::{encryption, identity};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{simple_js_error, PromisableResult};

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
    pub const AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS_PREFIX: &str =
        "aes128ctr_blake3_hmac_gateway_keys";
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Json(#[from] serde_wasm_bindgen::Error),

    #[error("DomException {name} ({code}): {message}")]
    DomException {
        /// DomException code
        code: u16,
        /// Specific name of the DomException
        name: String,
        /// Message given to the DomException
        message: String,
    },
}

impl From<StorageError> for JsValue {
    fn from(value: StorageError) -> Self {
        simple_js_error(value.to_string())
    }
}

impl From<DomException> for StorageError {
    fn from(value: DomException) -> StorageError {
        StorageError::DomException {
            name: value.name(),
            message: value.message(),
            code: value.code(),
        }
    }
}

#[wasm_bindgen]
pub struct ClientStorage {
    pub(crate) name: String,
    pub(crate) inner: Arc<ClientStorageInner>,
}

pub(crate) struct ClientStorageInner {
    pub(crate) inner: IdbDatabase,

    // TODO (maybe):
    store_cipher: Option<()>,
}

impl ClientStorageInner {
    fn serialize_value<T: Serialize>(&self, value: &T) -> Result<JsValue, StorageError> {
        // if let Some(key) = &self.store_cipher {
        //     // let value = key
        //     //     .encrypt_value(value)
        //     //     .map_err(CryptoStoreError::backend)?;
        //
        //     Ok(serde_wasm_bindgen::to_value(&value)?)
        // } else {
        //     Ok(serde_wasm_bindgen::to_value(&value)?)
        // }
        Ok(serde_wasm_bindgen::to_value(&value)?)
    }

    fn deserialize_value<T: DeserializeOwned>(&self, value: JsValue) -> Result<T, StorageError> {
        // if let Some(key) = &self.store_cipher {
        //     // let value: Vec<u8> = value.into_serde()?;
        //     // key.decrypt_value(&value).map_err(CryptoStoreError::backend)
        //     Ok(serde_wasm_bindgen::from_value(value)?)
        // } else {
        //     Ok(serde_wasm_bindgen::from_value(value)?)
        // }
        Ok(serde_wasm_bindgen::from_value(value)?)
    }

    async fn read_value<T, K>(&self, store: &str, key: K) -> Result<Option<T>, StorageError>
    where
        T: DeserializeOwned,
        K: wasm_bindgen::JsCast,
    {
        self.inner
            .transaction_on_one_with_mode(store, IdbTransactionMode::Readonly)?
            .object_store(store)?
            .get(&key)?
            .await?
            .map(|raw| self.deserialize_value(raw))
            .transpose()
    }

    async fn store_value<T, K>(&self, store: &str, key: K, value: &T) -> Result<(), StorageError>
    where
        T: Serialize,
        K: wasm_bindgen::JsCast,
    {
        self.inner
            .transaction_on_one_with_mode(store, IdbTransactionMode::Readwrite)?
            .object_store(store)?
            .put_key_val_owned(key, &self.serialize_value(&value)?)?
            .into_future()
            .await
            .map_err(Into::into)
    }

    async fn read_identity_keypair(&self) -> Result<Option<identity::KeyPair>, StorageError> {
        self.read_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
        )
        .await
    }

    async fn read_encryption_keypair(&self) -> Result<Option<encryption::KeyPair>, StorageError> {
        self.read_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
        )
        .await
    }

    async fn store_identity_keypair(
        &self,
        keypair: &identity::KeyPair,
    ) -> Result<(), StorageError> {
        self.store_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
            keypair,
        )
        .await
    }

    async fn store_encryption_keypair(
        &self,
        keypair: &encryption::KeyPair,
    ) -> Result<(), StorageError> {
        self.store_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
            keypair,
        )
        .await
    }
}

impl Drop for ClientStorageInner {
    fn drop(&mut self) {
        // Must release the database access manually as it's not done when
        // dropping it.
        self.inner.close();
    }
}

#[wasm_bindgen]
impl ClientStorage {
    fn db_name(client_id: &str) -> String {
        format!("{STORAGE_NAME_PREFIX}-{client_id}")
    }

    async fn new_async(client_id: &str) -> Result<Self, StorageError> {
        let name = Self::db_name(client_id);
        let mut db_req: OpenDbRequest = IdbDatabase::open_u32(&name, STORAGE_VERSION)?;

        db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
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
        }));

        let db: IdbDatabase = db_req.into_future().await?;

        Ok(ClientStorage {
            name,
            inner: Arc::new(ClientStorageInner {
                inner: db,
                store_cipher: None,
            }),
        })
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(client_id: String) -> Promise {
        future_to_promise(async move { Self::new_async(&client_id).await.into_promise_result() })
    }

    // pub fn read(&self) -> Promise {
    //     let this = Arc::clone(&self.inner);
    //     future_to_promise(async move { this.read_value().await.into_promise_result() })
    // }
    //
    // pub fn store(&self, value: DummyValue) -> Promise {
    //     let this = Arc::clone(&self.inner);
    //     future_to_promise(async move {
    //         this.store(value)
    //             .await
    //             .map(|_| JsValue::NULL)
    //             .into_promise_result()
    //     })
    // }
}

#[async_trait(?Send)]
impl KeyStore for ClientStorage {
    type StorageError = StorageError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        todo!()
    }

    async fn store_keys(&self, keys: KeyManager) -> Result<(), Self::StorageError> {
        todo!()
    }
}
