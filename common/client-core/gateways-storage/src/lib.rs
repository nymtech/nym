// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

use async_trait::async_trait;
use nym_crypto::asymmetric::identity;
use nym_gateway_requests::SharedSymmetricKey;
use std::error::Error;

pub mod backend;
pub mod error;
pub mod types;

// todo: export port types
pub use crate::types::*;
pub use backend::mem_backend::{InMemGatewaysDetails, InMemStorageError};
pub use error::BadGateway;

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-gateways-storage"))]
pub use backend::fs_backend::{error::StorageError, OnDiskGatewaysDetails};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GatewaysDetailsStore {
    type StorageError: Error + From<error::BadGateway>;

    /// Returns details of the currently active gateway, if available.
    async fn active_gateway(&self) -> Result<ActiveGateway, Self::StorageError>;

    /// Set the provided gateway as the currently active gateway.
    async fn set_active_gateway(&self, gateway_id: &str) -> Result<(), Self::StorageError>;

    /// Returns details of all registered gateways.
    async fn all_gateways(&self) -> Result<Vec<GatewayRegistration>, Self::StorageError>;

    /// Return identity keys of all registered gateways.
    async fn all_gateways_identities(
        &self,
    ) -> Result<Vec<identity::PublicKey>, Self::StorageError> {
        Ok(self
            .all_gateways()
            .await?
            .into_iter()
            .map(|gateway| gateway.details.gateway_id())
            .collect())
    }

    /// Check if the gateway with the provided id already exists in the store.
    async fn has_gateway_details(&self, gateway_id: &str) -> Result<bool, Self::StorageError>;

    /// Returns details of the particular gateway.
    async fn load_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<GatewayRegistration, Self::StorageError>;

    /// Store the provided gateway details.
    async fn store_gateway_details(
        &self,
        details: &GatewayRegistration,
    ) -> Result<(), Self::StorageError>;

    async fn upgrade_stored_remote_gateway_key(
        &self,
        gateway_id: identity::PublicKey,
        updated_key: &SharedSymmetricKey,
    ) -> Result<(), Self::StorageError>;

    /// Remove given gateway details from the underlying store.
    async fn remove_gateway_details(&self, gateway_id: &str) -> Result<(), Self::StorageError>;
}
