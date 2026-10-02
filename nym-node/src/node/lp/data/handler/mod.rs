// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP Data Handler - UDP listener for LP data plane (port 51264)
//!
//! This module handles the data plane for LP clients that have completed registration
//! via the control plane (TCP:41264). LP-wrapped Sphinx packets arrive here, get
//! decrypted, and are forwarded into the mixnet.
//!
//! # Packet Flow
//!
//! ```text
//! LP Client → UDP:51264 → LP Data Handler → Mixnet Entry
//!           LP(Sphinx)      decrypt LP      forward Sphinx
//! ```
//!

use crate::node::lp::control::egress::dialer::LpDialer;
use crate::node::lp::data::PACKET_BUFFER_SIZE;
use crate::node::lp::data::handler::outgoing::OutgoingFrames;
use crate::node::lp::data::handler::pipeline::{
    LpTransport, MixingNodeDataPipeline, NymNodeDataPipeline,
};
use crate::node::lp::data::shared::{SharedGatewayLpDataState, SharedLpDataState};
use crate::node::lp::error::LpHandlerError;

use nym_lp_data::common::traits::TransportUnwrap;
use nym_lp_data::nymnodes::traits::NymNodeProcessingPipeline;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_lp_data::{AddressedTimedData, TimedData};
use nym_metrics::inc;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use rand::rngs::OsRng;
use std::sync::{Arc, mpsc};
use std::time::Instant;
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::interval;
use tracing::*;

pub mod error;
pub mod messages;
mod outgoing;
pub mod pipeline;
mod processing;

const PIPELINE_TICKING_DURATION: Duration = Duration::from_millis(1);

/// Bounded queue depth in front of each worker; keeps memory bounded under
/// bursty load and provides drop-based backpressure.
const WORKER_QUEUE_DEPTH: usize = 128;

/// Workers emit frames addressed by the *identity* of their next hop; the handler resolves that to
/// a wire address when it applies the transport wrap at release time.
type WorkerOutput = Vec<AddressedTimedData<LpFrame, NymNodeRoutingAddress>>;

/// LP Data Handler for UDP data plane, acts as a pipeline driver and buffer
/// for delaying packets. Heavy per-packet processing is fanned out across a
/// pool of worker threads spawned on the shared blocking pool tracked by the
/// surrounding [`nym_task::ShutdownTracker`].
pub struct LpDataHandler {
    /// Shared state
    shared_state: Arc<SharedLpDataState>,

    /// Channel to receive incoming data
    input_rx: mpsc::Receiver<(EncryptedLpPacket, SocketAddr)>,

    /// Channel to send outgoing data
    output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,

    /// Per-worker job queues (round-robin dispatch).
    worker_input_txs: Vec<mpsc::SyncSender<AddressedTimedData<EncryptedLpPacket>>>,

    /// Aggregated processed packets returned by the workers.
    worker_output_rx: mpsc::Receiver<WorkerOutput>,

    /// Frames awaiting their scheduled send time. See [`OutgoingFrames`].
    outgoing: OutgoingFrames,

