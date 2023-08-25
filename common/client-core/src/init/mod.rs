// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use crate::client::base_client::storage::gateway_details::{
    GatewayDetailsStore, PersistedGatewayDetails,
};
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::ManagedKeys;
use crate::error::ClientCoreError;
use crate::init::helpers::current_gateways;
use crate::init::types::{GatewaySetup, InitialisationResult};
use log::error;
use nym_topology::gateway;
use rand::rngs::OsRng;
use serde::Serialize;
use std::ops::Deref;
use url::Url;

pub mod helpers;
pub mod types;

// helpers for error wrapping
async fn _store_gateway_details<D>(
    details_store: &D,
    details: &PersistedGatewayDetails,
) -> Result<(), ClientCoreError>
where
    D: GatewayDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .store_gateway_details(details)
        .await
        .map_err(|source| ClientCoreError::GatewayDetailsStoreError {
            source: Box::new(source),
        })
}

async fn _load_gateway_details<D>(
    details_store: &D,
) -> Result<PersistedGatewayDetails, ClientCoreError>
where
    D: GatewayDetailsStore,
    D::StorageError: Send + Sync + 'static,
{
    details_store
        .load_gateway_details()
        .await
        .map_err(|source| ClientCoreError::UnavailableGatewayDetails {
            source: Box::new(source),
        })
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
    match details {
        PersistedGatewayDetails::Default(details) => {
            if !details.verify(
                loaded_keys
                    .gateway_shared_key()
                    .ok_or(ClientCoreError::UnavailableSharedKey)?
                    .deref(),
            ) {
                Err(ClientCoreError::MismatchedGatewayDetails {
                    gateway_id: details.details.gateway_id.clone(),
                })
            } else {
                Ok(())
            }
        }
        PersistedGatewayDetails::Custom(_) => {
            if loaded_keys.gateway_shared_key().is_some() {
                error!("using custom persisted gateway setup with shared key present - are you sure that's what you want?");
                // but technically we could still continue. just ignore the key
            }
            Ok(())
        }
    }
}

