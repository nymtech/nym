// Copyright 2024 Nym Technologies SA
// SPDX-License-Identifier: Apache-2.0

//! Mixnet bridge for WASM - adapts the Nym client stream to channel-based async.
//!
//! This module bridges the NymClientStream to the channel-based architecture
//! used by the libp2p transport.

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::{SinkExt, StreamExt};
use log::debug;
use nym_client_wasm::stream::NymClientStream;
use nym_sphinx_addressing::clients::Recipient;
use nym_wasm_client_core::client::inbound_messages::InputMessage;
use nym_wasm_client_core::nym_task::connections::TransmissionLane;
use std::sync::Arc;
use tokio_with_wasm::sync::RwLock;
use wasm_bindgen_futures::spawn_local;

use super::error::Error;
use super::message::*;

/// Default number of reply SURBs to attach when sending anonymous messages.
const DEFAULT_REPLY_SURBS: u32 = 10;

/// Initialize the mixnet bridge for libp2p transport.
///
/// This function bridges the NymClientStream to the channel-based async pattern
/// used by the libp2p transport. It spawns background tasks to:
/// 1. Forward outbound messages from the transport to the mixnet client
/// 2. Forward inbound messages from the mixnet client to the transport
pub(crate) async fn initialize_mixnet(
    self_address: Recipient,
    stream: NymClientStream,
    notify_inbound_tx: Option<UnboundedSender<()>>,
) -> Result<
    (
        Recipient,
        UnboundedReceiver<InboundMessage>,
        UnboundedSender<OutboundMessage>,
    ),
    Error,
> {
    // Channel for inbound messages from the mixnet to the transport
    let (inbound_tx, inbound_rx) = unbounded::<InboundMessage>();

    // Channel for outbound messages from the transport to the mixnet
    let (outbound_tx, outbound_rx) = unbounded::<OutboundMessage>();

    // Get client_input for sending before we move stream
    let client_input = stream.client_input();

    // Wrap stream in Arc<RwLock> so we can share it
    let stream = Arc::new(RwLock::new(stream));

    // Spawn the outbound message handler
    spawn_local(run_outbound_loop(client_input, outbound_rx));

    // Spawn the inbound message handler
    spawn_local(run_inbound_loop(stream, inbound_tx, notify_inbound_tx));

    Ok((self_address, inbound_rx, outbound_tx))
}

/// Background task that forwards outbound messages from the transport to the mixnet.
async fn run_outbound_loop(
    client_input: Arc<RwLock<nym_wasm_client_core::client::base_client::ClientInput>>,
    mut outbound_rx: UnboundedReceiver<OutboundMessage>,
) {
    while let Some(message) = outbound_rx.next().await {
        if let Err(e) = send_outbound_message(&client_input, message).await {
            debug!("Failed to send outbound message: {:?}", e);
        }
    }
    debug!("Outbound message loop ended");
}

/// Send a single outbound message via the mixnet client.
async fn send_outbound_message(
    client_input: &Arc<RwLock<nym_wasm_client_core::client::base_client::ClientInput>>,
    message: OutboundMessage,
) -> Result<(), Error> {
    log_outbound_message(&message);

    let data = message.message.to_bytes();

    let input_msg = match (&message.recipient, &message.sender_tag) {
        // Reply using SURB (anonymous reply)
        (_, Some(sender_tag)) => {
            debug!(
                "Sending reply to sender_tag {:?}",
                sender_tag.to_base58_string()
            );
            InputMessage::new_reply(sender_tag.clone(), data, TransmissionLane::General, None)
        }
        // Regular message with recipient, include SURBs for reply capability
        (Some(recipient), None) => {
            debug!("Sending anonymous message to recipient {}", recipient);
            InputMessage::new_anonymous(
                *recipient,
                data,
                DEFAULT_REPLY_SURBS,
                TransmissionLane::General,
                None,
            )
        }
        // No recipient or sender_tag - cannot route
        (None, None) => {
            debug!("No recipient or sender_tag provided, cannot route message");
            return Err(Error::OutboundSendFailure(
                "No recipient or sender_tag provided".to_string(),
            ));
        }
    };

    // Send via the client input
    let mut client = client_input.write().await;
    client
        .input_sender
        .send(input_msg)
        .await
        .map_err(|_| Error::OutboundSendFailure("InputMessageReceiver stopped".to_string()))
}

/// Log outbound message details for debugging.
fn log_outbound_message(message: &OutboundMessage) {
    match &message.message {
        Message::TransportMessage(tm) => match &tm.message.message_type {
            SubstreamMessageType::OpenResponse => {
                debug!(
                    "Outbound OpenResponse: nonce={}, substream={:?}",
                    tm.nonce, tm.message.substream_id
                );
            }
            SubstreamMessageType::OpenRequest => {
                debug!(
                    "Outbound OpenRequest: nonce={}, substream={:?}",
                    tm.nonce, tm.message.substream_id
                );
            }
            SubstreamMessageType::Data(_) => {
                debug!(
                    "Outbound Data: nonce={}, substream={:?}",
                    tm.nonce, tm.message.substream_id
                );
            }
            SubstreamMessageType::Close => {
                debug!(
                    "Outbound Close: nonce={}, substream={:?}",
                    tm.nonce, tm.message.substream_id
                );
            }
        },
        Message::ConnectionRequest(_) => debug!("Outbound ConnectionRequest"),
        Message::ConnectionResponse(_) => debug!("Outbound ConnectionResponse"),
    }
}

/// Background task that forwards inbound messages from the mixnet to the transport.
async fn run_inbound_loop(
    stream: Arc<RwLock<NymClientStream>>,
    inbound_tx: UnboundedSender<InboundMessage>,
    notify_inbound_tx: Option<UnboundedSender<()>>,
) {
    loop {
        // Lock the stream to poll for next message
        let msg = {
            let mut stream_guard = stream.write().await;
            stream_guard.next().await
        };

        match msg {
            Some(reconstructed_msg) => {
                // Notify if requested
                if let Some(ref notify_tx) = notify_inbound_tx {
                    let _ = notify_tx.unbounded_send(());
                }

                let (message_bytes, sender_tag) = reconstructed_msg.into_inner();
                match parse_message_data(&message_bytes, sender_tag) {
                    Ok(data) => {
                        if inbound_tx.unbounded_send(data).is_err() {
                            debug!("Inbound channel closed");
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("Failed to parse inbound message: {:?}", e);
                    }
                }
            }
            None => {
                debug!("Inbound stream ended");
                break;
            }
        }
    }

    debug!("Inbound message loop ended");
}
