// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The nym-node subsystem that publishes this node's signed entries to the directory
//! contract. See the `node-directory-publishing` change for the full design.

use crate::node::nyx_client::NyxClient;
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownToken;
use std::sync::Arc;
use tracing::trace;

pub(crate) mod payload;

use crate::error::NymNodeError;
pub(crate) use payload::DirectoryPayload;

pub(crate) type DirectoryPublisherEventsSender = ();

pub(crate) struct DirectoryPublisher {
    nyx_client: NyxClient,
    ed25519_identity_keys: Arc<ed25519::KeyPair>,
    shutdown_token: ShutdownToken,
}

impl DirectoryPublisher {
    pub(crate) fn events_sender(&self) -> DirectoryPublisherEventsSender {
        todo!()
    }
}

impl DirectoryPublisher {
    pub(crate) async fn new(
        nyx_client: NyxClient,
        ed25519_identity_keys: Arc<ed25519::KeyPair>,
        shutdown_token: ShutdownToken,
    ) -> Result<Self, NymNodeError> {
        // blow up at this point if the directory contract address is not set
        if nyx_client
            .get_nym_contracts()
            .await
            .directory_contract_address
            .is_none()
        {
            return Err(NymNodeError::MissingDirectoryContractAddress);
        }

        Ok(DirectoryPublisher {
            nyx_client,
            ed25519_identity_keys,
            shutdown_token,
        })
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
