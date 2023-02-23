// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::key_manager::KeyManager,
    config::{persistence::key_pathfinder::ClientKeyPathfinder, Config},
    error::ClientCoreError,
};
#[cfg(target_arch = "wasm32")]
use gateway_client::wasm_mockups::SigningNyxdClient;
use gateway_client::GatewayClient;
use gateway_requests::registration::handshake::SharedKeys;
use nym_config::NymConfig;
use nym_crypto::asymmetric::identity;
use nym_topology::{filter::VersionFilterable, gateway};
use rand::{seq::SliceRandom, thread_rng};
use std::{sync::Arc, time::Duration};
use tap::TapFallible;
use url::Url;
#[cfg(not(target_arch = "wasm32"))]
use validator_client::nyxd::SigningNyxdClient;

pub(super) async fn query_gateway_details(
    validator_servers: Vec<Url>,
    chosen_gateway_id: Option<identity::PublicKey>,
) -> Result<gateway::Node, ClientCoreError> {
    let nym_api = validator_servers
        .choose(&mut thread_rng())
        .ok_or(ClientCoreError::ListOfNymApisIsEmpty)?;
    let validator_client = validator_client::client::NymApiClient::new(nym_api.clone());

    log::trace!("Fetching list of gateways from: {}", nym_api);
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
            .find(|gateway| gateway.identity_key == gateway_id)
            .ok_or_else(|| ClientCoreError::NoGatewayWithId(gateway_id.to_string()))
            .cloned()
    } else {
        filtered_gateways
            .choose(&mut rand::thread_rng())
            .ok_or(ClientCoreError::NoGatewaysOnNetwork)
            .cloned()
    }
}

pub(super) async fn register_with_gateway(
    gateway: &gateway::Node,
    our_identity: Arc<identity::KeyPair>,
) -> Result<Arc<SharedKeys>, ClientCoreError> {
    let timeout = Duration::from_millis(1500);
    let mut gateway_client: GatewayClient<SigningNyxdClient> = GatewayClient::new_init(
        gateway.clients_address(),
        gateway.identity_key,
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

pub(super) fn store_keys<T>(
    key_manager: &KeyManager,
    config: &Config<T>,
) -> Result<(), ClientCoreError>
where
    T: NymConfig,
{
    let pathfinder = ClientKeyPathfinder::new_from_config(config);
    Ok(key_manager
        .store_keys(&pathfinder)
        .tap_err(|err| log::error!("Failed to generate keys: {err}"))?)
}
