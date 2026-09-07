// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Node-to-node dialing, driven by demand.
//!
//! Sessions are established when something needs one, not ahead of time. Route selection is
//! uniform over a whole mixnet layer, so a node's traffic reaches its entire routable peer set
//! within seconds of coming up - pre-dialing would produce the same fan-out as demand does, just
//! fired in a burst instead of spread by arrival. What needs bounding is the dial *rate*, which
//! [`LpDebug::max_concurrent_handshakes`] does.
//!
//! # Shape
//!
//! [`LpDialer`] is a cheap cloneable handle over a map of peers, and a peer's entry in that map
//! *is* its state: an entry with `in_flight` set means a handshake is running, so concurrent demand
//! coalesces onto it simply by finding it there. Each dial is one task owning that peer's whole
//! attempt - wait out the backoff, take a permit, handshake, publish the result. This mirrors
//! `nym_mixnet_client::client::Client`, which solves the same problem for the legacy stack.
//!
//! # Two ways to ask
//!
//! [`LpDialer::request`] is fire-and-forget, for callers that only want to nudge a session into
//! existence - the data plane's release-time send path and its unknown-index path.
//! [`LpDialer::ensure_session`] resolves once a session exists, for callers that cannot proceed
//! without one. Both land on the same entry, so a hint and an `ensure_session` for one peer share a
//! single handshake regardless of which arrived first.
//!
//! # Timing
//!
//! The data plane must only raise a request at a packet's *release* time, never on arrival. A dial
//! is a loud, unmistakable outbound event; triggered on arrival it would be an undelayed outbound
//! event correlated 1:1 with an inbound packet, handing an observer the packet's next hop and
//! defeating the mixing delay. Raised at release time it coincides with the packet's own scheduled
//! departure and reveals nothing further.

use std::marker::PhantomData;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_task::ShutdownToken;
use rand::{Rng, rngs::OsRng};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::{Semaphore, watch};
use tracing::{debug, info, trace, warn};

use crate::config::LpDebug;
use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::control::egress::connection::InitialLpEgressNodeConnectionHandler;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::state::SharedLpNodeControlState;

/// Why a dial could not produce a session.
/// We can't reuse LpHandlerError here because we need the Clone bound
#[derive(Debug, Clone, Error)]
pub enum LpDialError {
    #[error("{ip_addr} is not a known LP node")]
    NotLpNode { ip_addr: IpAddr },

    #[error("the KKT/PSQ handshake with {peer_ip} failed")]
    HandshakeFailed { peer_ip: IpAddr },

    #[error("the node is shutting down")]
    ShuttingDown,
}

/// Channels to signal dial outcome; `None` means still running.
type DialResultReceiver = watch::Receiver<Option<Result<LpReceiverIndex, LpDialError>>>;
type DialResultSender = watch::Sender<Option<Result<LpReceiverIndex, LpDialError>>>;

/// What is known about one peer's dialing.
///
/// Held in a [`DashMap`] keyed by peer, and the entry is the whole state machine: present with
/// `in_flight` set means a handshake is running, which is what makes coalescing free.
#[derive(Default)]
struct PeerDial {
    /// Set while a handshake is running; awaiting it yields that attempt's outcome.
    in_flight: Option<DialResultReceiver>,

    /// Outlives the attempt that incremented it, so the backoff escalates across attempts.
    consecutive_failures: u32,

    /// When the current backoff interval expires. `None` while there is no failure streak.
    retry_at: Option<Instant>,
}

/// Establishes node-to-node sessions on demand.
///
/// Cloneable and cheap; clones share the same peer map and handshake budget. Generic over the
/// channel so it can be driven without sockets - see [`LpTransportChannel`].
pub struct LpDialer<S = TcpStream> {
    state: SharedLpNodeControlState,

    /// One entry per peer ever dialled. See [`PeerDial`].
    dials: Arc<DashMap<IpAddr, PeerDial>>,

    /// Caps handshakes in flight across all peers.
    permits: Arc<Semaphore>,

    backoff_initial: Duration,
    backoff_max: Duration,

    shutdown: ShutdownToken,

    _channel: PhantomData<S>,
}

// cloneable regardless of S.
// a derived `Clone` would require `S: Clone`, which isn't needed to be cloned
impl<S> Clone for LpDialer<S> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            dials: self.dials.clone(),
            permits: self.permits.clone(),
            backoff_initial: self.backoff_initial,
            backoff_max: self.backoff_max,
            shutdown: self.shutdown.clone(),
            _channel: PhantomData,
        }
    }
}

