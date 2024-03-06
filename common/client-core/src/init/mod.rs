// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use crate::client::base_client::storage::gateway_details::PersistedGatewayDetails;
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::ManagedKeys;
use crate::config::GatewayEndpointConfig;
use crate::error::ClientCoreError;
use crate::init::helpers::{
    choose_gateway_by_latency, get_specified_gateway, uniformly_random_gateway,
};
use crate::init::types::{
    CustomGatewayDetails, GatewayDetails, GatewaySelectionSpecification, GatewaySetup,
    InitialisationResult,
};
use nym_client_core_gateways_storage::GatewaysDetailsStore;
use nym_gateway_client::client::InitGatewayClient;
use nym_topology::gateway;
use rand::rngs::OsRng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

pub mod helpers;
pub mod types;

// helpers for error wrapping
async fn _store_gateway_details<D>(
    details_store: &D,
    details: &PersistedGatewayDetails,
) -> Result<(), ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    todo!()
    // details_store
    //     .store_gateway_details(details)
    //     .await
    //     .map_err(|source| ClientCoreError::GatewaysDetailsStoreError {
    //         source: Box::new(source),
    //     })
}

async fn _load_gateway_details<D>(
    details_store: &D,
) -> Result<PersistedGatewayDetails, ClientCoreError>
where
    D: GatewaysDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    todo!()
    // details_store
    //     .load_gateway_details()
    //     .await
    //     .map_err(|source| ClientCoreError::UnavailableGatewayDetails {
    //         source: Box::new(source),
    //     })
}

async fn _load_managed_keys<K>(key_store: &K) -> Result<ManagedKeys, ClientCoreError>
where
    K: KeyStore,
    K::StorageError: Send + Sync + 'static,
{
    ManagedKeys::try_load(key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })
}

fn ensure_valid_details(
    details: &PersistedGatewayDetails,
    loaded_keys: &ManagedKeys,
) -> Result<(), ClientCoreError> {
    details.validate(loaded_keys.gateway_shared_key().as_deref())
}

async fn setup_new_gateway<K, D>(
    key_store: &K,
    details_store: &D,
    overwrite_data: bool,
    selection_specification: GatewaySelectionSpecification,
    available_gateways: Vec<gateway::Node>,
) -> Result<InitialisationResult, ClientCoreError>
where
    K: KeyStore,
    D: GatewaysDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    log::trace!("Setting up new gateway");

    // if we're setting up new gateway, failing to load existing information is fine.
    // as a matter of fact, it's only potentially a problem if we DO succeed

    todo!("check gateway details (maybe not even needed anymore, idk)");
    // if _load_gateway_details(details_store).await.is_ok() && !overwrite_data {
    //     return Err(ClientCoreError::ForbiddenKeyOverwrite);
    // }

    if _load_managed_keys(key_store).await.is_ok() && !overwrite_data {
        return Err(ClientCoreError::ForbiddenKeyOverwrite);
    }

    let mut rng = OsRng;
    let mut new_keys = ManagedKeys::generate_new(&mut rng);

    let gateway_details = match selection_specification {
        GatewaySelectionSpecification::UniformRemote { must_use_tls } => {
            let gateway = uniformly_random_gateway(&mut rng, &available_gateways, must_use_tls)?;
            GatewayDetails::Configured(GatewayEndpointConfig::from_node(gateway, must_use_tls)?)
        }
        GatewaySelectionSpecification::RemoteByLatency { must_use_tls } => {
            let gateway =
                choose_gateway_by_latency(&mut rng, &available_gateways, must_use_tls).await?;
            GatewayDetails::Configured(GatewayEndpointConfig::from_node(gateway, must_use_tls)?)
        }
        GatewaySelectionSpecification::Specified {
            must_use_tls,
            identity,
        } => {
            let gateway = get_specified_gateway(&identity, &available_gateways, must_use_tls)?;
            GatewayDetails::Configured(GatewayEndpointConfig::from_node(gateway, must_use_tls)?)
        }
        GatewaySelectionSpecification::Custom {
            gateway_identity,
            additional_data,
        } => GatewayDetails::Custom(CustomGatewayDetails::new(gateway_identity, additional_data)),
    };

    let registration_result = if let GatewayDetails::Configured(gateway_cfg) = &gateway_details {
        // if we're using a 'normal' gateway setup, do register
        let our_identity = new_keys.identity_keypair();
        Some(helpers::register_with_gateway(gateway_cfg, our_identity).await?)
    } else {
        None
    };

    let maybe_shared_keys = registration_result
        .as_ref()
        .map(|r| Arc::clone(&r.shared_keys));

    let persisted_details =
        PersistedGatewayDetails::new(gateway_details, maybe_shared_keys.as_deref())?;

    // persist the keys
    new_keys
        .deal_with_gateway_key(maybe_shared_keys, key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })?;

    // persist gateway configs
    _store_gateway_details(details_store, &persisted_details).await?;

    Ok(InitialisationResult {
        gateway_details: persisted_details.into(),
        managed_keys: new_keys,
        authenticated_ephemeral_client: registration_result
            .map(|r| r.authenticated_ephemeral_client),
    })
}

async fn use_loaded_gateway_details<K, D>(
    key_store: &K,
    details_store: &D,
) -> Result<InitialisationResult, ClientCoreError>
where
    K: KeyStore,
    D: GatewaysDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    let loaded_details = _load_gateway_details(details_store).await?;
    let loaded_keys = _load_managed_keys(key_store).await?;

    ensure_valid_details(&loaded_details, &loaded_keys)?;

    // no need to persist anything as we got everything from the storage
    Ok(InitialisationResult::new_loaded(
        loaded_details.into(),
        loaded_keys,
    ))
}

fn reuse_gateway_connection(
    authenticated_ephemeral_client: InitGatewayClient,
    gateway_details: GatewayDetails,
    managed_keys: ManagedKeys,
) -> InitialisationResult {
    InitialisationResult {
        gateway_details,
        managed_keys,
        authenticated_ephemeral_client: Some(authenticated_ephemeral_client),
    }
}

pub async fn setup_gateway<K, D>(
    setup: GatewaySetup,
    key_store: &K,
    details_store: &D,
) -> Result<InitialisationResult, ClientCoreError>
where
    K: KeyStore,
    D: GatewaysDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    log::debug!("Setting up gateway");
    match setup {
        GatewaySetup::MustLoad => use_loaded_gateway_details(key_store, details_store).await,
        GatewaySetup::New {
            specification,
            available_gateways,
            overwrite_data,
        } => {
            setup_new_gateway(
                key_store,
                details_store,
                overwrite_data,
                specification,
                available_gateways,
            )
            .await
        }
        GatewaySetup::ReuseConnection {
            authenticated_ephemeral_client,
            gateway_details,
            managed_keys,
        } => Ok(reuse_gateway_connection(
            authenticated_ephemeral_client,
            gateway_details,
            managed_keys,
        )),
    }
}

pub fn output_to_json<T: Serialize>(init_results: &T, output_file: &str) {
    match std::fs::File::create(output_file) {
        Ok(file) => match serde_json::to_writer_pretty(file, init_results) {
            Ok(_) => println!("Saved: {output_file}"),
            Err(err) => eprintln!("Could not save {output_file}: {err}"),
        },
        Err(err) => eprintln!("Could not save {output_file}: {err}"),
    }
}
