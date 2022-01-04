// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::reply_key_storage::ReplyKeyStorage;
use crypto::asymmetric::encryption;
use crypto::symmetric::stream_cipher;
use crypto::Digest;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
use gateway_client::MixnetMessageReceiver;
use log::*;
use nymsphinx::anonymous_replies::{encryption_key::EncryptionKeyDigest, SurbEncryptionKey};
use nymsphinx::params::{ReplySurbEncryptionAlgorithm, ReplySurbKeyDigestAlgorithm};
use nymsphinx::receiver::{MessageReceiver, MessageRecoveryError, ReconstructedMessage};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::task::JoinHandle;

// Buffer Requests to say "hey, send any reconstructed messages to this channel"
// or to say "hey, I'm going offline, don't send anything more to me. Just buffer them instead"
pub type ReceivedBufferRequestSender = mpsc::UnboundedSender<ReceivedBufferMessage>;
pub type ReceivedBufferRequestReceiver = mpsc::UnboundedReceiver<ReceivedBufferMessage>;

// The channel set for the above
pub type ReconstructedMessagesSender = mpsc::UnboundedSender<Vec<ReconstructedMessage>>;
pub type ReconstructedMessagesReceiver = mpsc::UnboundedReceiver<Vec<ReconstructedMessage>>;

struct ReceivedMessagesBufferInner {
    messages: Vec<ReconstructedMessage>,
    local_encryption_keypair: Arc<encryption::KeyPair>,

    // TODO: looking how it 'looks' here, perhaps `MessageReceiver` should be renamed to something
    // else instead.
    message_receiver: MessageReceiver,
    message_sender: Option<ReconstructedMessagesSender>,

    // TODO: this will get cleared upon re-running the client
    // but perhaps it should be changed to include timestamps of when the message was reconstructed
    // and every now and then remove ids older than X
    recently_reconstructed: HashSet<i32>,
}

impl ReceivedMessagesBufferInner {
    fn process_received_fragment(&mut self, raw_fragment: Vec<u8>) -> Option<ReconstructedMessage> {
        let fragment_data = match self
            .message_receiver
            .recover_plaintext(self.local_encryption_keypair.private_key(), raw_fragment)
        {
            Err(e) => {
                warn!("failed to recover fragment data: {:?}. The whole underlying message might be corrupted and unrecoverable!", e);
                return None;
            }
            Ok(frag_data) => frag_data,
        };

        if nymsphinx::cover::is_cover(&fragment_data) {
            trace!("The message was a loop cover message! Skipping it");
            return None;
        }

        let fragment = match self.message_receiver.recover_fragment(&fragment_data) {
            Err(e) => {
                warn!("failed to recover fragment from raw data: {:?}. The whole underlying message might be corrupted and unrecoverable!", e);
                return None;
            }
            Ok(frag) => frag,
        };

        if self.recently_reconstructed.contains(&fragment.id()) {
            debug!("Received a chunk of already re-assembled message ({:?})! It probably got here because the ack got lost", fragment.id());
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
                            error!("Reconstructed another message containing already used set id!")
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
                            error!("Reconstructed another message containing already used set id!")
                        }
                    }
                    Some(reconstructed_message)
                }
                None => None,
            },
        }
    }
}

#[derive(Debug, Clone)]
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
struct ReceivedMessagesBuffer {
    inner: Arc<Mutex<ReceivedMessagesBufferInner>>,

    /// Storage containing keys to all [`ReplySURB`]s ever sent out that we did not receive back.
    // There's no need to put it behind a Mutex since it's already properly concurrent
    reply_key_storage: ReplyKeyStorage,
}

impl ReceivedMessagesBuffer {
    fn new(
        local_encryption_keypair: Arc<encryption::KeyPair>,
        reply_key_storage: ReplyKeyStorage,
    ) -> Self {
        ReceivedMessagesBuffer {
            inner: Arc::new(Mutex::new(ReceivedMessagesBufferInner {
                messages: Vec::new(),
                local_encryption_keypair,
                message_receiver: MessageReceiver::new(),
                message_sender: None,
                recently_reconstructed: HashSet::new(),
            })),
            reply_key_storage,
        }
    }