impl<S> LpDialer<S>
where
    S: LpHandshakeChannel + LpTransportChannel + Unpin + Send + 'static,
{
    pub(crate) fn new(
        state: SharedLpNodeControlState,
        cfg: &LpDebug,
        shutdown: ShutdownToken,
    ) -> Self {
        Self {
            state,
            dials: Arc::new(DashMap::new()),
            permits: Arc::new(Semaphore::new(cfg.max_concurrent_handshakes)),
            backoff_initial: cfg.dial_backoff_initial,
            backoff_max: cfg.dial_backoff_max,
            shutdown,
            _channel: PhantomData,
        }
    }

    /// Signal that a session with `peer_ip` is wanted, without waiting for one.
    ///
    /// Never blocks and never reports failure. That is what the data plane's release-time send path
    /// needs: it drives every peer from one loop, so it cannot afford to wait on a handshake for
    /// any one of them. See [`Self::ensure_session`] for the awaitable form.
    pub fn request(&self, peer_ip: IpAddr) {
        if let Err(err) = self.dial(peer_ip) {
            trace!("LP dialer: not dialing {peer_ip}: {err}");
        }
    }

    /// Resolve to the receiver index `peer_ip` is reachable on, dialing first if there is no
    /// session yet.
    ///
    /// Returns immediately when a session already exists. Concurrent callers for one peer coalesce
    /// onto a single handshake and all receive its index, and a call sharing a peer with a
    /// hint-driven dial joins that one.
    ///
    /// The index is the one that was current when the handshake completed. Rotation can supersede
    /// it, so a caller holding it across time should resolve the peer again rather than assume it
    /// still sends.
    ///
    /// A peer that is backing off after a failure is waited out, so this can block for as long as
    /// [`LpDebug::dial_backoff_max`]. A caller that cannot wait that long should use
    /// [`Self::request`], or wrap this in a timeout.
    pub async fn ensure_session(&self, peer_ip: IpAddr) -> Result<LpReceiverIndex, LpDialError> {
        let mut outcome = self.dial(peer_ip)?;

        loop {
            if let Some(result) = outcome.borrow_and_update().clone() {
                return result;
            }

            // the only sender is the dial task, so a closed channel means it died without
            // publishing - which happens on shutdown
            if outcome.changed().await.is_err() {
                return Err(LpDialError::ShuttingDown);
            }
        }
    }

    /// Start a handshake with `peer_ip` unless one is already running or unnecessary.
    ///
    /// The returned channel carries the receiver index this peer is reachable on, whether that
    /// comes from an existing session, a handshake already in flight, or one started here.
    fn dial(&self, peer_ip: IpAddr) -> Result<DialResultReceiver, LpDialError> {
        // both the session map and the directory are keyed canonically
        let peer_ip = peer_ip.to_canonical();

        // a session already exists, so the answer is known: hand back a resolved channel. The
        // sender is dropped immediately, which is fine - the value is readable without it.
        if let Some(receiver_index) = self
            .state
            .shared
            .sessions
            .sending_index_for(LpPeer::node(peer_ip))
        {
            let (_, resolved) = watch::channel(Some(Ok(receiver_index)));
            return Ok(resolved);
        }

        // We can only dial a known LP Node
        let Some(details) = self.state.nodes.get_node_details(peer_ip) else {
            return Err(LpDialError::NotLpNode { ip_addr: peer_ip });
        };

        let mut entry = self.dials.entry(peer_ip).or_default();

        // this is the whole of the coalescing: a handshake is already running, so join it
        if let Some(in_flight) = &entry.in_flight {
            return Ok(in_flight.clone());
        }

        let (dial_tx, dial_rx) = watch::channel(None);
        entry.in_flight = Some(dial_rx.clone());

        // only the part of the interval still outstanding, so a peer left alone for longer than
        // its backoff asked for is dialled immediately
        let backoff = entry
            .retry_at
            .map(|retry_at| retry_at.saturating_duration_since(Instant::now()))
            .unwrap_or_default();

        // the guard must not be held across the spawn, and nothing below needs it
        drop(entry);

        tokio::spawn(self.clone().run_dial(peer_ip, details, backoff, dial_tx));

        Ok(dial_rx)
    }

    /// Dial `peer_ip`, record what happened, and publish the outcome to everyone waiting on it.
    async fn run_dial(
        self,
        peer_ip: IpAddr,
        details: LpNodeDetails,
        backoff: Duration,
        result_tx: DialResultSender,
    ) {
        let remote = SocketAddr::new(peer_ip, details.control_port);

        let result = attempt_dial::<S>(
            self.state.clone(),
            remote,
            details,
            backoff,
            self.permits.clone(),
            self.shutdown.clone(),
        )
        .await;

        // Record the outcome before publishing, so a waiter that is woken immediately sees a
        // consistent entry.
        if let Some(mut entry) = self.dials.get_mut(&peer_ip) {
            entry.in_flight = None;
            match &result {
                Ok(_) => {
                    entry.consecutive_failures = 0;
                    entry.retry_at = None;
                }
                Err(_) => {
                    let now = Instant::now();

                    // A failure arriving long after the previous interval expired starts a fresh
                    // streak: the peer went untried for longer than the backoff asked for, so the
                    // old streak says nothing about its reachability. Only a success clears the
                    // count otherwise, and the wait runs *before* the attempt that could produce
                    // one - so a single bad spell would keep imposing its final interval on every
                    // later attempt.
                    if entry
                        .retry_at
                        .is_some_and(|at| now.saturating_duration_since(at) > self.backoff_max)
                    {
                        entry.consecutive_failures = 0;
                    }

                    entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
                    entry.retry_at = Some(
                        now + backoff_delay(
                            entry.consecutive_failures,
                            self.backoff_initial,
                            self.backoff_max,
                            &mut OsRng,
                        ),
                    );
                }
            }
        }

        // fails only if every waiter has gone away, which is fine
        let _ = result_tx.send(Some(result));
    }
}

