// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixFetchError;
use crate::harbourmaster;
use crate::harbourmaster::HarbourMasterApiClientExt;
use rand::seq::SliceRandom;
use rand::thread_rng;
use wasm_utils::console_log;

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
