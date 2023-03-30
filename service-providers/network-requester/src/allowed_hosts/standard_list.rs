// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::allowed_hosts::host::Host;

/// Fetch the standard allowed list from nymtech.net
pub(crate) async fn fetch() -> Vec<Host> {
    log::info!("Refreshing standard allowed hosts");
    get_standard_allowed_list()
        .await
        .split_whitespace()
        .map(|s| Host::from(s.to_string()))
        .collect()
}

async fn get_standard_allowed_list() -> String {
    reqwest::get("https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt")
        .await
        .expect("failed to get allowed hosts")
        .text()
        .await
        .expect("failed to get allowed hosts text")
}
