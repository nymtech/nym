// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use crate::client::base_client::storage::gateway_details::{
    GatewayDetailsStore, PersistedGatewayDetails,
};
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::{KeyManager, ManagedKeys};
use crate::init::helpers::{choose_gateway_by_latency, current_gateways, uniformly_random_gateway};
use crate::{
    config::{Config, GatewayEndpointConfig},
    error::ClientCoreError,
};
use nym_crypto::asymmetric::identity;
use nym_sphinx::addressing::{clients::Recipient, nodes::NodeIdentity};
use nym_validator_client::client::IdentityKey;
use rand::rngs::OsRng;
use serde::Serialize;
use std::fmt::{Debug, Display};
use url::Url;

mod helpers;

// TODO: rename to something better...
pub struct InitialisationDetails {
    pub gateway_details: GatewayEndpointConfig,
    pub managed_keys: ManagedKeys,
}

impl InitialisationDetails {
    pub fn new(gateway_details: GatewayEndpointConfig, managed_keys: ManagedKeys) -> Self {
        InitialisationDetails {
            gateway_details,
            managed_keys,
        }
    }

    pub fn client_address(&self) -> Result<Recipient, ClientCoreError> {
        let client_recipient = Recipient::new(
            *self.managed_keys.identity_public_key(),
            *self.managed_keys.encryption_public_key(),
            // TODO: below only works under assumption that gateway address == gateway id
            // (which currently is true)
            NodeIdentity::from_base58_string(&self.gateway_details.gateway_id)?,
        );

        Ok(client_recipient)
    }
}

#[derive(Clone)]
pub enum GatewaySetup {
    /// The gateway specification MUST BE loaded from the underlying storage.
    MustLoad,

    /// Specifies usage of a new, random, gateway.
    New {
        /// Should the new gateway be selected based on latency.
        by_latency: bool,
    },
    Specified {
        /// Identity key of the gateway we want to try to use.
        gateway_identity: IdentityKey,
    },
    Predefined {
        /// Full gateway configuration
        details: PersistedGatewayDetails,
    },
}

impl From<PersistedGatewayDetails> for GatewaySetup {
    fn from(details: PersistedGatewayDetails) -> Self {
        GatewaySetup::Predefined { details }
    }
}

impl From<IdentityKey> for GatewaySetup {
    fn from(gateway_identity: IdentityKey) -> Self {
        GatewaySetup::Specified { gateway_identity }
    }
}

impl Default for GatewaySetup {
    fn default() -> Self {
        GatewaySetup::New { by_latency: false }
    }
}

impl GatewaySetup {
    pub fn new_fresh(
        gateway_identity: Option<String>,
        latency_based_selection: Option<bool>,
    ) -> Self {
        if let Some(gateway_identity) = gateway_identity {
            GatewaySetup::Specified { gateway_identity }
        } else {
            GatewaySetup::New {
                by_latency: latency_based_selection.unwrap_or_default(),
            }
        }
    }

    pub fn is_must_load(&self) -> bool {
        matches!(self, GatewaySetup::MustLoad)
    }

    pub fn has_full_details(&self) -> bool {
        matches!(self, GatewaySetup::Predefined { .. }) || self.is_must_load()
    }

    pub async fn try_get_new_gateway_details(
        &self,
        validator_servers: &[Url],
    ) -> Result<GatewayEndpointConfig, ClientCoreError> {
        match self {
            GatewaySetup::New { by_latency } => {
                let mut rng = OsRng;
                let gateways = current_gateways(&mut rng, validator_servers).await?;
                if *by_latency {
                    choose_gateway_by_latency(&mut rng, gateways).await
                } else {
                    uniformly_random_gateway(&mut rng, gateways)
                }
            }
            .map(Into::into),
            GatewaySetup::Specified { gateway_identity } => {
                let user_gateway = identity::PublicKey::from_base58_string(gateway_identity)
                    .map_err(ClientCoreError::UnableToCreatePublicKeyFromGatewayId)?;

                let mut rng = OsRng;
                let gateways = current_gateways(&mut rng, validator_servers).await?;
                gateways
                    .into_iter()
                    .find(|gateway| gateway.identity_key == user_gateway)
                    .ok_or_else(|| ClientCoreError::NoGatewayWithId(gateway_identity.to_string()))
            }
            .map(Into::into),
            _ => Err(ClientCoreError::UnexpectedGatewayDetails),
        }
    }
}

