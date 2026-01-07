// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use nym_statistics_common::clients::{packet_statistics::PacketStatisticsEvent, ClientStatsSender};

use crate::client::rtt_analyzer::{RttAnalyzer, RttEvent};
use futures::StreamExt;
use nym_gateway_client::AcknowledgementReceiver;
use nym_sphinx::{
    acknowledgements::{identifier::recover_identifier, AckKey},
    chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID},
};
use nym_task::ShutdownToken;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::*;

/// Module responsible for listening for any data resembling acknowledgements from the network
/// and firing actions to remove them from the 'Pending' state.
pub(crate) struct AcknowledgementListener {
    ack_key: Arc<AckKey>,
    ack_receiver: AcknowledgementReceiver,
    action_sender: AckActionSender,
    stats_tx: ClientStatsSender,
}

impl AcknowledgementListener {
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_receiver: AcknowledgementReceiver,
        action_sender: AckActionSender,
        stats_tx: ClientStatsSender,
    ) -> Self {
        AcknowledgementListener {
            ack_key,
            ack_receiver,
            action_sender,
            stats_tx,
        }
    }

    async fn on_ack(
        &mut self,
        ack_content: Vec<u8>,
        rtt_producer: Option<tokio::sync::mpsc::Sender<RttEvent>>,
    ) {
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

        if let Some(ref producer) = rtt_producer {
            if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
                let now = duration.as_millis();
                let _ = producer.try_send(RttEvent::FragmentAckReceived {
                    fragment_id: frag_id.set_id().to_string(),
                    timestamp: now,
                });
            }
        }

        trace!("Received {frag_id} from the mix network");
        self.stats_tx
            .report(PacketStatisticsEvent::RealAckReceived(ack_content.len()).into());
        let _ = self
            .action_sender
            .unbounded_send(Action::new_remove(frag_id));
    }

    async fn handle_ack_receiver_item(
        &mut self,
        item: Vec<Vec<u8>>,
        rtt_producer: Option<tokio::sync::mpsc::Sender<RttEvent>>,
    ) {
        // realistically we would only be getting one ack at the time
        for ack in item {
            self.on_ack(ack, rtt_producer.clone()).await;
        }
    }

    pub(crate) async fn run(&mut self, shutdown_token: ShutdownToken) {
        debug!("Started AcknowledgementListener with graceful shutdown support");
        let rtt_producer = RttAnalyzer::producer();

        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    tracing::trace!("AcknowledgementListener: Received shutdown");
                    break;
                }
                acks = self.ack_receiver.next() => match acks {
                    Some(acks) => self.handle_ack_receiver_item(acks,rtt_producer.clone()).await,
                    None => {
                        tracing::trace!("AcknowledgementListener: Stopping since channel closed");
                        break;
                    }
                },

            }
        }
        tracing::debug!("AcknowledgementListener: Exiting");
    }
}
