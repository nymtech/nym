// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::AccountId;
use itertools::Itertools;
use nyxd_scraper_shared::ParsedTransactionResponse;
use std::str::FromStr;

// replicate behaviour of `CosmosMessageAddressesParser` from juno
pub(crate) fn parse_addresses_from_events(tx: &ParsedTransactionResponse) -> Vec<String> {
    let mut addresses: Vec<String> = Vec::new();
    for event in &tx.tx_result.events {
        for attribute in &event.attributes {
            let Ok(value) = attribute.value_str() else {
                continue;
            };

            // Try parsing the address as an account address
            if let Ok(address) = AccountId::from_str(value) {
                addresses.push(address.to_string());
            }
        }
    }
    addresses.into_iter().unique().collect()
}
