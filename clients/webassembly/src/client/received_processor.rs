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

use crypto::asymmetric::encryption;
use futures::StreamExt;
use gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};
use nymsphinx::acknowledgements::identifier::recover_identifier;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::chunking::fragment::{FragmentIdentifier, COVER_FRAG_ID};
use nymsphinx::receiver::{MessageReceiver, MessageRecoveryError, ReconstructedMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_utils::{console_error, console_log, console_warn};

#[derive(Serialize, Deserialize)]
pub struct ProcessedMessage {
    pub message: String,
    pub reply_surb: Option<String>,
}

impl From<ReconstructedMessage> for ProcessedMessage {
    fn from(reconstructed: ReconstructedMessage) -> Self {
        ProcessedMessage {
            message: String::from_utf8_lossy(&reconstructed.message).into_owned(),
            reply_surb: reconstructed
                .reply_surb
                .map(|reply_surb| reply_surb.to_base58_string()),
        }
    }
}

pub(crate) struct ReceivedMessagesProcessor {
    local_encryption_keypair: Arc<encryption::KeyPair>,
    ack_key: Arc<AckKey>,
    message_receiver: MessageReceiver,

    recently_reconstructed: HashSet<i32>,
}

impl ReceivedMessagesProcessor {
    pub(crate) fn new(
        local_encryption_keypair: Arc<encryption::KeyPair>,
        ack_key: Arc<AckKey>,
    ) -> Self {
        ReceivedMessagesProcessor {
            local_encryption_keypair,
            ack_key,
            message_receiver: MessageReceiver::new(),
            recently_reconstructed: HashSet::new(),
        }
    }

    // TODO: duplicate code from received_buffer.rs in client-core....
    fn process_received_fragment(&mut self, raw_fragment: Vec<u8>) -> Option<ProcessedMessage> {
        let fragment_data = match self
            .message_receiver
            .recover_plaintext(self.local_encryption_keypair.private_key(), raw_fragment)
        {
            Err(e) => {
                console_warn!("failed to recover fragment data: {:?}. The whole underlying message might be corrupted and unrecoverable!", e);
                return None;
            }
            Ok(frag_data) => frag_data,
        };

        if nymsphinx::cover::is_cover(&fragment_data) {
            // currently won't be the case for a loooong time
            console_log!("The message was a loop cover message! Skipping it");
            return None;
        }

        let fragment = match self.message_receiver.recover_fragment(&fragment_data) {
            Err(e) => {
                console_warn!("failed to recover fragment from raw data: {:?}. The whole underlying message might be corrupted and unrecoverable!", e);
                return None;
            }
            Ok(frag) => frag,
        };

        if self.recently_reconstructed.contains(&fragment.id()) {
            console_warn!("Received a chunk of already re-assembled message ({:?})! It probably got here because the ack got lost", fragment.id());
            return None;
        }

        // if we returned an error the underlying message is malformed in some way
        match self.message_receiver.insert_new_fragment(fragment) {
            Err(err) => match err {
                MessageRecoveryError::MalformedReconstructedMessage(message_sets) => {
                    // TODO: should we really insert reconstructed sets? could this be abused for some attack?
                    for set_id in message_sets {
                        if !self.recently_reconstructed.insert(set_id) {
                            // or perhaps we should even panic at this point?
                            console_error!(
                                "Reconstructed another message containing already used set id!"
                            )
                        }
                    }
                    None
                }
                _ => unreachable!(
                    "no other error kind should have been returned here! If so, it's a bug!"
                ),
            },
            Ok(reconstruction_result) => match reconstruction_result {
                Some((reconstructed_message, used_sets)) => {
                    for set_id in used_sets {
                        if !self.recently_reconstructed.insert(set_id) {
                            // or perhaps we should even panic at this point?
                            console_error!(
                                "Reconstructed another message containing already used set id!"
                            )
                        }
                    }
                    Some(reconstructed_message.into())
                }
                None => None,
            },
        }
    }

    // TODO: duplicate code from acknowledgement listener...
    fn process_received_ack(&self, ack_content: Vec<u8>) {
        let frag_id = match recover_identifier(&self.ack_key, &ack_content)
            .map(FragmentIdentifier::try_from_bytes)
        {
            Some(Ok(frag_id)) => frag_id,
            _ => {
                console_warn!("Received invalid ACK!"); // should we do anything else about that?
                return;
            }
        };

        // if we received an ack for cover message or a reply there will be nothing to remove,
        // because nothing was inserted in the first place
        if frag_id == COVER_FRAG_ID {
            return;
        } else if frag_id.is_reply() {
            console_warn!("Received an ack for a reply message - no need to do anything! (don't know what to do!)");
            // TODO: probably there will need to be some extra procedure here, something to notify
            // user that his reply reached the recipient (since we got an ack)
            return;
        }

        console_log!(
            "Received an ack for fragment {:?} but can't do anything more about it just yet...",
            frag_id
        );

        // here be ack handling...
    }

    // TODO: this needs to have a shutdown signal!
    pub(crate) async fn start_processing(
        mut self,
        mixnet_messages_receiver: MixnetMessageReceiver,
        ack_receiver: AcknowledgementReceiver,
        on_message: js_sys::Function,
    ) {
        let mut fused_mixnet_messages_receiver = mixnet_messages_receiver.fuse();
        let mut fused_ack_receiver = ack_receiver.fuse();
        let this = JsValue::null();

        loop {
            futures::select! {
                mix_msgs = fused_mixnet_messages_receiver.next() => {
                    for mix_msg in mix_msgs.unwrap() {
                        if let Some(processed) = self.process_received_fragment(mix_msg) {
                            let arg1 = JsValue::from_serde(&processed).unwrap();
                            on_message.call1(&this, &arg1).expect("on message failed!");
                        }
                    }
                }
                acks = fused_ack_receiver.next() => {
                    for ack in acks.unwrap() {
                        self.process_received_ack(ack);
                    }
                }
            }
        }
    }
}
