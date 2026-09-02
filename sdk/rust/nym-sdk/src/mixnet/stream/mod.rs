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
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::{trace, warn};

use nym_client_core::client::base_client::ClientInput;
use nym_client_core::client::inbound_messages::InputMessage;
use nym_client_core::client::received_buffer::ReconstructedMessagesReceiver;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::params::PacketType;
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
pub(crate) const MAX_CLEANUP_INTERVAL: Duration = Duration::from_secs(10);

/// Default interval between keepalive pings on an idle outbound stream.
/// Streams with inbound traffic inside the interval are never pinged.
pub(crate) const DEFAULT_PING_INTERVAL: Duration = Duration::from_secs(60);

/// Default number of consecutive unanswered pings before an armed stream
/// fails with [`StreamFailure::PeerUnresponsive`].
pub(crate) const DEFAULT_MISSED_PONGS_THRESHOLD: u32 = 3;

/// Reply SURBs attached to each keepalive ping: more than the single SURB
/// the pong consumes, so an idle stream does not deplete the peer's pool,
/// and few enough to keep the ping a single Sphinx packet.
pub(crate) const PING_SURBS: u32 = 2;
/// A stream failure, sent through the data channel so it arrives in
/// order with the data around it. `recv()` returns it once and keeps
/// delivering later messages; `AsyncRead` fails the stream for good.
#[derive(Debug, Clone, Copy)]
pub(crate) enum StreamFailure {
    /// The reorder buffer overflowed and skipped past missing messages.
    DataLoss,
    /// The peer stopped answering keepalive pings.
    PeerUnresponsive,
}

impl StreamFailure {
    pub(crate) fn as_io_error(self) -> std::io::Error {
        match self {
            StreamFailure::DataLoss => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stream data lost: reorder buffer overflow skipped missing messages",
            ),
            StreamFailure::PeerUnresponsive => std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "stream peer unresponsive: keepalive pings unanswered",
            ),
        }
    }
}

/// Where liveness frames for a stream are sent. Outbound streams address
/// the peer directly; inbound streams reply through the dialer's SURBs.
#[derive(Clone)]
pub(crate) enum StreamPeer {
    Address(Box<Recipient>),
    SenderTag(AnonymousSenderTag),
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

    /// Flips to true when the peer acknowledges the stream (or, for
    /// inbound streams, when we accept it ourselves).
    established_tx: watch::Sender<bool>,
    /// Destination for pings and pongs. `None` in unit tests only.
    peer: Option<StreamPeer>,
    /// True once the peer has sent any liveness frame (OpenAck, Ping or
    /// Pong), proving it speaks the extension. Keepalive acts only on
    /// armed streams: an unarmed stream is never pinged and never fails.
    ///
    /// This is a deliberate scope decision, not a missing case. A peer
    /// that never sends a liveness frame is either an old SDK or a server
    /// that tunnels a different protocol over the stream (the IP packet
    /// routers), and this module leaves its liveness to the consumer
    /// driving that traffic. Confining keepalive to armed streams is what
    /// lets the IPR path stay untouched with no per-caller opt-out.
    armed: bool,
    /// Nonce of the ping awaiting a pong. One nonce per outage: re-pings
    /// repeat it so a pong slower than the ping interval still matches.
    outstanding_nonce: Option<u32>,
    /// Consecutive sent pings that went a full interval without any response.
    missed_pongs: u32,
    /// When the outstanding ping actually left the client; `None` while a
    /// reserved nonce is still waiting for space on the input channel.
    last_ping_sent: Option<Instant>,
    /// Set at the miss threshold: failed (armed) or given up (unarmed).
    /// Cleared again when an armed peer shows life.
    ping_stopped: bool,
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

