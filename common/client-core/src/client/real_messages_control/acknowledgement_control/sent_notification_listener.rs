// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use super::SentPacketNotificationReceiver;
use futures::StreamExt;
use nym_sphinx::chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID};
use nym_task::ShutdownToken;
use tracing::*;

/// Module responsible for starting up retransmission timers.
/// It is required because when we send our packet to the `real traffic stream` controlled
/// by a poisson timer, there's no guarantee the message will be sent immediately, so we might
/// accidentally fire retransmission way quicker than we should have.
pub(super) struct SentNotificationListener {
    sent_notifier: SentPacketNotificationReceiver,
    action_sender: AckActionSender,
    shutdown_token: ShutdownToken,
}

impl SentNotificationListener {
    pub(super) fn new(
        sent_notifier: SentPacketNotificationReceiver,
        action_sender: AckActionSender,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SentNotificationListener {
            sent_notifier,
            action_sender,
            shutdown_token,
        }
    }

    async fn on_sent_message(&mut self, frag_id: FragmentIdentifier) {
        if frag_id == COVER_FRAG_ID {
            trace!("sent off a cover message - no need to start retransmission timer!");
            return;
        }
        if let Err(err) = self
            .action_sender
            .unbounded_send(Action::new_start_timer(frag_id))
        {
            if !self.shutdown_token.is_cancelled() {
                error!("Failed to send start timer action to action controller: {err}");
            }
        }
    }

    pub(super) async fn run(&mut self) {
        debug!("Started SentNotificationListener with graceful shutdown support");

        while !self.shutdown_token.is_cancelled() {
            tokio::select! {
                frag_id = self.sent_notifier.next() => match frag_id {
                    Some(frag_id) => {
                        self.on_sent_message(frag_id).await;
                    }
                    None => {
                        tracing::trace!("SentNotificationListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = self.shutdown_token.cancelled() => {
                    tracing::trace!("SentNotificationListener: Received shutdown");
                    break;
                }
            }
        }
        assert!(self.shutdown_token.is_cancelled());
        tracing::debug!("SentNotificationListener: Exiting");
    }
}
