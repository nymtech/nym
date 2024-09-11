// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::logs::Log;
use crate::nyxd::TxResponse;
use cosmrs::tendermint::abci;

pub use abci::Event;

// Searches in events for an event of the given event type which contains an
// attribute for with the given key.
pub fn find_tx_attribute(tx: &TxResponse, event_type: &str, attribute_key: &str) -> Option<String> {
    find_event_attribute(&tx.tx_result.events, event_type, attribute_key)
}

pub fn find_event_attribute(
    events: &[Event],
    event_type: &str,
    attribute_key: &str,
) -> Option<String> {
    let event = events.iter().find(|e| e.kind == event_type)?;
    let attribute = event.attributes.iter().find(|&attr| {
        if let Ok(key_str) = attr.key_str() {
            key_str == attribute_key
        } else {
            false
        }
    })?;
    Some(attribute.value_str().ok().map(|str| str.to_string())).flatten()
}

pub fn find_attribute_value_in_logs_or_events(
    logs: &[Log],
    events: &[Event],
    event_type: &str,
    attribute_key: &str,
) -> Option<String> {
    // if logs are empty, i.e. we're using post 0.50 code, parse the events instead
    if !logs.is_empty() {
        #[allow(deprecated)]
        return crate::nyxd::cosmwasm_client::logs::find_attribute_in_logs(
            logs,
            event_type,
            attribute_key,
        )
        .map(|attr| attr.value.clone());
    }

    find_event_attribute(events, event_type, attribute_key)
}