/// The interval a streak of `consecutive_failures` earns, measured from the failure that ended it.
///
/// Exponential in the failure count and capped, with jitter: an epoch transition lands on every
/// node in the network at the same instant, so un-jittered retries would have the whole mixnet
/// redialing in lockstep. Zero for an unblemished peer, so the common case is not delayed at all.
fn backoff_delay(
    consecutive_failures: u32,
    initial: Duration,
    max: Duration,
    rng: &mut impl Rng,
) -> Duration {
    if consecutive_failures == 0 {
        return Duration::ZERO;
    }

    let exponential = initial
        .saturating_mul(1u32 << consecutive_failures.min(16).saturating_sub(1))
        .min(max);

    // equal jitter: half the interval fixed, half random
    exponential.mul_f64(rng.gen_range(0.5..=1.0))
}

/// Wait out `backoff`, take a handshake permit, then open a control connection and complete the
/// mutual KKT/PSQ handshake.
///
/// The session is stored by the handshake itself; the connection carries nothing else.
async fn attempt_dial<S>(
    state: SharedLpNodeControlState,
    remote: SocketAddr,
    details: LpNodeDetails,
    backoff: Duration,
    permits: Arc<Semaphore>,
    shutdown: ShutdownToken,
) -> Result<LpReceiverIndex, LpDialError>
where
    S: LpHandshakeChannel + LpTransportChannel + Unpin,
{
    let node_id = details.node_id;
    let peer_ip = remote.ip();

    if !backoff.is_zero() {
        debug!("LP dialer: holding off {backoff:?} before redialing {peer_ip}");
        tokio::select! {
            _ = tokio::time::sleep(backoff) => {}
            _ = shutdown.cancelled() => return Err(LpDialError::ShuttingDown),
        }
    }

    // Queue for a permit rather than giving up when the pool is saturated: waiting is not a dial
    // failure, and treating it as one would arm this peer's backoff over a purely local condition.
    // Tasks parked here are bounded by the peer count, since the in-flight guard admits one per
    // peer.
    let Ok(_permit) = permits.acquire().await else {
        return Err(LpDialError::ShuttingDown);
    };

    let mut stream = match S::connect(remote).await {
        Ok(stream) => stream,
        Err(err) => {
            debug!("LP dialer: failed to connect to node {node_id} at {remote}: {err}");
            return Err(LpDialError::HandshakeFailed { peer_ip });
        }
    };

    // matches the ingress side, so handshake messages are not coalesced
    if let Err(err) = stream.set_no_delay(true) {
        warn!("LP dialer: failed to disable Nagle for {remote}: {err}");
    }

    let handler = InitialLpEgressNodeConnectionHandler::new(stream, remote, details, state);

    match handler.complete_initial_handshake().await {
        Ok(receiver_index) => {
            info!(
                "LP dialer: established session {receiver_index} with node {node_id} at {remote}"
            );
            Ok(receiver_index)
        }
        Err(err) => {
            warn!("LP dialer: handshake with node {node_id} at {remote} failed: {err}");
            Err(LpDialError::HandshakeFailed { peer_ip })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::lp::SharedLpState;
    use crate::node::lp::directory::LpNodes;
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_topology::NodeId;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;

    fn peer(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, n))
    }

    /// A dialer whose directory contains exactly `known` peers.
    ///
    /// Instantiated over [`MockIOStream`] so nothing here touches a socket. The mock's `connect`
    /// hands back an unpaired stream, so these tests assert on what `dial` records; a completing
    /// handshake needs a mock that pairs the two halves.
    fn dialer_knowing(known: &[IpAddr]) -> LpDialer<MockIOStream> {
        use nym_lp::peer::mock_peers;
        use nym_lp_data::packet::version;

        let (local, remote) = mock_peers();
        let remote = remote.as_remote();

        let nodes: HashMap<IpAddr, LpNodeDetails> = known
            .iter()
            .enumerate()
            .map(|(i, ip)| {
                let details = LpNodeDetails::new(
                    i as NodeId,
                    remote.kem_key_digests().clone(),
                    *remote.x25519(),
                    41264,
                    51264,
                    version::CURRENT,
                );
                (*ip, details)
            })
            .collect();

        let state = SharedLpNodeControlState {
            local_lp_peer: local,
            nodes: LpNodes::new(nodes),
            shared: SharedLpState {
                sessions: Default::default(),
                metrics: Default::default(),
                lp_config: Default::default(),
            },
        };

        LpDialer::new(state, &LpDebug::default(), ShutdownToken::new())
    }

    /// A flood of requests for one peer produces exactly one handshake.
    ///
    /// This is what keeps a down peer from turning every packet addressed to it into a dial
    /// attempt.
    #[tokio::test]
    async fn requests_for_one_peer_are_coalesced() {
        let dialer = dialer_knowing(&[peer(1)]);

        for _ in 0..1_000 {
            dialer.request(peer(1));
        }

        assert_eq!(dialer.dials.len(), 1);
        assert!(dialer.dials.get(&peer(1)).unwrap().in_flight.is_some());
    }

    /// Coalescing is per peer, so one busy peer does not hold up others.
    #[tokio::test]
    async fn different_peers_dial_independently() {
        let dialer = dialer_knowing(&[peer(1), peer(2)]);

        dialer.request(peer(1));
        dialer.request(peer(2));

        assert_eq!(dialer.dials.len(), 2);
    }

    /// A source that is not a known LP node cannot make this node dial.
    ///
    /// This is the guard on the unknown-session path: an inbound packet naming a session this node
    /// does not hold raises a request for its *source*, and the source is whatever an attacker put
    /// in the IP header. Without this, spraying forged packets would be a way to make a node open
    /// PQ handshakes against arbitrary addresses.
    #[tokio::test]
    async fn an_unknown_source_cannot_make_us_dial() {
        let dialer = dialer_knowing(&[]);

        dialer.request(peer(9));

        assert!(
            dialer.dials.is_empty(),
            "an unrecognised source must not even be recorded"
        );
        assert!(matches!(
            dialer.dial(peer(9)),
            Err(LpDialError::NotLpNode { .. })
        ));
    }

    /// The same request for a peer that *is* in the directory does dial, so the guard above is
    /// rejecting on directory membership rather than rejecting everything.
    #[tokio::test]
    async fn a_known_source_is_dialled() {
        let dialer = dialer_knowing(&[peer(9)]);

        dialer.request(peer(9));

        assert!(dialer.dials.get(&peer(9)).unwrap().in_flight.is_some());
    }

    /// A peer that already has a session is not dialled again.
    #[tokio::test]
    async fn an_existing_session_short_circuits() {
        use nym_lp::SessionsMock;

        let dialer = dialer_knowing(&[peer(1)]);
        let session = SessionsMock::mock_seeded_post_handshake(1, nym_lp::KEM::MlKem768).initiator;
        let receiver_index = session.receiver_index();
        dialer
            .state
            .shared
            .sessions
            .insert_addressed_session(LpPeer::node(peer(1)), session)
            .unwrap();

        // resolved without dialing, and carrying the index the session is already reachable on
        let outcome = dialer.dial(peer(1)).unwrap();
        assert!(matches!(*outcome.borrow(), Some(Ok(index)) if index == receiver_index));
        assert!(dialer.dials.is_empty(), "no dial should have been recorded");
    }

    /// The first attempt is immediate; later ones grow and stay under the ceiling.
    ///
    /// The escalation is the part worth pinning: it only works because the failure count outlives
    /// the attempt that incremented it.
    #[test]
    fn backoff_is_zero_then_grows_and_is_capped() {
        let cfg = LpDebug::default();
        let mut rng = OsRng;

        assert_eq!(
            backoff_delay(0, cfg.dial_backoff_initial, cfg.dial_backoff_max, &mut rng),
            Duration::ZERO,
            "a first attempt must not be delayed"
        );

        let mut previous = Duration::ZERO;
        for failures in 1..20 {
            let delay = backoff_delay(
                failures,
                cfg.dial_backoff_initial,
                cfg.dial_backoff_max,
                &mut rng,
            );

            assert!(
                delay <= cfg.dial_backoff_max,
                "backoff {delay:?} exceeded the ceiling"
            );

            // jitter makes this non-monotonic step to step, so only assert the trend early on
            if failures < 5 {
                assert!(delay > previous / 2, "backoff should be growing");
            }
            previous = delay;
        }
    }
}
