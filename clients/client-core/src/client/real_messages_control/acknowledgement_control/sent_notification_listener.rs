// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{Action, ActionSender};
use super::SentPacketNotificationReceiver;
use futures::StreamExt;
use log::*;
use nymsphinx::chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID};
use task::ShutdownListener;

/// Module responsible for starting up retransmission timers.
/// It is required because when we send our packet to the `real traffic stream` controlled
/// by a poisson timer, there's no guarantee the message will be sent immediately, so we might
/// accidentally fire retransmission way quicker than we should have.
pub(super) struct SentNotificationListener {
    sent_notifier: SentPacketNotificationReceiver,
    action_sender: ActionSender,
    shutdown: ShutdownListener,
}

impl SentNotificationListener {
    pub(super) fn new(
        sent_notifier: SentPacketNotificationReceiver,
        action_sender: ActionSender,
        shutdown: ShutdownListener,
    ) -> Self {
        SentNotificationListener {
            sent_notifier,
            action_sender,
            shutdown,
        }
    }

    async fn on_sent_message(&mut self, frag_id: FragmentIdentifier) {
        if frag_id == COVER_FRAG_ID {
            trace!("sent off a cover message - no need to start retransmission timer!");
            return;
        } else if frag_id.is_reply() {
            debug!("sent off a reply message - no need to start retransmission timer!");
            // TODO: probably there will need to be some extra procedure here, like it would
            // be nice to know that our reply actually reached the recipient (i.e. we got the ack)
            return;
        }
        self.action_sender
            .unbounded_send(Action::new_start_timer(frag_id))
            .unwrap();
    }

    pub(super) async fn run(&mut self) {
        debug!("Started SentNotificationListener");
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                Some(frag_id) = self.sent_notifier.next() => {
                    self.on_sent_message(frag_id).await;
                },
                _ = self.shutdown.recv() => {
                    log::trace!("SentNotificationListener: Received shutdown");
                }
            }
        }
        log::debug!("SentNotificationListener: Exiting");
    }
}
