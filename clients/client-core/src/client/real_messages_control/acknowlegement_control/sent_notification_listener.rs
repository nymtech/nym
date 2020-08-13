// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{PendingAcksMap, RetransmissionRequestSender, SentPacketNotificationReceiver};
use futures::StreamExt;
use log::*;
use nymsphinx::chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID};
use std::sync::Arc;
use std::time::Duration;

// responsible for starting and controlling retransmission timers
// it is required because when we send our packet to the `real traffic stream` controlled
// with poisson timer, there's no guarantee the message will be sent immediately, so we might
// accidentally fire retransmission way quicker than we would have wanted.
pub(super) struct SentNotificationListener {
    ack_wait_multiplier: f64,
    ack_wait_addition: Duration,
    sent_notifier: SentPacketNotificationReceiver,
    pending_acks: PendingAcksMap,
    retransmission_sender: RetransmissionRequestSender,
}

impl SentNotificationListener {
    pub(super) fn new(
        ack_wait_multiplier: f64,
        ack_wait_addition: Duration,
        sent_notifier: SentPacketNotificationReceiver,
        pending_acks: PendingAcksMap,
        retransmission_sender: RetransmissionRequestSender,
    ) -> Self {
        SentNotificationListener {
            ack_wait_multiplier,
            ack_wait_addition,
            sent_notifier,
            pending_acks,
            retransmission_sender,
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

        let pending_acks_map_read_guard = self.pending_acks.read().await;
        // if the unwrap failed here, we have some weird bug somewhere
        // although when I think about it, it *theoretically* could happen under extremely heavy client
        // load that `on_sent_message()` is not called (and we do not receive the read permit)
        // until we already received and processed an ack for the packet
        // but this seems extremely unrealistic, but perhaps we should guard against that?
        let pending_ack_data = pending_acks_map_read_guard
            .get(&frag_id)
            .expect("on_sent_message: somehow we already received an ack for this packet?");

        // if this assertion ever fails, we have some bug due to some unintended leak.
        // the only reason I see it could happen if the `tokio::select` in the spawned
        // task below somehow did not drop it
        debug_assert_eq!(
            Arc::strong_count(&pending_ack_data.retransmission_cancel),
            1
        );

        // TODO: read more about Arc::downgrade. it could be useful here
        let retransmission_cancel = Arc::clone(&pending_ack_data.retransmission_cancel);

        let retransmission_timeout = tokio::time::delay_for(
            (pending_ack_data.delay.clone() * self.ack_wait_multiplier).to_duration()
                + self.ack_wait_addition,
        );

        let retransmission_sender = self.retransmission_sender.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = retransmission_cancel.notified() => {
                    trace!("received ack for the fragment. Cancelling retransmission future");
                }
                _ = retransmission_timeout => {
                    trace!("did not receive an ack - will retransmit the packet");
                    retransmission_sender.unbounded_send(frag_id).unwrap();
                }
            }
        });
    }

    pub(super) async fn run(&mut self) {
        debug!("Started SentNotificationListener");
        while let Some(frag_id) = self.sent_notifier.next().await {
            self.on_sent_message(frag_id).await;
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}
