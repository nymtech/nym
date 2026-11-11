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

use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::control::egress::dialer::LpDialer;
use crate::node::lp::data::PACKET_BUFFER_SIZE;
use crate::node::lp::data::handler::error::LpDataHandlerError;
use crate::node::lp::data::handler::outgoing::OutgoingFrames;
use crate::node::lp::data::handler::pipeline::{
    LpTransport, MixingNodeDataPipeline, NymNodeDataPipeline,
};
use crate::node::lp::data::shared::{SharedGatewayLpDataState, SharedLpDataState};
use crate::node::lp::error::LpHandlerError;

use nym_crypto::rng::os_rng;
use nym_lp_data::common::traits::TransportUnwrap;
use nym_lp_data::nymnodes::traits::NymNodeProcessingPipeline;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_lp_data::{AddressedTimedData, TimedData};
use nym_metrics::{inc, inc_by};
use nym_service_providers_common::lp::ServiceProviderOutputReceiver;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio::time::{Interval, MissedTickBehavior};
use tracing::*;

pub mod error;
pub mod messages;
mod outgoing;
pub mod pipeline;
mod processing;

/// Per worker queue size, the total buffer before them is WORKER_COUNT * WORKER_QUEUE_DEPTH; keeps memory bounded under
/// bursty load and provides drop-based backpressure.
const WORKER_QUEUE_DEPTH: usize = 128;

/// Most worker outputs taken in one go. Batching saves a pass through the event loop per output;
/// the cap keeps a backlog from delaying the release of frames already due by more than one batch.
const WORKER_OUTPUT_BATCH: usize = 32;

/// How often peers with frames waiting for a session are looked at again, to drop what has stalled
/// past the timeout and to ask again for a session whose dial failed.
///
/// A session coming up is announced, so this does not decide how soon waiting frames leave, and
/// stalled frames are dropped before any release; it only bounds how long a peer that never gets a
/// session holds memory, and how late a dial is retried after its backoff.
const STALLED_RECHECK_INTERVAL: Duration = Duration::from_millis(500);

/// Stands in as the sender of a frame handed over in-process, which never travelled.
///
/// Belongs to nobody, so anything that judges a frame by who sent it refuses it on the same terms
/// it would refuse a stranger.
const LOCAL_SOURCE: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

/// Workers emit frames addressed by the *identity* of their next hop; the handler resolves that to
/// a wire address when it applies the transport wrap at release time.
type WorkerOutput = Vec<AddressedTimedData<LpFrame, NymNodeRoutingAddress>>;

/// One unit of work for a data worker, and where it came from.
///
/// The two differ only in how much of the pipeline they enter at. A packet off the wire is
/// encrypted and possibly a fragment, so it is decrypted and reassembled first; a frame from a
/// provider in this process is neither, so it goes straight to mixing. Everything after that -
/// routing, the filter, framing, the outgoing queue, the release-time encrypt - is common.
///
/// Every job carries two times, which need not agree. The one on `data` travels with the payload
/// and tags the frame the transport unwrap produces. `processing_time` is when the handler
/// dispatched the job, and is what mixing schedules the frame's delay from.
///
/// Kept as one enum rather than two channels of work because the two must share the worker pool:
/// a provider that could not be starved by forwarded traffic could starve it instead.
enum WorkerJob {
    /// Arrived on the socket, from a peer.
    Wire {
        data: TimedData<EncryptedLpPacket>,
        processing_time: Instant,

        /// Where the packet came from. The only thing identifying which peer to re-establish with
        /// when the packet names a session this node does not hold, and where an authenticated
        /// client is reachable from now.
        source: SocketAddr,
    },

    /// Handed over by a service provider this node hosts. Never touched the wire, so there is
    /// nothing to decrypt and no fragments to put back together.
    Local {
        data: TimedData<LpFrame>,
        processing_time: Instant,
    },
}

