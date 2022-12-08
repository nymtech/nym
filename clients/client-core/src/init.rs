// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use std::{sync::Arc, time::Duration};

use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use gateway_client::GatewayClient;
use gateway_requests::registration::handshake::SharedKeys;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::addressing::nodes::NodeIdentity;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use tap::TapFallible;
use topology::{filter::VersionFilterable, gateway};
use url::Url;

use crate::{
    client::key_manager::KeyManager,
    config::{persistence::key_pathfinder::ClientKeyPathfinder, Config},
    error::ClientCoreError,
};

pub async fn query_gateway_details(
    validator_servers: Vec<Url>,
    chosen_gateway_id: Option<&str>,
) -> Result<gateway::Node, ClientCoreError> {
    let validator_api = validator_servers
        .choose(&mut thread_rng())
        .ok_or(ClientCoreError::ListOfValidatorApisIsEmpty)?;
    let validator_client = validator_client::client::ApiClient::new(validator_api.clone());

    log::trace!("Fetching list of gateways from: {}", validator_api);
    let gateways = validator_client.get_cached_gateways().await?;
    let valid_gateways = gateways
        .into_iter()
        .filter_map(|gateway| gateway.try_into().ok())
        .collect::<Vec<gateway::Node>>();

    let filtered_gateways = valid_gateways.filter_by_version(env!("CARGO_PKG_VERSION"));

    // if we have chosen particular gateway - use it, otherwise choose a random one.
    // (remember that in active topology all gateways have at least 100 reputation so should
    // be working correctly)
    if let Some(gateway_id) = chosen_gateway_id {
        filtered_gateways
            .iter()
            .find(|gateway| gateway.identity_key.to_base58_string() == gateway_id)
            .ok_or_else(|| ClientCoreError::NoGatewayWithId(gateway_id.to_string()))
            .cloned()
    } else {
        filtered_gateways
            .choose(&mut rand::thread_rng())
            .ok_or(ClientCoreError::NoGatewaysOnNetwork)
            .cloned()
    }
}

pub async fn register_with_gateway_and_store_keys<T>(
    gateway_details: gateway::Node,
    config: &Config<T>,
) -> Result<(), ClientCoreError>
where
    T: NymConfig,
{
    let mut rng = OsRng;
    let mut key_manager = KeyManager::new(&mut rng);

    let shared_keys =
        register_with_gateway(&gateway_details, key_manager.identity_keypair()).await?;
    key_manager.insert_gateway_shared_key(shared_keys);

    let pathfinder = ClientKeyPathfinder::new_from_config(config);
    Ok(key_manager
        .store_keys(&pathfinder)
        .tap_err(|err| log::error!("Failed to generate keys: {err}"))?)
}

async fn register_with_gateway(
    gateway: &gateway::Node,
    our_identity: Arc<identity::KeyPair>,
) -> Result<Arc<SharedKeys>, ClientCoreError> {
    let timeout = Duration::from_millis(1500);
    let mut gateway_client = GatewayClient::new_init(
        gateway.clients_address(),
        gateway.identity_key,
        gateway.owner.clone(),
        our_identity.clone(),
        timeout,
    );
    gateway_client
        .establish_connection()
        .await
        .tap_err(|_| log::warn!("Failed to establish connection with gateway!"))?;
    let shared_keys = gateway_client
        .perform_initial_authentication()
        .await
        .tap_err(|_| log::warn!("Failed to register with the gateway!"))?;
    Ok(shared_keys)
}

pub fn show_address<T>(config: &Config<T>) -> Result<(), ClientCoreError>
where
    T: config::NymConfig,
{
    fn load_identity_keys(
        pathfinder: &ClientKeyPathfinder,
    ) -> Result<identity::KeyPair, ClientCoreError> {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
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

    println!("\nThe address of this client is: {}", client_recipient);
    Ok(())
}