    /// Register a new stream, returning the receiver ends of its data and
    /// established channels. Any orphan frames that arrived before
    /// registration are drained into the stream immediately. Returns `None`
    /// if a stream with this id is already active (a duplicate `Open`):
    /// replacing the existing entry would close its reader and let the old
    /// handle's `Drop` deregister the replacement.
    async fn register_stream(
        &self,
        stream_id: StreamId,
        peer: Option<StreamPeer>,
    ) -> Option<(
        mpsc::UnboundedReceiver<Result<Vec<u8>, StreamFailure>>,
        watch::Receiver<bool>,
    )> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (established_tx, established_rx) = watch::channel(false);
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
            established_tx,
            peer,
            armed: false,
            outstanding_nonce: None,
            missed_pongs: 0,
            last_ping_sent: None,
            ping_stopped: false,
        };
        // The receiver cannot have been dropped yet - we still hold it.
        entry.drain_ready();
        inner.streams.insert(stream_id, entry);
        Some((rx, established_rx))
    }

    /// Mark a stream established and armed: an OpenAck arrived (outbound),
    /// or we accepted the stream ourselves (inbound). Unknown ids are
    /// ignored; acks carry no data worth orphan-buffering.
    async fn mark_established(&self, stream_id: &StreamId) {
        let mut inner = self.inner.lock().await;
        if let Some(entry) = inner.streams.get_mut(stream_id) {
            let _ = entry.established_tx.send(true);
            entry.armed = true;
            entry.last_activity = Instant::now();
            // Positive proof of life: pings sent before the peer had the
            // stream registered can never be answered and must not keep
            // counting toward the failure threshold.
            entry.outstanding_nonce = None;
            entry.missed_pongs = 0;
            entry.ping_stopped = false;
        }
    }

    /// Handle an inbound keepalive ping. Returns the reply destination for
    /// the pong, or `None` for unknown streams: silence tells the peer the
    /// stream is gone. A ping also proves the peer speaks the liveness
    /// extension and counts as establishment.
    async fn on_ping(&self, stream_id: &StreamId) -> Option<StreamPeer> {
        let mut inner = self.inner.lock().await;
        let entry = inner.streams.get_mut(stream_id)?;
        let _ = entry.established_tx.send(true);
        entry.armed = true;
        entry.last_activity = Instant::now();
        entry.outstanding_nonce = None;
        entry.missed_pongs = 0;
        entry.ping_stopped = false;
        entry.peer.clone()
    }

    /// Handle a pong. Only the outstanding nonce counts: stale or unknown
    /// nonces are ignored so a replayed pong cannot mask a dead peer.
    async fn on_pong(&self, stream_id: &StreamId, nonce: u32) {
        let mut inner = self.inner.lock().await;
        let Some(entry) = inner.streams.get_mut(stream_id) else {
            return;
        };
        if entry.outstanding_nonce != Some(nonce) {
            trace!("Stream {stream_id}: ignoring pong with stale nonce {nonce}");
            return;
        }
        let _ = entry.established_tx.send(true);
        entry.outstanding_nonce = None;
        entry.missed_pongs = 0;
        entry.armed = true;
        entry.ping_stopped = false;
        entry.last_activity = Instant::now();
    }

    /// Instant of the most recent inbound frame for a stream, or `None` if
    /// the stream is no longer registered.
    async fn last_activity(&self, stream_id: &StreamId) -> Option<Instant> {
        let inner = self.inner.lock().await;
        inner
            .streams
            .get(stream_id)
            .map(|entry| entry.last_activity)
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
            // Data is proof of life and proof the peer accepted the
            // stream: reset miss tracking, resolve wait_established, and
            // resume keepalive on an armed stream that had given up. Data
            // does not arm; it proves nothing about the liveness
            // extension.
            entry.last_activity = Instant::now();
            entry.outstanding_nonce = None;
            entry.missed_pongs = 0;
            if entry.armed {
                entry.ping_stopped = false;
            }
            // Only the first frame needs to flip the watch; re-sending on
            // every later Data frame would wake watchers for nothing.
            if !*entry.established_tx.borrow() {
                let _ = entry.established_tx.send(true);
            }
        }
    }

    /// Keepalive sweep for outbound streams, run from the router's tick.
    /// Only armed streams are pinged: a peer that has never sent a
    /// liveness frame has not proved it speaks the extension, so it is
    /// left alone (see Arming). An armed stream idle past `ping_interval`
    /// gets a ping, sent with a non-blocking `try_send` so a congested
    /// input channel can never stall the router: a ping that does not fit
    /// is retried next tick and never counts as a miss. An outstanding
    /// nonce whose ping left the client a full interval ago is a miss. At
    /// the miss threshold the stream fails in-band with
    /// [`StreamFailure::PeerUnresponsive`].
    ///
    /// Returns the ids pinged this sweep (used by tests).
    async fn ping_sweep(
        &self,
        ping_interval: Duration,
        missed_pongs_threshold: u32,
        client_input: &ClientInput,
        packet_type: Option<PacketType>,
    ) -> Vec<StreamId> {
        let now = Instant::now();
        let mut pinged = Vec::new();
        let mut dropped = Vec::new();
        let mut inner = self.inner.lock().await;
        for (id, entry) in inner.streams.iter_mut() {
            // Only the dialer pings. An inbound stream is registered with
            // a SenderTag peer and is skipped here; acceptor-side pings
            // would spend the dialer's SURBs on every exchange.
            let recipient = match &entry.peer {
                Some(StreamPeer::Address(recipient)) => recipient.clone(),
                _ => continue,
            };
            // Only ping a stream once it has armed. An unarmed peer has
            // never sent a liveness frame, so it may be an old SDK or a
            // server that tunnels a different protocol (the IP packet
            // routers): probing it would send frames it cannot answer.
            if !entry.armed || entry.ping_stopped {
                continue;
            }
            let last_signal = match entry.last_ping_sent {
                Some(sent) => std::cmp::max(sent, entry.last_activity),
                None => entry.last_activity,
            };
            if now.duration_since(last_signal) < ping_interval {
                continue;
            }
            // A miss requires a ping that actually left the client.
            if entry.outstanding_nonce.is_some() && entry.last_ping_sent.is_some() {
                entry.missed_pongs += 1;
                if entry.missed_pongs >= missed_pongs_threshold {
                    entry.ping_stopped = true;
                    warn!(
                        "Stream {id}: peer unresponsive, {} consecutive pings unanswered",
                        entry.missed_pongs
                    );
                    if entry
                        .sender
                        .send(Err(StreamFailure::PeerUnresponsive))
                        .is_err()
                    {
                        dropped.push(*id);
                    }
                    continue;
                }
            }
            // One nonce per outage: re-pings repeat it, so a pong slower
            // than the ping interval still matches and clears the count.
            let nonce = *entry.outstanding_nonce.get_or_insert_with(rand::random);
            let wire = encode_stream_message(id, SphinxStreamMsgType::Ping, nonce, &[]);
            let msg = InputMessage::new_anonymous(
                *recipient,
                wire,
                PING_SURBS,
                TransmissionLane::General,
                packet_type,
            );
            if client_input.input_sender.try_send(msg).is_ok() {
                entry.last_ping_sent = Some(now);
                pinged.push(*id);
            } else {
                trace!("Stream {id}: input channel full, keepalive ping deferred");
            }
        }
        for id in dropped {
            inner.streams.remove(&id);
            inner.orphans.remove(&id);
        }
        pinged
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
    client_input: ClientInput,
    packet_type: Option<PacketType>,
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

            let Some((rx, established_rx)) = self
                .streams
                .register_stream(req.stream_id, Some(StreamPeer::SenderTag(sender_tag)))
                .await
            else {
                warn!(
                    "Listener: duplicate Open for active stream {}, ignoring",
                    req.stream_id
                );
                continue;
            };

            // We are the accepting side, so the stream is established by
            // construction; this also lets `wait_established` on the
            // returned handle resolve immediately.
            self.streams.mark_established(&req.stream_id).await;

            // Best-effort ack: costs one of the dialer's SURBs and tells it
            // someone is listening. The stream works without it, so a
            // failed send (for example a dialer that attached no SURBs)
            // must not lose the stream.
            let ack = encode_stream_message(&req.stream_id, SphinxStreamMsgType::OpenAck, 0, &[]);
            let ack_msg = InputMessage::new_reply(
                sender_tag,
                ack,
                TransmissionLane::General,
                self.packet_type,
            );
            // Non-blocking: accept() must not park behind unrelated
            // application writes on the bounded input channel for an ack
            // the protocol treats as optional.
            if self.client_input.input_sender.try_send(ack_msg).is_err() {
                warn!(
                    "Stream {}: could not send OpenAck (channel busy or closed)",
                    req.stream_id
                );
            }

            return Some(MixnetStream::new_inbound(
                req.stream_id,
                sender_tag,
                self.client_input.clone(),
                self.packet_type,
                self.streams.clone(),
                rx,
                established_rx,
                req.initial_data,
            ));
        }
    }
}

