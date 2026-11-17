// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ScraperError;
use std::collections::BTreeMap;
use tendermint::{Block, Hash, abci, block, tx};
use tendermint_rpc::endpoint::{block as block_endpoint, block_results, validators};
use tendermint_rpc::event::{Event, EventData};
use tracing::warn;

/// Message decoded from the raw transaction and converted into json.
/// Note that it might have gone through additional processing as set by the `MessageRegistry`
#[derive(Clone, Debug)]
pub struct DecodedMessage {
    pub type_url: String,
    pub decoded_content: serde_json::Value,
}

#[derive(Clone, Debug)]
pub struct ParsedTransactionDetails {
    /// The hash of the transaction.
    ///
    /// Deserialized from a hex-encoded string (there is a discrepancy between
    /// the format used for the request and the format used for the response in
    /// the Tendermint RPC).
    pub hash: Hash,

    pub index: u32,

    pub tx_result: abci::types::ExecTxResult,

    pub tx: cosmrs::tx::Tx,

    pub proof: Option<tx::Proof>,

    pub block: Block,
}

impl ParsedTransactionDetails {
    pub fn height(&self) -> block::Height {
        self.block.header.height
    }
}

// just get all everything out of tx::Response, but parse raw `tx` bytes
#[derive(Clone, Debug)]
pub struct ParsedTransactionResponse {
    pub tx_details: ParsedTransactionDetails,

    pub decoded_messages: BTreeMap<usize, DecodedMessage>,
}

#[derive(Debug)]
pub struct FullBlockInformation {
    /// Basic block information, including its signers.
    pub block: Block,

    /// All of the emitted events alongside any tx results.
    pub results: Option<block_results::Response>,

    /// Validator set for this particular block
    pub validators: Option<validators::Response>,

    /// Transaction results from this particular block
    pub transactions: Option<Vec<ParsedTransactionResponse>>,
}

pub(crate) struct BlockToProcess {
    pub(crate) height: u32,
    pub(crate) block: Block,

    /// Execution result of each tx; absent when the block was fetched by height.
    pub(crate) tx_results: Option<Vec<abci::types::ExecTxResult>>,
}

impl BlockToProcess {
    /// Whether the block holds txs whose results it did not arrive with.
    pub(crate) fn lacks_tx_results(&self) -> bool {
        self.tx_results.is_none() && !self.block.data.is_empty()
    }
}

impl From<Block> for BlockToProcess {
    fn from(block: Block) -> Self {
        BlockToProcess {
            height: block.header.height.value() as u32,
            block,
            tx_results: None,
        }
    }
}

impl TryFrom<Event> for BlockToProcess {
    type Error = ScraperError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        let query = event.query.clone();

        // TODO: we're losing `result_begin_block` and `result_end_block` here but maybe that's fine?
        let (maybe_block, tx_results) = match event.data {
            EventData::NewBlock {
                block,
                result_finalize_block,
                ..
            } => (block, result_finalize_block.map(|res| res.tx_results)),
            EventData::LegacyNewBlock { block, .. } => (block, None),
            EventData::Tx { .. } => {
                return Err(ScraperError::InvalidSubscriptionEvent {
                    query,
                    kind: "Tx".to_string(),
                });
            }
            EventData::GenericJsonEvent(_) => {
                return Err(ScraperError::InvalidSubscriptionEvent {
                    query,
                    kind: "GenericJsonEvent".to_string(),
                });
            }
        };

        let Some(block) = maybe_block else {
            return Err(ScraperError::EmptyBlockData { query });
        };

        let height = block.header.height.value() as u32;

        // results that cannot be paired with the block's txs are of no use; have them fetched instead
        let tx_results = tx_results.filter(|results| {
            let paired = results.len() == block.data.len();
            if !paired {
                warn!(
                    "block {height} arrived with {} tx result(s) for {} tx(s) - ignoring them",
                    results.len(),
                    block.data.len()
                );
            }
            paired
        });

        Ok(BlockToProcess {
            height,
            block: *block,
            tx_results,
        })
    }
}

