// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Stream wrapper for NymClient.
//!
//! This module provides `NymClientStream`, a Rust-only wrapper around `NymClient`
//! that implements `futures::Stream` for receiving messages. This makes it easy
//! to use the Nym client with async patterns, particularly for libp2p transport
//! integration.

use futures::channel::mpsc::UnboundedReceiver;
use futures::{ready, SinkExt, Stream, StreamExt};
use nym_wasm_client_core::client::base_client::{ClientInput, ClientOutput};
use nym_wasm_client_core::client::inbound_messages::InputMessage;
use nym_wasm_client_core::client::received_buffer::ReceivedBufferMessage;
use nym_wasm_client_core::ReconstructedMessage;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio_with_wasm::sync::RwLock;

use crate::client::NymClient;
use crate::error::WasmClientError;

/// A stream wrapper around `NymClient` that implements `futures::Stream`.
///
/// This type is for Rust-only use (not exposed to JavaScript) and provides
/// a cleaner API for async message handling, particularly useful for
/// libp2p transport integration.
///
/// # Example
/// ```ignore
/// use nym_client_wasm::client::NymClientBuilder;
/// use nym_client_wasm::stream::NymClientStream;
/// use futures::StreamExt;
///
/// let (client, client_output) = builder.start_client_for_transport().await?;
/// let mut stream = NymClientStream::new(client, client_output);
///
/// // Use as a Stream
/// while let Some(msg) = stream.next().await {
///     println!("Received: {:?}", msg);
/// }
/// ```
pub struct NymClientStream {
    /// The underlying NymClient (kept alive to prevent shutdown)
    #[allow(dead_code)]
    client: NymClient,

    /// Receiver for reconstructed messages from the mixnet
    reconstructed_receiver: UnboundedReceiver<Vec<ReconstructedMessage>>,

    /// Buffer for messages received in batches
    buffered_messages: Vec<ReconstructedMessage>,

    /// Client input for sending messages
    client_input: Arc<RwLock<ClientInput>>,
}

impl NymClientStream {
    /// Create a new `NymClientStream` from a `NymClient` and `ClientOutput`.
    ///
    /// Use `NymClientBuilder::start_client_for_transport()` to get both components.
    pub fn new(client: NymClient, client_output: ClientOutput) -> Self {
        // Register to receive reconstructed messages
        let (tx, rx) = futures::channel::mpsc::unbounded();

        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(tx))
            .expect("Failed to register for reconstructed messages");

        let client_input = client.client_input();

        Self {
            client,
            reconstructed_receiver: rx,
            buffered_messages: Vec::new(),
            client_input,
        }
    }

    /// Get our Nym address as a string.
    pub fn self_address(&self) -> String {
        self.client.self_address()
    }

    /// Get access to the underlying client input for sending messages directly.
    pub fn client_input(&self) -> Arc<RwLock<ClientInput>> {
        self.client_input.clone()
    }

    /// Send an input message to the mixnet.
    pub async fn send(&self, message: InputMessage) -> Result<(), WasmClientError> {
        let mut input = self.client_input.write().await;
        input
            .input_sender
            .send(message)
            .await
            .map_err(|e| WasmClientError::SendFailure(e.to_string()))
    }
}

impl Stream for NymClientStream {
    type Item = ReconstructedMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First, return any buffered messages
        if let Some(msg) = self.buffered_messages.pop() {
            // If there are more buffered, wake immediately
            if !self.buffered_messages.is_empty() {
                cx.waker().wake_by_ref();
            }
            return Poll::Ready(Some(msg));
        }

        // Poll for new batch of messages
        match ready!(self.reconstructed_receiver.poll_next_unpin(cx)) {
            None => Poll::Ready(None),
            Some(mut msgs) => {
                if let Some(msg) = msgs.pop() {
                    // Buffer remaining messages
                    if !msgs.is_empty() {
                        self.buffered_messages = msgs;
                        cx.waker().wake_by_ref();
                    }
                    Poll::Ready(Some(msg))
                } else {
                    // Empty batch, poll again
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
    }
}
