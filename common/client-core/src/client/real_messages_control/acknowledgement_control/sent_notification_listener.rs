// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use super::SentPacketNotificationReceiver;
use futures::StreamExt;
use nym_sphinx::chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID};
use nym_task::TaskClient;
use tracing::*;

/// Module responsible for starting up retransmission timers.
/// It is required because when we send our packet to the `real traffic stream` controlled
/// by a poisson timer, there's no guarantee the message will be sent immediately, so we might
/// accidentally fire retransmission way quicker than we should have.
pub(super) struct SentNotificationListener {
    sent_notifier: SentPacketNotificationReceiver,
    action_sender: AckActionSender,
    task_client: TaskClient,
}

impl SentNotificationListener {
    pub(super) fn new(
        sent_notifier: SentPacketNotificationReceiver,
        action_sender: AckActionSender,
        task_client: TaskClient,
    ) -> Self {
        SentNotificationListener {
            sent_notifier,
            action_sender,
            task_client,
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
            if !self.task_client.is_shutdown_poll() {
                error!("Failed to send start timer action to action controller: {err}");
            }
        }
    }

    pub(super) async fn run(&mut self) {
        debug!("Started SentNotificationListener with graceful shutdown support");

        while !self.task_client.is_shutdown() {
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
                _ = self.task_client.recv() => {
                    tracing::trace!("SentNotificationListener: Received shutdown");
                    break;
                }
            }
        }
        assert!(self.task_client.is_shutdown_poll());
        tracing::debug!("SentNotificationListener: Exiting");
    }
}
