// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use indexed_db_futures::prelude::*;
use indexed_db_futures::web_sys::DomException;
use js_sys::Promise;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{simple_js_error, PromisableResult};

const STORAGE_NAME_PREFIX: &str = "wasm-client-storage";
const STORAGE_VERSION: u32 = 1;

type DummyValue = String;

// v1 tables
mod v1 {
    // stores
    pub const KEYS_STORE: &str = "keys";

    // keys
    pub const IDENTITY: &str = "identity";
    // pub const ENCRYPTION: &str = "encryption";
    // pub const GATEWAY_SHARED_KEY_PREFIX: &str = "gateway-shared-key";
    // pub const ACK: &str = "ack";
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
pub struct TestStorage {
    pub(crate) name: String,
    pub(crate) inner: Arc<TestStorageInner>,
}

pub(crate) struct TestStorageInner {
    pub(crate) inner: IdbDatabase,

    // TODO (maybe):
    store_cipher: Option<()>,
}

impl TestStorageInner {
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

    async fn read(&self) -> Result<Option<DummyValue>, StorageError> {
        self.inner
            .transaction_on_one_with_mode(v1::KEYS_STORE, IdbTransactionMode::Readonly)?
            .object_store(v1::KEYS_STORE)?
            .get(&JsValue::from_str(v1::IDENTITY))?
            .await?
            .map(|raw| self.deserialize_value(raw))
            .transpose()
    }

    async fn store(&self, value: DummyValue) -> Result<(), StorageError> {
        self.inner
            .transaction_on_one_with_mode(v1::KEYS_STORE, IdbTransactionMode::Readwrite)?
            .object_store(v1::KEYS_STORE)?
            .put_key_val_owned(
                JsValue::from_str(v1::IDENTITY),
                &self.serialize_value(&value)?,
            )?
            .into_future()
            .await
            .map_err(Into::into)
    }
}

impl Drop for TestStorageInner {
    fn drop(&mut self) {
        // Must release the database access manually as it's not done when
        // dropping it.
        self.inner.close();
    }
}

#[wasm_bindgen]
impl TestStorage {
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

        Ok(TestStorage {
            name,
            inner: Arc::new(TestStorageInner {
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

    pub fn read(&self) -> Promise {
        let this = Arc::clone(&self.inner);
        future_to_promise(async move { this.read().await.into_promise_result() })
    }

    pub fn store(&self, value: DummyValue) -> Promise {
        let this = Arc::clone(&self.inner);
        future_to_promise(async move {
            this.store(value)
                .await
                .map(|_| JsValue::NULL)
                .into_promise_result()
        })
    }
}
