// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::GatewayDetails;
use crate::{BadGateway, GatewaysDetailsStore};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum InMemStorageError {
    #[error("gateway {gateway_id} does not exist")]
    GatewayDoesNotExist { gateway_id: String },

    #[error(transparent)]
    MalformedGateway(#[from] BadGateway),
}

#[derive(Debug, Default)]
pub struct InMemGatewaysDetails {
    inner: Arc<RwLock<InMemStorageInner>>,
}

#[derive(Debug, Default)]
struct InMemStorageInner {
    active_gateway: Option<String>,
    gateways: HashMap<String, GatewayDetails>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl GatewaysDetailsStore for InMemGatewaysDetails {
    type StorageError = InMemStorageError;

    async fn active_gateway(&self) -> Result<Option<GatewayDetails>, Self::StorageError> {
        // let guard = self.inner.read().await;
        //
        // let foo = guard.active_gateway.map(|id| {
        //     // SAFETY: if particular gateway is set as active, its details MUST exist
        //     #[allow(clippy::unwrap_used)]
        //     guard.gateways.get(&id).unwrap()
        // });

        todo!()
        // foo.cloned()
    }

    async fn set_active_gateway(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
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
