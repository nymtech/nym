// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The nym-node subsystem that publishes this node's signed entries to the directory
//! contract. See the `node-directory-publishing` change for the full design.

use crate::node::nyx_client::NyxClient;
use nym_task::ShutdownToken;
use tracing::trace;

pub(crate) mod payload;

pub(crate) use payload::DirectoryPayload;

pub(crate) type DirectoryPublisherEventsSender = ();

pub(crate) struct DirectoryPublisher {
    nyx_client: NyxClient,
    shutdown_token: ShutdownToken,
}

impl DirectoryPublisher {
    pub(crate) fn events_sender(&self) -> DirectoryPublisherEventsSender {
        todo!()
    }
}

impl DirectoryPublisher {
    pub(crate) fn new(nyx_client: NyxClient, shutdown_token: ShutdownToken) -> Self {
        DirectoryPublisher {
            nyx_client,
            shutdown_token,
        }
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("DirectoryPublisher: Received shutdown");
                    break;
                }
            }
        }

        trace!("DirectoryPublisher: exiting")
    }
}
