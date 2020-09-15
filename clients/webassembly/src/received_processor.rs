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
use nymsphinx::receiver::{MessageReceiver, MessageRecoveryError, ReconstructedMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
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
                .reply_SURB
                .map(|reply_surb| reply_surb.to_base58_string()),
        }
    }
}

pub(crate) struct ReceivedMessagesProcessor {
    message_receiver: MessageReceiver,
    local_encryption_keypair: Arc<encryption::KeyPair>,

    recently_reconstructed: HashSet<i32>,
}

// TODO: duplicate code from received_buffer.rs in client-core....
impl ReceivedMessagesProcessor {
    pub(crate) fn new(local_encryption_keypair: Arc<encryption::KeyPair>) -> Self {
        ReceivedMessagesProcessor {
            message_receiver: MessageReceiver::new(),
            local_encryption_keypair,
            recently_reconstructed: HashSet::new(),
        }
    }

    pub(crate) fn process_received_fragment(
        &mut self,
        raw_fragment: Vec<u8>,
    ) -> Option<ProcessedMessage> {
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
}
