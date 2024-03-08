// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ActiveGateway, BadGateway, GatewayDetails, GatewayRegistration, GatewayType,
    GatewaysDetailsStore, StorageError,
};
use async_trait::async_trait;
use manager::StorageManager;
use nym_crypto::asymmetric::identity::PublicKey;
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
        Ok(self
            .manager
            .maybe_get_registered_gateway(gateway_id)
            .await?
            .is_some())
    }

    async fn active_gateway(&self) -> Result<ActiveGateway, Self::StorageError> {
        let raw_active = self.manager.get_active_gateway().await?;
        let registration = match raw_active.active_gateway_id_bs58 {
            None => None,
            Some(gateway_id) => Some(self.load_gateway_details(&gateway_id).await?),
        };

        Ok(ActiveGateway { registration })
    }

    async fn set_active_gateway(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        Ok(self.manager.set_active_gateway(Some(gateway_id)).await?)
    }

    async fn all_gateways(&self) -> Result<Vec<GatewayRegistration>, Self::StorageError> {
        let identities = self.manager.registered_gateways().await?;
        let mut registered = Vec::with_capacity(identities.len());
        for gateway_id in identities {
            registered.push(self.load_gateway_details(&gateway_id).await?)
        }

        Ok(registered)
    }

    async fn all_gateways_identities(&self) -> Result<Vec<PublicKey>, Self::StorageError> {
        Ok(self
            .manager
            .registered_gateways()
            .await?
            .into_iter()
            .map(|gateway_id| {
                gateway_id
                    .as_str()
                    .parse()
                    .map_err(|source| BadGateway::MalformedGatewayIdentity { gateway_id, source })
            })
            .collect::<Result<_, _>>()?)
    }

    async fn load_gateway_details(
        &self,
        gateway_id: &str,
    ) -> Result<GatewayRegistration, Self::StorageError> {
        let raw_registration = self.manager.must_get_registered_gateway(gateway_id).await?;
        let typ: GatewayType = raw_registration.gateway_type.parse()?;

        let details = match typ {
            GatewayType::Remote => {
                let raw_details = self.manager.get_remote_gateway_details(gateway_id).await?;
                GatewayDetails::Remote(raw_details.try_into()?)
            }
            GatewayType::Custom => {
                let raw_details = self.manager.get_custom_gateway_details(gateway_id).await?;
                GatewayDetails::Custom(raw_details.try_into()?)
            }
        };

        Ok(GatewayRegistration {
            details,
            registration_timestamp: raw_registration.registration_timestamp,
        })
    }

    async fn store_gateway_details(
        &self,
        details: &GatewayRegistration,
    ) -> Result<(), Self::StorageError> {
        let raw_registration = details.into();
        self.manager
            .set_registered_gateway(&raw_registration)
            .await?;

        match &details.details {
            GatewayDetails::Remote(remote_details) => {
                let raw_details = remote_details.into();
                self.manager
                    .set_remote_gateway_details(&raw_details)
                    .await?;
            }
            GatewayDetails::Custom(custom_details) => {
                let raw_details = custom_details.into();
                self.manager
                    .set_custom_gateway_details(&raw_details)
                    .await?;
            }
        }
        Ok(())
    }

    // ideally all of those should be run under a storage tx to ensure storage consistency,
    // but at that point it's fine
    async fn remove_gateway_details(&self, gateway_id: &str) -> Result<(), Self::StorageError> {
        let active = self.manager.get_active_gateway().await?;
        if let Some(currently_active) = &active.active_gateway_id_bs58 {
            if currently_active == gateway_id {
                self.manager.set_active_gateway(None).await?;
            }
        }

        // just try remove it from all tables even if it doesn't actually exist
        self.manager.remove_registered_gateway(gateway_id).await?;
        self.manager
            .remove_remote_gateway_details(gateway_id)
            .await?;
        self.manager
            .remove_custom_gateway_details(gateway_id)
            .await?;
        Ok(())
    }
}
