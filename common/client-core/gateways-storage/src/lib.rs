// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use std::error::Error;

pub mod error;
pub mod models;

pub use error::GatewaysStorageError;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GatewayDetailsStore {
    type StorageError: Error;

    /// Returns details of the currently active gateway, if available.
    async fn active_gateway(&self) -> Result<Option<()>, Self::StorageError>;

    /// Returns details of all registered gateways.
    async fn all_gateways(&self) -> Result<Vec<()>, Self::StorageError>;

    /// Returns details of the particular gateway.
    async fn load_gateway_details(&self) -> Result<(), Self::StorageError>;

    /// Store the provided gateway details.
    async fn store_gateway_details(&self) -> Result<(), Self::StorageError>;
}