/// Background loop that demuxes incoming mixnet messages into per-stream channels.
#[allow(clippy::too_many_arguments)]
async fn run_router(
    mut reconstructed_rx: ReconstructedMessagesReceiver,
    streams: StreamMap,
    listener_tx: mpsc::UnboundedSender<InboundOpen>,
    shutdown: CancellationToken,
    idle_timeout: Duration,
    client_input: ClientInput,
    packet_type: Option<PacketType>,
) {
    let check_every = std::cmp::min(idle_timeout, MAX_CLEANUP_INTERVAL);
    let mut cleanup_interval = tokio::time::interval(check_every);
    cleanup_interval.tick().await; // consume the immediate first tick

    loop {
        let messages = tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = cleanup_interval.tick() => {
                streams.cleanup_stale(idle_timeout).await;
                streams
                    .ping_sweep(
                        DEFAULT_PING_INTERVAL,
                        DEFAULT_MISSED_PONGS_THRESHOLD,
                        &client_input,
                        packet_type,
                    )
                    .await;
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
                SphinxStreamMsgType::OpenAck => {
                    streams.mark_established(&stream_id).await;
                }
                SphinxStreamMsgType::Ping => {
                    let Some(peer) = streams.on_ping(&stream_id).await else {
                        trace!("Router: ping for unknown stream {stream_id}, dropping");
                        continue;
                    };
                    let wire = encode_stream_message(
                        &stream_id,
                        SphinxStreamMsgType::Pong,
                        frame.sequence_num,
                        &[],
                    );
                    let reply = match peer {
                        StreamPeer::SenderTag(tag) => InputMessage::new_reply(
                            tag,
                            wire,
                            TransmissionLane::General,
                            packet_type,
                        ),
                        StreamPeer::Address(recipient) => InputMessage::new_anonymous(
                            *recipient,
                            wire,
                            0,
                            TransmissionLane::General,
                            packet_type,
                        ),
                    };
                    // Non-blocking: a full input channel must not stall the
                    // demux loop. A dropped pong just means the peer
                    // re-pings next interval.
                    if client_input.input_sender.try_send(reply).is_err() {
                        trace!("Stream {stream_id}: input channel full, pong dropped");
                    }
                }
                SphinxStreamMsgType::Pong => {
                    streams.on_pong(&stream_id, frame.sequence_num).await;
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
            client.client_input.clone(),
            client.packet_type,
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

    // Register as an outbound peer so the keepalive sweep can address
    // pings here. The stream only actually pings once it arms (the peer
    // proves it speaks the liveness extension by answering), so a peer
    // that never sends OpenAck/Ping/Pong is registered but never pinged.
    let peer = Some(StreamPeer::Address(Box::new(recipient)));

    // Random ids make collisions vanishingly unlikely, but regenerate on
    // the off chance rather than clobbering an active stream.
    let (stream_id, rx, established_rx) = loop {
        let stream_id = StreamId::random();
        if let Some((rx, established_rx)) = streams.register_stream(stream_id, peer.clone()).await {
            break (stream_id, rx, established_rx);
        }
    };

    // Open message with seq=0. The receiver's reorder buffer starts at
    // next_seq=0 so this could later carry an initial seq to resume a
    // dropped stream from where it left off. The reply SURBs attached here
    // also prepay the OpenAck; Data frames attach the same count.
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
        established_rx,
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

    /// Register a stream with no liveness peer and return just the data
    /// receiver: what most reorder-buffer tests care about.
    async fn register(
        map: &StreamMap,
        id: StreamId,
    ) -> mpsc::UnboundedReceiver<Result<Vec<u8>, StreamFailure>> {
        map.register_stream(id, None)
            .await
            .expect("fresh stream id")
            .0
    }

    /// Any well-formed address serves: sweeps only carry it back out.
    fn test_recipient() -> Recipient {
        Recipient::try_from_base58_string(
            "D1rrpsysCGCYXy9saP8y3kmNpGtJZUXN9SvFoUcqAsM9.9Ssso1ea5NfkbMASdiseDSjTN1fSWda5SgEVjdSN4CvV@GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN",
        )
        .expect("valid test address")
    }

    fn peer_address() -> Option<StreamPeer> {
        Some(StreamPeer::Address(Box::new(test_recipient())))
    }

    /// A [`ClientInput`] whose input channel has the given capacity,
    /// plus the receiver that keeps sends succeeding. The unrelated
    /// request/connection receivers are leaked (test-only) so their
    /// channels stay open without naming crate-private types.
    fn test_client_input(
        capacity: usize,
    ) -> (ClientInput, tokio::sync::mpsc::Receiver<InputMessage>) {
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(capacity);
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(1);
        std::mem::forget(request_rx);
        let (connection_tx, connection_rx) = futures::channel::mpsc::unbounded();
        std::mem::forget(connection_rx);
        (
            ClientInput {
                connection_command_sender: connection_tx,
                input_sender: input_tx,
                client_request_sender: request_tx,
            },
            input_rx,
        )
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_stale_removes_idle_streams() {
        let map = StreamMap::new();
        let timeout = Duration::from_secs(10);

        // Register two streams
        let _rx_a = register(&map, StreamId::random()).await;
        let _rx_b = register(&map, StreamId::random()).await;

        // Advance time past the timeout
        tokio::time::advance(timeout + Duration::from_secs(1)).await;

        // Register a fresh stream (should survive cleanup)
        let id_c = StreamId::random();
        let _rx_c = register(&map, id_c).await;

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

        let _rx = register(&map, id).await;

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
        let _rx = register(&map, id).await;

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
        let mut rx = register(&map, id).await;
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

        let mut rx = register(&map, id).await;
        // A duplicate Open for an active stream must not clobber the entry.
        assert!(map.register_stream(id, None).await.is_none());

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
        let mut rx = register(&map, id).await;

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

        let mut rx = register(&map, id).await;
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

        let mut rx = register(&map, id).await;
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![0]);
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![10]);
    }

    #[tokio::test]
    async fn duplicate_seq_is_dropped() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let mut rx = register(&map, id).await;

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
        let mut rx = register(&map, id).await;
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
        let mut rx = register(&map, id).await;
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

    #[tokio::test]
    async fn open_ack_fires_established_watch() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let (_rx, mut established) = map
            .register_stream(id, None)
            .await
            .expect("fresh stream id");
        assert!(!*established.borrow());

        // What the router does on OpenAck.
        map.mark_established(&id).await;
        assert!(established.wait_for(|v| *v).await.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn no_ack_within_timeout_leaves_stream_usable() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let (mut rx, mut established) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");

        // No ack arrives: the wait times out.
        let wait = tokio::time::timeout(Duration::from_secs(15), established.wait_for(|v| *v));
        assert!(wait.await.is_err());

        // The stream still delivers data afterwards.
        map.send_to_stream(&id, 0, vec![9]).await;
        assert_eq!(rx.recv().await.unwrap().unwrap(), vec![9]);
    }

    #[tokio::test]
    async fn pings_answered_only_for_registered_streams() {
        let map = StreamMap::new();
        let id = StreamId::random();

        // Unknown stream: silence, and no orphan state.
        assert!(map.on_ping(&id).await.is_none());
        assert!(map.inner.lock().await.orphans.is_empty());

        let tag = AnonymousSenderTag::from([7u8; 16]);
        let (_rx, _est) = map
            .register_stream(id, Some(StreamPeer::SenderTag(tag)))
            .await
            .expect("fresh stream id");
        assert!(map.on_ping(&id).await.is_some());

        map.remove(&id).await;
        assert!(map.on_ping(&id).await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn stale_pong_nonce_is_ignored() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, _input_rx) = test_client_input(8);
        let id = StreamId::random();
        let (_rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");
        // An OpenAck arms the stream, so keepalive will probe it once idle.
        map.mark_established(&id).await;

        tokio::time::advance(Duration::from_secs(61)).await;
        assert_eq!(
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await
                .len(),
            1
        );
        let nonce = map.inner.lock().await.streams[&id]
            .outstanding_nonce
            .expect("ping outstanding");

        // The wrong nonce changes nothing: a replayed pong cannot mask a
        // dead peer.
        map.on_pong(&id, nonce.wrapping_add(1)).await;
        {
            let inner = map.inner.lock().await;
            assert_eq!(inner.streams[&id].outstanding_nonce, Some(nonce));
            assert!(inner.streams[&id].armed);
        }

        // The right nonce clears the outstanding ping and arms the stream.
        map.on_pong(&id, nonce).await;
        let inner = map.inner.lock().await;
        assert_eq!(inner.streams[&id].outstanding_nonce, None);
        assert!(inner.streams[&id].armed);
    }

    #[tokio::test(start_paused = true)]
    async fn armed_stream_fails_in_band_after_threshold() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, _input_rx) = test_client_input(8);
        let id = StreamId::random();
        let (mut rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");

        // An OpenAck arms the stream; deliver data to check ordering.
        map.mark_established(&id).await;
        map.send_to_stream(&id, 0, vec![1]).await;

        // Sweep 1 sends a fresh ping; sweeps 2-4 each count a miss. The
        // third miss reaches the threshold and fails the stream.
        for _ in 0..4 {
            tokio::time::advance(Duration::from_secs(61)).await;
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await;
        }

        // In order: the data, then the failure, then nothing.
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![1]);
        assert!(matches!(
            rx.try_recv().unwrap(),
            Err(StreamFailure::PeerUnresponsive)
        ));
        assert!(rx.try_recv().is_err());

        // Keepalive has stopped for this stream.
        tokio::time::advance(Duration::from_secs(61)).await;
        assert!(map
            .ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
            .await
            .is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn unarmed_stream_is_never_pinged() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, _input_rx) = test_client_input(8);
        let id = StreamId::random();
        let (mut rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");

        // The peer never sends a liveness frame, so the stream never arms.
        // Keepalive leaves it alone entirely: no pings and no failure. It
        // may be an old SDK, or a server that tunnels another protocol
        // whose liveness is the consumer's concern (see StreamEntry.armed).
        for _ in 0..4 {
            tokio::time::advance(Duration::from_secs(61)).await;
            assert!(map
                .ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await
                .is_empty());
        }
        assert!(rx.try_recv().is_err());

        // Inbound data establishes the stream but does not arm it, so
        // keepalive still sends nothing.
        map.send_to_stream(&id, 0, vec![1]).await;
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![1]);
        tokio::time::advance(Duration::from_secs(61)).await;
        assert!(map
            .ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
            .await
            .is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn active_streams_are_not_pinged() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, _input_rx) = test_client_input(8);
        let id = StreamId::random();
        let (_rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");
        // An OpenAck arms the stream, so keepalive will probe it once idle.
        map.mark_established(&id).await;

        // Data arrives 30 s in: the stream is active.
        tokio::time::advance(Duration::from_secs(30)).await;
        map.send_to_stream(&id, 0, vec![1]).await;

        // 61 s since registration, but only 31 s since inbound data.
        tokio::time::advance(Duration::from_secs(31)).await;
        assert!(map
            .ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
            .await
            .is_empty());

        // A full interval with nothing inbound: pinged.
        tokio::time::advance(Duration::from_secs(30)).await;
        assert_eq!(
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await
                .len(),
            1
        );
    }

    #[tokio::test(start_paused = true)]
    async fn inbound_data_resets_missed_pongs() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, _input_rx) = test_client_input(8);
        let id = StreamId::random();
        let (mut rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");
        // An OpenAck arms the stream, so keepalive will probe it once idle.
        map.mark_established(&id).await;

        // A ping goes out and two sweeps count misses.
        for _ in 0..3 {
            tokio::time::advance(Duration::from_secs(61)).await;
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await;
        }

        // Data arrives: proof of life clears the miss tracking.
        map.send_to_stream(&id, 0, vec![1]).await;
        {
            let inner = map.inner.lock().await;
            assert_eq!(inner.streams[&id].missed_pongs, 0);
            assert_eq!(inner.streams[&id].outstanding_nonce, None);
        }
        assert_eq!(rx.try_recv().unwrap().unwrap(), vec![1]);
    }

    #[tokio::test(start_paused = true)]
    async fn inbound_streams_are_not_pinged() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, _input_rx) = test_client_input(8);
        let tag = AnonymousSenderTag::from([7u8; 16]);
        let (_rx, _est) = map
            .register_stream(StreamId::random(), Some(StreamPeer::SenderTag(tag)))
            .await
            .expect("fresh stream id");

        // Only the dialer pings; the acceptor side stays passive.
        tokio::time::advance(Duration::from_secs(120)).await;
        assert!(map
            .ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
            .await
            .is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn same_nonce_is_resent_until_answered() {
        let map = StreamMap::new();
        let threshold = 5;
        let (input, _input_rx) = test_client_input(8);
        let id = StreamId::random();
        let (_rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");
        // An OpenAck arms the stream, so keepalive will probe it once idle.
        map.mark_established(&id).await;

        tokio::time::advance(Duration::from_secs(61)).await;
        map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
            .await;
        let first = map.inner.lock().await.streams[&id]
            .outstanding_nonce
            .expect("ping outstanding");

        // Two more intervals pass unanswered: misses accumulate but the
        // nonce stays the same, so a pong slower than one interval still
        // counts as proof of life.
        for _ in 0..2 {
            tokio::time::advance(Duration::from_secs(61)).await;
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await;
        }
        {
            let inner = map.inner.lock().await;
            assert_eq!(inner.streams[&id].outstanding_nonce, Some(first));
            assert_eq!(inner.streams[&id].missed_pongs, 2);
        }

        map.on_pong(&id, first).await;
        let inner = map.inner.lock().await;
        assert_eq!(inner.streams[&id].missed_pongs, 0);
        assert!(inner.streams[&id].armed);
    }

    #[tokio::test(start_paused = true)]
    async fn full_channel_defers_ping_without_counting_a_miss() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, mut input_rx) = test_client_input(1);
        let id = StreamId::random();
        let (_rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");
        // An OpenAck arms the stream, so keepalive will probe it once idle.
        map.mark_established(&id).await;

        // An application write occupies the capacity-1 input channel.
        input
            .input_sender
            .try_send(InputMessage::new_anonymous(
                test_recipient(),
                vec![0],
                0,
                TransmissionLane::General,
                None,
            ))
            .expect("channel has capacity");

        tokio::time::advance(Duration::from_secs(61)).await;
        assert!(map
            .ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
            .await
            .is_empty());
        {
            let inner = map.inner.lock().await;
            // The nonce is reserved but the ping never left the client,
            // so nothing counts toward the miss threshold.
            assert!(inner.streams[&id].outstanding_nonce.is_some());
            assert!(inner.streams[&id].last_ping_sent.is_none());
            assert_eq!(inner.streams[&id].missed_pongs, 0);
        }

        // Channel drains: the next sweep retries without waiting another
        // full interval, and still counts no miss.
        input_rx.try_recv().expect("queued application write");
        assert_eq!(
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await
                .len(),
            1
        );
        assert_eq!(map.inner.lock().await.streams[&id].missed_pongs, 0);
    }

    #[tokio::test]
    async fn inbound_data_establishes_without_arming() {
        let map = StreamMap::new();
        let id = StreamId::random();
        let (_rx, established) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");
        assert!(!*established.borrow());

        // Data from the peer proves the stream was accepted, so
        // wait_established resolves even if the lone OpenAck was lost.
        // It proves nothing about the liveness extension, so no arming.
        map.send_to_stream(&id, 0, vec![1]).await;
        assert!(*established.borrow());
        assert!(!map.inner.lock().await.streams[&id].armed);
    }

    #[tokio::test(start_paused = true)]
    async fn armed_stream_resumes_keepalive_after_data() {
        let map = StreamMap::new();
        let threshold = 3;
        let (input, _input_rx) = test_client_input(8);
        let id = StreamId::random();
        let (mut rx, _est) = map
            .register_stream(id, peer_address())
            .await
            .expect("fresh stream id");

        // An OpenAck arms the stream; then trip the threshold through a
        // transient outage.
        map.mark_established(&id).await;
        for _ in 0..4 {
            tokio::time::advance(Duration::from_secs(61)).await;
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await;
        }
        assert!(matches!(
            rx.recv().await.unwrap(),
            Err(StreamFailure::PeerUnresponsive)
        ));

        // The peer shows life again: an armed stream gets keepalive back,
        // so a later real death is still detected at ping cadence.
        map.send_to_stream(&id, 0, vec![1]).await;
        tokio::time::advance(Duration::from_secs(61)).await;
        assert_eq!(
            map.ping_sweep(DEFAULT_PING_INTERVAL, threshold, &input, None)
                .await
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn accept_survives_open_ack_send_failure() {
        // A ClientInput whose channels are all closed: every send fails,
        // as it would for a dialer that attached no reply SURBs.
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(1);
        drop(input_rx);
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(1);
        drop(request_rx);
        let (connection_tx, connection_rx) = futures::channel::mpsc::unbounded();
        drop(connection_rx);
        let client_input = ClientInput {
            connection_command_sender: connection_tx,
            input_sender: input_tx,
            client_request_sender: request_tx,
        };

        let (open_tx, open_rx) = mpsc::unbounded_channel();
        let mut listener = MixnetListener {
            inbound_rx: open_rx,
            client_input,
            packet_type: None,
            streams: StreamMap::new(),
        };

        open_tx
            .send(InboundOpen {
                stream_id: StreamId::random(),
                sender_tag: Some(AnonymousSenderTag::from([7u8; 16])),
                initial_data: Vec::new(),
            })
            .expect("listener alive");

        // The ack cannot be sent, but the stream must still be returned.
        let stream = listener.accept().await;
        assert!(stream.is_some());
    }
}
