// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared Nym API client construction, used by both SOCKS5 network-requester
//! discovery and IPR gateway discovery.

use nym_network_defaults::ApiUrl;
use tracing::error;

use crate::Error;

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
