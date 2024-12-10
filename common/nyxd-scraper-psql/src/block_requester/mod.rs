// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::BlockToProcess;
use crate::error::ScraperError;
use crate::rpc_client::RpcClient;
use futures::StreamExt;
use std::ops::Range;
use tokio::sync::mpsc::{Receiver, UnboundedSender};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};

#[derive(Debug)]
pub enum BlockRequest {
    Range(Range<u32>),

    // UNIMPLEMENTED:
    #[allow(dead_code)]
    Specific(Vec<u32>),
}

pub(crate) struct BlockRequester {
    cancel: CancellationToken,
    rpc_client: RpcClient,
    requests: ReceiverStream<BlockRequest>,
    blocks: UnboundedSender<BlockToProcess>,
}

impl BlockRequester {
    pub(crate) fn new(
        cancel: CancellationToken,
        rpc_client: RpcClient,
        requests: Receiver<BlockRequest>,
        blocks: UnboundedSender<BlockToProcess>,
    ) -> Self {
        BlockRequester {
            cancel,
            rpc_client,
            requests: requests.into(),
            blocks,
        }
    }

    async fn request_and_send(&self, height: u32) -> Result<(), ScraperError> {
        let block = self.rpc_client.get_basic_block_details(height).await?;
        self.blocks.send(block.into())?;
        Ok(())
    }

    async fn request_blocks<I: IntoIterator<Item = u32>>(&self, heights: I) {
        futures::stream::iter(heights)
            .for_each_concurrent(4, |height| async move {
                if let Err(err) = self.request_and_send(height).await {
                    error!("failed to request block data: {err}")
                }
            })
            .await
    }

    #[instrument(skip(self))]
    async fn handle_blocks_request(&self, request: BlockRequest) {
        info!("received request for missed blocks");

        match request {
            BlockRequest::Range(range) => self.request_blocks(range).await,
            BlockRequest::Specific(heights) => self.request_blocks(heights).await,
        }
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
               _ = self.cancel.cancelled() => {
                    info!("received cancellation token");
                    break
                }
                maybe_request = self.requests.next() => {
                    match maybe_request {
                        Some(request) => self.handle_blocks_request(request).await,
                        None => {
                            warn!("stopped receiving new requests");
                            self.cancel.cancel();
                            break
                        }
                    }
                }
            }
        }
    }
}
