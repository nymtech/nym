// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub use nym_coconut_bandwidth_contract_common::event_attributes::*;
pub use nym_coconut_dkg_common::event_attributes::*;

// it seems that currently validators just emit stringified events (which are also returned as part of deliverTx response)
// as their logs
#[derive(Debug, Serialize, Deserialize)]
pub struct Log {
    #[serde(default)]
    // weird thing is that the first msg_index seems to always be undefined on the raw logs
    pub msg_index: usize,
    pub events: Vec<cosmwasm_std::Event>,
}

/// Searches in logs for the first event of the given event type and in that event
/// for the first attribute with the given attribute key.
pub fn find_attribute<'a>(
    logs: &'a [Log],
    event_type: &str,
    attribute_key: &str,
) -> Option<&'a cosmwasm_std::Attribute> {
    logs.iter()
        .flat_map(|log| log.events.iter())
        .find(|event| event.ty == event_type)?
        .attributes
        .iter()
        .find(|attr| attr.key == attribute_key)
}

// these two functions were separated so that the internal logic could actually be tested
fn parse_raw_str_logs(raw: &str) -> Result<Vec<Log>, NyxdError> {
    // From Cosmos SDK > 0.50 onwards, log field is not populated
    if raw.is_empty() {
        return Ok(Vec::new());
    }

    let logs: Vec<Log> = serde_json::from_str(raw).map_err(|_| NyxdError::MalformedLogString)?;
    if logs.len() != logs.iter().unique_by(|log| log.msg_index).count() {
        // this check is only here because I don't yet fully understand raw log string generation and
        // the fact the first entry does not seem to have `msg_index` defined on it.
        return Err(NyxdError::MalformedLogString);
    }
    Ok(logs)
}

pub fn parse_raw_logs(raw: String) -> Result<Vec<Log>, NyxdError> {
    parse_raw_str_logs(raw.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_parsing_with_single_tx() {
        let raw = r#"[{"events":[{"type":"message","attributes":[{"key":"action","value":"store-code"},{"key":"module","value":"wasm"},{"key":"signer","value":"punk1m4aj8tgc0rqlms3s0c8jf3pcrma5xw2waafzjt"},{"key":"code_id","value":"1"}]}]}]"#;
        let parsed = parse_raw_str_logs(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].msg_index, 0);
        assert_eq!(parsed[0].events.len(), 1);
        assert_eq!(parsed[0].events[0].ty, "message");
        assert_eq!(parsed[0].events[0].attributes[3].key, "code_id");
        assert_eq!(parsed[0].events[0].attributes[3].value, "1");
    }

    #[test]
    fn logs_parsing_with_multiple_txs() {
        let raw = r#"[{"events":[{"type":"message","attributes":[{"key":"action","value":"store-code"},{"key":"module","value":"wasm"},{"key":"signer","value":"punk1q9n5a3cgw3azegcddr82s0f5nxeel4pup8vxzt"},{"key":"code_id","value":"9"}]}]},{"msg_index":1,"events":[{"type":"message","attributes":[{"key":"action","value":"store-code"},{"key":"module","value":"wasm"},{"key":"signer","value":"punk1q9n5a3cgw3azegcddr82s0f5nxeel4pup8vxzt"},{"key":"code_id","value":"10"}]}]},{"msg_index":2,"events":[{"type":"message","attributes":[{"key":"action","value":"store-code"},{"key":"module","value":"wasm"},{"key":"signer","value":"punk1q9n5a3cgw3azegcddr82s0f5nxeel4pup8vxzt"},{"key":"code_id","value":"11"}]}]}]"#;
        let parsed = parse_raw_str_logs(raw).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].msg_index, 0);
        assert_eq!(parsed[1].msg_index, 1);
        assert_eq!(parsed[2].msg_index, 2);

        assert_eq!(parsed[0].events.len(), 1);
        assert_eq!(parsed[0].events[0].ty, "message");
        assert_eq!(parsed[0].events[0].attributes[3].key, "code_id");
        assert_eq!(parsed[0].events[0].attributes[3].value, "9");

        assert_eq!(parsed[2].events.len(), 1);
        assert_eq!(parsed[2].events[0].ty, "message");
        assert_eq!(parsed[2].events[0].attributes[2].key, "signer");
        assert_eq!(
            parsed[2].events[0].attributes[2].value,
            "punk1q9n5a3cgw3azegcddr82s0f5nxeel4pup8vxzt"
        );
    }
}
