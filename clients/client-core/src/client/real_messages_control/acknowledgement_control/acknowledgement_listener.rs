// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use futures::StreamExt;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::{
    acknowledgements::{identifier::recover_identifier, AckKey},
    chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID},
};
use std::sync::Arc;

/// Module responsible for listening for any data resembling acknowledgements from the network
/// and firing actions to remove them from the 'Pending' state.
pub(super) struct AcknowledgementListener {
    ack_key: Arc<AckKey>,
    ack_receiver: AcknowledgementReceiver,
    action_sender: AckActionSender,
}

impl AcknowledgementListener {
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_receiver: AcknowledgementReceiver,
        action_sender: AckActionSender,
    ) -> Self {
        AcknowledgementListener {
            ack_key,
            ack_receiver,
            action_sender,
        }
    }

    async fn on_ack(&mut self, ack_content: Vec<u8>) {
        trace!("Received an ack");
        let frag_id = match recover_identifier(&self.ack_key, &ack_content)
            .map(FragmentIdentifier::try_from_bytes)
        {
            Some(Ok(frag_id)) => frag_id,
            _ => {
                warn!("Received invalid ACK!"); // should we do anything else about that?
                return;
            }
        };

        // if we received an ack for cover message or a reply there will be nothing to remove,
        // because nothing was inserted in the first place
        if frag_id == COVER_FRAG_ID {
            trace!("Received an ack for a cover message - no need to do anything");
            return;
        } else if frag_id.is_reply() {
            error!("please let @jstuczyn know if you see this message");
            info!("Received an ack for a reply message - no need to do anything! (don't know what to do!)");
            // TODO: probably there will need to be some extra procedure here, something to notify
            // user that his reply reached the recipient (since we got an ack)
            return;
        }

        trace!("Received {} from the mix network", frag_id);

        self.action_sender
            .unbounded_send(Action::new_remove(frag_id))
            .unwrap();
    }

    async fn handle_ack_receiver_item(&mut self, item: Vec<Vec<u8>>) {
        // realistically we would only be getting one ack at the time
        for ack in item {
            self.on_ack(ack).await;
        }
    }

    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started AcknowledgementListener with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                acks = self.ack_receiver.next() => match acks {
                    Some(acks) => self.handle_ack_receiver_item(acks).await,
                    None => {
                        log::trace!("AcknowledgementListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv() => {
                    log::trace!("AcknowledgementListener: Received shutdown");
                }
            }
        }
        shutdown.recv_timeout().await;
        log::debug!("AcknowledgementListener: Exiting");
    }

    // todo: think whether this is still required
    #[allow(dead_code)]
    pub(super) async fn run(&mut self) {
        debug!("Started AcknowledgementListener without graceful shutdown support");

        while let Some(acks) = self.ack_receiver.next().await {
            self.handle_ack_receiver_item(acks).await
        }
    }
}