impl WorkerJob {
    fn processing_time(&self) -> Instant {
        match self {
            WorkerJob::Wire {
                processing_time, ..
            }
            | WorkerJob::Local {
                processing_time, ..
            } => *processing_time,
        }
    }

    /// [`LOCAL_SOURCE`] for a local job, which has no sender.
    fn source(&self) -> SocketAddr {
        match self {
            WorkerJob::Wire { source, .. } => *source,
            WorkerJob::Local { .. } => LOCAL_SOURCE,
        }
    }
}

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
    output_tx: mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,

    /// Job queue shared by all workers; whichever is free takes the next job.
    worker_input_tx: crossbeam_channel::Sender<WorkerJob>,

    /// Aggregated processed packets returned by the workers.
    worker_output_rx: mpsc::Receiver<WorkerOutput>,

    /// Reused for each batch taken off `worker_output_rx`, emptied as soon as it is handled.
    worker_output_buf: Vec<WorkerOutput>,

    /// Frames from the service providers this node hosts, one channel each.
    ///
    /// Per provider rather than shared, so one cannot crowd out another, and so that the bandwidth
    /// check the forward path still owes can later tell whose traffic it is looking at.
    provider_rxs: Vec<ServiceProviderOutputReceiver>,

    /// Frames awaiting their scheduled send time. See [`OutgoingFrames`].
    outgoing: OutgoingFrames,

    /// Peers that have just been given a sending session, whose waiting frames can now leave.
    sessions_established_rx: mpsc::Receiver<LpPeer>,

    /// Asks the control plane to establish a session. A hint, never awaited - see
    /// [`LpDialer::request`].
    dialer: LpDialer,

    /// Shutdown token
    shutdown: nym_task::ShutdownToken,
}

impl LpDataHandler {
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn new(
        shared_state: Arc<SharedLpDataState>,
        gateway_state: Option<Arc<SharedGatewayLpDataState>>,
        input_rx: mpsc::Receiver<(EncryptedLpPacket, SocketAddr)>,
        output_tx: mpsc::Sender<(EncryptedLpPacket, SocketAddr)>,
        sessions_established_rx: mpsc::Receiver<LpPeer>,
        provider_rxs: Vec<ServiceProviderOutputReceiver>,
        dialer: LpDialer,
        shutdown_tracker: &nym_task::ShutdownTracker,
    ) -> Result<Self, LpHandlerError> {
        let worker_count = shared_state.lp_config.debug.data_worker_count.get();

        // Are we running a full size node or just a mixing one
        let gateway_mode = shared_state.processing_config.client_forwarding_enabled;

        // Validate gateway state once up-front: required iff this node forwards client
        // traffic. Workers downstream see an already-unwrapped Arc.
        let gateway_state = if gateway_mode {
            Some(gateway_state.ok_or(LpHandlerError::MissingGatewayState)?)
        } else {
            None
        };

        let (worker_input_tx, worker_output_rx) = Self::create_workers(
            worker_count,
            shared_state.clone(),
            gateway_state,
            dialer.clone(),
            shutdown_tracker,
        );

        Ok(Self {
            shared_state,
            input_rx,
            output_tx,
            worker_input_tx,
            provider_rxs,
            worker_output_rx,
            worker_output_buf: Vec::with_capacity(WORKER_OUTPUT_BATCH),
            outgoing: OutgoingFrames::default(),
            sessions_established_rx,
            dialer,
            shutdown: shutdown_tracker.clone_shutdown_token(),
        })
    }

