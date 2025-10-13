// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::helpers::get_time_now;
use crate::client::replies::{
    reply_controller::ReplyControllerSender, reply_storage::SentReplyKeys,
};
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
use nym_crypto::asymmetric::x25519;
use nym_crypto::Digest;
use nym_gateway_client::MixnetMessageReceiver;
use nym_sphinx::anonymous_replies::requests::{
    RepliableMessage, RepliableMessageContent, ReplyMessage, ReplyMessageContent,
};
use nym_sphinx::anonymous_replies::{encryption_key::EncryptionKeyDigest, SurbEncryptionKey};
use nym_sphinx::message::{NymMessage, PlainMessage};
use nym_sphinx::params::ReplySurbKeyDigestAlgorithm;
use nym_sphinx::receiver::{MessageReceiver, MessageRecoveryError, ReconstructedMessage};
use nym_statistics_common::clients::{packet_statistics::PacketStatisticsEvent, ClientStatsSender};
use nym_task::ShutdownToken;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tracing::*;

// The interval at which we check for stale buffers
const STALE_BUFFER_CHECK_INTERVAL: Duration = Duration::from_secs(10);

// Buffer Requests to say "hey, send any reconstructed messages to this channel"
// or to say "hey, I'm going offline, don't send anything more to me. Just buffer them instead"
pub type ReceivedBufferRequestSender = mpsc::UnboundedSender<ReceivedBufferMessage>;
pub type ReceivedBufferRequestReceiver = mpsc::UnboundedReceiver<ReceivedBufferMessage>;

// The channel set for the above
pub type ReconstructedMessagesSender = mpsc::UnboundedSender<Vec<ReconstructedMessage>>;
pub type ReconstructedMessagesReceiver = mpsc::UnboundedReceiver<Vec<ReconstructedMessage>>;

struct ReceivedMessagesBufferInner<R: MessageReceiver> {
    messages: Vec<ReconstructedMessage>,
    local_encryption_keypair: Arc<x25519::KeyPair>,

    // TODO: looking how it 'looks' here, perhaps `MessageReceiver` should be renamed to something
    // else instead.
    message_receiver: R,
    message_sender: Option<ReconstructedMessagesSender>,

    // TODO: this will get cleared upon re-running the client
    // but perhaps it should be changed to include timestamps of when the message was reconstructed
    // and every now and then remove ids older than X
    recently_reconstructed: HashSet<i32>,

    stats_tx: ClientStatsSender,

    // Periodically check for stale buffers to clean up
    last_stale_check: crate::client::helpers::Instant,
}

