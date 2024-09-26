// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ActiveGateway, GatewayRegistration};
use crate::{BadGateway, GatewayDetails, GatewaysDetailsStore};
use async_trait::async_trait;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_gateway_requests::{SharedGatewayKey, SharedSymmetricKey};
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
    gateways: HashMap<String, GatewayRegistration>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl GatewaysDetailsStore for InMemGatewaysDetails {
    type StorageError = InMemStorageError;

    async fn active_gateway(&self) -> Result<ActiveGateway, Self::StorageError> {
        let guard = self.inner.read().await;

        let registration = guard.active_gateway.as_ref().map(|id| {
            // SAFETY: if particular gateway is set as active, its details MUST exist
            #[allow(clippy::unwrap_used)]
            guard.gateways.get(id).unwrap().clone()
        });

        Ok(ActiveGateway { registration })
    }

    async fn set_active_gateway(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        // ensure the gateway with provided id exists
        let mut guard = self.inner.write().await;

        if !guard.gateways.contains_key(gateway_id) {
            return Err(InMemStorageError::GatewayDoesNotExist {
                gateway_id: gateway_id.to_string(),
            });
        }

        guard.active_gateway = Some(gateway_id.to_string());
        Ok(())
    }

    async fn all_gateways(&self) -> Result<Vec<GatewayRegistration>, Self::StorageError> {
        Ok(self.inner.read().await.gateways.values().cloned().collect())
    }

    async fn has_gateway_details(&self, gateway_id: &str) -> Result<bool, Self::StorageError> {
        Ok(self.inner.read().await.gateways.contains_key(gateway_id))
    }

    async fn load_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<GatewayRegistration, Self::StorageError> {
        self.inner
            .read()
            .await
            .gateways
            .get(gateway_id)
            .cloned()
            .ok_or(InMemStorageError::GatewayDoesNotExist {
                gateway_id: gateway_id.to_string(),
            })
    }

    async fn store_gateway_details(
        &self,
        details: &GatewayRegistration,
    ) -> Result<(), Self::StorageError> {
        self.inner.write().await.gateways.insert(
            details.details.gateway_id().to_base58_string(),
            details.clone(),
        );
        Ok(())
    }

    async fn upgrade_stored_remote_gateway_key(
        &self,
        gateway_id: PublicKey,
        updated_key: &SharedSymmetricKey,
    ) -> Result<(), Self::StorageError> {
        let mut guard = self.inner.write().await;

        #[allow(clippy::unwrap_used)]
        if let Some(target) = guard.gateways.get_mut(&gateway_id.to_string()) {
            let GatewayDetails::Remote(details) = &mut target.details else {
                return Ok(());
            };
            assert_eq!(Arc::strong_count(&details.shared_key), 1);

            // eh. that's nasty, but it's only ever used for ephemeral clients so should be fine for now...
            details.shared_key = Arc::new(SharedGatewayKey::Current(
                SharedSymmetricKey::try_from_bytes(updated_key.as_bytes()).unwrap(),
            ))
        }

        Ok(())
    }

    async fn remove_gateway_details(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        let mut guard = self.inner.write().await;
        if let Some(active) = guard.active_gateway.as_ref() {
            if active == gateway_id {
                guard.active_gateway = None
            }
        }
        guard.gateways.remove(gateway_id);

        Ok(())
    }
}
