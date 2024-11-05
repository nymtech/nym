// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::BlockToProcess;
use crate::block_processor::BlockProcessor;
use crate::block_requester::{BlockRequest, BlockRequester};
use crate::error::ScraperError;
use crate::modules::{BlockModule, MsgModule, TxModule};
use crate::rpc_client::RpcClient;
use crate::scraper::subscriber::ChainSubscriber;
use crate::storage::ScraperStorage;
use crate::PruningOptions;
use futures::future::join_all;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::{
    channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{error, info};
use url::Url;

mod subscriber;

pub struct Config {
    /// Url to the websocket endpoint of a validator, for example `wss://rpc.nymtech.net/websocket`
    pub websocket_url: Url,

    /// Url to the rpc endpoint of a validator, for example `https://rpc.nymtech.net/`
    pub rpc_url: Url,

    pub database_path: PathBuf,

    pub pruning_options: PruningOptions,

    pub store_precommits: bool,
}

pub struct NyxdScraperBuilder {
    config: Config,

    block_modules: Vec<Box<dyn BlockModule + Send>>,
    tx_modules: Vec<Box<dyn TxModule + Send>>,
    msg_modules: Vec<Box<dyn MsgModule + Send>>,
}

impl NyxdScraperBuilder {
    pub async fn build_and_start(self) -> Result<NyxdScraper, ScraperError> {
        let scraper = NyxdScraper::new(self.config).await?;

        let (processing_tx, processing_rx) = unbounded_channel();
        let (req_tx, req_rx) = channel(5);

        let rpc_client = RpcClient::new(&scraper.config.rpc_url)?;

        // create the tasks
        let block_requester = BlockRequester::new(
            scraper.cancel_token.clone(),
            rpc_client.clone(),
            req_rx,
            processing_tx.clone(),
        );
        let mut block_processor = BlockProcessor::new(
            scraper.config.pruning_options,
            scraper.config.store_precommits,
            scraper.cancel_token.clone(),
            scraper.startup_sync.clone(),
            processing_rx,
            req_tx,
            scraper.storage.clone(),
            rpc_client,
        )
        .await?;
        block_processor.set_block_modules(self.block_modules);
        block_processor.set_tx_modules(self.tx_modules);
        block_processor.set_msg_modules(self.msg_modules);

        let chain_subscriber = ChainSubscriber::new(
            &scraper.config.websocket_url,
            scraper.cancel_token.clone(),
            scraper.task_tracker.clone(),
            processing_tx,
        )
        .await?;

        scraper.start_tasks(block_requester, block_processor, chain_subscriber);

        Ok(scraper)
    }

    pub fn new(config: Config) -> Self {
        NyxdScraperBuilder {
            config,
            block_modules: vec![],
            tx_modules: vec![],
            msg_modules: vec![],
        }
    }

    pub fn with_block_module<M: BlockModule + Send + 'static>(mut self, module: M) -> Self {
        self.block_modules.push(Box::new(module));
        self
    }

    pub fn with_tx_module<M: TxModule + Send + 'static>(mut self, module: M) -> Self {
        self.tx_modules.push(Box::new(module));
        self
    }

    pub fn with_msg_module<M: MsgModule + Send + 'static>(mut self, module: M) -> Self {
        self.msg_modules.push(Box::new(module));
        self
    }
}

pub struct NyxdScraper {
    config: Config,

    task_tracker: TaskTracker,
    cancel_token: CancellationToken,
    startup_sync: Arc<Notify>,
    pub storage: ScraperStorage,
    rpc_client: RpcClient,
}

impl NyxdScraper {
    pub fn builder(config: Config) -> NyxdScraperBuilder {
        NyxdScraperBuilder::new(config)
    }

    pub async fn new(config: Config) -> Result<Self, ScraperError> {
        config.pruning_options.validate()?;
        let storage = ScraperStorage::init(&config.database_path).await?;
        let rpc_client = RpcClient::new(&config.rpc_url)?;

        Ok(NyxdScraper {
            config,
            task_tracker: TaskTracker::new(),
            cancel_token: CancellationToken::new(),
            startup_sync: Arc::new(Default::default()),
            storage,
            rpc_client,
        })
    }

    fn start_tasks(
        &self,
        mut block_requester: BlockRequester,
        mut block_processor: BlockProcessor,
        mut chain_subscriber: ChainSubscriber,
    ) {
        self.task_tracker
            .spawn(async move { block_requester.run().await });
        self.task_tracker
            .spawn(async move { block_processor.run().await });
        self.task_tracker
            .spawn(async move { chain_subscriber.run().await });

        self.task_tracker.close();
    }

