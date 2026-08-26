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
//!
//! See the [tutorial](https://nymtech.net/docs/developers/rust/stream/tutorial)
//! for a step-by-step walkthrough.
//!
#![doc = include_str!("ARCHITECTURE.md")]

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

use nym_lp_data::packet::frame::SphinxStreamMsgType;
use protocol::{decode_stream_message, encode_stream_message};

use crate::mixnet::native_client::MixnetClient;
use crate::{Error, Result};

/// Default idle timeout before a stream is considered stale and cleaned up.
pub(crate) const DEFAULT_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Maximum interval between stale-stream checks. The actual check interval
/// is `min(idle_timeout, MAX_CLEANUP_INTERVAL)` so that short idle timeouts
/// are respected promptly rather than waiting up to 60 s for the next sweep.
const MAX_CLEANUP_INTERVAL: Duration = Duration::from_secs(10);

/// A stream failure, sent through the data channel so it arrives in
/// order with the data around it. `recv()` returns it once and keeps
/// delivering later messages; `AsyncRead` fails the stream for good.
#[derive(Debug, Clone, Copy)]
pub(crate) enum StreamFailure {
    /// The reorder buffer overflowed and skipped past missing messages.
    DataLoss,
}

impl StreamFailure {
    pub(crate) fn as_io_error(self) -> std::io::Error {
        match self {
            StreamFailure::DataLoss => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stream data lost: reorder buffer overflow skipped missing messages",
            ),
        }
    }
}

/// Per-stream state stored in the routing table.
///
/// Reorder buffer uses the same BTreeMap pattern as `OrderedMessageBuffer`
/// (`common/socks5/ordered-buffer/`) but drains per-message instead of
/// concatenating, so `recv()` preserves message boundaries.
struct StreamEntry {
    sender: mpsc::UnboundedSender<Result<Vec<u8>, StreamFailure>>,
    last_activity: Instant,
    next_seq: u32,
    pending: BTreeMap<u32, Vec<u8>>,
    /// Total payload bytes in `pending`; makes the overflow check O(1).
    pending_bytes: usize,
}

impl StreamEntry {
    /// Flush the contiguous prefix of `pending` to the stream's channel.
    /// Returns true if the receiver has been dropped.
    fn drain_ready(&mut self) -> bool {
        while let Some(msg) = self.pending.remove(&self.next_seq) {
            self.pending_bytes -= msg.len();
            if self.sender.send(Ok(msg)).is_err() {
                return true;
            }
            self.next_seq += 1;
        }
        false
    }
}

/// The mixnet routes every message independently, so a stream's first `Data`
/// frames can overtake the `Open` that creates the stream and arrive before
/// the stream is registered. Such orphan frames are buffered briefly and
/// drained into the stream's reorder buffer on registration, instead of
/// being silently dropped.
struct OrphanEntry {
    first_seen: Instant,
    pending: BTreeMap<u32, Vec<u8>>,
}

/// How long orphan frames are kept while waiting for their stream to be
/// registered. The Open/Data race window is milliseconds wide; anything
/// older belongs to a stream that will never be accepted. (The cleanup
/// sweep runs every [`MAX_CLEANUP_INTERVAL`], so effective retention is
/// up to `ORPHAN_TTL + MAX_CLEANUP_INTERVAL`.)
const ORPHAN_TTL: Duration = Duration::from_secs(5);
/// Maximum number of distinct unregistered streams to buffer frames for.
const MAX_ORPHAN_STREAMS: usize = 64;
/// Maximum frames buffered per orphan stream.
const MAX_ORPHAN_MESSAGES: usize = 32;

/// Maximum bytes of out-of-order messages buffered per stream before we
/// skip ahead. Without this cap, a malicious sender that deliberately skips
/// a sequence number (e.g. never sends seq 1) could cause the buffer to
/// grow indefinitely while the drain loop waits for the missing seq.
/// The idle timeout only reaps *inactive* streams, so an actively-sending
/// attacker would bypass it.
///
/// Sized so a late frame with a retransmit in flight cannot trip it:
/// 8 MiB is minutes of buffering at per-tunnel throughput, against
/// second-scale retransmits. A skip means real loss, and a skip fails
/// the stream (see [`StreamFailure`]).
const MAX_REORDER_BUFFER_BYTES: usize = 8 * 1024 * 1024;