    fn create_workers(
        worker_count: usize,
        shared_state: Arc<SharedLpDataState>,
        gateway_state: Option<Arc<SharedGatewayLpDataState>>,
        dialer: LpDialer,
        shutdown_tracker: &nym_task::ShutdownTracker,
    ) -> (
        crossbeam_channel::Sender<WorkerJob>,
        mpsc::Receiver<WorkerOutput>,
    ) {
        let (worker_output_tx, worker_output_rx) = mpsc::channel(PACKET_BUFFER_SIZE);
        let (worker_input_tx, worker_input_rx) =
            crossbeam_channel::bounded(WORKER_QUEUE_DEPTH * worker_count);

        // Create workers. They will stop naturally when worker_input_tx or worker_output_rx is dropped.
        // The mode is decided once here; each closure picks the right pipeline type so
        // the worker loop monomorphizes against a single concrete pipeline.
        for _ in 0..worker_count {
            let worker = DataWorker {
                shared_state: shared_state.clone(),
                input_rx: worker_input_rx.clone(),
                output_tx: worker_output_tx.clone(),
                dialer: dialer.clone(),
                shutdown: shutdown_tracker.clone_shutdown_token(),
            };
            let gateway_state = gateway_state.clone();
            shutdown_tracker.spawn_blocking(move || {
                let state = worker.shared_state.clone();
                match gateway_state {
                    Some(gw) => worker.run(NymNodeDataPipeline::new(state, gw, os_rng())),
                    None => worker.run(MixingNodeDataPipeline::new(state, os_rng())),
                }
            });
        }
        (worker_input_tx, worker_output_rx)
    }

