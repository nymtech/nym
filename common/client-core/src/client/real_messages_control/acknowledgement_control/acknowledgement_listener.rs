// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::packet_statistics_control::{PacketStatisticsEvent, PacketStatisticsReporter};

use super::action_controller::{AckActionSender, Action};
use futures::StreamExt;
use log::*;
use nym_gateway_client::AcknowledgementReceiver;
use nym_sphinx::{
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
    stats_tx: PacketStatisticsReporter,
}

impl AcknowledgementListener {
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_receiver: AcknowledgementReceiver,
        action_sender: AckActionSender,
        stats_tx: PacketStatisticsReporter,
    ) -> Self {
        AcknowledgementListener {
            ack_key,
            ack_receiver,
            action_sender,
            stats_tx,
        }
    }

    async fn on_ack(&mut self, ack_content: Vec<u8>) {
        trace!("Received an ack");
        self.stats_tx.report(PacketStatisticsEvent::AckReceived);

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
        }

        trace!("Received {} from the mix network", frag_id);
        self.stats_tx.report(PacketStatisticsEvent::RealAckReceived);
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

    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
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
                _ = shutdown.recv_with_delay() => {
                    log::trace!("AcknowledgementListener: Received shutdown");
                }
            }
        }
        shutdown.recv_timeout().await;
        log::debug!("AcknowledgementListener: Exiting");
    }
}
