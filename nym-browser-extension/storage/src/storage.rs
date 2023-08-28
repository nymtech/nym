// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ExtensionStorageError;
use js_sys::Promise;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::storage::{IdbVersionChangeEvent, WasmStorage};
use wasm_utils::{check_promise_result, PromisableResult, PromisableResultError};
use zeroize::Zeroizing;

const STORAGE_NAME: &str = "nym-wallet-extension";
const STORAGE_VERSION: u32 = 1;

// v1 tables
mod v1 {
    // stores
    pub const MNEMONICS_STORE: &str = "mnemonics";
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct ExtensionStorage {
    inner: Arc<WasmStorage>,
}

fn db_migration() -> Box<dyn Fn(&IdbVersionChangeEvent) -> Result<(), JsValue>> {
    Box::new(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
        // Even if the web-sys bindings expose the version as a f64, the IndexedDB API
        // works with an unsigned integer.
        // See <https://github.com/rustwasm/wasm-bindgen/issues/1149>
        let old_version = evt.old_version() as u32;

        if old_version < 1 {
            // migrating to version 1
            let db = evt.db();

            db.create_object_store(v1::MNEMONICS_STORE)?;
        }

        Ok(())
    })
}

#[wasm_bindgen]
impl ExtensionStorage {
    pub(crate) async fn new_async(passphrase: String) -> Result<Self, ExtensionStorageError> {
        // make sure the password is zeroized when no longer used, especially if we error out.
        // special care must be taken on JS side to ensure it's correctly used there.
        let passphrase = Zeroizing::new(passphrase);

        let pass_ref: &str = passphrase.as_ref();

        let inner = WasmStorage::new(
            STORAGE_NAME,
            STORAGE_VERSION,
            Some(db_migration()),
            Some(pass_ref.as_bytes()),
        )
        .await?;

        Ok(ExtensionStorage {
            inner: Arc::new(inner),
        })
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(passphrase: String) -> Promise {
        future_to_promise(async move { Self::new_async(passphrase).await.into_promise_result() })
    }

    pub(crate) async fn exists_async() -> Result<bool, ExtensionStorageError> {
        Ok(WasmStorage::exists(STORAGE_NAME).await?)
    }

    pub fn exists() -> Promise {
        future_to_promise(async move { Self::exists_async().await.into_promise_result() })
    }

    async fn store_mnemonic_async(
        &self,
        name: String,
        value: &bip39::Mnemonic,
    ) -> Result<(), ExtensionStorageError> {
        self.inner
            .store_value(v1::MNEMONICS_STORE, JsValue::from_str(&name), value)
            .await
            .map_err(Into::into)
    }

    async fn read_mnemonic_async(
        &self,
        name: String,
    ) -> Result<Option<bip39::Mnemonic>, ExtensionStorageError> {
        self.inner
            .read_value(v1::MNEMONICS_STORE, JsValue::from_str(&name))
            .await
            .map_err(Into::into)
    }

    async fn remove_mnemonic_async(&self, name: String) -> Result<(), ExtensionStorageError> {
        self.inner
            .remove_value(v1::MNEMONICS_STORE, JsValue::from_str(&name))
            .await
            .map_err(Into::into)
    }

    async fn has_mnemonic_async(&self, name: String) -> Result<bool, ExtensionStorageError> {
        self.inner
            .has_value(v1::MNEMONICS_STORE, JsValue::from_str(&name))
            .await
            .map_err(Into::into)
    }

    async fn get_all_mnemonic_keys_async(&self) -> Result<js_sys::Array, ExtensionStorageError> {
        self.inner
            .get_all_keys(v1::MNEMONICS_STORE)
            .await
            .map_err(Into::into)
    }

    #[wasm_bindgen]
    pub fn store_mnemonic(&self, name: String, value: String) -> Promise {
        let wrapped = Zeroizing::new(value);
        let inner: &str = wrapped.as_ref();

        let mnemonic = check_promise_result!(
            bip39::Mnemonic::parse(inner).map_err(ExtensionStorageError::from)
        );

        // this clones the Arc pointer
        let this = self.clone();
        future_to_promise(async move {
            this.store_mnemonic_async(name, &mnemonic)
                .await
                .map(|_| JsValue::null())
                .map_promise_err()
        })
    }

    #[wasm_bindgen]
    pub fn read_mnemonic(&self, name: String) -> Promise {
        // this clones the Arc pointer
        let this = self.clone();
        future_to_promise(async move {
            let maybe_mnemonic = this.read_mnemonic_async(name).await?;
            Ok(serde_wasm_bindgen::to_value(&maybe_mnemonic)?)
        })
    }

    #[wasm_bindgen]
    pub fn remove_mnemonic(&self, name: String) -> Promise {
        // this clones the Arc pointer
        let this = self.clone();
        future_to_promise(async move {
            this.remove_mnemonic_async(name)
                .await
                .map(|_| JsValue::null())
                .map_promise_err()
        })
    }

    #[wasm_bindgen]
    pub fn has_mnemonic(&self, name: String) -> Promise {
        // this clones the Arc pointer
        let this = self.clone();
        future_to_promise(async move { this.has_mnemonic_async(name).await.into_promise_result() })
    }

    #[wasm_bindgen]
    pub fn get_all_mnemonic_keys(&self) -> Promise {
        // this clones the Arc pointer
        let this = self.clone();
        future_to_promise(async move {
            this.get_all_mnemonic_keys_async()
                .await
                .into_promise_result()
        })
    }
}
