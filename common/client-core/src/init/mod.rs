// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use crate::client::base_client::storage::helpers::{
    has_gateway_details, load_active_gateway_details, load_client_keys, load_gateway_details,
    store_gateway_details,
};
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::ClientKeys;
use crate::error::ClientCoreError;
use crate::init::helpers::{
    choose_gateway_by_latency, get_specified_gateway, uniformly_random_gateway,
};
use crate::init::types::{
    GatewaySelectionSpecification, GatewaySetup, InitialisationResult, SelectedGateway,
};
use nym_client_core_gateways_storage::GatewaysDetailsStore;
use nym_client_core_gateways_storage::{GatewayDetails, GatewayRegistration};
use nym_gateway_client::client::InitGatewayClient;
use nym_topology::gateway;
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use serde::Serialize;
use std::net::IpAddr;

pub mod helpers;
pub mod types;

// helpers for error wrapping

pub async fn generate_new_client_keys<K, R>(
    rng: &mut R,
    key_store: &K,
) -> Result<(), ClientCoreError>
where
    R: RngCore + CryptoRng,
    K: KeyStore,
    K::StorageError: Send + Sync + 'static,
{
    ClientKeys::generate_new(rng)
        .persist_keys(key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })
}

async fn setup_new_gateway<K, D>(
    key_store: &K,
    details_store: &D,
    selection_specification: GatewaySelectionSpecification,
    available_gateways: Vec<gateway::Node>,
    wg_tun_ip_address: Option<IpAddr>,
) -> Result<InitialisationResult, ClientCoreError>
where
    K: KeyStore,
    D: GatewaysDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    log::trace!("Setting up new gateway");

    // if we're setting up new gateway, we must have had generated long-term client keys before
    let client_keys = load_client_keys(key_store).await?;

    let mut rng = OsRng;

    let selected_gateway = match selection_specification {
        GatewaySelectionSpecification::UniformRemote { must_use_tls } => {
            let gateway = uniformly_random_gateway(&mut rng, &available_gateways, must_use_tls)?;
            SelectedGateway::from_topology_node(gateway, wg_tun_ip_address, must_use_tls)?
        }
        GatewaySelectionSpecification::RemoteByLatency { must_use_tls } => {
            let gateway =
                choose_gateway_by_latency(&mut rng, &available_gateways, must_use_tls).await?;
            SelectedGateway::from_topology_node(gateway, wg_tun_ip_address, must_use_tls)?
        }
        GatewaySelectionSpecification::Specified {
            must_use_tls,
            identity,
        } => {
            let gateway = get_specified_gateway(&identity, &available_gateways, must_use_tls)?;
            SelectedGateway::from_topology_node(gateway, wg_tun_ip_address, must_use_tls)?
        }
        GatewaySelectionSpecification::Custom {
            gateway_identity,
            additional_data,
        } => SelectedGateway::custom(gateway_identity, additional_data)?,
    };

    // check if we already have details associated with this particular gateway
    // and if so, see if we can overwrite it
    let selected_id = selected_gateway.gateway_id().to_base58_string();
    if has_gateway_details(details_store, &selected_id).await? {
        return Err(ClientCoreError::AlreadyRegistered {
            gateway_id: selected_id,
        });
    }

    let (gateway_details, authenticated_ephemeral_client) = match selected_gateway {
        SelectedGateway::Remote {
            gateway_id,
            gateway_owner_address,
            gateway_listener,
            wg_tun_address,
        } => {
            // if we're using a 'normal' gateway setup, do register
            let our_identity = client_keys.identity_keypair();

            // if wg address is set, use that one
            let url = wg_tun_address.clone().unwrap_or(gateway_listener.clone());

            let registration =
                helpers::register_with_gateway(gateway_id, url, our_identity).await?;
            (
                GatewayDetails::new_remote(
                    gateway_id,
                    registration.shared_keys,
                    gateway_owner_address,
                    gateway_listener,
                    wg_tun_address,
                ),
                Some(registration.authenticated_ephemeral_client),
            )
        }
        SelectedGateway::Custom {
            gateway_id,
            additional_data,
        } => (
            GatewayDetails::new_custom(gateway_id, additional_data),
            None,
        ),
    };

    let gateway_registration = gateway_details.into();

    // persist gateway details
    store_gateway_details(details_store, &gateway_registration).await?;

    Ok(InitialisationResult {
        gateway_registration,
        client_keys,
        authenticated_ephemeral_client,
    })
}

async fn use_loaded_gateway_details<K, D>(
    key_store: &K,
    details_store: &D,
    gateway_id: Option<String>,
) -> Result<InitialisationResult, ClientCoreError>
where
    K: KeyStore,
    D: GatewaysDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    let loaded_details = if let Some(gateway_id) = gateway_id {
        load_gateway_details(details_store, &gateway_id).await?
    } else {
        load_active_gateway_details(details_store)
            .await?
            .registration
            .ok_or(ClientCoreError::NoActiveGatewaySet)?
    };

    let loaded_keys = load_client_keys(key_store).await?;

    // no need to persist anything as we got everything from the storage
    Ok(InitialisationResult::new_loaded(
        loaded_details,
        loaded_keys,
    ))
}

fn reuse_gateway_connection(
    authenticated_ephemeral_client: InitGatewayClient,
    gateway_registration: GatewayRegistration,
    client_keys: ClientKeys,
) -> InitialisationResult {
    InitialisationResult {
        gateway_registration,
        client_keys,
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
        GatewaySetup::MustLoad { gateway_id } => {
            log::debug!("GatewaySetup::MustLoad with id: {gateway_id:?}");
            use_loaded_gateway_details(key_store, details_store, gateway_id).await
        }
        GatewaySetup::New {
            specification,
            available_gateways,
            wg_tun_address,
        } => {
            log::debug!("GatewaySetup::New with spec: {specification:?}");
            setup_new_gateway(
                key_store,
                details_store,
                specification,
                available_gateways,
                wg_tun_address,
            )
            .await
        }
        GatewaySetup::ReuseConnection {
            authenticated_ephemeral_client,
            gateway_details,
            client_keys: managed_keys,
        } => {
            log::debug!("GatewaySetup::ReuseConnection");
            Ok(reuse_gateway_connection(
                authenticated_ephemeral_client,
                *gateway_details,
                managed_keys,
            ))
        }
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
