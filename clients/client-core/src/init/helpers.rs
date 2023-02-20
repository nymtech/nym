// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::key_manager::KeyManager,
    config::{persistence::key_pathfinder::ClientKeyPathfinder, Config},
    error::ClientCoreError,
};
use futures::{SinkExt, StreamExt};
use gateway_client::GatewayClient;
use gateway_requests::registration::handshake::SharedKeys;
use nym_config::NymConfig;
use nym_crypto::asymmetric::identity;
use nym_topology::{filter::VersionFilterable, gateway};
use rand::{seq::SliceRandom, thread_rng};
use std::{sync::Arc, time::Duration};
use tap::TapFallible;
use tokio_tungstenite::tungstenite::Message;
use topology::{filter::VersionFilterable, gateway};
use url::Url;
use validator_client::client::GatewayBond;

#[cfg(not(target_arch = "wasm32"))]
use validator_client::nyxd::SigningNyxdClient;

#[cfg(target_arch = "wasm32")]
use gateway_client::wasm_mockups::SigningNyxdClient;

const MEASUREMENTS: usize = 3;
const CONN_TIMEOUT: Duration = Duration::from_millis(1500);
const MAX_LATENCY: Duration = Duration::from_secs()

struct GatewayWithLatency {
    gateway: gateway::Node,
    latency: Duration,
}

async fn measure_latency(gateway: GatewayBond) -> Result<GatewayWithLatency, ClientCoreError> {
    let converted: gateway::Node = gateway.try_into()?;
    let mut stream = match tokio::time::timeout(
        CONN_TIMEOUT,
        tokio_tungstenite::connect_async(&converted.clients_address()),
    )
    .await
    {
        Err(elapsed) => todo!(),
        Ok(Err(conn_failure)) => todo!(),
        Ok(Ok((stream, _))) => stream,
    };

    todo!()
}

pub(super) async fn find_closest_gateway(
    nym_apis: Vec<Url>,
) -> Result<gateway::Node, ClientCoreError> {
    let nym_api = nym_apis
        .choose(&mut thread_rng())
        .ok_or(ClientCoreError::ListOfNymApisIsEmpty)?;
    let client = validator_client::client::NymApiClient::new(nym_api.clone());

    // log::trace!("Fetching list of gateways from: {}", nym_api);
    let gateways = client.get_cached_gateways().await?;
    //
    // let mut gateways_with_latency = Vec::new();
    // for gateway in gateways {
    //     let converted: gateway::Node = match gateway.try_into() {
    //         Ok(node) => node,
    //         Err(err) => todo!(),
    //     };
    //
    //     let endpoint = converted.clients_address();
    //     let (mut stream, res) = tokio_tungstenite::connect_async(&endpoint)
    //         .await
    //         .expect("todo");
    //
    //     let now = tokio::time::Instant::now();
    //     stream.send(Message::Ping(vec![1, 2, 3])).await.unwrap();
    //     if let Some(Ok(Message::Pong(content))) = stream.next().await {
    //         //
    //     }
    //     let received = stream.next().await.unwrap().unwrap();
    //     let elapsed = tokio::time::Instant::now().duration_since(now);
    //     println!("got: {:?}", received);
    //     println!("took {:?}", elapsed);
    // }
    //
    todo!()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn foo() {
        let nym_api = "https://validator.nymtech.net/api/".parse().unwrap();
        find_closest_gateway(vec![nym_api]).await;
    }
}
