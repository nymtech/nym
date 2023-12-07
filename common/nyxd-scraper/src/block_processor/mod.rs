// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::BlockToProcess;
use crate::block_requester::BlockRequest;
use crate::error::ScraperError;
use crate::modules::{BlockModule, MsgModule, TxModule};
use crate::rpc_client::RpcClient;
use crate::storage::{persist_block, ScraperStorage};
use futures::StreamExt;
use std::collections::BTreeMap;
use std::ops::Add;
use std::time::Duration;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::time::{interval_at, Instant};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub(crate) mod types;

const MISSING_BLOCKS_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const MAX_MISSING_BLOCKS_DELAY: Duration = Duration::from_secs(15);

pub struct BlockProcessor {
    cancel: CancellationToken,
    last_processed_height: u32,
    last_processed_at: Instant,
    queued_blocks: BTreeMap<u32, BlockToProcess>,

    rpc_client: RpcClient,
    incoming: UnboundedReceiverStream<BlockToProcess>,
    block_requester: Sender<BlockRequest>,
    storage: ScraperStorage,

    // future work: rather than sending each msg to every msg module,
    // let them subscribe based on `type_url` inside the message itself
    // (like "/cosmwasm.wasm.v1.MsgExecuteContract")
    block_modules: Vec<Box<dyn BlockModule + Send>>,
    tx_modules: Vec<Box<dyn TxModule + Send>>,
    msg_modules: Vec<Box<dyn MsgModule + Send>>,
}

impl BlockProcessor {
    pub fn new(
        cancel: CancellationToken,
        incoming: UnboundedReceiver<BlockToProcess>,
        block_requester: Sender<BlockRequest>,
        storage: ScraperStorage,
        rpc_client: RpcClient,
    ) -> Self {
        BlockProcessor {
            cancel,
            last_processed_height: 0,
            last_processed_at: Instant::now(),
            queued_blocks: Default::default(),
            rpc_client,
            incoming: incoming.into(),
            block_requester,
            storage,
            block_modules: vec![],
            tx_modules: vec![],
            msg_modules: vec![],
        }
    }

    async fn process_block(&mut self, block: BlockToProcess) -> Result<(), ScraperError> {
        info!("processing block at height {}", block.height);

        let full_info = self.rpc_client.try_get_full_details(block).await?;

        debug!(
            "this block has {} transaction(s)",
            full_info.transactions.len()
        );
        for tx in &full_info.transactions {
            debug!("{} has {} message(s)", tx.hash, tx.tx.body.messages.len());
            for (index, msg) in tx.tx.body.messages.iter().enumerate() {
                debug!("{index}: {:?}", msg.type_url)
            }
        }

        // process the entire block as a transaction so that if anything fails,
        // we won't end up with a corrupted storage.
        let mut tx = self.storage.begin_processing_tx().await?;

        persist_block(&full_info, &mut tx).await?;

        // let the modules do whatever they want
        // the ones wanting the full block:
        for block_module in &mut self.block_modules {
            block_module.handle_block(&full_info, &mut tx).await?;
        }

        // the ones wanting transactions:
        for block_tx in full_info.transactions {
            for tx_module in &mut self.tx_modules {
                tx_module.handle_tx(&block_tx, &mut tx).await?;
            }
            // the ones concerned with individual messages
            for (index, msg) in block_tx.tx.body.messages.iter().enumerate() {
                for msg_module in &mut self.msg_modules {
                    msg_module
                        .handle_msg(index, msg, &block_tx, &mut tx)
                        .await?
                }
            }
        }

        tx.commit()
            .await
            .map_err(|source| ScraperError::StorageTxCommitFailure { source })?;

        self.last_processed_height = full_info.block.header.height.value() as u32;
        self.last_processed_at = Instant::now();

        Ok(())
    }

    pub fn set_block_modules(&mut self, modules: Vec<Box<dyn BlockModule + Send>>) {
        self.block_modules = modules;
    }

    pub fn set_tx_modules(&mut self, modules: Vec<Box<dyn TxModule + Send>>) {
        self.tx_modules = modules;
    }

    pub fn set_msg_modules(&mut self, modules: Vec<Box<dyn MsgModule + Send>>) {
        self.msg_modules = modules;
    }

    async fn maybe_request_missing_blocks(&mut self) -> Result<(), ScraperError> {
        // we're still processing, so we're good
        if self.last_processed_at.elapsed() < MAX_MISSING_BLOCKS_DELAY {
            debug!("no need to request missing blocks");
            return Ok(());
        }

        // TODO: properly fill in the gaps later with BlockRequest::Specific,
        let request_range = if let Some((next_available, _)) = self.queued_blocks.first_key_value()
        {
            self.last_processed_height + 1..*next_available
        } else {
            let current_height = self.rpc_client.current_block_height().await? as u32;
            self.last_processed_height + 1..current_height + 1
        };

        self.block_requester
            .send(BlockRequest::Range(request_range))
            .await?;

        Ok(())
    }

    async fn next_incoming(&mut self, block: BlockToProcess) {
        let height = block.height;

        if self.last_processed_height == 0 {
            // TODO: load it from storage instead and make sure storage always has some data in there
            self.last_processed_height = height - 1
        }

        if self.last_processed_height + 1 != height {
            if self.queued_blocks.insert(height, block).is_some() {
                error!("duplicate queued up block for height {height}")
            }
            return;
        } else if let Err(err) = self.process_block(block).await {
            error!("failed to process block at height {height}: {err}");
            return;
        }

        // process as much as we can from the queue
        let mut next = height + 1;
        while let Some(next_block) = self.queued_blocks.remove(&next) {
            if let Err(err) = self.process_block(next_block).await {
                error!("failed to process queued-up block at height {next}: {err}")
            }
            next += 1;
        }
    }

    pub(crate) async fn run(&mut self) {
        info!("starting processing loop");

        // sure, we could be more efficient and reset it on every processed block,
        // but the overhead is so minimal that it doesn't matter
        let mut missing_check_interval = interval_at(
            Instant::now().add(MISSING_BLOCKS_CHECK_INTERVAL),
            MISSING_BLOCKS_CHECK_INTERVAL,
        );

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    info!("received cancellation token");
                    break
                }
                _ = missing_check_interval.tick() => {
                    if let Err(err) = self.maybe_request_missing_blocks().await {
                        error!("failed to request missing blocks: {err}")
                    }
                }
                block = self.incoming.next() => {
                    match block {
                        Some(block) => self.next_incoming(block).await,
                        None => {
                            warn!("stopped receiving new blocks");
                            self.cancel.cancel();
                            break
                        }
                    }
                }
            }
        }
    }
}