    async fn disconnect_sender(&mut self) {
        let mut guard = self.inner.lock().await;
        if guard.message_sender.is_none() {
            // in theory we could just ignore it, but that situation should have never happened
            // in the first place, so this way we at least know we have an important bug to fix
            panic!("trying to disconnect non-existent sender!")
        }
        guard.message_sender = None;
    }

    async fn connect_sender(&mut self, sender: ReconstructedMessagesSender) {
        let mut guard = self.inner.lock().await;
        if guard.message_sender.is_some() {
            // in theory we could just ignore it, but that situation should have never happened
            // in the first place, so this way we at least know we have an important bug to fix
            panic!("trying overwrite an existing sender!")
        }

        // while we're at it, also empty the buffer if we happened to receive anything while
        // no sender was connected
        let stored_messages = std::mem::take(&mut guard.messages);
        if !stored_messages.is_empty() {
            if let Err(err) = sender.unbounded_send(stored_messages) {
                error!(
                    "The sender channel we just received is already invalidated - {:?}",
                    err
                );
                // put the values back to the buffer
                // the returned error has two fields: err: SendError and val: T,
                // where val is the value that was failed to get sent;
                // it's returned by the `into_inner` call
                guard.messages = err.into_inner();
                return;
            }
        }
        guard.message_sender = Some(sender);
    }

    async fn add_reconstructed_messages(&mut self, msgs: Vec<ReconstructedMessage>) {
        debug!("Adding {:?} new messages to the buffer!", msgs.len());
        trace!("Adding new messages to the buffer! {:?}", msgs);
        self.inner.lock().await.messages.extend(msgs)
    }

    fn process_received_reply(
        reply_ciphertext: &[u8],
        reply_key: SurbEncryptionKey,
    ) -> Option<ReconstructedMessage> {
        let zero_iv = stream_cipher::zero_iv::<ReplySurbEncryptionAlgorithm>();

        let mut reply_msg = stream_cipher::decrypt::<ReplySurbEncryptionAlgorithm>(
            reply_key.inner(),
            &zero_iv,
            reply_ciphertext,
        );
        if let Err(err) = MessageReceiver::remove_padding(&mut reply_msg) {
            warn!("Received reply had malformed padding! - {:?}", err);
            None
        } else {
            // TODO: perhaps having to say it doesn't have a surb an indication the type should be changed?
            Some(ReconstructedMessage {
                message: reply_msg,
                reply_surb: None,
            })
        }
    }

    async fn handle_new_received(&mut self, msgs: Vec<Vec<u8>>) {
        debug!(
            "Processing {:?} new message that might get added to the buffer!",
            msgs.len()
        );

        let mut completed_messages = Vec::new();
        let mut inner_guard = self.inner.lock().await;

        let reply_surb_digest_size = ReplySurbKeyDigestAlgorithm::output_size();

        // first check if this is a reply or a chunked message
        // TODO: verify with @AP if this way of doing it is safe or whether it could
        // cause some attacks due to, I don't know, stupid edge case collisions?
        // Update: this DOES introduce a possible leakage: https://github.com/nymtech/nym/issues/296
        for msg in msgs {
            let possible_key_digest =
                EncryptionKeyDigest::clone_from_slice(&msg[..reply_surb_digest_size]);

            // check first `HasherOutputSize` bytes if they correspond to known encryption key
            // if yes - this is a reply message

            // TODO: this might be a bottleneck - since the keys are stored on disk we, presumably,
            // are doing a disk operation every single received fragment
            if let Some(reply_encryption_key) = self
                .reply_key_storage
                .get_and_remove_encryption_key(possible_key_digest)
                .expect("storage operation failed!")
            {
                if let Some(completed_message) = Self::process_received_reply(
                    &msg[reply_surb_digest_size..],
                    reply_encryption_key,
                ) {
                    completed_messages.push(completed_message)
                }
            } else {
                // otherwise - it's a 'normal' message
                if let Some(completed_message) = inner_guard.process_received_fragment(msg) {
                    completed_messages.push(completed_message)
                }
            }
        }

        if !completed_messages.is_empty() {
            if let Some(sender) = &inner_guard.message_sender {
                trace!("Sending reconstructed messages to announced sender");
                if let Err(err) = sender.unbounded_send(completed_messages) {
                    warn!("The reconstructed message receiver went offline without explicit notification (relevant error: - {:?})", err);
                    // make sure to drop the lock to not deadlock
                    // (it is required by `add_reconstructed_messages`)
                    inner_guard.message_sender = None;
                    drop(inner_guard);
                    self.add_reconstructed_messages(err.into_inner()).await;
                }
            } else {
                // make sure to drop the lock to not deadlock
                // (it is required by `add_reconstructed_messages`)
                drop(inner_guard);
                trace!("No sender available - buffering reconstructed messages");
                self.add_reconstructed_messages(completed_messages).await;
            }
        }
    }
}