/// The stream and orphan-frame tables, always locked together.
struct StreamMapInner {
    streams: HashMap<StreamId, StreamEntry>,
    orphans: HashMap<StreamId, OrphanEntry>,
}

/// The shared stream routing table.
///
/// Wraps the map of active streams behind an async mutex with focused
/// methods so callers never touch the lock directly.
#[derive(Clone)]
pub(crate) struct StreamMap {
    inner: Arc<tokio::sync::Mutex<StreamMapInner>>,
}

impl StreamMap {
    fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(StreamMapInner {
                streams: HashMap::new(),
                orphans: HashMap::new(),
            })),
        }
    }

    /// Register a new stream, returning the receiver end of its data channel.
    /// Any orphan frames that arrived before registration are drained into
    /// the stream immediately. Returns `None` if a stream with this id is
    /// already active (a duplicate `Open`): replacing the existing entry
    /// would close its reader and let the old handle's `Drop` deregister
    /// the replacement.
    async fn register_stream(
        &self,
        stream_id: StreamId,
    ) -> Option<mpsc::UnboundedReceiver<Result<Vec<u8>, StreamFailure>>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut inner = self.inner.lock().await;
        if inner.streams.contains_key(&stream_id) {
            return None;
        }
        let pending = inner
            .orphans
            .remove(&stream_id)
            .map(|orphan| orphan.pending)
            .unwrap_or_default();
        let pending_bytes = pending.values().map(Vec::len).sum();
        let mut entry = StreamEntry {
            sender: tx,
            last_activity: Instant::now(),
            next_seq: 0,
            pending,
            pending_bytes,
        };
        // The receiver cannot have been dropped yet - we still hold it.
        entry.drain_ready();
        inner.streams.insert(stream_id, entry);
        Some(rx)
    }

    /// Remove a stream from the map, along with any orphan frames held for
    /// its id, so post-close stragglers cannot occupy orphan slots.
    async fn remove(&self, stream_id: &StreamId) {
        let mut inner = self.inner.lock().await;
        inner.streams.remove(stream_id);
        inner.orphans.remove(stream_id);
    }

    /// Remove a stream without awaiting: for use in `Drop` and `poll_shutdown`
    /// where we cannot `.await`. Spawns a lightweight background task.
    fn remove_background(&self, stream_id: StreamId) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut inner = inner.lock().await;
            inner.streams.remove(&stream_id);
            inner.orphans.remove(&stream_id);
        });
    }

    /// Buffer a message and flush any contiguous sequence to the channel.
    /// Updates `last_activity` on success; removes the entry if the
    /// receiver has been dropped. Messages for streams that are not (yet)
    /// registered are held in the orphan buffer until registration.
    async fn send_to_stream(&self, stream_id: &StreamId, seq: u32, data: Vec<u8>) {
        let mut inner = self.inner.lock().await;
        let Some(entry) = inner.streams.get_mut(stream_id) else {
            Self::buffer_orphan(&mut inner.orphans, *stream_id, seq, data);
            return;
        };

        if seq < entry.next_seq {
            warn!(
                "Stream {stream_id}: dropping old seq {seq} (expected >= {})",
                entry.next_seq
            );
        } else {
            entry.pending_bytes += data.len();
            if let Some(replaced) = entry.pending.insert(seq, data) {
                // Duplicate seq: the replaced payload leaves the buffer.
                entry.pending_bytes -= replaced.len();
            }
        }

        // Over the cap: skip ahead to the lowest buffered seq and report
        // the discarded range in-band. `lowest == next_seq` means the
        // arriving frame filled the gap; everything drains below, so no
        // skip and no failure.
        if entry.pending_bytes > MAX_REORDER_BUFFER_BYTES {
            let lowest = entry
                .pending
                .keys()
                .next()
                .copied()
                .unwrap_or(entry.next_seq);
            if lowest > entry.next_seq {
                warn!(
                    "Stream {stream_id}: reorder buffer overflow ({} messages, \
                     {} bytes pending), skipping seq {} -> {lowest}",
                    entry.pending.len(),
                    entry.pending_bytes,
                    entry.next_seq
                );
                let _ = entry.sender.send(Err(StreamFailure::DataLoss));
                entry.next_seq = lowest;
            }
        }

        let receiver_dropped = entry.drain_ready();
        if receiver_dropped {
            inner.streams.remove(stream_id);
        } else {
            entry.last_activity = Instant::now();
        }
    }

    /// Hold a frame for a stream that has not been registered yet. Bounded
    /// in three dimensions: distinct streams, frames per stream, and age
    /// (swept by [`Self::cleanup_stale`] after [`ORPHAN_TTL`]).
    fn buffer_orphan(
        orphans: &mut HashMap<StreamId, OrphanEntry>,
        stream_id: StreamId,
        seq: u32,
        data: Vec<u8>,
    ) {
        if !orphans.contains_key(&stream_id) && orphans.len() >= MAX_ORPHAN_STREAMS {
            // Evict the oldest orphan: recent frames are the ones whose
            // Open is most likely still in flight.
            if let Some(oldest) = orphans
                .iter()
                .min_by_key(|(_, orphan)| orphan.first_seen)
                .map(|(id, _)| *id)
            {
                warn!("Orphan buffer full, evicting frames for stream {oldest}");
                orphans.remove(&oldest);
            }
        }
        let entry = orphans.entry(stream_id).or_insert_with(|| OrphanEntry {
            first_seen: Instant::now(),
            pending: BTreeMap::new(),
        });
        if entry.pending.len() < MAX_ORPHAN_MESSAGES {
            trace!("Stream {stream_id}: buffering seq {seq} until registration");
            entry.pending.insert(seq, data);
        } else {
            warn!("Stream {stream_id}: orphan buffer full, dropping seq {seq}");
        }
    }

    /// Remove streams that have been idle longer than `max_idle`, and orphan
    /// frames whose stream was never registered within [`ORPHAN_TTL`].
    async fn cleanup_stale(&self, max_idle: Duration) {
        let now = Instant::now();
        let mut inner = self.inner.lock().await;
        inner.streams.retain(|id, entry| {
            let stale = now.duration_since(entry.last_activity) >= max_idle;
            if stale {
                trace!("Cleaning up stale stream {id} (idle > {max_idle:?})");
            }
            !stale
        });
        inner.orphans.retain(|id, orphan| {
            let stale = now.duration_since(orphan.first_seen) >= ORPHAN_TTL;
            if stale {
                trace!("Cleaning up orphan frames for never-registered stream {id}");
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
///
/// Only one `MixnetListener` can exist per client; a second call to
/// `listener()` returns [`Error::ListenerAlreadyTaken`].
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
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. If cancelled before a stream arrives,
    /// the pending `Open` message remains in the channel for the next call.
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

            let Some(rx) = self.streams.register_stream(req.stream_id).await else {
                warn!(
                    "Listener: duplicate Open for active stream {}, ignoring",
                    req.stream_id
                );
                continue;
            };

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
    // Fail at dial time if the recipient's gateway is not in topology;
    // otherwise the Open dies in the send task with only a warn log. The
    // empty check separates "our view is gone" from "unknown gateway".
    {
        let permit = client
            .client_state
            .topology_accessor
            .get_read_permit()
            .await;
        permit
            .topology
            .ensure_not_empty()
            .and_then(|()| permit.egress_by_identity(recipient.gateway()).map(|_| ()))
    }
    .map_err(|source| Error::UnroutableRecipient {
        recipient: Box::new(recipient),
        source,
    })?;

    let streams = ensure_init(client)?.streams.clone();

    // Random ids make collisions vanishingly unlikely, but regenerate on
    // the off chance rather than clobbering an active stream.
    let (stream_id, rx) = loop {
        let stream_id = StreamId::random();
        if let Some(rx) = streams.register_stream(stream_id).await {
            break (stream_id, rx);
        }
    };

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
        let _rx_a = map
            .register_stream(StreamId::random())
            .await
            .expect("fresh stream id");
        let _rx_b = map
            .register_stream(StreamId::random())
            .await
            .expect("fresh stream id");

        // Advance time past the timeout
        tokio::time::advance(timeout + Duration::from_secs(1)).await;

        // Register a fresh stream (should survive cleanup)
        let id_c = StreamId::random();
        let _rx_c = map.register_stream(id_c).await.expect("fresh stream id");

        map.cleanup_stale(timeout).await;

        let inner = map.inner.lock().await;
        assert_eq!(inner.streams.len(), 1);
        assert!(inner.streams.contains_key(&id_c));
    }

    #[tokio::test(start_paused = true)]
    async fn send_to_stream_updates_last_activity() {
        let map = StreamMap::new();
        let timeout = Duration::from_secs(10);
        let id = StreamId::random();

        let _rx = map.register_stream(id).await.expect("fresh stream id");

        // Advance most of the way through the timeout
        tokio::time::advance(Duration::from_secs(8)).await;

        // Activity on the stream resets its timer
        map.send_to_stream(&id, 0, vec![1, 2, 3]).await;

        // Advance past the original timeout, but only 5s since last activity
        tokio::time::advance(Duration::from_secs(5)).await;

        map.cleanup_stale(timeout).await;

        // Stream should survive: last activity was 5s ago, not 13s
        assert_eq!(map.inner.lock().await.streams.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_does_not_remove_active_streams() {
        let map = StreamMap::new();
        let timeout = Duration::from_secs(10);

        let id = StreamId::random();
        let _rx = map.register_stream(id).await.expect("fresh stream id");

        // Advance less than the timeout
        tokio::time::advance(Duration::from_secs(5)).await;

        map.cleanup_stale(timeout).await;

        assert_eq!(map.inner.lock().await.streams.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn stale_orphans_are_swept() {
        let map = StreamMap::new();
        let id = StreamId::random();

        map.send_to_stream(&id, 0, vec![1]).await;

        tokio::time::advance(ORPHAN_TTL + Duration::from_secs(1)).await;
        map.cleanup_stale(Duration::from_secs(600)).await;

        // The orphan was swept: registering now delivers nothing.
        let mut rx = map.register_stream(id).await.expect("fresh stream id");
        assert!(rx.try_recv().is_err());
        assert!(map.inner.lock().await.orphans.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn orphan_streams_are_bounded_evicting_oldest() {
        let map = StreamMap::new();

        let oldest = StreamId::random();
        map.send_to_stream(&oldest, 0, vec![0]).await;
        // Later arrivals get distinct, younger timestamps.
        tokio::time::advance(Duration::from_millis(10)).await;

        for _ in 1..MAX_ORPHAN_STREAMS {
            map.send_to_stream(&StreamId::random(), 0, vec![0]).await;
        }
        tokio::time::advance(Duration::from_millis(10)).await;

        // One over capacity: the oldest orphan makes room for the newest.
        let newest = StreamId::random();
        map.send_to_stream(&newest, 0, vec![7]).await;

        let inner = map.inner.lock().await;
        assert_eq!(inner.orphans.len(), MAX_ORPHAN_STREAMS);
        assert!(!inner.orphans.contains_key(&oldest));
        assert!(inner.orphans.contains_key(&newest));
    }

    #[tokio::test]
    async fn duplicate_registration_is_rejected_keeping_original() {
        let map = StreamMap::new();
        let id = StreamId::random();

        let mut rx = map.register_stream(id).await.expect("fresh stream id");
        // A duplicate Open for an active stream must not clobber the entry.
        assert!(map.register_stream(id).await.is_none());

        // The original stream still receives data.
        map.send_to_stream(&id, 0, vec![1]).await;
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![1]);
    }

    #[tokio::test]
    async fn remove_clears_orphan_entry() {
        let map = StreamMap::new();
        let id = StreamId::random();

        // A frame arriving with no registered stream creates an orphan...
        map.send_to_stream(&id, 0, vec![1]).await;
        // ...which removal must clear along with the stream itself.
        map.remove(&id).await;

        assert!(map.inner.lock().await.orphans.is_empty());
    }

    #[tokio::test]
    async fn orphan_frames_per_stream_are_bounded() {
        let map = StreamMap::new();
        let id = StreamId::random();

        for seq in 0..(MAX_ORPHAN_MESSAGES as u32 + 10) {
            map.send_to_stream(&id, seq, vec![0]).await;
        }

        let buffered = map.inner.lock().await.orphans[&id].pending.len();
        assert_eq!(buffered, MAX_ORPHAN_MESSAGES);
    }

    #[tokio::test]
    async fn out_of_order_messages_delivered_in_sequence() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let mut rx = map.register_stream(id).await.expect("fresh stream id");

        // Send seq 2, 0, 1 out of order
        map.send_to_stream(&id, 2, vec![20]).await;
        map.send_to_stream(&id, 0, vec![0]).await;

        // seq 0 should be delivered now, but 2 is buffered (gap at 1)
        assert_eq!(rx.recv().await.unwrap().unwrap(), vec![0]);

        // Fill the gap: both 1 and 2 should flush
        map.send_to_stream(&id, 1, vec![10]).await;
        assert_eq!(rx.recv().await.unwrap().unwrap(), vec![10]);
        assert_eq!(rx.recv().await.unwrap().unwrap(), vec![20]);
    }

    #[tokio::test]
    async fn data_before_registration_is_buffered_until_open() {
        let map = StreamMap::new();
        let id = StreamId::random();

        // The mixnet routes every message independently, so a stream's first
        // Data frame can overtake the Open that creates it and arrive before
        // the stream is registered. It must be buffered, not dropped.
        map.send_to_stream(&id, 0, vec![42]).await;

        let mut rx = map.register_stream(id).await.expect("fresh stream id");
        assert_eq!(
            rx.try_recv()
                .expect("early data delivered on registration")
                .unwrap(),
            vec![42]
        );
    }

    #[tokio::test]
    async fn early_data_is_drained_in_sequence_on_registration() {
        let map = StreamMap::new();
        let id = StreamId::random();

        // Multiple frames arrive out of order before registration.
        map.send_to_stream(&id, 1, vec![10]).await;
        map.send_to_stream(&id, 0, vec![0]).await;

        let mut rx = map.register_stream(id).await.expect("fresh stream id");
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![0]);
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![10]);
    }

    #[tokio::test]
    async fn duplicate_seq_is_dropped() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let mut rx = map.register_stream(id).await.expect("fresh stream id");

        map.send_to_stream(&id, 0, vec![0]).await;
        map.send_to_stream(&id, 0, vec![99]).await; // duplicate, dropped
        map.send_to_stream(&id, 1, vec![1]).await;

        assert_eq!(rx.recv().await.unwrap().unwrap(), vec![0]);
        assert_eq!(rx.recv().await.unwrap().unwrap(), vec![1]);
    }

    #[tokio::test]
    async fn reorder_overflow_signals_data_loss_in_order() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let mut rx = map.register_stream(id).await.expect("fresh stream id");
        let chunk = MAX_REORDER_BUFFER_BYTES / 8;

        // seq 0 arrives and is delivered; seq 1 is lost in the mixnet.
        map.send_to_stream(&id, 0, vec![0]).await;

        // Frames pile up behind the gap: 2..=9 reach the cap exactly,
        // seq 10 tips the buffer over it.
        for seq in 2..=10 {
            map.send_to_stream(&id, seq, vec![1; chunk]).await;
        }

        // Reader sees the intact prefix, then the loss, then the post-gap
        // frames that the skip drained.
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![0]);
        assert!(matches!(
            rx.try_recv().unwrap(),
            Err(StreamFailure::DataLoss)
        ));
        for _ in 2..=10 {
            assert!(rx.try_recv().unwrap().is_ok());
        }
        // Outer Err: the channel is drained, not a stream failure.
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn late_gap_filler_at_capacity_is_not_data_loss() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let mut rx = map.register_stream(id).await.expect("fresh stream id");
        let chunk = MAX_REORDER_BUFFER_BYTES / 8;

        // seq 0 delivered; seq 1 is late; 2..=9 fill the buffer to the cap.
        map.send_to_stream(&id, 0, vec![0]).await;
        for seq in 2..=9 {
            map.send_to_stream(&id, seq, vec![1; chunk]).await;
        }
        // The late gap-filler tips the buffer over the cap, but nothing was
        // lost: everything drains and no failure is reported.
        map.send_to_stream(&id, 1, vec![1; chunk]).await;

        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![0]);
        for _ in 1..=9 {
            assert!(rx.try_recv().unwrap().is_ok());
        }
        // Outer Err: the channel is drained, not a stream failure.
        assert!(rx.try_recv().is_err());
    }
}
