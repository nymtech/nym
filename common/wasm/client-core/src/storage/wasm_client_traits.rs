// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::types::WasmRawRegisteredGateway;
use async_trait::async_trait;
use nym_client_core::client::base_client::storage::gateways_storage::RawActiveGateway;
use nym_crypto::asymmetric::{encryption, identity};
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
    // pub const GATEWAY_DETAILS: &str = "gateway_details";

    pub const ED25519_IDENTITY_KEYPAIR: &str = "ed25519_identity_keypair";
    pub const X25519_ENCRYPTION_KEYPAIR: &str = "x25519_encryption_keypair";

    // TODO: for those we could actually use the subtle crypto storage
    pub const AES128CTR_ACK_KEY: &str = "aes128ctr_ack_key";
    // pub const AES128CTR_BLAKE3_HMAC_GATEWAY_KEYS: &str = "aes128ctr_blake3_hmac_gateway_keys";
}

pub(crate) mod v2 {
    pub const GATEWAY_REGISTRATIONS_ACTIVE_GATEWAY_STORE: &str = "active_gateway";
    pub const ACTIVE_GATEWAY_KEY: &str = "active_gateway";

    // there's no concept of 'custom' gateways in wasm so the store is simpler
    pub const GATEWAY_REGISTRATIONS_REGISTERED_GATEWAYS_STORE: &str = "gateway_registrations";
}

#[derive(Debug, Error)]
pub enum WasmClientStorageError {
    #[error("{typ} cryptographic key is not available in storage")]
    CryptoKeyNotInStorage { typ: String },

    #[error(
        "the prior gateway details for gateway {gateway_id:?} are not available in the storage"
    )]
    GatewayDetailsNotInStorage { gateway_id: String },
}

#[async_trait(?Send)]
#[async_trait]
pub trait WasmClientStorage: BaseWasmStorage {
    type StorageError: Error
        + From<<Self as BaseWasmStorage>::StorageError>
        + From<WasmClientStorageError>;

    // keys:

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

    // gateways:

    async fn get_active_gateway_id(
        &self,
    ) -> Result<RawActiveGateway, <Self as WasmClientStorage>::StorageError> {
        let maybe_active: Option<RawActiveGateway> = self
            .read_value(
                v2::GATEWAY_REGISTRATIONS_ACTIVE_GATEWAY_STORE,
                JsValue::from_str(v2::ACTIVE_GATEWAY_KEY),
            )
            .await?;

        // a 'temporary' hack
        // (proper solution: make sure to insert empty value during db creation)
        Ok(RawActiveGateway {
            active_gateway_id_bs58: maybe_active.and_then(|a| a.active_gateway_id_bs58),
        })
    }

    async fn set_active_gateway(
        &self,
        gateway_id: Option<&str>,
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.store_value(
            v2::GATEWAY_REGISTRATIONS_ACTIVE_GATEWAY_STORE,
            JsValue::from_str(v2::ACTIVE_GATEWAY_KEY),
            &RawActiveGateway {
                active_gateway_id_bs58: gateway_id.map(|id| id.to_string()),
            },
        )
        .await
        .map_err(Into::into)
    }

    async fn maybe_get_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<Option<WasmRawRegisteredGateway>, <Self as WasmClientStorage>::StorageError> {
        self.read_value(
            v2::GATEWAY_REGISTRATIONS_REGISTERED_GATEWAYS_STORE,
            JsValue::from_str(gateway_id),
        )
        .await
        .map_err(Into::into)
    }

    async fn must_get_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<WasmRawRegisteredGateway, <Self as WasmClientStorage>::StorageError> {
        self.maybe_get_registered_gateway(gateway_id)
            .await?
            .ok_or(WasmClientStorageError::GatewayDetailsNotInStorage {
                gateway_id: gateway_id.to_string(),
            })
            .map_err(Into::into)
    }

    async fn store_registered_gateway(
        &self,
        registered_gateway: &WasmRawRegisteredGateway,
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.store_value(
            v2::GATEWAY_REGISTRATIONS_REGISTERED_GATEWAYS_STORE,
            JsValue::from_str(&registered_gateway.gateway_id_bs58),
            registered_gateway,
        )
        .await
        .map_err(Into::into)
    }

    async fn remove_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<(), <Self as WasmClientStorage>::StorageError> {
        self.remove_value(
            v2::GATEWAY_REGISTRATIONS_REGISTERED_GATEWAYS_STORE,
            JsValue::from_str(gateway_id),
        )
        .await
        .map_err(Into::into)
    }

    async fn has_registered_gateway(
        &self,
        gateway_id: &str,
    ) -> Result<bool, <Self as WasmClientStorage>::StorageError> {
        self.has_value(
            v2::GATEWAY_REGISTRATIONS_REGISTERED_GATEWAYS_STORE,
            JsValue::from_str(gateway_id),
        )
        .await
        .map_err(Into::into)
    }

    async fn registered_gateways(
        &self,
    ) -> Result<Vec<String>, <Self as WasmClientStorage>::StorageError> {
        self.get_all_keys(v2::GATEWAY_REGISTRATIONS_REGISTERED_GATEWAYS_STORE)
            .await
            .map_err(Into::into)
            .map(|arr| {
                arr.to_vec()
                    .into_iter()
                    .filter_map(|key| key.as_string())
                    .collect()
            })
    }
}
