// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
pub(crate) mod protocol;

pub use mixnet_stream::MixnetStream;
pub use protocol::StreamId;

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{trace, warn};

use nym_client_core::client::inbound_messages::InputMessage;
use nym_client_core::client::received_buffer::ReconstructedMessagesReceiver;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_task::connections::TransmissionLane;

use nym_lp::packet::frame::SphinxStreamMsgType;
use protocol::{decode_stream_message, encode_stream_message};

use crate::mixnet::native_client::MixnetClient;
use crate::{Error, Result};

/// Default idle timeout before a stream is considered stale and cleaned up.
pub(crate) const DEFAULT_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Maximum interval between stale-stream checks. The actual check interval
/// is `min(idle_timeout, MAX_CLEANUP_INTERVAL)` so that short idle timeouts
/// are respected promptly rather than waiting up to 60 s for the next sweep.
const MAX_CLEANUP_INTERVAL: Duration = Duration::from_secs(10);

/// Per-stream state stored in the routing table.
///
/// Reorder buffer uses the same BTreeMap pattern as `OrderedMessageBuffer`
/// (`common/socks5/ordered-buffer/`) but drains per-message instead of
/// concatenating, so `recv()` preserves message boundaries.
struct StreamEntry {
    sender: mpsc::UnboundedSender<Vec<u8>>,
    last_activity: Instant,
    next_seq: u32,
    pending: BTreeMap<u32, Vec<u8>>,
}

/// The shared stream routing table.
///
/// Wraps the map of active streams behind an async mutex with focused
/// methods so callers never touch the lock directly.
#[derive(Clone)]
pub(crate) struct StreamMap {
    inner: Arc<tokio::sync::Mutex<HashMap<StreamId, StreamEntry>>>,
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
        self.inner.lock().await.insert(
            stream_id,
            StreamEntry {
                sender: tx,
                last_activity: Instant::now(),
                next_seq: 0,
                pending: BTreeMap::new(),
            },
        );
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

    /// Buffer a message and flush any contiguous sequence to the channel.
    /// Updates `last_activity` on success; removes the entry if the
    /// receiver has been dropped.
    async fn send_to_stream(&self, stream_id: &StreamId, seq: u32, data: Vec<u8>) {
        let mut map = self.inner.lock().await;
        let should_remove = if let Some(entry) = map.get_mut(stream_id) {
            if seq < entry.next_seq {
                warn!(
                    "Stream {stream_id}: dropping old seq {seq} (expected >= {})",
                    entry.next_seq
                );
            } else {
                entry.pending.insert(seq, data);
            }

            // Drain contiguous messages
            let mut failed = false;
            while let Some(msg) = entry.pending.remove(&entry.next_seq) {
                if entry.sender.send(msg).is_err() {
                    failed = true;
                    break;
                }
                entry.next_seq += 1;
            }

            if !failed {
                entry.last_activity = Instant::now();
            }
            failed
        } else {
            false
        };
        if should_remove {
            map.remove(stream_id);
        }
    }

    /// Remove streams that have been idle longer than `max_idle`.
    async fn cleanup_stale(&self, max_idle: Duration) {
        let now = Instant::now();
        let mut map = self.inner.lock().await;
        map.retain(|id, entry| {
            let stale = now.duration_since(entry.last_activity) >= max_idle;
            if stale {
                trace!("Cleaning up stale stream {id} (idle > {max_idle:?})");
            }
            !stale
        });
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
    idle_timeout: Duration,
) {
    let check_every = std::cmp::min(idle_timeout, MAX_CLEANUP_INTERVAL);
    let mut cleanup_interval = tokio::time::interval(check_every);
    cleanup_interval.tick().await; // consume the immediate first tick

    loop {
        let messages = tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = cleanup_interval.tick() => {
                streams.cleanup_stale(idle_timeout).await;
                continue;
            }
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

            let stream_id = frame.stream_id;
            match frame.msg_type {
                SphinxStreamMsgType::Open => {
                    let _ = listener_tx.send(InboundOpen {
                        stream_id,
                        sender_tag: msg.sender_tag,
                        initial_data: frame.data.to_vec(),
                    });
                }
                SphinxStreamMsgType::Data => {
                    streams
                        .send_to_stream(&stream_id, frame.sequence_num, frame.data.to_vec())
                        .await;
                }
            }
        }
    }
}