impl<R: MessageReceiver> ReceivedMessagesBufferInner<R> {
    fn recover_from_fragment(
        &mut self,
        fragment_data: &[u8],
        fragment_data_size: usize,
    ) -> Option<NymMessage> {
        if nym_sphinx::cover::is_cover(fragment_data) {
            trace!("The message was a loop cover message! Skipping it");
            // NOTE: it's important to note that there is quite a bit of difference in size of
            // received and sent packets due to the sphinx layers being removed by the exit gateway
            // before it reaches the mixnet client.
            self.stats_tx
                .report(PacketStatisticsEvent::CoverPacketReceived(fragment_data_size).into());
            return None;
        }

        self.stats_tx
            .report(PacketStatisticsEvent::RealPacketReceived(fragment_data_size).into());

        let fragment = match self.message_receiver.recover_fragment(fragment_data) {
            Err(err) => {
                warn!("failed to recover fragment from raw data: {err}. The whole underlying message might be corrupted and unrecoverable!");
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
                MessageRecoveryError::MalformedReconstructedMessage { source, used_sets } => {
                    error!("message reconstruction failed - {source}. Attempting to re-use the message sets...");
                    // TODO: should we really insert reconstructed sets? could this be abused for some attack?
                    for set_id in used_sets {
                        if !self.recently_reconstructed.insert(set_id) {
                            // or perhaps we should even panic at this point?
                            error!("Reconstructed another message containing already used set id!")
                        }
                    }
                    None
                }
                _ => {
                    error!("unexpected error occurred during message reconstruction: {err}");
                    None
                }
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

    fn process_received_reply(
        &mut self,
        reply_ciphertext: &mut [u8],
        reply_key: SurbEncryptionKey,
    ) -> Result<Option<NymMessage>, MessageRecoveryError> {
        let reply_ciphertext_size = reply_ciphertext.len();
        // note: this performs decryption IN PLACE without extra allocation
        self.message_receiver
            .recover_plaintext_from_reply(reply_ciphertext, reply_key)?;
        let fragment_data = reply_ciphertext;

        Ok(self.recover_from_fragment(fragment_data, reply_ciphertext_size))
    }

    fn process_received_regular_packet(&mut self, mut raw_fragment: Vec<u8>) -> Option<NymMessage> {
        let raw_fragment_size = raw_fragment.len();
        let fragment_data = match self.message_receiver.recover_plaintext_from_regular_packet(
            self.local_encryption_keypair.private_key(),
            &mut raw_fragment,
        ) {
            Err(err) => {
                warn!("failed to recover fragment data: {err}. The whole underlying message might be corrupted and unrecoverable!");
                return None;
            }
            Ok(frag_data) => frag_data,
        };

        self.recover_from_fragment(fragment_data, raw_fragment_size)
    }

    fn cleanup_stale_buffers(&mut self) {
        let now = get_time_now();
        if now - self.last_stale_check > STALE_BUFFER_CHECK_INTERVAL {
            self.last_stale_check = now;
            self.message_receiver
                .reconstructor()
                .cleanup_stale_buffers();
        }
    }
}

#[derive(Debug, Clone)]
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
struct ReceivedMessagesBuffer<R: MessageReceiver> {
    inner: Arc<Mutex<ReceivedMessagesBufferInner<R>>>,
    reply_key_storage: SentReplyKeys,
    reply_controller_sender: ReplyControllerSender,
    shutdown_token: ShutdownToken,
}

impl<R: MessageReceiver> ReceivedMessagesBuffer<R> {
    fn new(
        local_encryption_keypair: Arc<x25519::KeyPair>,
        reply_key_storage: SentReplyKeys,
        reply_controller_sender: ReplyControllerSender,
        stats_tx: ClientStatsSender,
        shutdown_token: ShutdownToken,
    ) -> Self {
        ReceivedMessagesBuffer {
            inner: Arc::new(Mutex::new(ReceivedMessagesBufferInner {
                messages: Vec::new(),
                local_encryption_keypair,
                message_receiver: R::new(),
                message_sender: None,
                recently_reconstructed: HashSet::new(),
                stats_tx,
                last_stale_check: get_time_now(),
            })),
            reply_key_storage,
            reply_controller_sender,
            shutdown_token,
        }
    }

    #[allow(clippy::panic)]
    async fn disconnect_sender(&mut self) {
        let mut guard = self.inner.lock().await;
        if guard.message_sender.is_none() {
            // in theory we could just ignore it, but that situation should have never happened
            // in the first place, so this way we at least know we have an important bug to fix
            panic!("trying to disconnect non-existent sender!")
        }
        guard.message_sender = None;
    }

    #[allow(clippy::panic)]
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
                error!("The sender channel we just received is already invalidated - {err:?}");
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

    fn handle_reconstructed_plain_messages(
        &mut self,
        msgs: Vec<PlainMessage>,
    ) -> Vec<ReconstructedMessage> {
        msgs.into_iter().map(Into::into).collect()
    }

    fn handle_reconstructed_repliable_messages(
        &mut self,
        msgs: Vec<RepliableMessage>,
    ) -> Vec<ReconstructedMessage> {
        let mut reconstructed = Vec::new();
        for msg in msgs {
            let (reply_surbs, from_surb_request) = match msg.content {
                RepliableMessageContent::Data(content) => {
                    let reply_surbs = content.reply_surbs;
                    let message = content.message;

                    trace!(
                        "received message that also contained additional {} reply surbs from {:?}!",
                        reply_surbs.len(),
                        msg.sender_tag
                    );

                    reconstructed.push(ReconstructedMessage::new(message, msg.sender_tag));

                    (reply_surbs, false)
                }
                RepliableMessageContent::AdditionalSurbs(content) => {
                    let reply_surbs = content.reply_surbs;

                    trace!(
                        "received additional {} reply surbs from {:?}!",
                        reply_surbs.len(),
                        msg.sender_tag
                    );
                    (reply_surbs, true)
                }
                RepliableMessageContent::Heartbeat(content) => {
                    let additional_reply_surbs = content.additional_reply_surbs;
                    error!("received a repliable heartbeat message - we don't know how to handle it yet (and we won't know until future PRs)");
                    (additional_reply_surbs, false)
                }
                RepliableMessageContent::DataV2(content) => {
                    let reply_surbs = content.reply_surbs;
                    let message = content.message;

                    trace!(
                        "received message that also contained additional {} reply surbs from {:?}!",
                        reply_surbs.len(),
                        msg.sender_tag
                    );

                    reconstructed.push(ReconstructedMessage::new(message, msg.sender_tag));

                    (reply_surbs, false)
                }
                RepliableMessageContent::AdditionalSurbsV2(content) => {
                    let reply_surbs = content.reply_surbs;

                    trace!(
                        "received additional {} reply surbs from {:?}!",
                        reply_surbs.len(),
                        msg.sender_tag
                    );
                    (reply_surbs, true)
                }
                RepliableMessageContent::HeartbeatV2(content) => {
                    let additional_reply_surbs = content.additional_reply_surbs;
                    error!("received a repliable heartbeat message - we don't know how to handle it yet (and we won't know until future PRs)");
                    (additional_reply_surbs, false)
                }
            };

            if !reply_surbs.is_empty() {
                if let Err(err) = self.reply_controller_sender.send_additional_surbs(
                    msg.sender_tag,
                    reply_surbs,
                    from_surb_request,
                ) {
                    if !self.shutdown_token.is_cancelled() {
                        error!("{err}");
                    }
                }
            }
        }
        reconstructed
    }

    fn handle_reconstructed_reply_messages(
        &mut self,
        msgs: Vec<ReplyMessage>,
    ) -> Vec<ReconstructedMessage> {
        let mut reconstructed = Vec::new();
        for msg in msgs {
            match msg.content {
                ReplyMessageContent::Data { message } => reconstructed.push(message.into()),
                ReplyMessageContent::SurbRequest { recipient, amount } => {
                    debug!("received request for {amount} additional reply SURBs from {recipient}");
                    if let Err(err) = self
                        .reply_controller_sender
                        .send_additional_surbs_request(*recipient, amount)
                    {
                        if !self.shutdown_token.is_cancelled() {
                            error!("{err}");
                        }
                    }
                }
            }
        }
        reconstructed
    }

    async fn handle_reconstructed_messages(&mut self, msgs: Vec<NymMessage>) {
        if msgs.is_empty() {
            return;
        }

        let mut plain_messages = Vec::new();
        let mut repliable_messages = Vec::new();
        let mut reply_messages = Vec::new();

        for msg in msgs {
            match msg {
                NymMessage::Plain(plain) => plain_messages.push(plain),
                NymMessage::Repliable(repliable) => repliable_messages.push(repliable),
                NymMessage::Reply(reply) => reply_messages.push(reply),
            }
        }

        let mut reconstructed_messages = self.handle_reconstructed_plain_messages(plain_messages);
        reconstructed_messages
            .append(&mut self.handle_reconstructed_repliable_messages(repliable_messages));
        reconstructed_messages
            .append(&mut self.handle_reconstructed_reply_messages(reply_messages));

        let mut inner_guard = self.inner.lock().await;
        debug!(
            "Adding {:?} new messages to the buffer!",
            reconstructed_messages.len()
        );

        if let Some(sender) = &inner_guard.message_sender {
            trace!("Sending reconstructed messages to announced sender");
            if let Err(err) = sender.unbounded_send(reconstructed_messages) {
                warn!("The reconstructed message receiver went offline without explicit notification (relevant error: - {err})");
                inner_guard.message_sender = None;
                inner_guard.messages.extend(err.into_inner());
            }
        } else {
            trace!("No sender available - buffering reconstructed messages");
            inner_guard.messages.extend(reconstructed_messages)
        }
    }

    // this function doesn't really belong here...
    fn get_reply_key<'a>(
        &self,
        raw_message: &'a mut [u8],
    ) -> Option<(SurbEncryptionKey, &'a mut [u8])> {
        let reply_surb_digest_size = ReplySurbKeyDigestAlgorithm::output_size();
        if raw_message.len() < reply_surb_digest_size {
            return None;
        }

        let possible_key_digest =
            EncryptionKeyDigest::clone_from_slice(&raw_message[..reply_surb_digest_size]);
        self.reply_key_storage
            .try_pop(possible_key_digest)
            .map(|reply_encryption_key| {
                (
                    *reply_encryption_key,
                    &mut raw_message[reply_surb_digest_size..],
                )
            })
    }

    async fn handle_new_received(
        &mut self,
        msgs: Vec<Vec<u8>>,
    ) -> Result<(), MessageRecoveryError> {
        trace!(
            "Processing {:?} new message that might get added to the buffer!",
            msgs.len()
        );

        let mut completed_messages = Vec::new();
        let mut inner_guard = self.inner.lock().await;

        // first check if this is a reply or a chunked message
        // note: there's a possible information leakage associated with this check https://github.com/nymtech/nym/issues/296
        for mut msg in msgs {
            // check first `HasherOutputSize` bytes if they correspond to known encryption key
            // if yes - this is a reply message
            let completed_message =
                if let Some((reply_key, reply_message)) = self.get_reply_key(&mut msg) {
                    inner_guard.process_received_reply(reply_message, reply_key)?
                } else {
                    inner_guard.process_received_regular_packet(msg)
                };

            if let Some(completed) = completed_message {
                debug!("received {completed}");
                completed_messages.push(completed)
            }
        }

        // Cleanup stale buffers, if there are any fragments that simply never arrived.
        // We do this here as part of handling new received fragments so that we can keep the event
        // loop focused on processing new messages.
        inner_guard.cleanup_stale_buffers();

        drop(inner_guard);

        if !completed_messages.is_empty() {
            self.handle_reconstructed_messages(completed_messages).await
        }
        Ok(())
    }
}

pub enum ReceivedBufferMessage {
    // Signals a websocket connection (or a native implementation) was established and we should stop buffering messages,
    // and instead send them directly to the received channel
    ReceiverAnnounce(ReconstructedMessagesSender),

    // Explicit signal that Receiver connection will no longer accept messages
    ReceiverDisconnect,
}

pub(crate) struct RequestReceiver<R: MessageReceiver> {
    received_buffer: ReceivedMessagesBuffer<R>,
    query_receiver: ReceivedBufferRequestReceiver,
    shutdown_token: ShutdownToken,
}

impl<R: MessageReceiver> RequestReceiver<R> {
    fn new(
        received_buffer: ReceivedMessagesBuffer<R>,
        query_receiver: ReceivedBufferRequestReceiver,
        shutdown_token: ShutdownToken,
    ) -> Self {
        RequestReceiver {
            received_buffer,
            query_receiver,
            shutdown_token,
        }
    }

    async fn handle_message(&mut self, message: ReceivedBufferMessage) {
        match message {
            ReceivedBufferMessage::ReceiverAnnounce(sender) => {
                self.received_buffer.connect_sender(sender).await;
            }
            ReceivedBufferMessage::ReceiverDisconnect => {
                self.received_buffer.disconnect_sender().await
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started RequestReceiver with graceful shutdown support");

        let _drop_guard = self.shutdown_token.clone().drop_guard();
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    tracing::trace!("RequestReceiver: Received shutdown");
                    break;
                }
                request = self.query_receiver.next() => {
                    if let Some(message) = request {
                        self.handle_message(message).await
                    } else {
                        tracing::trace!("RequestReceiver: Stopping since channel closed");
                        self.shutdown_token.cancelled().await;
                        break;
                    }
                },
            }
        }
        tracing::debug!("RequestReceiver: Exiting");
    }
}

pub(crate) struct FragmentedMessageReceiver<R: MessageReceiver> {
    received_buffer: ReceivedMessagesBuffer<R>,
    mixnet_packet_receiver: MixnetMessageReceiver,
    shutdown_token: ShutdownToken,
}

impl<R: MessageReceiver> FragmentedMessageReceiver<R> {
    fn new(
        received_buffer: ReceivedMessagesBuffer<R>,
        mixnet_packet_receiver: MixnetMessageReceiver,
        shutdown_token: ShutdownToken,
    ) -> Self {
        FragmentedMessageReceiver {
            received_buffer,
            mixnet_packet_receiver,
            shutdown_token,
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), MessageRecoveryError> {
        debug!("Started FragmentedMessageReceiver with graceful shutdown support");

        let _drop_guard = self.shutdown_token.clone().drop_guard();
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    tracing::trace!("FragmentedMessageReceiver: Received shutdown");
                    break;
                }
                new_messages = self.mixnet_packet_receiver.next() => {
                    if let Some(new_messages) = new_messages {
                        self.received_buffer.handle_new_received(new_messages).await?;
                    } else {
                        tracing::trace!("FragmentedMessageReceiver: Stopping since channel closed");
                        self.shutdown_token.cancelled().await;
                        break;
                    }
                },

            }
        }
        tracing::debug!("FragmentedMessageReceiver: Exiting");
        Ok(())
    }
}

