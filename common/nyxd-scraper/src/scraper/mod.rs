// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::BlockProcessor;
use crate::block_requester::BlockRequester;
use crate::error::ScraperError;
use crate::modules::{BlockModule, MsgModule, TxModule};
use crate::rpc_client::RpcClient;
use crate::scraper::subscriber::{run_websocket_driver, ChainSubscriber};
use crate::storage::ScraperStorage;
use std::path::PathBuf;
use std::sync::Arc;
use tendermint_rpc::WebSocketClientDriver;
use tokio::sync::mpsc::{channel, unbounded_channel};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::info;
use url::Url;

mod subscriber;

pub struct Config {
    /// Url to the websocket endpoint of a validator, for example `wss://rpc.nymtech.net/websocket`
    pub websocket_url: Url,

    /// Url to the rpc endpoint of a validator, for example `https://rpc.nymtech.net/`
    pub rpc_url: Url,

    pub database_path: PathBuf,
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

        let mut chain_subscriber = ChainSubscriber::new(
            &scraper.config.websocket_url,
            scraper.cancel_token.clone(),
            processing_tx,
        )
        .await?;
        let ws_driver = chain_subscriber.ws_driver();

        scraper.start_tasks(
            block_requester,
            block_processor,
            chain_subscriber,
            ws_driver,
        );

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
}

impl NyxdScraper {
    pub fn builder(config: Config) -> NyxdScraperBuilder {
        NyxdScraperBuilder::new(config)
    }

    pub async fn new(config: Config) -> Result<Self, ScraperError> {
        let storage = ScraperStorage::init(&config.database_path).await?;

        Ok(NyxdScraper {
            config,
            task_tracker: TaskTracker::new(),
            cancel_token: CancellationToken::new(),
            startup_sync: Arc::new(Default::default()),
            storage,
        })
    }

    fn start_tasks(
        &self,
        mut block_requester: BlockRequester,
        mut block_processor: BlockProcessor,
        mut chain_subscriber: ChainSubscriber,
        ws_driver: WebSocketClientDriver,
    ) {
        self.task_tracker
            .spawn(async move { block_requester.run().await });
        self.task_tracker
            .spawn(async move { block_processor.run().await });
        self.task_tracker
            .spawn(async move { chain_subscriber.run().await });
        self.task_tracker
            .spawn(run_websocket_driver(ws_driver, self.cancel_token.clone()));
        self.task_tracker.close();
    }

    pub async fn start(&self) -> Result<(), ScraperError> {
        let (processing_tx, processing_rx) = unbounded_channel();
        let (req_tx, req_rx) = channel(5);

        let rpc_client = RpcClient::new(&self.config.rpc_url)?;

        // create the tasks
        let block_requester = BlockRequester::new(
            self.cancel_token.clone(),
            rpc_client.clone(),
            req_rx,
            processing_tx.clone(),
        );
        let block_processor = BlockProcessor::new(
            self.cancel_token.clone(),
            self.startup_sync.clone(),
            processing_rx,
            req_tx,
            self.storage.clone(),
            rpc_client,
        )
        .await?;
        let mut chain_subscriber = ChainSubscriber::new(
            &self.config.websocket_url,
            self.cancel_token.clone(),
            processing_tx,
        )
        .await?;
        let ws_driver = chain_subscriber.ws_driver();

        // spawn them
        self.start_tasks(
            block_requester,
            block_processor,
            chain_subscriber,
            ws_driver,
        );

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
