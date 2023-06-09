// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::GatewayEndpointConfig;
use async_trait::async_trait;
use nym_gateway_requests::registration::handshake::SharedKeys;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;

// TODO: to incorporate into `MixnetClientStorage`
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GatewayDetailsStore {
    type StorageError: Error;

    async fn load_gateway_details(&self) -> Result<PersistedGatewayDetails, Self::StorageError>;

    async fn store_gateway_details(
        &self,
        details: &PersistedGatewayDetails,
    ) -> Result<(), Self::StorageError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedGatewayDetails {
    /// (to be determined), hash, ciphertext or tag derived from the details and the shared key
    /// to ensure they correspond to the same instance.
    magic_crypto_field: (),

    /// Actual gateway details being persisted.
    details: GatewayEndpointConfig,
}

impl PersistedGatewayDetails {
    pub fn new(details: GatewayEndpointConfig, shared_key: Arc<SharedKeys>) -> Self {
        todo!()
    }

    pub fn verify(&self, shared_key: Arc<SharedKeys>) -> bool {
        todo!()
    }
}