    /// Asks the control plane to establish a session. A hint, never awaited - see
    /// [`LpDialer::request`].
    dialer: LpDialer,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataHandler {
    pub(crate) fn new(
        shared_state: Arc<SharedLpDataState>,
        gateway_state: Option<Arc<SharedGatewayLpDataState>>,
        input_rx: mpsc::Receiver<(EncryptedLpPacket, SocketAddr)>,
        output_tx: tokio::sync::mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
        dialer: LpDialer,
        shutdown_tracker: &nym_task::ShutdownTracker,
    ) -> Result<Self, LpHandlerError> {
        let (worker_output_tx, worker_output_rx) = mpsc::sync_channel(PACKET_BUFFER_SIZE);

        // Allow at least one worker, even if the config says 0
        let worker_count = shared_state.lp_config.debug.data_worker_count.max(1);

        // Are we running a full size node or just a mixing one
        let gateway_mode = shared_state.processing_config.client_forwarding_enabled;

        // Validate gateway state once up-front: required iff this node forwards client
        // traffic. Workers downstream see an already-unwrapped Arc.
        let gateway_state = if gateway_mode {
            Some(gateway_state.ok_or(LpHandlerError::MissingGatewayState)?)
        } else {
            None
        };

        // Create workers. They will stop naturally when worker_output_rx is dropped.
        // The mode is decided once here; each closure picks the right pipeline type so
        // the worker loop monomorphizes against a single concrete pipeline.
        let worker_input_txs = (0..worker_count)
            .map(|_| {
                let (worker_input_tx, worker_input_rx) = mpsc::sync_channel(WORKER_QUEUE_DEPTH);
                let worker_state = shared_state.clone();
                let worker_output = worker_output_tx.clone();
                // each worker can raise a dial request of its own: `request` is a non-blocking
                // `try_send`, so it is safe to call from a blocking thread
                let worker_dialer = dialer.clone();
                match &gateway_state {
                    Some(gw) => {
                        let worker_gateway_state = gw.clone();
                        shutdown_tracker.spawn_blocking(move || {
                            let pipeline = NymNodeDataPipeline::new(
                                worker_state.clone(),
                                worker_gateway_state,
                                OsRng,
                            );
                            Self::run_worker(
                                pipeline,
                                worker_state,
                                worker_input_rx,
                                worker_output,
                                worker_dialer,
                            );
                        });
                    }
                    None => {
                        shutdown_tracker.spawn_blocking(move || {
                            let pipeline = MixingNodeDataPipeline::new(worker_state.clone(), OsRng);
                            Self::run_worker(
                                pipeline,
                                worker_state,
                                worker_input_rx,
                                worker_output,
                                worker_dialer,
                            );
                        });
                    }
                }

                worker_input_tx
            })
            .collect();

        Ok(Self {
            shared_state,
            input_rx,
            output_tx,
            worker_input_txs,
            worker_output_rx,
            outgoing: OutgoingFrames::default(),
            dialer,
            shutdown: shutdown_tracker.clone_shutdown_token(),
        })
    }

    pub async fn run(&mut self) {
        info!(
            workers = self.worker_input_txs.len(),
            "Starting LP data handler"
        );
        let mut ticking_interval = interval(PIPELINE_TICKING_DURATION);
        let mut next_worker = 0;

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    info!("LP data handler: received shutdown signal");
                    break;
                }

                timestamp = ticking_interval.tick() => {
                    let std_timestamp: Instant = timestamp.into();

                    // Drain framed output returned by workers.
                    while let Ok(frames) = self.worker_output_rx.try_recv() {
                        self.buffer_frames(frames);
                    }

                    // Dispatch incoming packets to workers, which decrypt them before mixing.
                    while let Ok((packet, src)) = self.input_rx.try_recv() {
                        next_worker = self.dispatch_to_workers(
                            AddressedTimedData::new_addressed(std_timestamp, packet, src),
                            next_worker,
                        );
                    }

                    // Wrap and send everything whose scheduled time has arrived.
                    for (packet, dst) in self.wrap_due_frames(std_timestamp) {
                        if let Err(e) = self.output_tx.try_send((packet, dst)) {
                            match e {
                                TrySendError::Full(_) =>  {
                                    warn!("LP data handler: packet sending buffer is full, the node might be overloaded");
                                    self.shared_state.egress_overloaded_packet_dropped();
                                },
                                TrySendError::Closed(_) => {
                                    info!("LP data handler: outgoing channel is closed");
                                    break;
                                },
                            }
                        }
                    }
                }
            }
        }

        // Workers will stop because we are dropping the receiving channel
        info!("LP data handler shutdown complete");
    }

    /// Round-robin dispatch a job across worker queues. If the chosen worker is
    /// full, fall through to the next one; if all are saturated, drop the packet
    /// (UDP-style) and bump a metric. Returns the worker index to start from on
    /// the next dispatch.
    fn dispatch_to_workers(
        &self,
        mut job: AddressedTimedData<EncryptedLpPacket>,
        start: usize,
    ) -> usize {
        let n = self.worker_input_txs.len();
        for offset in 0..n {
            let idx = (start + offset) % n;
            match self.worker_input_txs[idx].try_send(job) {
                Ok(()) => return (idx + 1) % n,
                Err(mpsc::TrySendError::Full(returned)) => {
                    job = returned;
                }
                Err(mpsc::TrySendError::Disconnected(returned)) => {
                    error!(
                        "LP data worker {idx} disconnected; this shouldn't happen outside of shut down"
                    );
                    job = returned;
                }
            }
        }

        warn!("LP data handler: all workers saturated, dropping packet");
        self.shared_state.worker_pool_overloaded_packet_dropped();
        start
    }

    /// Wrap every frame whose scheduled send time has arrived, in release order.
    ///
    /// The wrap belongs here rather than in the worker because it assigns the LP counter, which is
    /// cleartext in `OuterHeader`. `mix()` stamps a future timestamp (arrival + sphinx delay), so
    /// numbering packets as they are processed rather than as they are sent would let an observer
    /// read each packet's displacement in the sequence and recover the delay it was given - the
    /// in-to-out correlation the delay exists to destroy.
    ///
    /// A peer with no session is skipped and its frames stay queued while a dial is requested; see
    /// [`OutgoingFrames`]. The session is checked before wrapping because the wrap consumes the
    /// frame, and an absent session is routine where a failing one is not.
    fn wrap_due_frames(&mut self, now: Instant) -> Vec<(EncryptedLpPacket, SocketAddr)> {
        let stall_timeout = self.shared_state.lp_config.debug.stalled_frame_timeout;
        let mut wrapped = Vec::new();

        for peer in self.outgoing.peers() {
            if !self.shared_state.has_session_for(peer) {
                if self.outgoing.has_due(peer, now) {
                    // A hint, deliberately not awaited: this loop drives every peer, so blocking on
                    // a handshake would stall traffic to all of them behind one slow or unreachable
                    // peer. Raised here, at the frame's release time, it coincides with the send it
                    // stands in for and so reveals nothing an observer would not have seen anyway.
                    //
                    // Only nodes are dialled. A client's session is established by its own
                    // registration over the control plane; this node cannot initiate one, and
                    // would not know where to dial even if it could.
                    if let NymNodeRoutingAddress::Node(addr) = peer {
                        self.dialer.request(addr.ip());
                    }
                    inc!("lp_no_session_pending");
                }

                for _ in 0..self.outgoing.drop_stalled(peer, now, stall_timeout) {
                    inc!("lp_stalled_dropped");
                }
                continue;
            }

            // the wrap is 1:1, so the order established here carries through to the wire
            for frame in self.outgoing.take_due(peer, now) {
                match LpTransport::frame_to_packet(&self.shared_state, frame) {
                    Ok(packet) => wrapped.push((packet.data.data, packet.dst)),
                    Err(err) => {
                        // The session was there a moment ago, so this is it failing rather than
                        // missing: evicted between the check and the wrap, or demoted and
                        // read-only. Either way the frame is already consumed, so it can only be
                        // counted.
                        warn!("LP data handler: failed to wrap a frame for {peer}: {err}");
                        inc!("lp_wrap_errors");
                    }
                }
            }
        }

        self.outgoing.prune_empty();

        wrapped
    }

    /// Queue frames produced by the workers, each against its own peer.
    fn buffer_frames(&mut self, frames: WorkerOutput) {
        for frame in frames {
            if self.outgoing.queue(frame) {
                inc!("lp_queue_overflow_dropped");
            }
        }
    }

    /// Worker loop: decrypt, then mix and re-frame.
    ///
    /// Only the *inbound* half of the transport layer runs here. The outbound wrap belongs to
    /// [`Self::wrap_due_frames`], which applies it at release time so that the cleartext counter
    /// it assigns matches send order - see there for why that is an anonymity property.
    ///
    /// Decryption carries no such constraint, so it runs across the pool: it changes nothing
    /// observable, and the receiver's replay window is a 1024-entry bitmap specifically to absorb
    /// out-of-order arrival. Concurrency here reorders by at most the pool depth.
    ///
    /// Note that two packets on the same session serialise on that session's lock regardless; the
    /// parallelism is across peers.
    ///
    /// # Recovering from a peer restart
    ///
    /// A packet naming a session this node does not hold is the signature of a peer that restarted
    /// and lost its half of the pairing - and nothing else reports it. The peer keeps sending on a
    /// session that no longer exists here, and since its own TTL is measured from last activity,
    /// its every send refreshes the dead entry rather than ageing it out. Left alone, traffic from
    /// that peer is black-holed indefinitely.
    ///
    /// So this raises a dial request for the packet's source, which re-establishes the pairing and
    /// repoints the peer's sending session without the peer being restarted or signalled. The
    /// busier the flow, the faster it heals.
    ///
    /// Two things bound the damage an attacker can do with forged packets: the dialer ignores any
    /// source that is not already a known LP node, and it coalesces per peer, so a flood produces
    /// one handshake rather than one per packet. There is deliberately no reply on the wire - a
    /// "session unknown" packet could not be authenticated (there is no session to authenticate
    /// with) and would therefore be a spoofable teardown primitive.
    ///
    /// Raising this on *arrival* is safe where the outbound path's request is not: the triggering
    /// packet is already visible to any observer, so dialling in response reveals nothing new.
    fn run_worker<P>(
        mut pipeline: P,
        shared_state: Arc<SharedLpDataState>,
        input_rx: mpsc::Receiver<AddressedTimedData<EncryptedLpPacket>>,
        output_tx: mpsc::SyncSender<WorkerOutput>,
        dialer: LpDialer,
    ) where
        P: NymNodeProcessingPipeline<LpFrame, NymNodeRoutingAddress>
            + TransportUnwrap<EncryptedLpPacket, Frame = LpFrame, Error = LpHandlerError>,
    {
        // `dst` carries where the packet came *from*: the source is the only thing identifying
        // which peer to re-establish with when a packet names a session this node does not hold.
        while let Ok(input) = input_rx.recv() {
            let src = input.dst;
            let TimedData {
                timestamp,
                data: packet,
            } = input.data;
            let receiver_index = packet.outer_header().receiver_idx;

            let frame = match pipeline.packet_to_frame(packet, timestamp) {
                Ok(frame) => frame,
                Err(LpHandlerError::MissingLpSession { receiver_index }) => {
                    debug!(
                        "LP data worker: {src} is sending on session {receiver_index}, which this node does not hold - asking to re-establish"
                    );
                    dialer.request(src.ip());
                    inc!("lp_unknown_session_packets");
                    continue;
                }
                Err(e) => {
                    // The session exists and failed to decrypt, or the counter was replayed.
                    // Re-establishing would be wrong, so this is only counted.
                    warn!("LP data worker: could not unwrap a packet from {src}: {e}");
                    inc!("lp_data_packet_errors");
                    continue;
                }
            };

            // The packet authenticated against the session, so `src` is where that peer is now.
            // Clients move; this is the only signal that they have.
            shared_state.refresh_client_address(receiver_index, src);

            // Blocking is fine, we don't want to unclog ourself and process a new packet that will be dropped anyway
            if let Err(e) = output_tx.send(pipeline.process(frame, timestamp)) {
                trace!(
                    "Failed to send processing data back to handler : {e}. We are probably shutting down"
                );
                return;
            }
        }
    }
}