/// Struct describing the results of the client initialization procedure.
#[derive(Debug, Serialize)]
pub struct InitResults {
    version: String,
    id: String,
    identity_key: String,
    encryption_key: String,
    gateway_id: String,
    gateway_listener: String,
}

impl InitResults {
    pub fn new(config: &Config, address: &Recipient, gateway: &GatewayEndpointConfig) -> Self {
        Self {
            version: config.client.version.clone(),
            id: config.client.id.clone(),
            identity_key: address.identity().to_base58_string(),
            encryption_key: address.encryption_key().to_base58_string(),
            gateway_id: gateway.gateway_id.clone(),
            gateway_listener: gateway.gateway_listener.clone(),
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Version: {}", self.version)?;
        writeln!(f, "ID: {}", self.id)?;
        writeln!(f, "Identity key: {}", self.identity_key)?;
        writeln!(f, "Encryption: {}", self.encryption_key)?;
        writeln!(f, "Gateway ID: {}", self.gateway_id)?;
        write!(f, "Gateway: {}", self.gateway_listener)
    }
}

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

pub async fn setup_gateway<K, D>(
    setup: &GatewaySetup,
    key_store: &K,
    details_store: &D,
    overwrite_data: bool,
    validator_servers: Option<&[Url]>,
) -> Result<InitialisationDetails, ClientCoreError>
where
    K: KeyStore,
    D: GatewayDetailsStore,
    K::StorageError: Send + Sync + 'static,
    D::StorageError: Send + Sync + 'static,
{
    let mut rng = OsRng;

    // try load gateway details
    let loaded_details = _load_gateway_details(details_store).await;

    // try load keys
    let mut managed_keys = match ManagedKeys::try_load(key_store).await {
        Ok(loaded_keys) => {
            if let GatewaySetup::MustLoad = setup {
                // get EVERYTHING from the storage
                let details = loaded_details?;
                if !details.verify(&loaded_keys.must_get_gateway_shared_key()) {
                    return Err(ClientCoreError::MismatchedGatewayDetails {
                        gateway_id: details.details.gateway_id,
                    });
                }
                // no need to persist anything as we got everything from the storage
                return Ok(InitialisationDetails::new(details.into(), loaded_keys));
            } else if let GatewaySetup::Predefined { details } = setup {
                // we already have defined gateway details AND a shared key
                if !details.verify(&loaded_keys.must_get_gateway_shared_key()) {
                    return Err(ClientCoreError::MismatchedGatewayDetails {
                        gateway_id: details.details.gateway_id.clone(),
                    });
                }

                // if nothing was stored or we're allowed to overwrite what's there, just persist the passed data
                if overwrite_data || loaded_details.is_err() {
                    _store_gateway_details(details_store, details).await?;
                }

                return Ok(InitialisationDetails::new(
                    details.clone().into(),
                    loaded_keys,
                ));
            } else if overwrite_data {
                // whatever the state of the loaded data was, we can't use it since we'll be deriving
                // fresh gateway key
                _ = loaded_details;
                ManagedKeys::generate_new(&mut rng)
            } else {
                return Err(ClientCoreError::ForbiddenKeyOverwrite);
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

    // choose gateway
    let gateway_details = setup
        .try_get_new_gateway_details(validator_servers.unwrap_or_default())
        .await?;

    // get our identity key
    let our_identity = managed_keys.identity_keypair();

    // Establish connection, authenticate and generate keys for talking with the gateway
    let shared_keys = helpers::register_with_gateway(&gateway_details, our_identity).await?;

    let persisted_details = PersistedGatewayDetails::new(gateway_details, &shared_keys);

    // persist gateway keys
    managed_keys
        .deal_with_gateway_key(shared_keys, key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })?;

    // persist gateway config
    _store_gateway_details(details_store, &persisted_details).await?;

    Ok(InitialisationDetails::new(
        persisted_details.into(),
        managed_keys,
    ))
}

/// Get the full client address from the client keys and the gateway identity
#[deprecated]
pub fn get_client_address(
    key_manager: &KeyManager,
    gateway_config: &GatewayEndpointConfig,
) -> Recipient {
    Recipient::new(
        *key_manager.identity_keypair().public_key(),
        *key_manager.encryption_keypair().public_key(),
        // TODO: below only works under assumption that gateway address == gateway id
        // (which currently is true)
        NodeIdentity::from_base58_string(&gateway_config.gateway_id).unwrap(),
    )
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
