// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::SentPacketNotificationReceiver;
use super::action_controller::{AckActionSender, Action};
use futures::StreamExt;
use nym_sphinx::chunking::fragment::{COVER_FRAG_ID, FragmentIdentifier};
use tracing::*;

/// Module responsible for starting up retransmission timers.
/// It is required because when we send our packet to the `real traffic stream` controlled
/// by a poisson timer, there's no guarantee the message will be sent immediately, so we might
/// accidentally fire retransmission way quicker than we should have.
pub(crate) struct SentNotificationListener {
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
        let _ = self
            .action_sender
            .unbounded_send(Action::new_start_timer(frag_id));
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started SentNotificationListener with graceful shutdown support");

        while let Some(frag_id) = self.sent_notifier.next().await {
            self.on_sent_message(frag_id).await;
        }

        tracing::debug!("SentNotificationListener: Exiting");
    }
}
