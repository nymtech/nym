// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use std::fmt::Display;

use nym_sphinx::addressing::{clients::Recipient, nodes::NodeIdentity};
use rand::rngs::OsRng;
use serde::Serialize;
use tap::TapFallible;

use nym_config::NymConfig;
use nym_credential_storage::storage::Storage;
use nym_crypto::asymmetric::{encryption, identity};
use url::Url;

use crate::client::key_manager::KeyManager;
use crate::{
    config::{
        persistence::key_pathfinder::ClientKeyPathfinder, ClientCoreConfigTrait, Config,
        GatewayEndpointConfig,
    },
    error::ClientCoreError,
};

mod helpers;

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
    pub fn new<T>(config: &Config<T>, address: &Recipient) -> Self
    where
        T: NymConfig,
    {
        Self {
            version: config.get_version().to_string(),
            id: config.get_id(),
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

/// Create a new set of client keys.
pub fn new_client_keys() -> KeyManager {
    let mut rng = OsRng;
    KeyManager::new(&mut rng)
}

/// Authenticate and register with a gateway.
/// Either pick one at random by querying the available gateways from the nym-api, or use the
/// chosen one if it's among the available ones.
/// The shared key is added to the supplied `KeyManager` and the endpoint details are returned.
pub async fn register_with_gateway<St: Storage>(
    key_manager: &mut KeyManager,
    nym_api_endpoints: Vec<Url>,
    chosen_gateway_id: Option<identity::PublicKey>,
    by_latency: bool,
) -> Result<GatewayEndpointConfig, ClientCoreError> {
    // Get the gateway details of the gateway we will use
    let gateway =
        helpers::query_gateway_details(nym_api_endpoints, chosen_gateway_id, by_latency).await?;
    log::debug!("Querying gateway gives: {}", gateway);

    let our_identity = key_manager.identity_keypair();

    // Establish connection, authenticate and generate keys for talking with the gateway
    let shared_keys = helpers::register_with_gateway::<St>(&gateway, our_identity).await?;
    key_manager.insert_gateway_shared_key(shared_keys);

    Ok(gateway.into())
}

/// Convenience function for setting up the gateway for a client given a `Config`. Depending on the
/// arguments given it will do the sensible thing. Either it will
///
/// a. Reuse existing gateway configuration from storage.
/// b. Create a new gateway configuration but keep existing keys. This assumes that the caller
///    knows what they are doing and that the keys match the requested gateway.
/// c. Create a new gateway configuration with a newly registered gateway and keys.
pub async fn setup_gateway_from_config<C, T, St>(
    register_gateway: bool,
    user_chosen_gateway_id: Option<identity::PublicKey>,
    config: &Config<T>,
    by_latency: bool,
) -> Result<GatewayEndpointConfig, ClientCoreError>
where
    C: NymConfig + ClientCoreConfigTrait,
    T: NymConfig,
    St: Storage,
{
    let id = config.get_id();

    // If we are not going to register gateway, and an explicitly chosen gateway is not passed in,
    // load the existing configuration file
    if !register_gateway && user_chosen_gateway_id.is_none() {
        eprintln!("Not registering gateway, will reuse existing config and keys");
        return load_existing_gateway_config::<C>(&id);
    }

    // Else, we proceed by querying the nym-api
    let gateway = helpers::query_gateway_details(
        config.get_nym_api_endpoints(),
        user_chosen_gateway_id,
        by_latency,
    )
    .await?;
    log::debug!("Querying gateway gives: {}", gateway);

    // If we are not registering, just return this and assume the caller has the keys already and
    // wants to keep the,
    if !register_gateway && user_chosen_gateway_id.is_some() {
        eprintln!("Using gateway provided by user, keeping existing keys");
        return Ok(gateway.into());
    }

    // Create new keys and derive our identity
    let mut key_manager = new_client_keys();
    let our_identity = key_manager.identity_keypair();

    // Establish connection, authenticate and generate keys for talking with the gateway
    eprintln!("Registering with new gateway");
    let shared_keys = helpers::register_with_gateway::<St>(&gateway, our_identity).await?;
    key_manager.insert_gateway_shared_key(shared_keys);

    // Write all keys to storage and just return the gateway endpoint config. It is assumed that we
    // will load keys from storage when actually connecting.
    helpers::store_keys(&key_manager, config)?;
    Ok(gateway.into())
}

/// Read and reuse the existing gateway configuration from a file that was generate earlier.
pub fn load_existing_gateway_config<T>(id: &str) -> Result<GatewayEndpointConfig, ClientCoreError>
where
    T: NymConfig + ClientCoreConfigTrait,
{
    T::load_from_file(id)
        .map(|existing_config| existing_config.get_gateway_endpoint().clone())
        .map_err(|err| {
            log::error!(
                "Unable to configure gateway: {err}. \n
                Seems like the client was already initialized but it was not possible to read \
                the existing configuration file. \n
                CAUTION: Consider backing up your gateway keys and try force gateway registration, or \
                removing the existing configuration and starting over."
            );
            ClientCoreError::CouldNotLoadExistingGatewayConfiguration(err)
        })
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

/// Get the client address by loading the keys from stored files.
pub fn get_client_address_from_stored_keys<T>(
    config: &Config<T>,
) -> Result<Recipient, ClientCoreError>
where
    T: nym_config::NymConfig,
{
    fn load_identity_keys(
        pathfinder: &ClientKeyPathfinder,
    ) -> Result<identity::KeyPair, ClientCoreError> {
        let identity_keypair: identity::KeyPair =
            nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .tap_err(|_| log::error!("Failed to read stored identity key files"))?;
        Ok(identity_keypair)
    }

    fn load_sphinx_keys(
        pathfinder: &ClientKeyPathfinder,
    ) -> Result<encryption::KeyPair, ClientCoreError> {
        let sphinx_keypair: encryption::KeyPair =
            nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ))
            .tap_err(|_| log::error!("Failed to read stored sphinx key files"))?;
        Ok(sphinx_keypair)
    }

    let pathfinder = ClientKeyPathfinder::new_from_config(config);
    let identity_keypair = load_identity_keys(&pathfinder)?;
    let sphinx_keypair = load_sphinx_keys(&pathfinder)?;

    let client_recipient = Recipient::new(
        *identity_keypair.public_key(),
        *sphinx_keypair.public_key(),
        // TODO: below only works under assumption that gateway address == gateway id
        // (which currently is true)
        NodeIdentity::from_base58_string(config.get_gateway_id())?,
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
