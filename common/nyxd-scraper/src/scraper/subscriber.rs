// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::BlockToProcess;
use crate::error::ScraperError;
use tendermint_rpc::event::Event;
use tendermint_rpc::query::EventType;
use tendermint_rpc::{SubscriptionClient, WebSocketClient, WebSocketClientDriver};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use url::Url;

const MAX_FAILURES: usize = 10;

pub struct ChainSubscriber {
    cancel: CancellationToken,
    block_sender: UnboundedSender<BlockToProcess>,

    websocket_client: WebSocketClient,
    websocket_driver: Option<WebSocketClientDriver>,
}

impl ChainSubscriber {
    pub async fn new(
        websocket_endpoint: &Url,
        cancel: CancellationToken,
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
            block_sender,
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

    pub(crate) async fn run(&mut self) -> Result<(), ScraperError> {
        let _drop_guard = self.cancel.clone().drop_guard();

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
                    break
                }
                maybe_event = subs.next() => {
                    let Some(maybe_event) = maybe_event else {
                        warn!("stopped receiving new events");
                        break;
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
                        // note: the drop_guard will get dropped and thus cause a shutdown
                        return Err(ScraperError::MaximumSubscriptionFailures);
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn ws_driver(&mut self) -> WebSocketClientDriver {
        #[allow(clippy::expect_used)]
        self.websocket_driver
            .take()
            .expect("websocket driver has already been started!")
    }
}

pub async fn run_websocket_driver(driver: WebSocketClientDriver, cancel: CancellationToken) {
    info!("starting websocket driver");
    tokio::select! {
        _ = cancel.cancelled() => {
            info!("received cancellation token")
        }
        res = driver.run() => {
            match res {
                Ok(_) => info!("our websocket driver has finished execution"),
                Err(err) => {
                    // TODO: in the future just attempt to reconnect
                    error!("our websocket driver has errored out: {err}");
                }
            }
            cancel.cancel()
        }
    }
}
