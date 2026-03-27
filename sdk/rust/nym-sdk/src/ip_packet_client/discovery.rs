// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! IPR gateway discovery — find and rank IPR-enabled exit gateways via the Nym API.

use std::collections::HashMap;

use nym_crypto::asymmetric::ed25519;
use nym_ip_packet_requests::v9;
use nym_network_defaults::ApiUrl;
use nym_sphinx::addressing::clients::Recipient;
use nym_validator_client::nym_api::NymApiClientExt;
use tracing::{debug, error, info};

use rand::seq::SliceRandom;

use crate::Error;

#[derive(Clone)]
pub struct IprWithPerformance {
    pub address: Recipient,
    pub identity: ed25519::PublicKey,
    pub performance: u8,
}

#[allow(clippy::result_large_err)]
pub fn create_nym_api_client(
    nym_api_urls: Vec<ApiUrl>,
) -> Result<nym_http_api_client::Client, Error> {
    let user_agent = format!("nym-sdk/{}", env!("CARGO_PKG_VERSION"));

    let urls = nym_api_urls
        .into_iter()
        .map(|url| url.url.parse())
        .collect::<Result<Vec<nym_http_api_client::Url>, _>>()
        .map_err(|err| {
            error!("malformed nym-api url: {err}");
            Error::NoNymAPIUrl
        })?;

    if urls.is_empty() {
        return Err(Error::NoNymAPIUrl);
    }

    let client = nym_http_api_client::ClientBuilder::new_with_urls(urls)?
        .with_user_agent(user_agent)
        .build()?;

    Ok(client)
}

pub async fn retrieve_exit_nodes_with_performance(
    client: nym_http_api_client::Client,
) -> Result<Vec<IprWithPerformance>, Error> {
    let all_nodes = client
        .get_all_described_nodes_v2()
        .await?
        .into_iter()
        .map(|described| (described.ed25519_identity_key(), described))
        .collect::<HashMap<_, _>>();

    let exit_gateways = client.get_all_basic_nodes_with_metadata().await?.nodes;

    let mut described = Vec::new();

    for exit in exit_gateways {
        let Some(node) = all_nodes.get(&exit.ed25519_identity_pubkey) else {
            continue;
        };

        // Only select nodes running a version that supports v9 (LP Stream framing)
        let Ok(node_version) = semver::Version::parse(node.version()) else {
            debug!(
                "Skipping node {}: unable to parse version '{}'",
                exit.ed25519_identity_pubkey,
                node.version()
            );
            continue;
        };
        if node_version < v9::MIN_RELEASE_VERSION {
            debug!(
                "Skipping node {}: version {} < minimum {}",
                exit.ed25519_identity_pubkey,
                node_version,
                v9::MIN_RELEASE_VERSION
            );
            continue;
        }

        if let Some(ipr_info) = node.description.ip_packet_router.clone() {
            if let Ok(parsed_address) = ipr_info.address.parse() {
                described.push(IprWithPerformance {
                    address: parsed_address,
                    identity: exit.ed25519_identity_pubkey,
                    performance: exit.performance.round_to_integer(),
                })
            }
        }
    }

    Ok(described)
}

/// Select the highest-performance IPR gateway from the directory.
pub async fn get_best_ipr(client: nym_http_api_client::Client) -> Result<Recipient, Error> {
    let nodes = retrieve_exit_nodes_with_performance(client).await?;
    info!("Found {} Exit Gateways", nodes.len());

    let selected_ipr = nodes
        .choose_weighted(&mut rand::thread_rng(), |gw| gw.performance as f64)
        .map_err(|_| Error::NoGatewayAvailable)?;

    let ipr_address = selected_ipr.address;

    info!(
        "Using IPR: {} (Gateway: {}, Performance: {:?})",
        ipr_address, selected_ipr.identity, selected_ipr.performance
    );

    Ok(ipr_address)
}
