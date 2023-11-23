// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use super::SentPacketNotificationReceiver;
use futures::StreamExt;
use log::*;
use nym_sphinx::chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID};

/// Module responsible for starting up retransmission timers.
/// It is required because when we send our packet to the `real traffic stream` controlled
/// by a poisson timer, there's no guarantee the message will be sent immediately, so we might
/// accidentally fire retransmission way quicker than we should have.
pub(super) struct SentNotificationListener {
    sent_notifier: SentPacketNotificationReceiver,
    action_sender: AckActionSender,
}

impl SentNotificationListener {
    pub(super) fn new(
        sent_notifier: SentPacketNotificationReceiver,
        action_sender: AckActionSender,
    ) -> Self {
        SentNotificationListener {
            sent_notifier,
            action_sender,
        }
    }

    async fn on_sent_message(&mut self, frag_id: FragmentIdentifier) {
        if frag_id == COVER_FRAG_ID {
            trace!("sent off a cover message - no need to start retransmission timer!");
            return;
        }
        self.action_sender
            .unbounded_send(Action::new_start_timer(frag_id))
            .unwrap();
    }

    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        debug!("Started SentNotificationListener with graceful shutdown support");

        loop {
            tokio::select! {
                frag_id = self.sent_notifier.next() => match frag_id {
                    Some(frag_id) => {
                        self.on_sent_message(frag_id).await;
                    }
                    None => {
                        log::trace!("SentNotificationListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv_with_delay() => {
                    log::trace!("SentNotificationListener: Received shutdown");
                    break;
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("SentNotificationListener: Exiting");
    }
}