pub(crate) struct ReceivedMessagesBufferController<R: MessageReceiver> {
    fragmented_message_receiver: FragmentedMessageReceiver<R>,
    request_receiver: RequestReceiver<R>,
}

impl<R: MessageReceiver + Clone + Send + 'static> ReceivedMessagesBufferController<R> {
    pub(crate) fn new(
        local_encryption_keypair: Arc<x25519::KeyPair>,
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_packet_receiver: MixnetMessageReceiver,
        reply_key_storage: SentReplyKeys,
        reply_controller_sender: ReplyControllerSender,
        metrics_reporter: ClientStatsSender,
        shutdown_token: ShutdownToken,
    ) -> Self {
        let received_buffer = ReceivedMessagesBuffer::new(
            local_encryption_keypair,
            reply_key_storage,
            reply_controller_sender,
            metrics_reporter,
            shutdown_token.clone(),
        );

        ReceivedMessagesBufferController {
            fragmented_message_receiver: FragmentedMessageReceiver::new(
                received_buffer.clone(),
                mixnet_packet_receiver,
                shutdown_token.clone(),
            ),
            request_receiver: RequestReceiver::new(
                received_buffer,
                query_receiver,
                shutdown_token.clone(),
            ),
        }
    }

    pub(crate) fn into_tasks(self) -> (FragmentedMessageReceiver<R>, RequestReceiver<R>) {
        (self.fragmented_message_receiver, self.request_receiver)
    }
}