/// Lazily initialise the stream subsystem and router on first use.
fn ensure_init(client: &mut MixnetClient) -> Result<&mut StreamState> {
    if client.streams.is_none() {
        let real_rx = client
            .reconstructed_receiver
            .take()
            .ok_or(Error::StreamInitFailure)?;

        // Set after take() succeeds so we don't leave the client in a
        // broken state (stream_mode=true but no router) on failure.
        client.stream_mode.store(true, Ordering::SeqCst);

        let streams = StreamMap::new();
        let (listener_tx, listener_rx) = mpsc::unbounded_channel();
        let shutdown = CancellationToken::new();

        let router_handle = tokio::spawn(run_router(
            real_rx,
            streams.clone(),
            listener_tx,
            shutdown.clone(),
            client.stream_idle_timeout,
        ));

        client.streams = Some(StreamState {
            streams,
            listener_rx: Some(listener_rx),
            shutdown,
            _router_handle: router_handle,
        });
    }
    client.streams.as_mut().ok_or(Error::StreamInitFailure)
}

/// Open a stream to a remote peer.
pub(crate) async fn open_stream(
    client: &mut MixnetClient,
    recipient: Recipient,
    reply_surbs: u32,
) -> Result<MixnetStream> {
    let streams = ensure_init(client)?.streams.clone();

    let stream_id = StreamId::random();
    let rx = streams.register_stream(stream_id).await;

    // Open message with seq=0. The receiver's reorder buffer starts at
    // next_seq=0 so this could later carry an initial seq to resume a
    // dropped stream from where it left off.
    let wire = encode_stream_message(&stream_id, SphinxStreamMsgType::Open, 0, &[]);
    let msg = InputMessage::new_anonymous(
        recipient,
        wire,
        reply_surbs,
        TransmissionLane::General,
        client.packet_type,
    );
    if (client.client_input.send(msg).await).is_err() {
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
    let state = ensure_init(client)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn cleanup_stale_removes_idle_streams() {
        let map = StreamMap::new();
        let timeout = Duration::from_secs(10);

        // Register two streams
        let _rx_a = map.register_stream(StreamId::random()).await;
        let _rx_b = map.register_stream(StreamId::random()).await;

        // Advance time past the timeout
        tokio::time::advance(timeout + Duration::from_secs(1)).await;

        // Register a fresh stream (should survive cleanup)
        let id_c = StreamId::random();
        let _rx_c = map.register_stream(id_c).await;

        map.cleanup_stale(timeout).await;

        let inner = map.inner.lock().await;
        assert_eq!(inner.len(), 1);
        assert!(inner.contains_key(&id_c));
    }

    #[tokio::test(start_paused = true)]
    async fn send_to_stream_updates_last_activity() {
        let map = StreamMap::new();
        let timeout = Duration::from_secs(10);
        let id = StreamId::random();

        let _rx = map.register_stream(id).await;

        // Advance most of the way through the timeout
        tokio::time::advance(Duration::from_secs(8)).await;

        // Activity on the stream resets its timer
        map.send_to_stream(&id, 0, vec![1, 2, 3]).await;

        // Advance past the original timeout, but only 5s since last activity
        tokio::time::advance(Duration::from_secs(5)).await;

        map.cleanup_stale(timeout).await;

        // Stream should survive — last activity was 5s ago, not 13s
        assert_eq!(map.inner.lock().await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_does_not_remove_active_streams() {
        let map = StreamMap::new();
        let timeout = Duration::from_secs(10);

        let id = StreamId::random();
        let _rx = map.register_stream(id).await;

        // Advance less than the timeout
        tokio::time::advance(Duration::from_secs(5)).await;

        map.cleanup_stale(timeout).await;

        assert_eq!(map.inner.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn out_of_order_messages_delivered_in_sequence() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let mut rx = map.register_stream(id).await;

        // Send seq 2, 0, 1 out of order
        map.send_to_stream(&id, 2, vec![20]).await;
        map.send_to_stream(&id, 0, vec![0]).await;

        // seq 0 should be delivered now, but 2 is buffered (gap at 1)
        assert_eq!(rx.recv().await.unwrap(), vec![0]);

        // Fill the gap — both 1 and 2 should flush
        map.send_to_stream(&id, 1, vec![10]).await;
        assert_eq!(rx.recv().await.unwrap(), vec![10]);
        assert_eq!(rx.recv().await.unwrap(), vec![20]);
    }

    #[tokio::test]
    async fn duplicate_seq_is_dropped() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let mut rx = map.register_stream(id).await;

        map.send_to_stream(&id, 0, vec![0]).await;
        map.send_to_stream(&id, 0, vec![99]).await; // duplicate, dropped
        map.send_to_stream(&id, 1, vec![1]).await;

        assert_eq!(rx.recv().await.unwrap(), vec![0]);
        assert_eq!(rx.recv().await.unwrap(), vec![1]);
    }
}