pub async fn setup_gateway_from<K, D>(
    setup: GatewaySetup,
    key_store: &K,
    details_store: &D,
    overwrite_data: bool,
    gateways: Option<&[gateway::Node]>,
) -> Result<InitialisationResult, ClientCoreError>
where
    K: KeyStore,
    D: GatewayDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    // I don't like how we can't deal with this variant in the match below, but we need to take ownership of internal values.
    if let GatewaySetup::ReuseConnection {
        authenticated_ephemeral_client,
        gateway_details,
        managed_keys,
    } = setup
    {
        // if we have already performed the full setup, forward the details.
        // it's up to the caller to ensure persistence
        return Ok(InitialisationResult {
            gateway_details,
            managed_keys,
            authenticated_ephemeral_client: Some(authenticated_ephemeral_client),
        });
    }

    let mut rng = OsRng;

    // try load gateway details
    let loaded_details = _load_gateway_details(details_store).await;

    // try load keys and decide what to do based on the GatewaySetup
    let mut managed_keys = match ManagedKeys::try_load(key_store).await {
        Ok(loaded_keys) => {
            match &setup {
                GatewaySetup::MustLoad => {
                    // get EVERYTHING from the storage
                    let details = loaded_details?;
                    ensure_valid_details(&details, &loaded_keys)?;

                    // no need to persist anything as we got everything from the storage
                    return Ok(InitialisationResult::new_loaded(
                        details.into(),
                        loaded_keys,
                    ));
                }
                GatewaySetup::Predefined { details } => {
                    // if nothing was stored or we're allowed to overwrite what's there, just persist the passed data
                    if overwrite_data || loaded_details.is_err() {
                        let shared_key = loaded_keys.gateway_shared_key();
                        let storable =
                            PersistedGatewayDetails::new(details.clone(), shared_key.as_deref())?;
                        _store_gateway_details(details_store, &storable).await?;
                    } else if let Ok(existing_details) = loaded_details {
                        // if there was some stored data and we can't overwrite it make sure it's exactly what we provided now
                        // (and that they match the key)

                        if !existing_details.matches(&details) {
                            return Err(ClientCoreError::MismatchedStoredGatewayDetails);
                        }

                        ensure_valid_details(&existing_details, &loaded_keys)?;
                    }

                    return Ok(InitialisationResult::new_loaded(
                        details.clone().into(),
                        loaded_keys,
                    )
                    .into());
                }
                GatewaySetup::Specified { gateway_identity } => {
                    // if that data was already stored...
                    if let Ok(existing_gateway) = loaded_details {
                        ensure_valid_details(&existing_gateway, &loaded_keys)?;
                        let PersistedGatewayDetails::Default(cfg) = existing_gateway else {
                            return Err(ClientCoreError::UnexpectedPersistedCustomGatewayDetails);
                        };
                        if &cfg.details.gateway_id != gateway_identity && !overwrite_data {
                            // if our loaded details don't match requested value and we CANT overwrite it...
                            return Err(ClientCoreError::UnexpectedGatewayDetails);
                        } else if &cfg.details.gateway_id == gateway_identity {
                            // if they do match up, just return it
                            return Ok(InitialisationResult::new_loaded(
                                cfg.details.into(),
                                loaded_keys,
                            )
                            .into());
                        }
                    }

                    // we didn't get full details from the store and we have loaded some keys
                    // so we can only continue if we're allowed to overwrite keys
                    if overwrite_data {
                        ManagedKeys::generate_new(&mut rng)
                    } else {
                        return Err(ClientCoreError::ForbiddenKeyOverwrite);
                    }
                }
                GatewaySetup::New { .. } => {
                    if let Ok(existing_gateway) = loaded_details {
                        ensure_valid_details(&existing_gateway, &loaded_keys)?;
                        return Ok(InitialisationResult::new_loaded(
                            existing_gateway.into(),
                            loaded_keys,
                        )
                        .into());
                    }

                    // we didn't get full details from the store and we have loaded some keys
                    // so we can only continue if we're allowed to overwrite keys
                    if overwrite_data {
                        ManagedKeys::generate_new(&mut rng)
                    } else {
                        return Err(ClientCoreError::ForbiddenKeyOverwrite);
                    }
                }
                GatewaySetup::ReuseConnection { .. } => {
                    unreachable!("the reuse connection variant was already manually covered")
                }
            }
        }
        Err(_) => {
            // if we failed to load the keys, ensure we didn't provide gateway details in some form
            // (in that case we CAN'T generate new keys
            if setup.has_full_details() {
                return Err(ClientCoreError::UnavailableSharedKey);
            }
            ManagedKeys::generate_new(&mut rng)
        }
    };

    // TODO: figure out how custom gateway fits into the below logic

    // choose gateway
    let gateway_details = setup.choose_gateway(gateways.unwrap_or_default()).await?;

    // get our identity key
    let our_identity = managed_keys.identity_keypair();

    // Establish connection, authenticate and generate keys for talking with the gateway
    let registration_result =
        helpers::register_with_gateway(&gateway_details, our_identity).await?;
    let shared_keys = registration_result.shared_keys;

    let persisted_details =
        PersistedGatewayDetails::new(gateway_details.into(), Some(shared_keys.deref()))?;

    // persist gateway keys
    managed_keys
        .deal_with_gateway_key(shared_keys, key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })?;

    // persist gateway config
    _store_gateway_details(details_store, &persisted_details).await?;

    Ok(InitialisationResult {
        gateway_details: persisted_details.into(),
        managed_keys,
        authenticated_ephemeral_client: Some(registration_result.authenticated_ephemeral_client),
    })
}

pub async fn setup_gateway<K, D>(
    setup: GatewaySetup,
    key_store: &K,
    details_store: &D,
    overwrite_data: bool,
    validator_servers: Option<&[Url]>,
) -> Result<InitialisationResult, ClientCoreError>
where
    K: KeyStore,
    D: GatewayDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    let mut rng = OsRng;
    let gateways = current_gateways(&mut rng, validator_servers.unwrap_or_default()).await?;

    setup_gateway_from(
        setup,
        key_store,
        details_store,
        overwrite_data,
        Some(&gateways),
    )
    .await
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
