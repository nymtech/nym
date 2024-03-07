// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::GatewayRegistration;
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
    
    async fn has_gateway_details(&self, gateway_id: &str) -> Result<bool, Self::StorageError> {
        todo!()
    }
    async fn active_gateway(&self) -> Result<Option<GatewayRegistration>, Self::StorageError> {
        todo!()
    }

    async fn set_active_gateway(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        todo!()
    }

    async fn all_gateways(&self) -> Result<Vec<GatewayRegistration>, Self::StorageError> {
        todo!()
    }

    async fn load_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<GatewayRegistration, Self::StorageError> {
        todo!()
    }

    async fn store_gateway_details(
        &self,
        details: &GatewayRegistration,
    ) -> Result<(), Self::StorageError> {
        todo!()
    }

    async fn remove_gateway_details(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        todo!()
    }
}
