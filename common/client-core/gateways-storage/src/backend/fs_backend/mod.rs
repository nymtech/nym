// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::GatewayDetails;
use crate::{GatewaysDetailsStore, StorageError};
use async_trait::async_trait;
use manager::StorageManager;
use std::path::Path;

pub mod error;
mod manager;
mod models;

pub struct OnDiskGatewaysDetails {
    manager: StorageManager,
}

impl OnDiskGatewaysDetails {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, StorageError> {
        Ok(OnDiskGatewaysDetails {
            manager: StorageManager::init(database_path).await?,
        })
    }
}

#[async_trait]
impl GatewaysDetailsStore for OnDiskGatewaysDetails {
    type StorageError = error::StorageError;

    async fn active_gateway(&self) -> Result<Option<GatewayDetails>, Self::StorageError> {
        todo!()
    }

    async fn all_gateways(&self) -> Result<Vec<GatewayDetails>, Self::StorageError> {
        todo!()
    }

    async fn load_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<GatewayDetails, Self::StorageError> {
        todo!()
    }

    async fn store_gateway_details(
        &self,
        details: GatewayDetails,
    ) -> Result<(), Self::StorageError> {
        todo!()
    }

    async fn remove_gateway_details(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        todo!()
    }
}
