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

use futures::channel::mpsc;
use futures::lock::{Mutex, MutexGuard};
use futures::StreamExt;
use gateway_client::MixnetMessageReceiver;
use log::*;
use nymsphinx::chunking::reconstruction::MessageReconstructor;
use nymsphinx::cover::LOOP_COVER_MESSAGE_PAYLOAD;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

// Buffer Requests to say "hey, send any reconstructed messages to this channel"
// or to say "hey, I'm going offline, don't send anything more to me. Just buffer them instead"
pub(crate) type ReceivedBufferRequestSender = mpsc::UnboundedSender<ReceivedBufferMessage>;
pub(crate) type ReceivedBufferRequestReceiver = mpsc::UnboundedReceiver<ReceivedBufferMessage>;

// The channel set for the above
pub(crate) type ReconstructedMessagesSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub(crate) type ReconstructedMessagesReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

struct ReceivedMessagesBufferInner {
    messages: Vec<Vec<u8>>,
    message_reconstructor: MessageReconstructor,
    message_sender: Option<ReconstructedMessagesSender>,

    // TODO: this will get cleared upon re-running the client
    // but perhaps it should be changed to include timestamps of when the message was reconstructed
    // and every now and then remove ids older than X
    recently_reconstructed: HashSet<i32>,
}

#[derive(Debug, Clone)]
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
struct ReceivedMessagesBuffer {
    inner: Arc<Mutex<ReceivedMessagesBufferInner>>,
}

impl ReceivedMessagesBuffer {
    fn new() -> Self {
        ReceivedMessagesBuffer {
            inner: Arc::new(Mutex::new(ReceivedMessagesBufferInner {
                messages: Vec::new(),
                message_reconstructor: MessageReconstructor::new(),
                message_sender: None,
                recently_reconstructed: HashSet::new(),
            })),
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
        let stored_messages = std::mem::replace(&mut guard.messages, Vec::new());
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

    async fn add_reconstructed_messages(&mut self, msgs: Vec<Vec<u8>>) {
        debug!("Adding {:?} new messages to the buffer!", msgs.len());
        trace!("Adding new messages to the buffer! {:?}", msgs);
        self.inner.lock().await.messages.extend(msgs)
    }

    fn process_received_fragment(
        mutex_guard: &mut MutexGuard<ReceivedMessagesBufferInner>,
        raw_fragment: Vec<u8>,
    ) -> Option<Vec<u8>> {
        if raw_fragment == LOOP_COVER_MESSAGE_PAYLOAD {
            trace!("The message was a loop cover message! Skipping it");
            return None;
        }

        let fragment = match mutex_guard
            .message_reconstructor
            .recover_fragment(raw_fragment)
        {
            Err(e) => {
                warn!("failed to recover fragment data: {:?}. The whole underlying message might be corrupted and unrecoverable!", e);
                return None;
            }
            Ok(frag) => frag,
        };

        if mutex_guard.recently_reconstructed.contains(&fragment.id()) {
            debug!("Received a chunk of already re-assembled message ({:?})! It probably got here because the ack got lost", fragment.id());
            return None;
        }

        if let Some(reconstructed_message) = mutex_guard
            .message_reconstructor
            .insert_new_fragment(fragment)
        {
            let (completed_message, message_sets) = reconstructed_message;
            for set_id in message_sets {
                if !mutex_guard.recently_reconstructed.insert(set_id) {
                    // or perhaps we should even panic at this point?
                    error!("Reconstructed another message containing already used set id!")
                }
            }
            return Some(completed_message);
        }
        None
    }

    async fn add_new_message_fragments(&mut self, msgs: Vec<Vec<u8>>) {
        debug!(
            "Adding {:?} new message fragments to the buffer!",
            msgs.len()
        );
        trace!("Adding new message fragments to the buffer! {:?}", msgs);

        let mut completed_messages = Vec::new();
        let mut inner_guard = self.inner.lock().await;
        for msg_fragment in msgs {
            if let Some(completed_message) =
                Self::process_received_fragment(&mut inner_guard, msg_fragment)
            {
                completed_messages.push(completed_message)
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

pub(crate) enum ReceivedBufferMessage {
    // Signals a websocket connection (or a native implementation) was established
    // and we should stop buffering messages, and instead send them directly to
    // the received channel
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

    fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
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
    fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            while let Some(new_messages) = self.mixnet_packet_receiver.next().await {
                self.received_buffer
                    .add_new_message_fragments(new_messages)
                    .await;
            }
        })
    }
}

pub(crate) struct ReceivedMessagesBufferController {
    fragmented_message_receiver: FragmentedMessageReceiver,
    request_receiver: RequestReceiver,
}

impl ReceivedMessagesBufferController {
    pub(crate) fn new(
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_packet_receiver: MixnetMessageReceiver,
    ) -> Self {
        let received_buffer = ReceivedMessagesBuffer::new();

        ReceivedMessagesBufferController {
            fragmented_message_receiver: FragmentedMessageReceiver::new(
                received_buffer.clone(),
                mixnet_packet_receiver,
            ),
            request_receiver: RequestReceiver::new(received_buffer, query_receiver),
        }
    }

    pub(crate) fn start(self, handle: &Handle) {
        // TODO: should we do anything with JoinHandle(s) returned by start methods?
        self.fragmented_message_receiver.start(handle);
        self.request_receiver.start(handle);
    }
}
