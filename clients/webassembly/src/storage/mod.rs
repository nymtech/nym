// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use async_trait::async_trait;
use js_sys::Promise;
use nym_client_core::client::key_manager::{KeyManager, KeyStore};
use nym_crypto::asymmetric::{encryption, identity};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::storage::{IdbVersionChangeEvent, WasmStorage};
use wasm_utils::PromisableResult;
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
    pub const AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS_PREFIX: &str =
        "aes128ctr_blake3_hmac_gateway_keys";
}

#[wasm_bindgen]
pub struct ClientStorage {
    pub(crate) name: String,
    pub(crate) inner: Arc<WasmStorage>,
}

#[wasm_bindgen]
impl ClientStorage {
    fn db_name(client_id: &str) -> String {
        format!("{STORAGE_NAME_PREFIX}-{client_id}")
    }

    async fn new_async(
        client_id: &str,
        passphrase: Option<String>,
    ) -> Result<Self, WasmClientError> {
        let name = Self::db_name(client_id);

        // make sure the password is zeroized when no longer used, especially if we error out.
        // special care must be taken on JS side to ensure it's correctly used there.
        let passphrase = Zeroizing::new(passphrase);

        let migrate_fn = (Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
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

    async fn read_identity_keypair(&self) -> Result<Option<identity::KeyPair>, WasmClientError> {
        self.inner
            .read_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
            )
            .await
            .map_err(Into::into)
    }

    async fn read_encryption_keypair(
        &self,
    ) -> Result<Option<encryption::KeyPair>, WasmClientError> {
        self.inner
            .read_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
            )
            .await
            .map_err(Into::into)
    }

    async fn store_identity_keypair(
        &self,
        keypair: &identity::KeyPair,
    ) -> Result<(), WasmClientError> {
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
    ) -> Result<(), WasmClientError> {
        self.inner
            .store_value(
                v1::KEYS_STORE,
                JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
                keypair,
            )
            .await
            .map_err(Into::into)
    }
}

#[async_trait(?Send)]
impl KeyStore for ClientStorage {
    type StorageError = WasmClientError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        todo!()
    }

    async fn store_keys(&self, keys: KeyManager) -> Result<(), Self::StorageError> {
        todo!()
    }
}
