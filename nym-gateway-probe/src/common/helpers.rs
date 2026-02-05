// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common::nodes::TestedNodeLpDetails;
use nym_crypto::asymmetric::ed25519;
use nym_ip_packet_requests::v8::response::{
    ControlResponse, DataResponse, InfoLevel, IpPacketResponse, IpPacketResponseData,
};
use nym_lp::peer::LpRemotePeer;
use nym_sdk::{
    DebugConfig, NymApiTopologyProvider, NymApiTopologyProviderConfig, NymNetworkDetails,
    TopologyProvider, mixnet::ReconstructedMessage,
};
use nym_topology::NymTopology;
use tracing::*;
use url::Url;

pub fn to_lp_remote_peer(identity: ed25519::PublicKey, data: TestedNodeLpDetails) -> LpRemotePeer {
    LpRemotePeer::new(identity, data.x25519).with_key_digests(
        data.expected_kem_key_hashes,
        data.expected_signing_key_hashes,
    )
}

pub fn mixnet_debug_config(
    min_gateway_performance: Option<u8>,
    ignore_egress_epoch_role: bool,
) -> nym_client_core::config::DebugConfig {
    let mut debug_config = nym_client_core::config::DebugConfig::default();
    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = true;
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;
    if let Some(minimum_gateway_performance) = min_gateway_performance {
        debug_config.topology.minimum_gateway_performance = minimum_gateway_performance;
    }
    if ignore_egress_epoch_role {
        debug_config.topology.ignore_egress_epoch_role = ignore_egress_epoch_role;
    }

    debug_config
}

pub fn unpack_data_response(reconstructed_message: &ReconstructedMessage) -> Option<DataResponse> {
    match IpPacketResponse::from_reconstructed_message(reconstructed_message) {
        Ok(response) => match response.data {
            IpPacketResponseData::Data(data_response) => Some(data_response),
            IpPacketResponseData::Control(control) => match *control {
                ControlResponse::Info(info) => {
                    let msg = format!("Received info response from the mixnet: {}", info.reply);
                    match info.level {
                        InfoLevel::Info => info!("{msg}"),
                        InfoLevel::Warn => warn!("{msg}"),
                        InfoLevel::Error => error!("{msg}"),
                    }
                    None
                }
                _ => {
                    info!("Ignoring: {:?}", control);
                    None
                }
            },
        },
        Err(err) => {
            warn!("Failed to parse mixnet message: {err}");
            None
        }
    }
}

pub async fn fetch_topology(
    network_details: &NymNetworkDetails,
    debug_config: &DebugConfig,
) -> Result<NymTopology, String> {
    // get Nym API URLs from network_details
    let nym_api_urls: Vec<Url> = network_details
        .nym_api_urls
        .as_ref()
        .map(|urls| urls.iter().filter_map(|u| u.url.parse().ok()).collect())
        .or_else(|| {
            network_details
                .endpoints
                .first()
                .and_then(|e| e.api_url())
                .map(|url| vec![url])
        })
        .unwrap_or_default();

    if nym_api_urls.is_empty() {
        return Err(String::from("No nym-api URLs available to fetch topology"));
    }

    let topology_config = NymApiTopologyProviderConfig {
        min_mixnode_performance: debug_config.topology.minimum_mixnode_performance,
        min_gateway_performance: debug_config.topology.minimum_gateway_performance,
        use_extended_topology: debug_config.topology.use_extended_topology,
        ignore_egress_epoch_role: debug_config.topology.ignore_egress_epoch_role,
    };

    let api_client = nym_http_api_client::Client::new_url(nym_api_urls[0].clone(), None)
        .map_err(|e| e.to_string())?;
    let mut provider = NymApiTopologyProvider::new(topology_config, nym_api_urls, api_client);

    match provider.get_new_topology().await {
        Some(topology) => {
            info!("Fetched network topology");
            Ok(topology)
        }
        None => Err(String::from("Failed to fetch network topology")),
    }
}
