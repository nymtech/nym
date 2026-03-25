// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! IPR gateway discovery — find and rank IPR-enabled exit gateways via the Nym API.

use std::collections::HashMap;

use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::ApiUrl;
use nym_sphinx::addressing::clients::Recipient;
use nym_validator_client::nym_api::NymApiClientExt;
use tracing::{error, info};

use nym_ip_packet_requests::{
    v8::response::{ConnectResponseReply, ControlResponse, IpPacketResponse, IpPacketResponseData},
    IpPair,
};

use crate::Error;

/// Parse an IPR connect response, returning allocated IPs on success.
#[allow(clippy::result_large_err)]
pub fn parse_connect_response(response: IpPacketResponse) -> Result<IpPair, Error> {
    let control_response = match response.data {
        IpPacketResponseData::Control(c) => c,
        other => return Err(Error::UnexpectedResponseType(other)),
    };

    match *control_response {
        ControlResponse::Connect(connect_resp) => match connect_resp.reply {
            ConnectResponseReply::Success(success) => Ok(success.ips),
            ConnectResponseReply::Failure(reason) => Err(Error::ConnectDenied(reason)),
        },
        _ => Err(Error::UnexpectedResponseType(
            IpPacketResponseData::Control(control_response),
        )),
    }
}

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
        if let Some(ipr_info) = all_nodes
            .get(&exit.ed25519_identity_pubkey)
            .and_then(|n| n.description.ip_packet_router.clone())
        {
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

    let selected_gateway = nodes
        .into_iter()
        .max_by_key(|gw| gw.performance)
        .ok_or_else(|| Error::NoGatewayAvailable)?;

    let ipr_address = selected_gateway.address;

    info!(
        "Using IPR: {} (Gateway: {}, Performance: {:?})",
        ipr_address, selected_gateway.identity, selected_gateway.performance
    );

    Ok(ipr_address)
}
