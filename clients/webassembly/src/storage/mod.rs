// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::storage::error::ClientStorageError;
use js_sys::Promise;
use nym_client_core::client::base_client::storage::gateway_details::PersistedGatewayDetails;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_client::SharedKeys;
use nym_sphinx::acknowledgements::AckKey;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::storage::{IdbVersionChangeEvent, WasmStorage};
use wasm_utils::PromisableResult;
use zeroize::Zeroizing;

pub(crate) mod error;
pub(crate) mod traits;

const STORAGE_NAME_PREFIX: &str = "wasm-client-storage";
const STORAGE_VERSION: u32 = 1;

// v1 tables
mod v1 {
    // stores
    pub const KEYS_STORE: &str = "keys";
    pub const CORE_STORE: &str = "core";

    // keys
    pub const CONFIG: &str = "config";
    pub const GATEWAY_DETAILS: &str = "gateway_details";

    pub const ED25519_IDENTITY_KEYPAIR: &str = "ed25519_identity_keypair";
    pub const X25519_ENCRYPTION_KEYPAIR: &str = "x25519_encryption_keypair";

    // TODO: for those we could actually use the subtle crypto storage
    pub const AES128CTR_ACK_KEY: &str = "aes128ctr_ack_key";
    pub const AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS: &str = "aes128ctr_blake3_hmac_gateway_keys";
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
                db.create_object_store(v1::CORE_STORE)?;
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

    // TODO: persist client's config
    #[allow(dead_code)]
    pub(crate) async fn read_config(&self) -> Result<Option<Config>, ClientStorageError> {
        self.inner
            .read_value(v1::CORE_STORE, JsValue::from_str(v1::CONFIG))
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn may_read_gateway_details(
        &self,
    ) -> Result<Option<PersistedGatewayDetails>, ClientStorageError> {
        self.inner
            .read_value(v1::CORE_STORE, JsValue::from_str(v1::GATEWAY_DETAILS))
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn must_read_gateway_details(
        &self,
    ) -> Result<PersistedGatewayDetails, ClientStorageError> {
        self.may_read_gateway_details()
            .await?
            .ok_or(ClientStorageError::GatewayDetailsNotInStorage)
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

    pub(crate) async fn store_gateway_details(
        &self,
        gateway_endpoint: &PersistedGatewayDetails,
    ) -> Result<(), ClientStorageError> {
        self.inner
            .store_value(
                v1::CORE_STORE,
                JsValue::from_str(v1::GATEWAY_DETAILS),
                gateway_endpoint,
            )
            .await
            .map_err(Into::into)
    }

    // TODO: persist client's config
    #[allow(dead_code)]
    pub(crate) async fn store_config(&self, config: &Config) -> Result<(), ClientStorageError> {
        self.inner
            .store_value(v1::CORE_STORE, JsValue::from_str(v1::CONFIG), config)
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn has_full_gateway_info(&self) -> Result<bool, ClientStorageError> {
        let has_keys = self.may_read_gateway_shared_key().await?.is_some();
        let has_details = self.may_read_gateway_details().await?.is_some();

        Ok(has_keys && has_details)
    }
}
