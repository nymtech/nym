// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::BlockToProcess;
use crate::error::ScraperError;
use tendermint_rpc::event::Event;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient, WebSocketClientDriver};
use time::{Duration, OffsetDateTime};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{error, info, warn};
use url::Url;

const MAX_FAILURES: usize = 10;
const MAX_RECONNECTION_ATTEMPTS: usize = 8;
const SOCKET_FAILURE_RESET: Duration = Duration::minutes(15);

pub struct ChainSubscriber {
    cancel: CancellationToken,
    task_tracker: TaskTracker,

    block_sender: UnboundedSender<BlockToProcess>,

    websocket_endpoint: Url,
    websocket_client: WebSocketClient,
    websocket_driver: Option<WebSocketClientDriver>,
}

impl ChainSubscriber {
    pub async fn new(
        websocket_endpoint: &Url,
        cancel: CancellationToken,
        task_tracker: TaskTracker,
        block_sender: UnboundedSender<BlockToProcess>,
    ) -> Result<Self, ScraperError> {
        // sure, we could have just used websocket client entirely, but let's keep the logic for
        // getting current blocks and historical blocks completely separate with the dual connection
        let (client, driver) = WebSocketClient::new(websocket_endpoint.as_str())
            .await
            .map_err(|source| ScraperError::WebSocketConnectionFailure {
                url: websocket_endpoint.to_string(),
                source,
            })?;

        Ok(ChainSubscriber {
            cancel,
            task_tracker,
            block_sender,
            websocket_endpoint: websocket_endpoint.clone(),
            websocket_client: client,
            websocket_driver: Some(driver),
        })
    }

    fn handle_new_event(&mut self, event: Event) -> Result<(), ScraperError> {
        if let Err(err) = self.block_sender.send(event.try_into()?) {
            // this error has nothing to do with the websocket or chain
            error!("failed to send block for processing: {err} - are we shutting down?")
        }
        Ok(())
    }

    async fn remake_connection(&mut self) -> Result<(), ScraperError> {
        info!(
            "attempting to reestablish connection to {}",
            self.websocket_endpoint
        );

        let (client, driver) = WebSocketClient::new(self.websocket_endpoint.as_str())
            .await
            .map_err(|source| ScraperError::WebSocketConnectionFailure {
                url: self.websocket_endpoint.to_string(),
                source,
            })?;
        self.websocket_client = client;
        self.websocket_driver = Some(driver);

        info!(
            "managed to reestablish the websocket connection to {}",
            self.websocket_endpoint
        );
        Ok(())
    }

    /// Returns whether the method exited due to the cancellation
    async fn run_chain_subscription(&mut self) -> Result<bool, ScraperError> {
        let Some(ws_driver) = self.websocket_driver.take() else {
            error!("the websocket driver hasn't been created - we probably failed to establish the connection");
            return Ok(false);
        };

        let driver_cancel = CancellationToken::new();
        let _driver_guard = driver_cancel.clone().drop_guard();

        // spawn the websocket driver task
        let driver_handle = {
            self.task_tracker.reopen();
            let handle = self
                .task_tracker
                .spawn(run_websocket_driver(ws_driver, driver_cancel));
            self.task_tracker.close();
            handle
        };
        tokio::pin!(driver_handle);

        info!("creating chain subscription");
        let mut subs = self
            .websocket_client
            .subscribe(EventType::NewBlock.into())
            .await
            .map_err(|source| ScraperError::ChainSubscriptionFailure { source })?;

        let mut failures = 0;

        info!("starting processing loop");
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    info!("received cancellation token");
                    // note: `_driver_guard` will get dropped here thus causing cancellation of the driver task
                    return Ok(true)
                }
                _ = &mut driver_handle => {
                    error!("our websocket driver has finished execution");
                    return Ok(self.cancel.is_cancelled())
                }
                maybe_event = subs.next() => {
                    let Some(maybe_event) = maybe_event else {
                        warn!("stopped receiving new events");
                        return Ok(false)
                    };
                    match maybe_event {
                        Ok(event) => {
                            if let Err(err) = self.handle_new_event(event) {
                                error!("failed to process received block: {err}");
                                failures += 1
                            } else {
                                failures = 0;
                            }
                        }
                        Err(err) => {
                            error!("failed to receive a valid subscription event: {err}");
                            failures += 1
                        }
                    }
                    if failures >= MAX_FAILURES {
                        return Ok(false)
                    }
                }
            }
        }
    }

    async fn websocket_backoff(&mut self, failure_count: usize) -> bool {
        const MINIMUM_WAIT_MS: u64 = 10_000;
        const INCREMENTAL_WAIT_MS: u64 = 30_000;

        let backoff_duration_ms = MINIMUM_WAIT_MS + INCREMENTAL_WAIT_MS * failure_count as u64;
        info!("going to wait {backoff_duration_ms} ms before re-attempting the reconnection");

        tokio::select! {
            _ = self.cancel.cancelled() => {
                info!("received cancellation token");
                true
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(backoff_duration_ms)) => false,
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), ScraperError> {
        let _drop_guard = self.cancel.clone().drop_guard();
        let mut socket_failures = 0;
        let mut last_failure = OffsetDateTime::now_utc();

        loop {
            if self.cancel.is_cancelled() {
                return Ok(());
            }

            match self.run_chain_subscription().await {
                Ok(cancelled) => {
                    if cancelled {
                        // we're in the middle of a shutdown
                        return Ok(());
                    }
                    socket_failures += 1;
                }
                Err(err) => {
                    error!("failed to create chain subscription: {err}");
                    socket_failures += 1;
                }
            }

            warn!("current socket failure count: {socket_failures}. the last failure was at {last_failure}");

            let now = OffsetDateTime::now_utc();

            // if it's been a while since the last failure, reset the count
            if now - last_failure > SOCKET_FAILURE_RESET {
                warn!("resetting the failure count to 1");
                socket_failures = 1;
            }
            last_failure = now;

            if socket_failures >= MAX_RECONNECTION_ATTEMPTS {
                error!("reached the maximum allowed failure count");
                return Err(ScraperError::MaximumWebSocketFailures);
            }

            // BACKOFF
            let cancelled = self.websocket_backoff(socket_failures).await;
            if cancelled {
                return Ok(());
            }

            if let Err(err) = self.remake_connection().await {
                error!("failed to re-establish the websocket connection: {err}");
            }
        }
    }
}

pub async fn run_websocket_driver(driver: WebSocketClientDriver, driver_cancel: CancellationToken) {
    info!("starting websocket driver");
    tokio::select! {
        _ = driver_cancel.cancelled() => {
            info!("received cancellation token")
        }
        res = driver.run() => {
            match res {
                Ok(_) => info!("our websocket driver has finished execution"),
                Err(err) => {
                    error!("our websocket driver has errored out: {err}");
                }
            }
        }
    }
}
