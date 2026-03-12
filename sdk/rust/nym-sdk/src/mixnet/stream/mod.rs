//! Stream multiplexing for `MixnetClient`.
//!
//! A [`MixnetStream`] is a byte channel (`AsyncRead + AsyncWrite`) to a
//! remote peer, identified by a [`StreamId`]. A single `MixnetClient`
//! can hold many streams to different peers concurrently.
//!
//! A background router task reads the client's `reconstructed_receiver`,
//! parses the stream header, and dispatches each payload to the right
//! stream's channel (or to the listener for `Open` messages).

mod mixnet_stream;
mod protocol;

pub use mixnet_stream::MixnetStream;
pub use protocol::StreamId;

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

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

/// The shared stream routing table.
///
/// Wraps the map of active streams behind an async mutex with focused
/// methods so callers never touch the lock directly.
#[derive(Clone)]
pub(crate) struct StreamMap {
    inner: Arc<tokio::sync::Mutex<HashMap<StreamId, mpsc::UnboundedSender<Vec<u8>>>>>,
}

impl StreamMap {
    fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Register a new stream, returning the receiver end of its data channel.
    async fn register_stream(&self, stream_id: StreamId) -> mpsc::UnboundedReceiver<Vec<u8>> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.inner.lock().await.insert(stream_id, tx);
        rx
    }

    /// Remove a stream from the map.
    async fn remove(&self, stream_id: &StreamId) {
        self.inner.lock().await.remove(stream_id);
    }

    /// Remove a stream without awaiting — for use in `Drop` and `poll_shutdown`
    /// where we cannot `.await`. Spawns a lightweight background task.
    fn remove_background(&self, stream_id: StreamId) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            inner.lock().await.remove(&stream_id);
        });
    }

    /// Dispatch data to a stream's channel. Removes the entry if the
    /// receiver has been dropped.
    async fn send_to_stream(&self, stream_id: &StreamId, data: Vec<u8>) {
        let mut map = self.inner.lock().await;
        let should_remove = map
            .get(stream_id)
            .map(|tx| tx.send(data).is_err())
            .unwrap_or(false);
        if should_remove {
            map.remove(stream_id);
        }
    }
}

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

            let rx = self.streams.register_stream(req.stream_id).await;

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
            let Some(frame) = decode_stream_message(&msg.message) else {
                trace!(
                    "Router: non-stream message ({} bytes), dropping",
                    msg.message.len()
                );
                continue;
            };

            let stream_id = frame.header.stream_id;
            match frame.header.message_type {
                StreamMessageType::Open => {
                    let _ = listener_tx.send(InboundOpen {
                        stream_id,
                        sender_tag: msg.sender_tag,
                        initial_data: frame.data.to_vec(),
                    });
                }
                StreamMessageType::Data => {
                    streams
                        .send_to_stream(&stream_id, frame.data.to_vec())
                        .await;
                } // TODO: if we decide we need close logic add another enum member
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

        let streams = StreamMap::new();
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
    let rx = streams.register_stream(stream_id).await;

    // Send Open to the peer
    let wire = encode_stream_message(&stream_id, StreamMessageType::Open, &[]);
    let msg = InputMessage::new_anonymous(
        recipient,
        wire,
        reply_surbs,
        TransmissionLane::General,
        client.packet_type,
    );
    if let Err(_) = client.client_input.send(msg).await {
        streams.remove(&stream_id).await;
        return Err(Error::MessageSendingFailure);
    }

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
