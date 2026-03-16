// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ScraperError;
use std::collections::BTreeMap;
use tendermint::{Block, Hash, abci, block, tx};
use tendermint_rpc::endpoint::{block as block_endpoint, block_results, validators};
use tendermint_rpc::event::{Event, EventData};

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
    /*
        pub parsed_messages: BTreeMap<usize, serde_json::Value>,

    pub parsed_message_urls: BTreeMap<usize, String>,
     */
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
}

impl From<Block> for BlockToProcess {
    fn from(block: Block) -> Self {
        BlockToProcess {
            height: block.header.height.value() as u32,
            block,
        }
    }
}

impl TryFrom<Event> for BlockToProcess {
    type Error = ScraperError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        let query = event.query.clone();

        // TODO: we're losing `result_begin_block` and `result_end_block` here but maybe that's fine?
        let maybe_block = match event.data {
            EventData::NewBlock { block, .. } => block,
            EventData::LegacyNewBlock { block, .. } => block,
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

        Ok((*block).into())
    }
}

impl From<block_endpoint::Response> for BlockToProcess {
    fn from(value: block_endpoint::Response) -> Self {
        value.block.into()
    }
}
