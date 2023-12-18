// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::TxResponse;

pub fn find_tx_attribute(tx: &TxResponse, event_type: &str, attribute_key: &str) -> Option<String> {
    let event = tx.tx_result.events.iter().find(|e| e.kind == event_type)?;
    let attribute = event
        .attributes
        .iter()
        .find(|attr| attr.key == attribute_key)?;
    Some(attribute.value.clone())
}