impl From<block_endpoint::Response> for BlockToProcess {
    fn from(value: block_endpoint::Response) -> Self {
        value.block.into()
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use tendermint_rpc::Response;
    use tendermint_rpc::event::Event;
    use tendermint_rpc::event::v0_38::DeEvent;

    /// Height of the recorded block.
    pub(crate) const BLOCK_HEIGHT: u32 = 26594016;

    /// Hash of the recorded block's only tx, as reported by the chain's own `tx` endpoint.
    pub(crate) const TX_HASH: &str =
        "3F8CAFA1914C73CF6D5C0BE4D4DA8305BBA85B207012CEC308DDA5061E199D0C";

    /// Gas used by that tx, as reported by the chain's own `tx` endpoint.
    pub(crate) const TX_GAS_USED: i64 = 116900;

    /// A new-block websocket message recorded from mainnet, trimmed of the parts nothing here reads:
    /// the commit signatures and every event.
    pub(crate) const NEW_BLOCK_MESSAGE: &str = r#"{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "query": "tm.event='NewBlock'",
    "data": {
      "type": "tendermint/event/NewBlock",
      "value": {
        "block": {
          "header": {
            "version": { "block": "11" },
            "chain_id": "nyx",
            "height": "26594016",
            "time": "2026-11-17T09:31:50.683353705Z",
            "last_block_id": {
              "hash": "6D670D8ED6C81E9AB60D29F101A3729A6D14AE438CE65D1212D677583116B08B",
              "parts": { "total": 1, "hash": "F6EAC194F65936E902A5125987124D3E4D726BA4A6E362D1E2AEEDEA061C144B" }
            },
            "last_commit_hash": "F37D71A6C836E37636904CB3C622ABFD23768BC77048BA492B8C8F87A9F08AE4",
            "data_hash": "905A0042575F60B647D449CFEF35EB07AFF77EB5413DF96A756B696D137A6B61",
            "validators_hash": "9BB2A45B37771236DF0EDD746B419EAF7DF4D8E30D8913C20F657B15F0A82A39",
            "next_validators_hash": "9BB2A45B37771236DF0EDD746B419EAF7DF4D8E30D8913C20F657B15F0A82A39",
            "consensus_hash": "048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F",
            "app_hash": "9224745E528104D7AFC032B132B6BBD28F669E845444C5FA8A745EB5CA1B77FE",
            "last_results_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
            "evidence_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
            "proposer_address": "4E4A4575F97EDCE249812A7AD125414AFCD86933"
          },
          "data": {
            "txs": [
              "CrUCCvwBCiQvY29zbXdhc20ud2FzbS52MS5Nc2dFeGVjdXRlQ29udHJhY3QS0wEKKG4xcWgyOGdyaGRyajU5enhteHdkd3NkZ3RyOHM2eDBncXo4M3N3amoSPG4xcjdzNmFrc3ljNnBxYXJkeDg4azNya2dmYWd3dmo0ejR6dW05bW16MnNmazN6bTJtaGEwc2Q0ZG51bhpdeyJkZXBvc2l0X3RpY2tldF9ib29rX2Z1bmRzIjp7ImlkZW50aXR5X2tleSI6IkVtalF1MXBBNWdkdWp1VnRFMU1ZUlN6QVl6VHhuOW1pemdkdlp4enlScXJ0In19KgoKBHVueW0SAjUwEjRub2RlLXN0YXR1cy1hcGktcHJveHktdW5rbm93bjogcGVyZm9ybWluZyAxIGRlcG9zaXRzEmgKUgpGCh8vY29zbW9zLmNyeXB0by5zZWNwMjU2azEuUHViS2V5EiMKIQMY0Yah3pNBoVVoZSFwUkMLHLul7j4DGpdpP3VdSGf6nBIECgIIARiUlA0SEgoMCgR1bnltEgQzNjU3EMn2CBpAwDABEj5I4vmVbg6FMmPvcAbw3iJhHWijOsC+TCMwZp1w2at/OmHpNSPbiic9E7aBhvpJLn0mwVK2QgAntjSdxw=="
            ]
          },
          "evidence": { "evidence": [] },
          "last_commit": {
            "height": "26594015",
            "round": 0,
            "block_id": {
              "hash": "6D670D8ED6C81E9AB60D29F101A3729A6D14AE438CE65D1212D677583116B08B",
              "parts": { "total": 1, "hash": "F6EAC194F65936E902A5125987124D3E4D726BA4A6E362D1E2AEEDEA061C144B" }
            },
            "signatures": []
          }
        },
        "block_id": {
          "hash": "90E47CAA41232BAAFEB341A99F656B6D136D2100C94C05DEFBEA01BD6DBB271A",
          "parts": { "total": 1, "hash": "8E94F2E77F361ECA29B21DE7F28000368619B1C18F7A5CFB2F10485400975948" }
        },
        "result_finalize_block": {
          "tx_results": [
            { "code": 0, "data": null, "log": "", "info": "", "gas_wanted": "146249", "gas_used": "116900", "events": [], "codespace": "" }
          ],
          "validator_updates": [],
          "app_hash": "jFFmd2CKVNhhAKfVgNq+Jc3ENlavuhsJ3DU9BikAcko="
        }
      }
    }
  }
}"#;

    pub(crate) fn parse_new_block_message(message: &str) -> Event {
        DeEvent::from_string(message)
            .expect("the message is not a valid event")
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::{
        BLOCK_HEIGHT, NEW_BLOCK_MESSAGE, TX_GAS_USED, parse_new_block_message,
    };
    use super::*;

    // Dropping the results an event carries sends every live block back to rpc lookups, which is
    // exactly the race this path exists to avoid.
    #[test]
    fn a_new_block_event_keeps_its_tx_results() {
        let block = BlockToProcess::try_from(parse_new_block_message(NEW_BLOCK_MESSAGE))
            .expect("the recorded event must convert");

        assert_eq!(BLOCK_HEIGHT, block.height);

        let tx_results = block
            .tx_results
            .expect("the event's tx results were dropped");
        assert_eq!(1, tx_results.len());
        assert_eq!(TX_GAS_USED, tx_results[0].gas_used);
    }

    // Results that cannot be paired one to one with the block's txs must not be trusted: without
    // them the block is treated like any other that arrived bare, and its results get fetched.
    #[test]
    fn tx_results_not_matching_the_block_txs_are_dropped() {
        let mut message: serde_json::Value =
            serde_json::from_str(NEW_BLOCK_MESSAGE).expect("the recorded message is valid json");
        message["result"]["data"]["value"]["result_finalize_block"]["tx_results"] =
            serde_json::json!([]);

        let block = BlockToProcess::try_from(parse_new_block_message(&message.to_string()))
            .expect("the event must still convert");

        assert_eq!(1, block.block.data.len());
        assert!(block.tx_results.is_none());
    }
}
