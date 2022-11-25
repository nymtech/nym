// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{AckActionSender, Action};
use super::SentPacketNotificationReceiver;
use futures::StreamExt;
use log::*;
use nymsphinx::chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID};

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
        } else if frag_id.is_reply() {
            error!("please let @jstuczyn know if you see this message");
            debug!("sent off a reply message - no need to start retransmission timer!");
            // TODO: probably there will need to be some extra procedure here, like it would
            // be nice to know that our reply actually reached the recipient (i.e. we got the ack)
            return;
        }
        self.action_sender
            .unbounded_send(Action::new_start_timer(frag_id))
            .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started SentNotificationListener with graceful shutdown support");

        while !shutdown.is_shutdown() {
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
                _ = shutdown.recv() => {
                    log::trace!("SentNotificationListener: Received shutdown");
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("SentNotificationListener: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn run(&mut self) {
        debug!("Started SentNotificationListener without graceful shutdown support");

        while let Some(frag_id) = self.sent_notifier.next().await {
            self.on_sent_message(frag_id).await;
        }
    }
}