    /// Queue a job for the next free worker. If the queue is full, drop the packet (UDP-style)
    /// and bump a metric.
    fn dispatch_to_workers(&self, job: WorkerJob) {
        match self.worker_input_tx.try_send(job) {
            Ok(()) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                warn!("LP data handler: all workers saturated, dropping packet");
                self.shared_state.worker_pool_overloaded_packet_dropped();
            }
            // Every worker is gone, and each one's drop guard has already cancelled the node.
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                if !self.shutdown.is_cancelled() {
                    error!(
                        "All LP data worker have died, most likely from a panic and shut down wasn't called; shutting down now"
                    );
                    self.shutdown.cancel();
                }
            }
        }
    }

    /// Wrap a frame and hand it to the socket. Fails only if the outgoing channel is closed.
    ///
    /// The wrap belongs here rather than in the worker because it assigns the LP counter, which is
    /// cleartext in `OuterHeader`. `mix()` stamps a future timestamp (arrival + sphinx delay), so
    /// numbering packets as they are processed rather than as they are sent would let an observer
    /// read each packet's displacement in the sequence and recover the delay it was given - the
    /// in-to-out correlation the delay exists to destroy.
    ///
    /// Callers check for a session first: the wrap consumes the frame, and an absent session is
    /// routine where a failing one is not.
    fn send_frame(
        &self,
        frame: AddressedTimedData<LpFrame, NymNodeRoutingAddress>,
    ) -> Result<(), LpDataHandlerError> {
        let peer = frame.dst;
        let packet = match LpTransport::frame_to_packet(&self.shared_state, frame) {
            Ok(packet) => packet,
            Err(err) => {
                // The session was there a moment ago, so this is it failing rather than missing:
                // evicted between the check and the wrap, or demoted and read-only. Either way the
                // frame is already consumed, so it can only be counted.
                warn!("LP data handler: failed to wrap a frame for {peer}: {err}");
                inc!("lp_wrap_errors");
                return Ok(());
            }
        };

        match self.output_tx.try_send((packet.data.data, packet.dst)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                warn!(
                    "LP data handler: packet sending buffer is full, the node might be overloaded"
                );
                self.shared_state.egress_overloaded_packet_dropped();
                Ok(())
            }
            Err(TrySendError::Closed(_)) => Err(LpDataHandlerError::OutgoingChannelClosed),
        }
    }

    /// Ask for a session with `peer`, whose frames are waiting for one.
    ///
    /// A hint, deliberately not awaited: the handler drives every peer, so blocking on a handshake
    /// would stall traffic to all of them behind one slow or unreachable peer. First raised at a
    /// frame's release time, it coincides with the send it stands in for and so reveals nothing an
    /// observer would not have seen anyway; the rechecks after that repeat it, and the dialer
    /// coalesces repeats per peer.
    ///
    /// Only nodes are dialled. A client's session is established by its own registration over the
    /// control plane; this node cannot initiate one, and would not know where to dial even if it
    /// could.
    fn request_session(&self, peer: NymNodeRoutingAddress) {
        if let NymNodeRoutingAddress::Node(addr) = peer {
            self.dialer.request(addr.ip());
        }
    }

    /// Discard `peer`'s waiting frames that are past the stall timeout. See
    /// [`OutgoingFrames::drop_stalled`] for why a late frame is dropped rather than sent.
    fn drop_stalled(&mut self, peer: NymNodeRoutingAddress, now: Instant) {
        let stall_timeout = self.shared_state.lp_config.debug.stalled_frame_timeout;
        let dropped = self.outgoing.drop_stalled(peer, now, stall_timeout);
        inc_by!("lp_stalled_dropped", dropped);
    }

    /// Send every frame waiting on `peer`, which has a session, except those past the stall
    /// timeout. Fails only if the outgoing channel is closed.
    fn release_waiting(
        &mut self,
        peer: NymNodeRoutingAddress,
        now: Instant,
    ) -> Result<(), LpDataHandlerError> {
        self.drop_stalled(peer, now);
        for frame in self.outgoing.take_waiting(peer) {
            self.send_frame(frame)?;
        }
        Ok(())
    }

    /// A frame's scheduled send time has arrived: send it, or hold it until its peer has a
    /// session. Fails only if the outgoing channel is closed.
    fn handle_due_frame(
        &mut self,
        frame: AddressedTimedData<LpFrame, NymNodeRoutingAddress>,
    ) -> Result<(), LpDataHandlerError> {
        let peer = frame.dst;
        if !self.shared_state.has_session_for(peer) {
            if self.outgoing.wait(frame) {
                inc!("lp_queue_overflow_dropped");
            }
            self.request_session(peer);
            return Ok(());
        }

        // anything held back for this peer goes first, so it is not overtaken now the session is up
        self.release_waiting(peer, Instant::now())?;
        self.send_frame(frame)
    }

    /// Look again at every peer with frames waiting for a session: send them once it exists,
    /// otherwise ask for it again and drop what has waited past the stall timeout. Fails only if
    /// the outgoing channel is closed.
    fn handle_waiting_frames(&mut self, now: Instant) -> Result<(), LpDataHandlerError> {
        for peer in self.outgoing.waiting_peers() {
            if self.shared_state.has_session_for(peer) {
                self.release_waiting(peer, now)?;
            } else {
                self.request_session(peer);
                self.drop_stalled(peer, now);
            }
        }

        Ok(())
    }

    /// A peer was given a sending session: release whatever was waiting on it. Fails only if the
    /// outgoing channel is closed.
    ///
    /// Every waiting address the peer is reached on is released, as a node's session is shared
    /// across ports. Each is still checked, since a client is not sendable until its address is
    /// known, and the session may have been evicted since it was announced. An announcement the
    /// store dropped is made up for by the next recheck.
    fn handle_established_session(&mut self, ready_peer: LpPeer) -> Result<(), LpDataHandlerError> {
        if !self.outgoing.has_waiting() {
            return Ok(());
        }

        let now = Instant::now();
        // We don't expect the map to ever be big, so scanning through it is fine
        for peer in self.outgoing.waiting_peers() {
            if LpPeer::from(peer) == ready_peer && self.shared_state.has_session_for(peer) {
                self.release_waiting(peer, now)?;
            }
        }
        Ok(())
    }

    /// Hand a packet off the wire to the workers, which decrypt it before mixing.
    fn handle_wire_packet(&self, packet: EncryptedLpPacket, source: SocketAddr) {
        let now = Instant::now();
        self.dispatch_to_workers(WorkerJob::Wire {
            data: TimedData::new(now, packet),
            processing_time: now,
            source,
        });
    }

    /// Hand a frame from a provider in this process to the workers. Same pool as wire traffic, so
    /// it is subject to the same backpressure as everyone else's.
    fn handle_local_frame(&self, frame: LpFrame) {
        self.shared_state.internal_sp_submitted();
        let now = Instant::now();
        self.dispatch_to_workers(WorkerJob::Local {
            data: TimedData::new(now, frame),
            processing_time: now,
        });
    }

    /// Hold every frame the workers produced, as received into `worker_output_buf`, until its
    /// scheduled send time.
    fn handle_worker_output(&mut self) {
        for frame in self.worker_output_buf.drain(..).flatten() {
            self.outgoing.delay(frame);
        }
    }

    async fn handle_next_event(
        &mut self,
        recheck: &mut Interval,
    ) -> Result<(), LpDataHandlerError> {
        // Biased out before in: frames already scheduled leave before more work is taken on.
        tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => info!("LP data handler: received shutdown signal"),

            // Release packets waiting for a session that just came up. Always polled, so
            // announcements do not pile up while nothing waits
            Some(ready_peer) = self.sessions_established_rx.recv() => {
                self.handle_established_session(ready_peer)?
            }

            // Release outgoing packets on schedule
            frame = self.outgoing.next_due() => self.handle_due_frame(frame)?,

            // Expire packets waiting for a session, and ask for it again
            _ = recheck.tick(), if self.outgoing.has_waiting() => {
                self.handle_waiting_frames(Instant::now())?
            }

            // Handle packets done with processing, a batch at a time. Capped so a backlog cannot
            // hold up releases; 0 means every worker is gone
            1.. = self.worker_output_rx.recv_many(&mut self.worker_output_buf, WORKER_OUTPUT_BATCH) => {
                self.handle_worker_output()
            }

            // Handle internal SP packets
            //Some(frame) = self.provider_rxs.recv() => self.handle_local_frame(frame),

            // Handle wire packets
            Some((packet, src)) = self.input_rx.recv() => self.handle_wire_packet(packet, src),
        }
        Ok(())
    }

    pub async fn run(&mut self) {
        info!(
            workers = self.shared_state.lp_config.debug.data_worker_count.get(),
            "Starting LP data handler"
        );

        // Only polled while frames are waiting for a session, so a node whose peers all have one
        // never wakes for it.
        let mut recheck = tokio::time::interval(STALLED_RECHECK_INTERVAL);
        recheck.set_missed_tick_behavior(MissedTickBehavior::Delay);

        while !self.shutdown.is_cancelled() {
            // Only fails if output is gone
            if let Err(err) = self.handle_next_event(&mut recheck).await {
                error!("LP data handler: {err} outside of shutdown");
                self.shutdown.cancel();
            }
        }

        // Workers will stop because we are dropping the receiving channel
        info!("LP data handler shutdown complete");
    }
}

