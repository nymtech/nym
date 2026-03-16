// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::PruningOptions;
use crate::block_processor::ephemeral_storage::Ephemeral;
use crate::block_processor::helpers::split_request_range;
use crate::block_processor::types::BlockToProcess;
use crate::block_requester::BlockRequest;
use crate::error::ScraperError;
use crate::modules::{BlockModule, MsgModule, TxModule};
use crate::rpc_client::{RetrievalConfig, RpcClient};
use crate::storage::{
    FullBlockInformation, NyxdScraperStorage, NyxdScraperTransaction, persist_block,
};
use futures::StreamExt;
use std::cmp::max;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::ops::{Add, Range};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::time::{Instant, interval_at};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, trace, warn};

mod ephemeral_storage;
mod helpers;
pub(crate) mod pruning;
pub(crate) mod types;

const MISSING_BLOCKS_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const MAX_MISSING_BLOCKS_DELAY: Duration = Duration::from_secs(15);
const MAX_RANGE_SIZE: usize = 30;

#[derive(Debug, Default)]
struct PendingSync {
    request_in_flight: HashSet<u32>,
    queued_requests: VecDeque<Range<u32>>,
}

impl PendingSync {
    fn is_empty(&self) -> bool {
        self.request_in_flight.is_empty() && self.queued_requests.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct BlockProcessorConfig {
    pub pruning_options: PruningOptions,
    pub store_precommits: bool,
    pub explicit_starting_block_height: Option<u32>,
    pub use_best_effort_start_height: bool,
}

impl Default for BlockProcessorConfig {
    fn default() -> Self {
        Self {
            pruning_options: PruningOptions::nothing(),
            store_precommits: true,
            explicit_starting_block_height: None,
            use_best_effort_start_height: false,
        }
    }
}

impl BlockProcessorConfig {
    pub fn new(
        pruning_options: PruningOptions,
        store_precommits: bool,
        explicit_starting_block_height: Option<u32>,
        use_best_effort_start_height: bool,
    ) -> Self {
        Self {
            pruning_options,
            store_precommits,
            explicit_starting_block_height,
            use_best_effort_start_height,
        }
    }
}

pub struct BlockProcessorPersistence<S> {
    config: BlockProcessorConfig,
    synced: Arc<Notify>,
    last_pruned_height: u32,

    storage: S,
}

impl<S> BlockProcessorPersistence<S>
where
    S: NyxdScraperStorage,
{
    pub(crate) async fn new(
        config: BlockProcessorConfig,
        synced: Arc<Notify>,
        storage: S,
    ) -> Result<Self, ScraperError> {
        let last_pruned = storage.get_pruned_height().await?;
        let last_pruned_height = last_pruned.try_into().unwrap_or_default();

        debug!(pruned_height = %last_pruned_height, "setting up block processor...");

        Ok(Self {
            config,
            synced,
            last_pruned_height,
            storage,
        })
    }

    #[must_use]
    pub fn with_pruning(mut self, pruning_options: PruningOptions) -> Self {
        self.config.pruning_options = pruning_options;
        self
    }

    async fn stored_last_processed_height(&self) -> Result<u32, ScraperError> {
        let last_processed = self.storage.get_last_processed_height().await?;
        let last_processed_height = last_processed.try_into().unwrap_or_default();
        Ok(last_processed_height)
    }

    async fn persist_block(
        &mut self,
        full_info: &FullBlockInformation,
    ) -> Result<(), ScraperError> {
        // process the entire block as a transaction so that if anything fails,
        // we wouldn't end up with a corrupted storage.
        let mut tx = self
            .storage
            .begin_processing_tx()
            .await
            .map_err(ScraperError::tx_begin_failure)?;

        persist_block(full_info, &mut tx, self.config.store_precommits).await?;

        let commit_start = Instant::now();
        tx.commit().await.map_err(ScraperError::tx_commit_failure)?;
        crate::storage::helpers::log_db_operation_time("committing processing tx", commit_start);

        let last_processed_height = full_info.block.header.height.value() as u32;
        if let Err(err) = self.maybe_prune_storage(last_processed_height).await {
            error!("failed to prune the storage: {err}");
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn prune_storage(&mut self, last_processed_height: u32) -> Result<(), ScraperError> {
        let keep_recent = self.config.pruning_options.strategy_keep_recent();
        let last_to_keep = last_processed_height - keep_recent;

        info!(
            keep_recent,
            oldest_to_keep = last_to_keep,
            "pruning the storage"
        );

        let lowest: u32 = self
            .storage
            .lowest_block_height()
            .await?
            .unwrap_or_default()
            .try_into()
            .unwrap_or_default();

        let to_prune = last_to_keep.saturating_sub(lowest);
        match to_prune {
            v if v > 1000 => warn!("approximately {v} blocks worth of data will be pruned"),
            v if v > 100 => info!("approximately {v} blocks worth of data will be pruned"),
            0 => trace!("no blocks to prune"),
            v => debug!("approximately {v} blocks worth of data will be pruned"),
        }

        if to_prune == 0 {
            self.last_pruned_height = last_processed_height;
            return Ok(());
        }

        self.storage
            .prune_storage(last_to_keep, last_processed_height)
            .await?;

        self.last_pruned_height = last_processed_height;
        Ok(())
    }

    async fn maybe_prune_storage(
        &mut self,
        last_processed_height: u32,
    ) -> Result<(), ScraperError> {
        debug!("checking for storage pruning");

        if self.config.pruning_options.strategy.is_nothing() {
            trace!("the current pruning strategy is 'nothing'");
            return Ok(());
        }

        let interval = self.config.pruning_options.strategy_interval();
        if self.last_pruned_height + interval <= last_processed_height {
            self.prune_storage(last_processed_height).await?;
        }

        Ok(())
    }
}

pub struct BlockProcessor<S = Ephemeral> {
    cancel: CancellationToken,
    last_processed_height: u32,
    last_processed_at: Instant,
    pending_sync: PendingSync,
    queued_blocks: BTreeMap<u32, BlockToProcess>,

    /// Specifies how much data to actually retrieve per block
    retrieval_config: RetrievalConfig,

    rpc_client: RpcClient,
    incoming: UnboundedReceiverStream<BlockToProcess>,
    block_requester: Sender<BlockRequest>,
    persistence: Option<BlockProcessorPersistence<S>>,

    // future work: rather than sending each msg to every msg module,
    // let them subscribe based on `type_url` inside the message itself
    // (like "/cosmwasm.wasm.v1.MsgExecuteContract")
    block_modules: Vec<Box<dyn BlockModule + Send>>,
    tx_modules: Vec<Box<dyn TxModule + Send>>,
    msg_modules: Vec<Box<dyn MsgModule + Send>>,
}

#[allow(clippy::too_many_arguments)]
impl<S> BlockProcessor<S>
where
    S: NyxdScraperStorage,
{
    pub fn new(
        cancel: CancellationToken,
        incoming: UnboundedReceiver<BlockToProcess>,
        block_requester: Sender<BlockRequest>,
        rpc_client: RpcClient,
    ) -> Self {
        BlockProcessor {
            cancel,
            last_processed_height: Default::default(),
            last_processed_at: Instant::now(),
            pending_sync: Default::default(),
            queued_blocks: Default::default(),
            retrieval_config: RetrievalConfig::default(),
            rpc_client,
            incoming: incoming.into(),
            block_requester,
            persistence: None,
            block_modules: vec![],
            tx_modules: vec![],
            msg_modules: vec![],
        }
    }

    #[must_use]
    pub fn with_retrieval_config(mut self, retrieval_config: RetrievalConfig) -> Self {
        self.retrieval_config = retrieval_config;
        self
    }

    pub async fn with_persistence(
        mut self,
        persistence: BlockProcessorPersistence<S>,
    ) -> Result<Self, ScraperError> {
        let last_processed_height = persistence.stored_last_processed_height().await?;
        debug!(last_processed_height = %last_processed_height, "setting up block processor...");

        self.persistence = Some(persistence);
        Ok(self)
    }

    pub(super) async fn process_block(
        &mut self,
        block: BlockToProcess,
    ) -> Result<(), ScraperError> {
        info!("processing block at height {}", block.height);

        let full_info = self
            .rpc_client
            .try_get_full_details(block, self.retrieval_config)
            .await?;

        if let Some(tx_info) = &full_info.transactions {
            debug!("this block has {} transaction(s)", tx_info.len());
            for tx in tx_info {
                let details = &tx.tx_details;
                debug!(
                    "{} has {} message(s)",
                    details.hash,
                    details.tx.body.messages.len()
                );
                for (index, msg) in details.tx.body.messages.iter().enumerate() {
                    debug!("{index}: {:?}", msg.type_url)
                }
            }
        }

        // if we have enabled persistence, do try to store the block information
        if let Some(persistence) = &mut self.persistence {
            persistence.persist_block(&full_info).await?;
        }

        // let the modules do whatever they want
        // the ones wanting the full block:
        for block_module in &mut self.block_modules {
            block_module.handle_block(&full_info).await?;
        }

        // the ones wanting transactions (assuming tx retrieval is enabled):
        if let Some(tx_info) = &full_info.transactions {
            for block_tx in tx_info {
                for tx_module in &mut self.tx_modules {
                    tx_module.handle_tx(block_tx).await?;
                }
                let tx_details = &block_tx.tx_details;

                // the ones concerned with individual messages
                for (index, msg) in tx_details.tx.body.messages.iter().enumerate() {
                    let Some(decoded) = block_tx.decoded_messages.get(&index) else {
                        warn!(
                            "height: {} tx: {} tx_index: {}, msg_index: {index}: message failed to get decoded",
                            tx_details.height(),
                            tx_details.hash,
                            tx_details.index,
                        );
                        continue;
                    };
                    for msg_module in &mut self.msg_modules {
                        if msg.type_url == msg_module.type_url() {
                            msg_module
                                .handle_msg(index, msg, decoded, tx_details)
                                .await?
                        }
                    }
                }
            }
        }

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

    pub(super) fn last_process_height(&self) -> u32 {
        self.last_processed_height
    }

    async fn maybe_request_missing_blocks(&mut self) -> Result<(), ScraperError> {
        // we're still processing, so we're good
        if self.last_processed_at.elapsed() < MAX_MISSING_BLOCKS_DELAY {
            debug!("no need to request missing blocks");
            return Ok(());
        }

        if self.try_request_pending().await {
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

        self.request_missing_blocks(request_range).await?;

        Ok(())
    }

    async fn request_missing_blocks(
        &mut self,
        request_range: Range<u32>,
    ) -> Result<(), ScraperError> {
        let request_range = if request_range.len() > MAX_RANGE_SIZE {
            let mut split = split_request_range(request_range);

            // SAFETY: we know that after the split of a non-empty range we have AT LEAST one value
            #[allow(clippy::unwrap_used)]
            let first = split.pop_front().unwrap();
            self.pending_sync.queued_requests = split;
            self.pending_sync.request_in_flight = first.clone().collect();

            first
        } else {
            request_range
        };

        self.send_blocks_request(request_range).await
    }

    // technically we're not mutating self here,
    // but we need it to help the compiler figure out the future is `Send`
    async fn send_blocks_request(&mut self, request_range: Range<u32>) -> Result<(), ScraperError> {
        debug!("requesting missing blocks: {request_range:?}");

        self.block_requester
            .send(BlockRequest::Range(request_range))
            .await?;
        Ok(())
    }

    async fn next_incoming(&mut self, block: BlockToProcess) {
        let height = block.height;

        self.pending_sync.request_in_flight.remove(&height);

        if self.last_processed_height == 0 {
            // this is the first time we've started up the process
            debug!("setting up initial processing height");
            self.last_processed_height = height - 1
        }

        if height <= self.last_processed_height {
            warn!("we have already processed block for height {height}");
            return;
        }

        if self.last_processed_height + 1 != height {
            if self.queued_blocks.insert(height, block).is_some() {
                warn!("we have already queued up block for height {height}");
            }
            return;
        }

        if let Err(err) = self.process_block(block).await {
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

        self.try_request_pending().await;

        if let Some(persistence) = &self.persistence
            && self.pending_sync.is_empty()
        {
            persistence.synced.notify_one();
        }
    }

    async fn try_request_pending(&mut self) -> bool {
        if self.pending_sync.request_in_flight.is_empty() {
            if let Some(next_sync) = self.pending_sync.queued_requests.pop_front() {
                debug!(
                    "current request range has been resolved. requesting another bunch of blocks"
                );
                if let Err(err) = self.send_blocks_request(next_sync.clone()).await {
                    error!("failed to request resync blocks: {err}");
                    self.pending_sync.queued_requests.push_front(next_sync);
                } else {
                    self.pending_sync.request_in_flight = next_sync.collect()
                }

                return true;
            }
        }

        false
    }

    // technically we're not mutating self here,
    // but we need it to help the compiler figure out the future is `Send`
    async fn startup_resync(&mut self) -> Result<(), ScraperError> {
        assert!(self.pending_sync.is_empty());
        info!("attempting to run startup resync...");

        let Some(persistence) = self.persistence.as_mut() else {
            // without data persistence, we're always starting from scratch
            return Ok(());
        };

        persistence
            .maybe_prune_storage(self.last_processed_height)
            .await?;

        let latest_block = self.rpc_client.current_block_height().await? as u32;
        info!("obtained latest block height: {latest_block}");

        if latest_block > self.last_processed_height && self.last_processed_height != 0 {
            info!("we have already processed some blocks in the past - attempting to resume...");
            // in case we were offline for a while,
            // make sure we don't request blocks we'd have to prune anyway
            let keep_recent = persistence.config.pruning_options.strategy_keep_recent();
            let last_to_keep = latest_block - keep_recent;

            if !persistence.config.pruning_options.strategy.is_nothing() {
                self.last_processed_height = max(self.last_processed_height, last_to_keep);
            }

            let request_range = self.last_processed_height + 1..latest_block + 1;
            info!(
                keep_recent = %keep_recent,
                last_to_keep = %last_to_keep,
                last_processed_height = %self.last_processed_height,
                "we need to request {request_range:?} to resync"
            );
            self.request_missing_blocks(request_range).await?;
            return Ok(());
        }

        // this is the first time starting up
        if self.last_processed_height == 0 {
            info!("this is the first time starting up");
            let Some(starting_height) = persistence.config.explicit_starting_block_height else {
                info!("no starting block height set - will use the default behaviour");
                // nothing to do
                return Ok(());
            };

            info!("attempting to start the scraper from block {starting_height}");
            let earliest_available =
                self.rpc_client.earliest_available_block_height().await? as u32;
            info!("earliest available block height: {earliest_available}");

            if earliest_available > starting_height
                && persistence.config.use_best_effort_start_height
            {
                error!("the earliest available block is higher than the desired starting height");
                return Err(ScraperError::BlocksUnavailable {
                    height: starting_height,
                });
            }

            let starting_height = if earliest_available > starting_height {
                // add few additional blocks to account for all the startup waiting
                // because the node might have pruned few blocks since
                earliest_available + 10
            } else {
                starting_height
            };

            let request_range = starting_height..latest_block + 1;

            info!("going to start the scraper from block {starting_height}");
            info!("we need to request {request_range:?} before properly starting up");

            self.request_missing_blocks(request_range).await?;
        }

        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        info!("starting block processor processing loop");

        // sure, we could be more efficient and reset it on every processed block,
        // but the overhead is so minimal that it doesn't matter
        let mut missing_check_interval = interval_at(
            Instant::now().add(MISSING_BLOCKS_CHECK_INTERVAL),
            MISSING_BLOCKS_CHECK_INTERVAL,
        );

        if let Err(err) = self.startup_resync().await {
            error!("failed to perform startup sync: {err}");
            self.cancel.cancel();
            return;
        };

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
