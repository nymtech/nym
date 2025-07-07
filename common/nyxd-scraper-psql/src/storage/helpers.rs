// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::AccountId;
use itertools::Itertools;
use nyxd_scraper_shared::ParsedTransactionResponse;
use serde::Serialize;
use std::str::FromStr;

#[derive(Serialize)]
pub(crate) struct PlaceholderStruct {
    pub(crate) typ: String,
    pub(crate) placeholder: String,
}

impl PlaceholderStruct {
    pub(crate) fn new<T>(_: T) -> Self {
        PlaceholderStruct {
            typ: std::any::type_name::<T>().to_string(),
            placeholder: "PLACEHOLDER CONTENT - SOMETHING IS MISSING serde DERIVES".to_string(),
        }
    }
}

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
