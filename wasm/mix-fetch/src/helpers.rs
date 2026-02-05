// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixFetchError;
use crate::error::MixFetchError::NoNetworkRequesters;
use nym_http_api_client::Client;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_wasm_utils::console_log;
use rand::seq::SliceRandom;
use rand::thread_rng;

// since this client is temporary (and will be properly integrated into nym-api eventually),
// we're using hardcoded URL for mainnet
const NYM_API_URL: &str = "https://validator.nymtech.net/api/";

pub(crate) async fn get_network_requester(
    nym_api_url: Option<String>,
    preferred: Option<String>,
) -> Result<String, MixFetchError> {
    if let Some(sp) = preferred {
        return Ok(sp);
    }

    let client = Client::new(
        url::Url::parse(&nym_api_url.unwrap_or(NYM_API_URL.to_string()))?,
        None,
    );
    let nodes = client.get_all_described_nodes().await?;
    let providers: Vec<_> = nodes
        .iter()
        .filter_map(|node| {
            node.description
                .network_requester
                .clone()
                .map(|n| n.address)
        })
        .collect();
    console_log!(
        "obtained list of {} service providers on the network",
        providers.len()
    );

    // this will only return a `None` if the list is empty
    let mut rng = thread_rng();
    providers
        .choose(&mut rng)
        .ok_or(NoNetworkRequesters)
        .cloned()
}
