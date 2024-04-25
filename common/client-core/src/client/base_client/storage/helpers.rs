// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::ClientKeys;
use crate::error::ClientCoreError;
use nym_client_core_gateways_storage::{ActiveGateway, GatewayRegistration, GatewaysDetailsStore};
use nym_crypto::asymmetric::identity;

// helpers for error wrapping
pub async fn set_active_gateway<D>(
    details_store: &D,
    gateway_id: &str,
) -> Result<(), ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .set_active_gateway(gateway_id)
        .await
        .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        })
}

pub async fn get_active_gateway_identity<D>(
    details_store: &D,
) -> Result<Option<identity::PublicKey>, ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .active_gateway()
        .await
        .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        })
        .map(|a| a.registration.map(|r| r.details.gateway_id()))
}

pub async fn get_all_registered_identities<D>(
    details_store: &D,
) -> Result<Vec<identity::PublicKey>, ClientCoreError>
where
    D: GatewaysDetailsStore + Sync,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .all_gateways_identities()
        .await
        .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        })
}

pub async fn get_gateway_registrations<D>(
    details_store: &D,
) -> Result<Vec<GatewayRegistration>, ClientCoreError>
where
    D: GatewaysDetailsStore + Sync,
    D::StorageError: Send + Sync + 'static,
{
    details_store.all_gateways().await.map_err(|source| {
        ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        }
    })
}

pub async fn store_gateway_details<D>(
    details_store: &D,
    details: &GatewayRegistration,
) -> Result<(), ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .store_gateway_details(details)
        .await
        .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        })
}

pub async fn load_active_gateway_details<D>(
    details_store: &D,
) -> Result<ActiveGateway, ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store.active_gateway().await.map_err(|source| {
        ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        }
    })
}

pub async fn load_gateway_details<D>(
    details_store: &D,
    gateway_id: &str,
) -> Result<GatewayRegistration, ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .load_gateway_details(gateway_id)
        .await
        .map_err(|source| ClientCoreError::UnavailableGatewayDetails {
            gateway_id: gateway_id.to_string(),
            source: Box::new(source),
        })
}

pub async fn has_gateway_details<D>(
    details_store: &D,
    gateway_id: &str,
) -> Result<bool, ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .has_gateway_details(gateway_id)
        .await
        .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        })
}

pub async fn load_client_keys<K>(key_store: &K) -> Result<ClientKeys, ClientCoreError>
where
    K: KeyStore,
    K::StorageError: Send + Sync + 'static,
{
    ClientKeys::load_keys(key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })
}

pub async fn store_client_keys<K>(keys: ClientKeys, key_store: &K) -> Result<(), ClientCoreError>
where
    K: KeyStore,
    K::StorageError: Send + Sync + 'static,
{
    keys.persist_keys(key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })
}
