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

use super::action_controller::{Action, ActionSender};
use futures::StreamExt;
use gateway_client::AcknowledgementReceiver;
use log::*;
use nymsphinx::{
    acknowledgements::{identifier::recover_identifier, AckKey},
    chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID},
};
use std::sync::Arc;

/// Module responsible for listening for any data resembling acknowledgements from the network
/// and firing actions to remove them from the 'Pending' state.
pub(super) struct AcknowledgementListener {
    ack_key: Arc<AckKey>,
    ack_receiver: AcknowledgementReceiver,
    action_sender: ActionSender,
}

impl AcknowledgementListener {
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_receiver: AcknowledgementReceiver,
        action_sender: ActionSender,
    ) -> Self {
        AcknowledgementListener {
            ack_key,
            ack_receiver,
            action_sender,
        }
    }

    async fn on_ack(&mut self, ack_content: Vec<u8>) {
        debug!("Received an ack");
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
            return;
        } else if frag_id.is_reply() {
            info!("Received an ack for a reply message - no need to do anything! (don't know what to do!)");
            // TODO: probably there will need to be some extra procedure here, something to notify
            // user that his reply reached the recipient (since we got an ack)
            return;
        }

        trace!("Received {} from the mix network", frag_id);

        self.action_sender
            .unbounded_send(Action::new_remove(frag_id))
            .unwrap();
    }

    pub(super) async fn run(&mut self) {
        debug!("Started AcknowledgementListener");
        while let Some(acks) = self.ack_receiver.next().await {
            // realistically we would only be getting one ack at the time
            for ack in acks {
                self.on_ack(ack).await;
            }
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}