/// What a data worker needs besides its pipeline.
struct DataWorker {
    shared_state: Arc<SharedLpDataState>,
    input_rx: crossbeam_channel::Receiver<WorkerJob>,
    output_tx: mpsc::Sender<WorkerOutput>,

    /// Each worker can raise a dial request of its own: `request` only takes a brief `DashMap`
    /// entry lock and `tokio::spawn`s the handshake, which is safe from a blocking thread as it
    /// still runs inside the runtime context.
    dialer: LpDialer,

    shutdown: nym_task::ShutdownToken,
}
impl DataWorker {
    /// Worker loop: decrypt, then mix and re-frame.
    ///
    /// Only the *inbound* half of the transport layer runs here. The outbound wrap belongs to
    /// [`LpDataHandler::send_frame`], which applies it at release time so that the cleartext counter
    /// it assigns matches send order - see there for why that is an anonymity property.
    ///
    /// Decryption carries no such constraint, so it runs across the pool: it changes nothing
    /// observable, and the receiver's replay window is a 1024-entry bitmap specifically to absorb
    /// out-of-order arrival. Concurrency here reorders by at most the number of workers.
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
    /// The request is a fresh one, made even when this node still holds a session to send to that
    /// peer on: the same symptom follows simultaneous dials, where each side ends up sending on a
    /// session the other demoted and has since dropped. This node's sending session is fine then,
    /// and only a new handshake fixes the direction that is broken.
    ///
    /// Three things bound the damage an attacker can do with forged packets: the dialer ignores any
    /// source that is not already a known LP node, it coalesces per peer, so a flood produces one
    /// handshake rather than one per packet, and it does not replace a session it established
    /// within [`LpDebug::redial_cooldown`]. There is deliberately no reply on the wire - a
    /// "session unknown" packet could not be authenticated (there is no session to authenticate
    /// with) and would therefore be a spoofable teardown primitive.
    ///
    /// Raising this on *arrival* is safe where the outbound path's request is not: the triggering
    /// packet is already visible to any observer, so dialling in response reveals nothing new.
    ///
    /// [`LpDebug::redial_cooldown`]: crate::config::LpDebug::redial_cooldown
    fn run<P>(self, mut pipeline: P)
    where
        P: NymNodeProcessingPipeline<LpFrame, NymNodeRoutingAddress>
            + TransportUnwrap<EncryptedLpPacket, Frame = LpFrame, Error = LpHandlerError>,
    {
        // This loop only exists for two reasons :
        // 1. Input is empty and gone.
        // 2. Output is gone
        // In any case, shutdown should either be called already or be called now

        // In case of panics (though worker should not panic), we're also calling shutdown as to not hide it and run on a shrunken pool
        let _guard = self.shutdown.clone().drop_guard();

        while let Ok(job) = self.input_rx.recv() {
            let job_processing_time = job.processing_time();
            let job_src = job.source();
            let maybe_frame = match job {
                WorkerJob::Wire { data, source, .. } => {
                    self.handle_wire_job(data, source, &mut pipeline)
                }

                // Nothing to decrypt and no peer to place: a provider in this process handed this
                // over whole. `process` begins at the framing unwrap, which is exactly where a
                // frame that never travelled belongs.
                WorkerJob::Local { data, .. } => Some(data),
            };

            if let Some(frame) = maybe_frame {
                // Blocking is fine, we don't want to unclog ourself and process a new packet that will be dropped anyway
                if let Err(e) = self.output_tx.blocking_send(pipeline.process(
                    frame,
                    job_processing_time,
                    job_src,
                )) {
                    trace!(
                        "Failed to send processing data back to handler : {e}. We are probably shutting down"
                    );
                    return;
                }
            }
        }
    }

