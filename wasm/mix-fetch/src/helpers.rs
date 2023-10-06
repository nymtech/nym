// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixFetchError;
use crate::harbourmaster;
use crate::harbourmaster::HarbourMasterApiClientExt;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use url::Url;
use wasm_client_core::init::helpers::{current_gateways, current_mixnodes};
use wasm_client_core::topology::{gateway, SerializableGateway};
use wasm_client_core::{HardcodedTopologyProvider, NymTopology};
use wasm_utils::{console_log, console_warn};

// since this client is temporary (and will be properly integrated into nym-api eventually),
// we're using hardcoded URL for mainnet
const HARBOUR_MASTER: &str = "https://harbourmaster.nymtech.net";

pub(crate) async fn get_network_requester(
    preferred: Option<String>,
) -> Result<String, MixFetchError> {
    if let Some(sp) = preferred {
        return Ok(sp);
    }

    let client = harbourmaster::Client::new_url(HARBOUR_MASTER, None)?;
    let providers = client.get_services_new().await?;
    console_log!(
        "obtained list of {} service providers on the network",
        providers.items.len()
    );

    // this will only return a `None` if the list is empty
    let mut rng = thread_rng();
    providers
        .items
        .choose(&mut rng)
        .map(|service| &service.service_provider_client_id)
        .cloned()
        .ok_or(MixFetchError::NoNetworkRequesters)
}

pub(crate) async fn get_combined_gateways(
    hidden: Vec<SerializableGateway>,
    nym_apis: &[Url],
) -> Result<Vec<gateway::Node>, MixFetchError> {
    let mut rng = thread_rng();

    let mut api_gateways = current_gateways(&mut rng, nym_apis).await?;
    if !hidden.is_empty() {
        // make sure to override duplicates
        let mut gateways: HashMap<_, _> = api_gateways
            .into_iter()
            .map(|g| (g.identity_key.to_base58_string(), g))
            .collect();

        for node in hidden {
            let id = node.identity_key.clone();
            let converted: Result<gateway::Node, _> = node.try_into();
            match converted {
                Err(err) => {
                    console_warn!("failed to add gateway '{id}' into the topology: {err}");
                }
                Ok(gateway) => {
                    if gateways
                        .insert(gateway.identity_key.to_base58_string(), gateway)
                        .is_some()
                    {
                        console_warn!("overridden gateway '{id}'")
                    }
                }
            }
        }

        api_gateways = gateways.into_values().collect();
    }

    Ok(api_gateways)
}

#[allow(non_snake_case)]
pub(crate) async fn _hack__get_topology_provider(
    combined_gateways: Vec<gateway::Node>,
    nym_apis: &[Url],
) -> Result<HardcodedTopologyProvider, MixFetchError> {
    let mut rng = thread_rng();

    let mixnodes = current_mixnodes(&mut rng, nym_apis).await?;
    Ok(HardcodedTopologyProvider::new(NymTopology::new_unordered(
        mixnodes,
        combined_gateways,
    )))
}
