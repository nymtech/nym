//! Stream multiplexing for `MixnetClient`.
//!
//! A [`MixnetStream`] is a byte channel (`AsyncRead + AsyncWrite`) to a
//! remote peer, identified by a [`StreamId`]. A single `MixnetClient`
//! can hold many streams to different peers concurrently.
//!
//! A background router task reads the client's `reconstructed_receiver`,
//! parses the stream header, and dispatches each payload to the right
//! stream's channel (or to the listener for `Open` messages).

mod protocol;
#[allow(clippy::module_inception)]
mod stream;

pub use protocol::StreamId;
pub use stream::MixnetStream;

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{trace, warn};

use nym_client_core::client::inbound_messages::InputMessage;
use nym_client_core::client::received_buffer::ReconstructedMessagesReceiver;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_task::connections::TransmissionLane;

use protocol::{decode_stream_message, encode_stream_message, StreamMessageType};

use crate::mixnet::native_client::MixnetClient;
use crate::{Error, Result};

/// The shared stream routing table. The router task reads it to dispatch
/// incoming data; streams insert/remove themselves directly.
pub(crate) type StreamMap = Arc<Mutex<HashMap<StreamId, mpsc::UnboundedSender<Vec<u8>>>>>;

/// Delivered to the listener when a remote peer opens a new stream.
struct InboundOpen {
    stream_id: StreamId,
    sender_tag: Option<AnonymousSenderTag>,
    initial_data: Vec<u8>,
}

/// Owns the router task and the shared state for all streams on a client.
/// The router is a background task that reads reconstructed messages from the
/// mixnet, decodes the stream header, and dispatches each payload to the
/// correct stream's channel (or to the listener for new `Open` messages).
pub(crate) struct StreamState {
    streams: StreamMap,
    listener_rx: Option<mpsc::UnboundedReceiver<InboundOpen>>,
    shutdown: CancellationToken,
    _router_handle: tokio::task::JoinHandle<()>,
}

impl Drop for StreamState {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

/// Accepts inbound streams opened by remote peers.
///
/// Created via [`MixnetClient::listener`]. Each `accept()` returns a
/// `MixnetStream` ready for reading and writing.
pub struct MixnetListener {
    inbound_rx: mpsc::UnboundedReceiver<InboundOpen>,
    client_input: nym_client_core::client::base_client::ClientInput,
    packet_type: Option<nym_sphinx::params::PacketType>,
    streams: StreamMap,
}

impl MixnetListener {
    /// Wait for a remote peer to open a stream.
    ///
    /// Returns `None` if the router has shut down.
    pub async fn accept(&mut self) -> Option<MixnetStream> {
        loop {
            let req = self.inbound_rx.recv().await?;

            let sender_tag = match req.sender_tag {
                Some(tag) => tag,
                None => {
                    warn!(
                        "Listener: Open for {} has no sender_tag, skipping",
                        req.stream_id
                    );
                    continue;
                }
            };

            let (tx, rx) = mpsc::unbounded_channel();
            self.streams
                .lock()
                .expect("stream map poisoned")
                .insert(req.stream_id, tx);

            return Some(MixnetStream::new_inbound(
                req.stream_id,
                sender_tag,
                self.client_input.clone(),
                self.packet_type,
                self.streams.clone(),
                rx,
                req.initial_data,
            ));
        }
    }
}

/// Background loop that demuxes incoming mixnet messages into per-stream channels.
async fn run_router(
    mut reconstructed_rx: ReconstructedMessagesReceiver,
    streams: StreamMap,
    listener_tx: mpsc::UnboundedSender<InboundOpen>,
    shutdown: CancellationToken,
) {
    loop {
        let messages = tokio::select! {
            _ = shutdown.cancelled() => break,
            msg = reconstructed_rx.next() => match msg {
                Some(messages) => messages,
                None => break,
            },
        };

        for msg in messages {
            let Some((stream_id, msg_type, payload)) = decode_stream_message(&msg.message) else {
                trace!(
                    "Router: non-stream message ({} bytes), dropping",
                    msg.message.len()
                );
                continue;
            };

            match msg_type {
                StreamMessageType::Open => {
                    let _ = listener_tx.send(InboundOpen {
                        stream_id,
                        sender_tag: msg.sender_tag,
                        initial_data: payload.to_vec(),
                    });
                }
                StreamMessageType::Data => {
                    let mut map = streams.lock().expect("stream map poisoned");
                    let should_remove = map
                        .get(&stream_id)
                        .map(|tx| tx.send(payload.to_vec()).is_err())
                        .unwrap_or(false);
                    if should_remove {
                        map.remove(&stream_id);
                    }
                } // MAX TODO if we decide we need close logic add another enum member
            }
        }
    }
}

/// Lazily initialise the stream subsystem and router on first use.
fn ensure_init(client: &mut MixnetClient) -> &mut StreamState {
    if client.streams.is_none() {
        client.stream_mode.store(true, Ordering::SeqCst);

        let (_, dummy_rx) = futures::channel::mpsc::unbounded();
        let real_rx = std::mem::replace(&mut client.reconstructed_receiver, dummy_rx);

        let streams: StreamMap = Arc::new(Mutex::new(HashMap::new()));
        let (listener_tx, listener_rx) = mpsc::unbounded_channel();
        let shutdown = CancellationToken::new();

        let router_handle = tokio::spawn(run_router(
            real_rx,
            streams.clone(),
            listener_tx,
            shutdown.clone(),
        ));

        client.streams = Some(StreamState {
            streams,
            listener_rx: Some(listener_rx),
            shutdown,
            _router_handle: router_handle,
        });
    }
    client.streams.as_mut().unwrap()
}

/// Open a stream to a remote peer.
pub(crate) async fn open_stream(
    client: &mut MixnetClient,
    recipient: Recipient,
    reply_surbs: u32,
) -> Result<MixnetStream> {
    let streams = ensure_init(client).streams.clone();

    let stream_id = StreamId::random();
    let (tx, rx) = mpsc::unbounded_channel();
    streams
        .lock()
        .expect("stream map poisoned")
        .insert(stream_id, tx);

    // Send Open to the peer
    let wire = encode_stream_message(&stream_id, StreamMessageType::Open, &[]);
    let msg = InputMessage::new_anonymous(
        recipient,
        wire,
        reply_surbs,
        TransmissionLane::General,
        client.packet_type,
    );
    client.client_input.send(msg).await.map_err(|_| {
        streams
            .lock()
            .expect("stream map poisoned")
            .remove(&stream_id);
        Error::MessageSendingFailure
    })?;

    Ok(MixnetStream::new_outbound(
        stream_id,
        recipient,
        reply_surbs,
        client.client_input.clone(),
        client.packet_type,
        streams,
        rx,
    ))
}

/// Create a listener that accepts inbound streams. Can only be called once.
pub(crate) fn listener(client: &mut MixnetClient) -> Result<MixnetListener> {
    let state = ensure_init(client);
    let listener_rx = state
        .listener_rx
        .take()
        .ok_or(Error::ListenerAlreadyTaken)?;
    let streams = state.streams.clone();

    Ok(MixnetListener {
        inbound_rx: listener_rx,
        client_input: client.client_input.clone(),
        packet_type: client.packet_type,
        streams,
    })
}
