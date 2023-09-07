// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_client_core::client::base_client::storage::gateway_details::PersistedGatewayDetails;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_client::SharedKeys;
use nym_sphinx_acknowledgements::AckKey;
use std::error::Error;
use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_storage::traits::BaseWasmStorage;

// v1 tables
pub(crate) mod v1 {
    // stores
    pub const KEYS_STORE: &str = "keys";
    pub const CORE_STORE: &str = "core";

    // keys
    // pub const CONFIG: &str = "config";
    pub const GATEWAY_DETAILS: &str = "gateway_details";

    pub const ED25519_IDENTITY_KEYPAIR: &str = "ed25519_identity_keypair";
    pub const X25519_ENCRYPTION_KEYPAIR: &str = "x25519_encryption_keypair";

    // TODO: for those we could actually use the subtle crypto storage
    pub const AES128CTR_ACK_KEY: &str = "aes128ctr_ack_key";
    pub const AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS: &str = "aes128ctr_blake3_hmac_gateway_keys";
}

#[derive(Debug, Error)]
pub enum WasmClientStorageError {
    #[error("{typ} cryptographic key is not available in storage")]
    CryptoKeyNotInStorage { typ: String },

    #[error("the prior gateway details are not available in the storage")]
    GatewayDetailsNotInStorage,
}

#[async_trait(?Send)]
pub trait WasmClientStorage: BaseWasmStorage {
    type StorageError: Error
        + From<<Self as BaseWasmStorage>::StorageError>
        + From<WasmClientStorageError>;

    async fn may_read_gateway_details(
        &self,
    ) -> Result<Option<PersistedGatewayDetails>, <Self as WasmClientStorage>::StorageError> {
        self.read_value(v1::CORE_STORE, JsValue::from_str(v1::GATEWAY_DETAILS))
            .await
            .map_err(Into::into)
    }

    async fn must_read_gateway_details(
        &self,
    ) -> Result<PersistedGatewayDetails, <Self as WasmClientStorage>::StorageError> {
        self.may_read_gateway_details()
            .await?
            .ok_or(WasmClientStorageError::GatewayDetailsNotInStorage)
            .map_err(Into::into)
    }

    async fn may_read_identity_keypair(
        &self,
    ) -> Result<Option<identity::KeyPair>, <Self as WasmClientStorage>::StorageError> {
        self.read_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::ED25519_IDENTITY_KEYPAIR),
        )
        .await
        .map_err(Into::into)
    }

    async fn may_read_encryption_keypair(
        &self,
    ) -> Result<Option<encryption::KeyPair>, <Self as WasmClientStorage>::StorageError> {
        self.read_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
        )
        .await
        .map_err(Into::into)
    }

    async fn may_read_ack_key(
        &self,
    ) -> Result<Option<AckKey>, <Self as WasmClientStorage>::StorageError> {
        self.read_value(v1::KEYS_STORE, JsValue::from_str(v1::AES128CTR_ACK_KEY))
            .await
            .map_err(Into::into)
    }

    async fn may_read_gateway_shared_key(
        &self,
    ) -> Result<Option<SharedKeys>, <Self as WasmClientStorage>::StorageError> {
        self.read_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS),
        )
        .await
        .map_err(Into::into)
    }

    async fn must_read_identity_keypair(
        &self,
    ) -> Result<identity::KeyPair, <Self as WasmClientStorage>::StorageError> {
        self.may_read_identity_keypair()
            .await?
            .ok_or(WasmClientStorageError::CryptoKeyNotInStorage {
                typ: v1::ED25519_IDENTITY_KEYPAIR.to_string(),
            })
            .map_err(Into::into)
    }

    async fn must_read_encryption_keypair(
        &self,
    ) -> Result<encryption::KeyPair, <Self as WasmClientStorage>::StorageError> {
        self.may_read_encryption_keypair()
            .await?
            .ok_or(WasmClientStorageError::CryptoKeyNotInStorage {
                typ: v1::X25519_ENCRYPTION_KEYPAIR.to_string(),
            })
            .map_err(Into::into)
    }

    async fn must_read_ack_key(&self) -> Result<AckKey, <Self as WasmClientStorage>::StorageError> {
        self.may_read_ack_key()
            .await?
            .ok_or(WasmClientStorageError::CryptoKeyNotInStorage {
                typ: v1::AES128CTR_ACK_KEY.to_string(),
            })
            .map_err(Into::into)
    }

    async fn must_read_gateway_shared_key(
        &self,
    ) -> Result<SharedKeys, <Self as WasmClientStorage>::StorageError> {
        self.may_read_gateway_shared_key()
            .await?
            .ok_or(WasmClientStorageError::CryptoKeyNotInStorage {
                typ: v1::AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS.to_string(),
            })
            .map_err(Into::into)
    }

    async fn store_identity_keypair(
        &self,
        keypair: &identity::KeyPair,
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.store_value(
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
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.store_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::X25519_ENCRYPTION_KEYPAIR),
            keypair,
        )
        .await
        .map_err(Into::into)
    }

    async fn store_ack_key(
        &self,
        key: &AckKey,
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.store_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::AES128CTR_ACK_KEY),
            key,
        )
        .await
        .map_err(Into::into)
    }

    async fn store_gateway_shared_key(
        &self,
        key: &SharedKeys,
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.store_value(
            v1::KEYS_STORE,
            JsValue::from_str(v1::AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS),
            key,
        )
        .await
        .map_err(Into::into)
    }

    async fn store_gateway_details(
        &self,
        gateway_endpoint: &PersistedGatewayDetails,
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.store_value(
            v1::CORE_STORE,
            JsValue::from_str(v1::GATEWAY_DETAILS),
            gateway_endpoint,
        )
        .await
        .map_err(Into::into)
    }

    async fn has_full_gateway_info(
        &self,
    ) -> Result<bool, <Self as WasmClientStorage>::StorageError> {
        let has_keys = self.may_read_gateway_shared_key().await?.is_some();
        let has_details = self.may_read_gateway_details().await?.is_some();

        Ok(has_keys && has_details)
    }
}
