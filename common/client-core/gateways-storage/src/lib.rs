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

    async fn active_gateway(&self) -> Result<Option<()>, Self::StorageError>;

    async fn all_gateways(&self) -> Result<Vec<()>, Self::StorageError>;

    async fn load_gateway_details(&self) -> Result<(), Self::StorageError>;

    async fn store_gateway_details(&self) -> Result<(), Self::StorageError>;
}
