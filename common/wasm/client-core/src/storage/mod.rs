// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmCoreError;
use crate::storage::wasm_client_traits::{v1, v2, WasmClientStorage};
use async_trait::async_trait;
use js_sys::{Array, Promise};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_storage::traits::BaseWasmStorage;
use wasm_storage::{IdbVersionChangeEvent, WasmStorage};
use wasm_utils::error::{simple_js_error, PromisableResult};
use zeroize::Zeroizing;

pub mod core_client_traits;
mod types;
pub mod wasm_client_traits;

const STORAGE_NAME_PREFIX: &str = "wasm-client-storage";
const STORAGE_VERSION: u32 = 2;

#[wasm_bindgen]
pub struct ClientStorage {
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) inner: WasmStorage,
}

#[wasm_bindgen]
impl ClientStorage {
    fn db_name(client_id: &str) -> String {
        format!("{STORAGE_NAME_PREFIX}-{client_id}")
    }

    pub async fn new_async(
        client_id: &str,
        passphrase: Option<String>,
    ) -> Result<ClientStorage, WasmCoreError> {
        let name = Self::db_name(client_id);

        // make sure the password is zeroized when no longer used, especially if we error out.
        // special care must be taken on JS side to ensure it's correctly used there.
        let passphrase = Zeroizing::new(passphrase);

        let migrate_fn = Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
            // Even if the web-sys bindings expose the version as a f64, the IndexedDB API
            // works with an unsigned integer.
            // See <https://github.com/rustwasm/wasm-bindgen/issues/1149>
            let old_version = evt.old_version() as u32;
            let db = evt.db();

            if old_version < 1 {
                // migrating to version 2

                db.create_object_store(v1::KEYS_STORE)?;
                db.create_object_store(v1::CORE_STORE)?;

                db.create_object_store(v2::GATEWAY_REGISTRATIONS_ACTIVE_GATEWAY_STORE)?;
                db.create_object_store(v2::GATEWAY_REGISTRATIONS_REGISTERED_GATEWAYS_STORE)?;
            }

            // version 1 -> unimplemented migration
            if old_version < 2 {
                return Err(simple_js_error("this client is incompatible with existing storage. please initialise it again."));
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

        Ok(ClientStorage { inner, name })
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
}

#[async_trait(?Send)]
impl BaseWasmStorage for ClientStorage {
    type StorageError = WasmCoreError;

    async fn exists(db_name: &str) -> Result<bool, Self::StorageError> {
        Ok(WasmStorage::exists(db_name).await?)
    }

    async fn read_value<T, K>(&self, store: &str, key: K) -> Result<Option<T>, Self::StorageError>
    where
        T: DeserializeOwned,
        K: JsCast,
    {
        Ok(self.inner.read_value(store, key).await?)
    }

    async fn store_value<T, K>(
        &self,
        store: &str,
        key: K,
        value: &T,
    ) -> Result<(), Self::StorageError>
    where
        T: Serialize,
        K: JsCast,
    {
        Ok(self.inner.store_value(store, key, value).await?)
    }

    async fn remove_value<K>(&self, store: &str, key: K) -> Result<(), Self::StorageError>
    where
        K: JsCast,
    {
        Ok(self.inner.remove_value(store, key).await?)
    }

    async fn has_value<K>(&self, store: &str, key: K) -> Result<bool, Self::StorageError>
    where
        K: JsCast,
    {
        Ok(self.inner.has_value(store, key).await?)
    }

    async fn key_count<K>(&self, store: &str, key: K) -> Result<u32, Self::StorageError>
    where
        K: JsCast,
    {
        Ok(self.inner.key_count(store, key).await?)
    }

    async fn get_all_keys(&self, store: &str) -> Result<Array, Self::StorageError> {
        Ok(self.inner.get_all_keys(store).await?)
    }
}

#[async_trait(?Send)]
impl WasmClientStorage for ClientStorage {
    type StorageError = WasmCoreError;
}