    /// Strip the transport layer from a packet off the wire, or `None` if it does not unwrap.
    ///
    /// Only a packet that authenticated moves its client's address to `src`: the receiver index is
    /// cleartext, so trusting it before decryption would let anyone redirect a client's traffic.
    fn handle_wire_job<P>(
        &self,
        packet: TimedData<EncryptedLpPacket>,
        src: SocketAddr,
        pipeline: &mut P,
    ) -> Option<TimedData<LpFrame>>
    where
        P: TransportUnwrap<EncryptedLpPacket, Frame = LpFrame, Error = LpHandlerError>,
    {
        let receiver_index = packet.data.outer_header().receiver_idx;

        match pipeline.packet_to_frame(packet.data, packet.timestamp) {
            Ok(frame) => {
                // The packet authenticated against the session, so `src` is where that peer is
                // now. Clients move; this is the only signal that they have.
                self.shared_state
                    .refresh_client_address(receiver_index, src);
                Some(frame)
            }
            Err(LpHandlerError::MissingLpSession { receiver_index }) => {
                debug!(
                    "LP data worker: {src} is sending on session {receiver_index}, which this node does not hold - asking to re-establish"
                );
                self.dialer.request_fresh(src.ip());
                inc!("lp_unknown_session_packets");
                None
            }
            Err(e) => {
                // The session exists and failed to decrypt, or the counter was
                // replayed. Re-establishing would be wrong, so this is only counted.
                warn!("LP data worker: could not unwrap a packet from {src}: {e}");
                inc!("lp_data_packet_errors");
                None
            }
        }
    }
}
