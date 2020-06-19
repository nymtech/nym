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

use super::PendingAcksMap;
use futures::StreamExt;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::{
    acknowledgements::{identifier::recover_identifier, AckAes128Key},
    chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID},
};
use std::sync::Arc;

// responsible for cancelling retransmission timers and removed entries from the map
pub(super) struct AcknowledgementListener {
    ack_key: Arc<AckAes128Key>,
    ack_receiver: AcknowledgementReceiver,
    pending_acks: PendingAcksMap,
}

impl AcknowledgementListener {
    pub(super) fn new(
        ack_key: Arc<AckAes128Key>,
        ack_receiver: AcknowledgementReceiver,
        pending_acks: PendingAcksMap,
    ) -> Self {
        AcknowledgementListener {
            ack_key,
            ack_receiver,
            pending_acks,
        }
    }

    async fn on_ack(&mut self, ack_content: Vec<u8>) {
        debug!("Received an ack");
        let frag_id = match recover_identifier(&self.ack_key, &ack_content) {
            None => {
                warn!("Received invalid ACK!"); // should we do anything else about that?
                return;
            }
            Some(frag_id_bytes) => match FragmentIdentifier::try_from_bytes(&frag_id_bytes) {
                Ok(frag_id) => frag_id,
                Err(err) => {
                    warn!("Received invalid ACK! - {:?}", err); // should we do anything else about that?
                    return;
                }
            },
        };

        if frag_id == COVER_FRAG_ID {
            trace!("Received an ack for a cover message - no need to do anything");
            return;
        }

        if let Some(pending_ack) = self.pending_acks.write().await.remove(&frag_id) {
            // cancel the retransmission future
            pending_ack.retransmission_cancel.notify();
        } else {
            warn!("received ACK for packet we haven't stored! - {:?}", frag_id);
        }
    }

    pub(super) async fn run(&mut self) {
        debug!("Started AcknowledgementListener");
        while let Some(acks) = self.ack_receiver.next().await {
            // realistically we would only be getting one ack at the time, but if we managed to
            // introduce batching in gateway client, this call should be improved to not re-acquire
            // write permit on the map every loop iteration
            for ack in acks {
                self.on_ack(ack).await;
            }
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}
