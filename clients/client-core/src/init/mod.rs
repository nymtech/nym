// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use std::fmt::Display;

use nymsphinx::addressing::{clients::Recipient, nodes::NodeIdentity};
use serde::Serialize;
use tap::TapFallible;

use config::NymConfig;
use crypto::asymmetric::{encryption, identity};

use crate::client::replies::reply_storage::ReplyStorageBackend;
use crate::{
    config::{
        persistence::key_pathfinder::ClientKeyPathfinder, ClientCoreConfigTrait, Config,
        GatewayEndpointConfig,
    },
    error::ClientCoreError,
    init::helpers::{query_gateway_details, register_with_gateway_and_store_keys},
};

mod helpers;

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

/// Convenience function for setting up the gateway for a client. Depending on the arguments given
/// it will do the sensible thing.
pub async fn setup_gateway<B, C, T>(
    register_gateway: bool,
    // TODO: this should get refactored to instead take Option<identity::PublicKey>
    user_chosen_gateway_id: Option<String>,
    config: &Config<T>,
) -> Result<GatewayEndpointConfig, ClientCoreError<B>>
where
    B: ReplyStorageBackend,
    C: NymConfig + ClientCoreConfigTrait,
    T: NymConfig,
{
    let id = config.get_id();
    if register_gateway {
        register_with_gateway(user_chosen_gateway_id, config).await
    } else if let Some(user_chosen_gateway_id) = user_chosen_gateway_id {
        config_gateway_with_existing_keys(user_chosen_gateway_id, config).await
    } else {
        reuse_existing_gateway_config::<B, C>(&id)
    }
}

/// Get the gateway details by querying the validator-api. Either pick one at random or use
/// the chosen one if it's among the available ones.
/// Saves keys to disk, specified by the paths in `config`.
pub async fn register_with_gateway<B, T>(
    user_chosen_gateway_id: Option<String>,
    config: &Config<T>,
) -> Result<GatewayEndpointConfig, ClientCoreError<B>>
where
    B: ReplyStorageBackend,
    T: NymConfig,
{
    println!("Configuring gateway");
    let gateway =
        query_gateway_details(config.get_nym_api_endpoints(), user_chosen_gateway_id).await?;
    log::debug!("Querying gateway gives: {}", gateway);

    // Registering with gateway by setting up and writing shared keys to disk
    log::trace!("Registering gateway");
    register_with_gateway_and_store_keys(gateway.clone(), config).await?;
    println!("Saved all generated keys");

    Ok(gateway.into())
}

/// Set the gateway using the usual procedue of querying the validator-api, but don't register or
/// create any keys.
/// This assumes that the user knows what they are doing, and that the existing keys are valid for
/// the gateway being used
pub async fn config_gateway_with_existing_keys<B, T>(
    user_chosen_gateway_id: String,
    config: &Config<T>,
) -> Result<GatewayEndpointConfig, ClientCoreError<B>>
where
    B: ReplyStorageBackend,
    T: NymConfig,
{
    println!("Using gateway provided by user, keeping existing keys");
    let gateway =
        query_gateway_details(config.get_nym_api_endpoints(), Some(user_chosen_gateway_id)).await?;
    log::debug!("Querying gateway gives: {}", gateway);
    Ok(gateway.into())
}

/// Read and reuse the existing gateway configuration from a file that was generate earlier.
pub fn reuse_existing_gateway_config<B, T>(
    id: &str,
) -> Result<GatewayEndpointConfig, ClientCoreError<B>>
where
    B: ReplyStorageBackend,
    T: NymConfig + ClientCoreConfigTrait,
{
    println!("Not registering gateway, will reuse existing config and keys");
    T::load_from_file(Some(id))
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

/// Get the client address by loading the keys from stored files.
pub fn get_client_address_from_stored_keys<B, T>(
    config: &Config<T>,
) -> Result<Recipient, ClientCoreError<B>>
where
    T: config::NymConfig,
    B: ReplyStorageBackend,
{
    fn load_identity_keys<B>(
        pathfinder: &ClientKeyPathfinder,
    ) -> Result<identity::KeyPair, ClientCoreError<B>>
    where
        B: ReplyStorageBackend,
    {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .tap_err(|_| log::error!("Failed to read stored identity key files"))?;
        Ok(identity_keypair)
    }

    fn load_sphinx_keys<B>(
        pathfinder: &ClientKeyPathfinder,
    ) -> Result<encryption::KeyPair, ClientCoreError<B>>
    where
        B: ReplyStorageBackend,
    {
        let sphinx_keypair: encryption::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
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
            Ok(_) => println!("Saved: {}", output_file),
            Err(err) => eprintln!("Could not save {}: {err}", output_file),
        },
        Err(err) => eprintln!("Could not save {}: {err}", output_file),
    }
}
