// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::BlockProcessor;
use crate::block_requester::BlockRequester;
use crate::error::ScraperError;
use crate::rpc_client::{RetrievalConfig, RpcClient};
use crate::subscriber::ChainSubscriber;
use crate::{BlockModule, MsgModule, TxModule};
use tokio::sync::mpsc::{channel, unbounded_channel};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::info;
use url::Url;

pub struct WatcherConfig {
    /// Url to the websocket endpoint of a validator, for example, `wss://rpc.nymtech.net/websocket`
    pub websocket_url: Url,

    /// Url to the rpc endpoint of a validator, for example, `https://rpc.nymtech.net/`
    pub rpc_url: Url,
}

pub struct NyxdWatcherBuilder {
    config: WatcherConfig,
    custom_shutdown: CancellationToken,

    block_modules: Vec<Box<dyn BlockModule + Send>>,
    tx_modules: Vec<Box<dyn TxModule + Send>>,
    msg_modules: Vec<Box<dyn MsgModule + Send>>,
}

impl NyxdWatcherBuilder {
    pub fn new(config: WatcherConfig) -> Self {
        NyxdWatcherBuilder {
            config,
            custom_shutdown: CancellationToken::new(),
            block_modules: vec![],
            tx_modules: vec![],
            msg_modules: vec![],
        }
    }

    #[must_use]
    pub fn with_custom_shutdown(mut self, token: CancellationToken) -> Self {
        self.custom_shutdown = token;
        self
    }

    #[must_use]
    pub fn with_block_module<M: BlockModule + Send + 'static>(mut self, module: M) -> Self {
        self.block_modules.push(Box::new(module));
        self
    }

    #[must_use]
    pub fn with_tx_module<M: TxModule + Send + 'static>(mut self, module: M) -> Self {
        self.tx_modules.push(Box::new(module));
        self
    }

    #[must_use]
    pub fn with_msg_module<M: MsgModule + Send + 'static>(mut self, module: M) -> Self {
        self.msg_modules.push(Box::new(module));
        self
    }

    pub async fn build_and_start(self) -> Result<NyxdWatcher, ScraperError> {
        // we must have at least something configured to run the watcher
        if self.block_modules.is_empty()
            && self.tx_modules.is_empty()
            && self.msg_modules.is_empty()
        {
            return Err(ScraperError::NoModulesConfigured);
        }

        let watcher = NyxdWatcher::new();
        let rpc_client = RpcClient::new(&self.config.rpc_url)?;

        let (processing_tx, processing_rx) = unbounded_channel();
        let (req_tx, req_rx) = channel(5);

        // create the tasks
        let block_requester = BlockRequester::new(
            watcher.cancel_token(),
            rpc_client.clone(),
            req_rx,
            processing_tx.clone(),
        );

        let mut block_processor =
            BlockProcessor::new(watcher.cancel_token(), processing_rx, req_tx, rpc_client)
                .with_retrieval_config(RetrievalConfig {
                    get_validators: false,
                    get_transactions: true,
                    get_block_results: false,
                });
        block_processor.set_block_modules(self.block_modules);
        block_processor.set_tx_modules(self.tx_modules);
        block_processor.set_msg_modules(self.msg_modules);

        let chain_subscriber = ChainSubscriber::new(
            &self.config.websocket_url,
            watcher.cancel_token(),
            watcher.task_tracker.clone(),
            processing_tx,
        )
        .await?;

        watcher.start_tasks(block_requester, block_processor, chain_subscriber);

        Ok(watcher)
    }
}

/// A simpler alternative to the `NyxdScraper` that does not persist any received block information.
/// Instead, it only calls the registered modules on the processed data.
///
/// Furthermore, it also does not retrieve any validator information or detailed block information
pub struct NyxdWatcher {
    task_tracker: TaskTracker,
    cancel_token: CancellationToken,
}

impl NyxdWatcher {
    pub fn builder(config: WatcherConfig) -> NyxdWatcherBuilder {
        NyxdWatcherBuilder::new(config)
    }

    fn new() -> Self {
        Self {
            task_tracker: TaskTracker::new(),
            cancel_token: CancellationToken::new(),
        }
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

    pub async fn stop(self) {
        info!("stopping the chain watcher");
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
