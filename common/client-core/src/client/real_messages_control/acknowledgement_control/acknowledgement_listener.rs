// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use nym_statistics_common::clients::{packet_statistics::PacketStatisticsEvent, ClientStatsSender};

use futures::StreamExt;
use nym_gateway_client::AcknowledgementReceiver;
use nym_sphinx::{
    acknowledgements::{identifier::recover_identifier, AckKey},
    chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID},
};
use nym_task::TaskClient;
use std::sync::Arc;
use tracing::*;

/// Module responsible for listening for any data resembling acknowledgements from the network
/// and firing actions to remove them from the 'Pending' state.
pub(super) struct AcknowledgementListener {
    ack_key: Arc<AckKey>,
    ack_receiver: AcknowledgementReceiver,
    action_sender: AckActionSender,
    stats_tx: ClientStatsSender,
    task_client: TaskClient,
}

impl AcknowledgementListener {
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_receiver: AcknowledgementReceiver,
        action_sender: AckActionSender,
        stats_tx: ClientStatsSender,
        task_client: TaskClient,
    ) -> Self {
        AcknowledgementListener {
            ack_key,
            ack_receiver,
            action_sender,
            stats_tx,
            task_client,
        }
    }

    async fn on_ack(&mut self, ack_content: Vec<u8>) {
        trace!("Received an ack");
        self.stats_tx
            .report(PacketStatisticsEvent::AckReceived(ack_content.len()).into());

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
            self.stats_tx
                .report(PacketStatisticsEvent::CoverAckReceived(ack_content.len()).into());
            return;
        }

        trace!("Received {frag_id} from the mix network");
        self.stats_tx
            .report(PacketStatisticsEvent::RealAckReceived(ack_content.len()).into());
        if let Err(err) = self
            .action_sender
            .unbounded_send(Action::new_remove(frag_id))
        {
            if !self.task_client.is_shutdown_poll() {
                error!("Failed to send remove action to action controller: {err}");
            }
        }
    }

    async fn handle_ack_receiver_item(&mut self, item: Vec<Vec<u8>>) {
        // realistically we would only be getting one ack at the time
        for ack in item {
            self.on_ack(ack).await;
        }
    }

    pub(super) async fn run(&mut self) {
        debug!("Started AcknowledgementListener with graceful shutdown support");

        while !self.task_client.is_shutdown() {
            tokio::select! {
                acks = self.ack_receiver.next() => match acks {
                    Some(acks) => self.handle_ack_receiver_item(acks).await,
                    None => {
                        tracing::trace!("AcknowledgementListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = self.task_client.recv() => {
                    tracing::trace!("AcknowledgementListener: Received shutdown");
                }
            }
        }
        self.task_client.recv_timeout().await;
        tracing::debug!("AcknowledgementListener: Exiting");
    }
}
