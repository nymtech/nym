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

use crate::client::provider_poller::PolledMessagesReceiver;
use futures::channel::{mpsc, oneshot};
use futures::lock::Mutex;
use futures::StreamExt;
use log::*;
use nymsphinx::chunking::reconstruction::MessageReconstructor;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub(crate) type ReceivedBufferResponse = oneshot::Sender<Vec<Vec<u8>>>;
pub(crate) type ReceivedBufferRequestSender = mpsc::UnboundedSender<ReceivedBufferResponse>;
pub(crate) type ReceivedBufferRequestReceiver = mpsc::UnboundedReceiver<ReceivedBufferResponse>;

struct ReceivedMessagesBufferInner {
    messages: Vec<Vec<u8>>,
    message_reconstructor: MessageReconstructor,
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
            })),
        }
    }

    async fn add_reconstructed_messages(&mut self, msgs: Vec<Vec<u8>>) {
        debug!("Adding {:?} new messages to the buffer!", msgs.len());
        trace!("Adding new messages to the buffer! {:?}", msgs);
        self.inner.lock().await.messages.extend(msgs)
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
            if let Some(reconstructed_message) =
                inner_guard.message_reconstructor.new_fragment(msg_fragment)
            {
                completed_messages.push(reconstructed_message);
            }
        }
        // make sure to drop the lock to not deadlock
        drop(inner_guard);
        if !completed_messages.is_empty() {
            self.add_reconstructed_messages(completed_messages).await;
        }
    }

    async fn acquire_and_empty(&mut self) -> Vec<Vec<u8>> {
        trace!("Emptying the buffer and returning all messages");
        let mut mutex_guard = self.inner.lock().await;
        std::mem::replace(&mut mutex_guard.messages, Vec::new())
    }
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
                let messages = self.received_buffer.acquire_and_empty().await;
                if let Err(failed_messages) = request.send(messages) {
                    error!(
                        "Failed to send the messages to the requester. Adding them back to the buffer"
                    );
                    self.received_buffer.add_reconstructed_messages(failed_messages).await;
                }
            }
        })
    }
}

struct MessageReceiver {
    received_buffer: ReceivedMessagesBuffer,
    poller_receiver: PolledMessagesReceiver,
}

impl MessageReceiver {
    fn new(
        received_buffer: ReceivedMessagesBuffer,
        poller_receiver: PolledMessagesReceiver,
    ) -> Self {
        MessageReceiver {
            received_buffer,
            poller_receiver,
        }
    }
    fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            while let Some(new_messages) = self.poller_receiver.next().await {
                self.received_buffer
                    .add_new_message_fragments(new_messages)
                    .await;
            }
        })
    }
}

pub(crate) struct ReceivedMessagesBufferController {
    messsage_receiver: MessageReceiver,
    request_receiver: RequestReceiver,
}

impl ReceivedMessagesBufferController {
    pub(crate) fn new(
        query_receiver: ReceivedBufferRequestReceiver,
        poller_receiver: PolledMessagesReceiver,
    ) -> Self {
        let received_buffer = ReceivedMessagesBuffer::new();

        ReceivedMessagesBufferController {
            messsage_receiver: MessageReceiver::new(received_buffer.clone(), poller_receiver),
            request_receiver: RequestReceiver::new(received_buffer, query_receiver),
        }
    }

    pub(crate) fn start(self, handle: &Handle) {
        // TODO: should we do anything with JoinHandle(s) returned by start methods?
        self.messsage_receiver.start(handle);
        self.request_receiver.start(handle);
    }
}
