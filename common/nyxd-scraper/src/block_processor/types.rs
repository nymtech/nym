// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ScraperError;
use crate::helpers;
use tendermint::{abci, block, tx, Block, Hash};
use tendermint_rpc::endpoint::{block as block_endpoint, block_results, validators};
use tendermint_rpc::event::{Event, EventData};

// just get all everything out of tx::Response, but parse raw `tx` bytes
#[derive(Clone, Debug)]
pub struct ParsedTransactionResponse {
    /// The hash of the transaction.
    ///
    /// Deserialized from a hex-encoded string (there is a discrepancy between
    /// the format used for the request and the format used for the response in
    /// the Tendermint RPC).
    pub hash: Hash,

    pub height: block::Height,

    pub index: u32,

    pub tx_result: abci::types::ExecTxResult,

    pub tx: cosmrs::tx::Tx,

    pub proof: Option<tx::Proof>,
}

#[derive(Debug)]
pub struct FullBlockInformation {
    /// Basic block information, including its signers.
    pub block: Block,

    /// All of the emitted events alongside any tx results.
    pub results: block_results::Response,

    /// Validator set for this particular block
    pub validators: validators::Response,

    /// Transaction results from this particular block
    pub transactions: Vec<ParsedTransactionResponse>,
}

impl FullBlockInformation {
    pub fn ensure_proposer(&self) -> Result<(), ScraperError> {
        let block_proposer = self.block.header.proposer_address;
        if !self
            .validators
            .validators
            .iter()
            .any(|v| v.address == block_proposer)
        {
            let proposer = helpers::validator_consensus_address(block_proposer)?;
            return Err(ScraperError::BlockProposerNotInValidatorSet {
                height: self.block.header.height.value() as u32,
                proposer: proposer.to_string(),
            });
        }
        Ok(())
    }
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
            // we don't care about `NewBlock` until CometBFT 0.38, i.e. until we upgrade to wasmd 0.50
            EventData::NewBlock { .. } => {
                return Err(ScraperError::InvalidSubscriptionEvent {
                    query,
                    kind: "NewBlock".to_string(),
                })
            }
            EventData::LegacyNewBlock { block, .. } => block,
            EventData::Tx { .. } => {
                return Err(ScraperError::InvalidSubscriptionEvent {
                    query,
                    kind: "Tx".to_string(),
                })
            }
            EventData::GenericJsonEvent(_) => {
                return Err(ScraperError::InvalidSubscriptionEvent {
                    query,
                    kind: "GenericJsonEvent".to_string(),
                })
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