    pub async fn process_single_block(&self, height: u32) -> Result<(), ScraperError> {
        info!(height = height, "attempting to process a single block");
        if !self.task_tracker.is_empty() {
            return Err(ScraperError::ScraperAlreadyRunning);
        }

        let (_, processing_rx) = unbounded_channel();
        let (req_tx, _) = channel(5);

        let mut block_processor = self
            .new_block_processor(req_tx.clone(), processing_rx)
            .await?
            .with_pruning(PruningOptions::nothing());

        let block = self.rpc_client.get_basic_block_details(height).await?;

        block_processor.process_block(block.into()).await
    }

    pub async fn process_block_range(
        &self,
        starting_height: Option<u32>,
        end_height: Option<u32>,
    ) -> Result<(), ScraperError> {
        if !self.task_tracker.is_empty() {
            return Err(ScraperError::ScraperAlreadyRunning);
        }

        let (_, processing_rx) = unbounded_channel();
        let (req_tx, _) = channel(5);

        let mut block_processor = self
            .new_block_processor(req_tx.clone(), processing_rx)
            .await?
            .with_pruning(PruningOptions::nothing());

        let current_height = self.rpc_client.current_block_height().await? as u32;
        let last_processed = block_processor.last_process_height();

        let starting_height = match starting_height {
            // always attempt to use whatever the user has provided
            Some(explicit) => explicit,
            None => {
                // otherwise, attempt to resume where we last stopped
                // and if we haven't processed anything, start from the current height
                if last_processed != 0 {
                    last_processed
                } else {
                    current_height
                }
            }
        };

        let end_height = match end_height {
            // always attempt to use whatever the user has provided
            Some(explicit) => explicit,
            None => {
                // otherwise, attempt to either go from the start height to the height right
                // before the final processed block held in the storage (in case there are gaps)
                // or finally, just go to the current block height
                if last_processed > starting_height {
                    last_processed - 1
                } else {
                    current_height
                }
            }
        };

        info!(
            starting_height = starting_height,
            end_height = end_height,
            "attempting to process block range"
        );

        let range = (starting_height..=end_height).collect::<Vec<_>>();

        // the most likely bottleneck here are going to be the chain queries,
        // so batch multiple requests
        for batch in range.chunks(4) {
            let batch_result = join_all(
                batch
                    .iter()
                    .map(|height| self.rpc_client.get_basic_block_details(*height)),
            )
            .await;
            for result in batch_result {
                match result {
                    Ok(block) => block_processor.process_block(block.into()).await?,
                    Err(err) => {
                        error!("failed to retrieve the block: {err}. stopping...");
                        return Err(err);
                    }
                }
            }
        }

        Ok(())
    }

    fn new_block_requester(
        &self,
        req_rx: Receiver<BlockRequest>,
        processing_tx: UnboundedSender<BlockToProcess>,
    ) -> BlockRequester {
        BlockRequester::new(
            self.cancel_token.clone(),
            self.rpc_client.clone(),
            req_rx,
            processing_tx.clone(),
        )
    }

    async fn new_block_processor(
        &self,
        req_tx: Sender<BlockRequest>,
        processing_rx: UnboundedReceiver<BlockToProcess>,
    ) -> Result<BlockProcessor, ScraperError> {
        BlockProcessor::new(
            self.config.pruning_options,
            self.config.store_precommits,
            self.cancel_token.clone(),
            self.startup_sync.clone(),
            processing_rx,
            req_tx,
            self.storage.clone(),
            self.rpc_client.clone(),
        )
        .await
    }

    async fn new_chain_subscriber(
        &self,
        processing_tx: UnboundedSender<BlockToProcess>,
    ) -> Result<ChainSubscriber, ScraperError> {
        ChainSubscriber::new(
            &self.config.websocket_url,
            self.cancel_token.clone(),
            self.task_tracker.clone(),
            processing_tx,
        )
        .await
    }

    pub async fn start(&self) -> Result<(), ScraperError> {
        let (processing_tx, processing_rx) = unbounded_channel();
        let (req_tx, req_rx) = channel(5);

        // create the tasks
        let block_requester = self.new_block_requester(req_rx, processing_tx.clone());
        let block_processor = self.new_block_processor(req_tx, processing_rx).await?;
        let chain_subscriber = self.new_chain_subscriber(processing_tx).await?;

        // spawn them
        self.start_tasks(block_requester, block_processor, chain_subscriber);

        Ok(())
    }

    pub async fn wait_for_startup_sync(&self) {
        info!("awaiting startup chain sync");
        self.startup_sync.notified().await
    }

    pub async fn stop(self) {
        info!("stopping the chain scraper");
        assert!(self.task_tracker.is_closed());

        self.cancel_token.cancel();
        self.task_tracker.wait().await
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}