pub enum ReceivedBufferMessage {
    // Signals a websocket connection (or a native implementation) was established and we should stop buffering messages,
    // and instead send them directly to the received channel
    ReceiverAnnounce(ReconstructedMessagesSender),

    // Explicit signal that Receiver connection will no longer accept messages
    ReceiverDisconnect,
}

struct RequestReceiver {
    received_buffer: ReceivedMessagesBuffer,
    query_receiver: ReceivedBufferRequestReceiver,
}

impl RequestReceiver {
    fn new(
        received_buffer: ReceivedMessagesBuffer,
        query_receiver: ReceivedBufferRequestReceiver,
    ) -> Self {
        RequestReceiver {
            received_buffer,
            query_receiver,
        }
    }

    fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(request) = self.query_receiver.next().await {
                match request {
                    ReceivedBufferMessage::ReceiverAnnounce(sender) => {
                        self.received_buffer.connect_sender(sender).await;
                    }
                    ReceivedBufferMessage::ReceiverDisconnect => {
                        self.received_buffer.disconnect_sender().await
                    }
                }
            }
        })
    }
}

struct FragmentedMessageReceiver {
    received_buffer: ReceivedMessagesBuffer,
    mixnet_packet_receiver: MixnetMessageReceiver,
}

impl FragmentedMessageReceiver {
    fn new(
        received_buffer: ReceivedMessagesBuffer,
        mixnet_packet_receiver: MixnetMessageReceiver,
    ) -> Self {
        FragmentedMessageReceiver {
            received_buffer,
            mixnet_packet_receiver,
        }
    }
    fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(new_messages) = self.mixnet_packet_receiver.next().await {
                self.received_buffer.handle_new_received(new_messages).await;
            }
        })
    }
}

pub struct ReceivedMessagesBufferController {
    fragmented_message_receiver: FragmentedMessageReceiver,
    request_receiver: RequestReceiver,
}

impl ReceivedMessagesBufferController {
    pub fn new(
        local_encryption_keypair: Arc<encryption::KeyPair>,
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_packet_receiver: MixnetMessageReceiver,
        reply_key_storage: ReplyKeyStorage,
    ) -> Self {
        let received_buffer =
            ReceivedMessagesBuffer::new(local_encryption_keypair, reply_key_storage);

        ReceivedMessagesBufferController {
            fragmented_message_receiver: FragmentedMessageReceiver::new(
                received_buffer.clone(),
                mixnet_packet_receiver,
            ),
            request_receiver: RequestReceiver::new(received_buffer, query_receiver),
        }
    }

    pub fn start(self) {
        // TODO: should we do anything with JoinHandle(s) returned by start methods?
        self.fragmented_message_receiver.start();
        self.request_receiver.start();
    }
}
