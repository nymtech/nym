// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use crate::client::base_client::storage::MixnetClientStorage;
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::{KeyManager, ManagedKeys};
use crate::init::helpers::{choose_gateway_by_latency, current_gateways, uniformly_random_gateway};
use crate::{
    config::{
        disk_persistence::key_pathfinder::ClientKeysPathfinder, Config, GatewayEndpointConfig,
    },
    error::ClientCoreError,
};
use nym_crypto::asymmetric::{encryption, identity};
use nym_sphinx::addressing::{clients::Recipient, nodes::NodeIdentity};
use nym_validator_client::client::IdentityKey;
use rand::rngs::OsRng;
use serde::Serialize;
use std::fmt::{Debug, Display};
use tap::TapFallible;
use url::Url;

mod helpers;

#[derive(Clone)]
pub enum GatewaySetup {
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
        config: GatewayEndpointConfig,
    },
}

impl From<GatewayEndpointConfig> for GatewaySetup {
    fn from(config: GatewayEndpointConfig) -> Self {
        GatewaySetup::Predefined { config }
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
    pub fn new(
        full_config: Option<GatewayEndpointConfig>,
        gateway_identity: Option<IdentityKey>,
        latency_based_selection: Option<bool>,
    ) -> Self {
        if let Some(config) = full_config {
            GatewaySetup::Predefined { config }
        } else if let Some(gateway_identity) = gateway_identity {
            GatewaySetup::Specified { gateway_identity }
        } else {
            GatewaySetup::New {
                by_latency: latency_based_selection.unwrap_or_default(),
            }
        }
    }

    pub async fn try_get_gateway_details(
        self,
        validator_servers: &[Url],
    ) -> Result<GatewayEndpointConfig, ClientCoreError> {
        match self {
            GatewaySetup::New { by_latency } => {
                let mut rng = OsRng;
                let gateways = current_gateways(&mut rng, validator_servers).await?;
                if by_latency {
                    choose_gateway_by_latency(&mut rng, gateways).await
                } else {
                    uniformly_random_gateway(&mut rng, gateways)
                }
            }
            .map(Into::into),
            GatewaySetup::Specified { gateway_identity } => {
                let user_gateway = identity::PublicKey::from_base58_string(&gateway_identity)
                    .map_err(ClientCoreError::UnableToCreatePublicKeyFromGatewayId)?;

                let mut rng = OsRng;
                let gateways = current_gateways(&mut rng, validator_servers).await?;
                gateways
                    .into_iter()
                    .find(|gateway| gateway.identity_key == user_gateway)
                    .ok_or_else(|| ClientCoreError::NoGatewayWithId(gateway_identity.to_string()))
            }
            .map(Into::into),
            GatewaySetup::Predefined { config } => Ok(config),
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
    pub fn new(config: &Config, address: &Recipient) -> Self {
        Self {
            version: config.client.version.clone(),
            id: config.client.id.clone(),
            identity_key: address.identity().to_base58_string(),
            encryption_key: address.encryption_key().to_base58_string(),
            gateway_id: config.get_gateway_id(),
            gateway_listener: config.get_gateway_listener(),
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

/// Recovers the already present gateway information or attempts to register with new gateway
/// and stores the newly obtained key
pub async fn get_registered_gateway<S>(
    validator_servers: Vec<Url>,
    key_store: &S::KeyStore,
    setup: GatewaySetup,
    overwrite_keys: bool,
) -> Result<(GatewayEndpointConfig, ManagedKeys), ClientCoreError>
where
    S: MixnetClientStorage,
    <S::KeyStore as KeyStore>::StorageError: Send + Sync + 'static,
{
    let mut rng = OsRng;

    // try load keys
    let mut managed_keys = match ManagedKeys::try_load(key_store).await {
        Ok(loaded_keys) => {
            // if we loaded something and we don't have full gateway details, check if we can overwrite the data
            if let GatewaySetup::Predefined { config } = setup {
                // we already have defined gateway details AND a shared key, so nothing more for us to do
                return Ok((config, loaded_keys));
            } else if overwrite_keys {
                ManagedKeys::generate_new(&mut rng)
            } else {
                return Err(ClientCoreError::ForbiddenKeyOverwrite);
            }
        }
        Err(_) => ManagedKeys::generate_new(&mut rng),
    };

    // choose gateway
    let gateway_details = setup.try_get_gateway_details(&validator_servers).await?;

    // get our identity key
    let our_identity = managed_keys.identity_keypair();

    // Establish connection, authenticate and generate keys for talking with the gateway
    let shared_keys = helpers::register_with_gateway(&gateway_details, our_identity).await?;

    managed_keys
        .deal_with_gateway_key(shared_keys, key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })?;

    // TODO: here we should be probably persisting gateway details as opposed to returning them

    Ok((gateway_details, managed_keys))
}

/// Convenience function for setting up the gateway for a client given a `Config`. Depending on the
/// arguments given it will do the sensible thing. Either it will
///
/// a. Reuse existing gateway configuration from storage.
/// b. Create a new gateway configuration but keep existing keys. This assumes that the caller
///    knows what they are doing and that the keys match the requested gateway.
/// c. Create a new gateway configuration with a newly registered gateway and keys.
pub async fn setup_gateway_from_config<KSt>(
    key_store: &KSt,
    register_gateway: bool,
    user_chosen_gateway_id: Option<identity::PublicKey>,
    config: &Config,
    by_latency: bool,
) -> Result<GatewayEndpointConfig, ClientCoreError>
where
    KSt: KeyStore,
    <KSt as KeyStore>::StorageError: Send + Sync + 'static,
{
    // If we are not going to register gateway, and an explicitly chosen gateway is not passed in,
    // load the existing configuration file
    if !register_gateway && user_chosen_gateway_id.is_none() {
        eprintln!("Not registering gateway, will reuse existing config and keys");
        return Ok(config.client.gateway_endpoint.clone());
    }

    let gateway_setup = GatewaySetup::new(
        None,
        user_chosen_gateway_id.map(|id| id.to_base58_string()),
        Some(by_latency),
    );
    // Else, we proceed by querying the nym-api
    let gateway = gateway_setup
        .try_get_gateway_details(&config.get_nym_api_endpoints())
        .await?;
    log::debug!("Querying gateway gives: {:?}", gateway);

    // If we are not registering, just return this and assume the caller has the keys already and
    // wants to keep the,
    if !register_gateway && user_chosen_gateway_id.is_some() {
        eprintln!("Using gateway provided by user, keeping existing keys");
        return Ok(gateway);
    }

    let mut rng = OsRng;
    let mut managed_keys =
        crate::client::key_manager::ManagedKeys::load_or_generate(&mut rng, key_store).await;

    // Create new keys and derive our identity
    let our_identity = managed_keys.identity_keypair();

    // Establish connection, authenticate and generate keys for talking with the gateway
    eprintln!("Registering with new gateway");
    let shared_keys = helpers::register_with_gateway(&gateway, our_identity).await?;
    managed_keys
        .deal_with_gateway_key(shared_keys, key_store)
        .await
        .map_err(|source| ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })?;

    Ok(gateway)
}

/// Get the full client address from the client keys and the gateway identity
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

pub fn load_identity_keys(
    pathfinder: &ClientKeyPathfinder,
) -> Result<identity::KeyPair, ClientCoreError> {
    let identity_keypair: identity::KeyPair =
        nym_pemstore::load_keypair(&pathfinder.identity_key_pair_path())
            .tap_err(|_| log::error!("Failed to read stored identity key files"))?;
    Ok(identity_keypair)
}

/// Get the client address by loading the keys from stored files.
// TODO: rethink that sucker
pub fn get_client_address_from_stored_ondisk_keys(
    pathfinder: &ClientKeysPathfinder,
    gateway_config: &GatewayEndpointConfig,
) -> Result<Recipient, ClientCoreError> {
    let public_identity: identity::PublicKey =
        nym_pemstore::load_key(&pathfinder.public_identity_key_file)?;
    let public_encryption: encryption::PublicKey =
        nym_pemstore::load_key(&pathfinder.public_encryption_key_file)?;

    let client_recipient = Recipient::new(
        public_identity,
        public_encryption,
        // TODO: below only works under assumption that gateway address == gateway id
        // (which currently is true)
        NodeIdentity::from_base58_string(&gateway_config.gateway_id)?,
    );

    Ok(client_recipient)
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
