// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! How a session with a gateway comes to exist.
//!
//! One caller so far: the home gateway is dialled at startup, and awaited, so that a client knows
//! on the way up whether it has a session. Nothing raises a dial after that - the data plane drops
//! what it cannot address rather than asking - so [`LpGatewayDialer::request`] and everything that
//! makes several dials coexist is structure waiting for its callers.
//!
//! # Shape
//!
//! Ported from the node's `LpDialer`, which solves the same problem between nodes. [`LpGatewayDialer`]
//! is a cheap cloneable handle over a map of gateways, and a gateway's entry in that map *is* its
//! state: an entry with `in_flight` set means a handshake is running, so concurrent demand
//! coalesces onto it simply by finding it there. Each dial is one task owning that gateway's whole
//! attempt - wait out the backoff, take a permit, handshake, register, publish the result.
//!
//! # Timing
//!
//! Unlike a node, a client originates its own traffic and mixes nothing. So whenever a dialing arrives, it can be raised immediatley.
//!
//! # What a dial leaves behind
//!
//! A session, and nothing else. The control connection is a handshake carrier: it is opened by the
//! dial task, used for the handshake and the registration that names the session, and closed before
//! the task ends. What outlives it is the entry in [`LpGatewaySessions`], which the data plane
//! reads without ever knowing a dial happened.

use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use nym_crypto::asymmetric::ed25519;
use nym_lp::peer::{DHKeyPair, LpLocalPeer};
use nym_lp::psq::initiator::HandshakeMode;
use nym_lp::transport::{LpHandshakeChannel, LpTransportChannel};
use nym_lp_gateway_client::{
    LpConnectionDetails, LpGatewayControlClient, LpMixnetRegistrationClient,
};
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_task::ShutdownToken;
use nym_topology::NymTopologyError;
use rand::{Rng, rngs::OsRng};
use tokio::net::TcpStream;
use tokio::sync::{Semaphore, watch};
use tracing::{debug, info, trace, warn};

use crate::client::lp::control::LpControlError;
use crate::client::lp::data::shared::{LpGatewaySession, LpGatewaySessions};
use crate::client::topology_control::TopologyAccessor;
use crate::config::LewesProtocol;
use crate::error::ClientCoreError;

/// What a dial produced: the address the gateway takes data on, or why it could not.
type DialResult = Result<SocketAddr, LpControlError>;

/// Channels to signal dial outcome; `None` means still running.
type DialResultReceiver = watch::Receiver<Option<DialResult>>;
type DialResultSender = watch::Sender<Option<DialResult>>;

/// What is known about one gateway's dialing.
///
/// Held in a [`DashMap`] keyed by gateway, and the entry is the whole state machine: present with
/// `in_flight` set means a handshake is running, which is what makes coalescing free.
#[derive(Default)]
struct GatewayDial {
    /// Set while a handshake is running; awaiting it yields that attempt's outcome.
    in_flight: Option<DialResultReceiver>,

    /// Outlives the attempt that incremented it, so the backoff escalates across attempts.
    consecutive_failures: u32,

    /// When the current backoff interval expires. `None` while there is no failure streak.
    retry_at: Option<Instant>,
}

/// Establishes sessions with gateways on demand.
///
/// Cloneable and cheap; clones share the same gateway map and handshake budget. Generic over the
/// channel so it can be driven without sockets - see [`LpTransportChannel`].
pub struct LpGatewayDialer<S = TcpStream> {
    /// Where a gateway's LP details are resolved from, per dial. No cached directory: topology
    /// stays the single source, so a gateway that leaves it stops being dialable.
    topology_accessor: TopologyAccessor,

    /// Where a completed dial files its session, and what the data plane reads.
    sessions: LpGatewaySessions,

    /// This client's LP identity, held for the dialer's lifetime so every session it establishes
    /// shares one - rather than each minting a keypair it then forgets.
    lp_keypair: Arc<DHKeyPair>,

    /// What a gateway fingerprints into the [`ClientAddress`] it registers us under.
    ///
    /// [`ClientAddress`]: nym_sphinx_addressing::ClientAddress
    identity_keys: Arc<ed25519::KeyPair>,

    /// One entry per gateway ever dialled. See [`GatewayDial`].
    dials: Arc<DashMap<NodeIdentity, GatewayDial>>,

    /// Caps handshakes in flight across all gateways.
    permits: Arc<Semaphore>,

    backoff_initial: Duration,
    backoff_max: Duration,

    shutdown: ShutdownToken,

    // as a function pointer rather than a bare `S`, so the dialer is `Send + Sync` whatever the
    // channel is: a dial task holds `&self` across its awaits
    _channel: PhantomData<fn() -> S>,
}

// cloneable regardless of S.
// a derived `Clone` would require `S: Clone`, which isn't needed to be cloned
impl<S> Clone for LpGatewayDialer<S> {
    fn clone(&self) -> Self {
        Self {
            topology_accessor: self.topology_accessor.clone(),
            sessions: self.sessions.clone(),
            lp_keypair: self.lp_keypair.clone(),
            identity_keys: self.identity_keys.clone(),
            dials: self.dials.clone(),
            permits: self.permits.clone(),
            backoff_initial: self.backoff_initial,
            backoff_max: self.backoff_max,
            shutdown: self.shutdown.clone(),
            _channel: PhantomData,
        }
    }
}

impl<S> LpGatewayDialer<S>
where
    S: LpHandshakeChannel + LpTransportChannel + Unpin + Send + 'static,
{
    pub(crate) fn new(
        topology_accessor: TopologyAccessor,
        sessions: LpGatewaySessions,
        identity_keys: Arc<ed25519::KeyPair>,
        config: &LewesProtocol,
        shutdown: ShutdownToken,
    ) -> Self {
        Self {
            topology_accessor,
            sessions,
            lp_keypair: Arc::new(DHKeyPair::new(&mut rand010::rng())),
            identity_keys,
            dials: Arc::new(DashMap::new()),
            permits: Arc::new(Semaphore::new(config.max_concurrent_handshakes)),
            backoff_initial: config.dial_backoff_initial,
            backoff_max: config.dial_backoff_max,
            shutdown,
            _channel: PhantomData,
        }
    }

    /// Signal that a session with `gateway` is wanted, without waiting for one.
    ///
    /// Never blocks and never reports failure, which is what a caller that cannot wait on a
    /// handshake needs - the outbound tick loop, when it comes to raise dials, runs every
    /// millisecond. Nothing calls this yet; see [`Self::ensure_session`] for the awaitable form,
    /// which is what startup uses.
    pub fn request(&self, gateway: NodeIdentity) {
        if let Err(err) = self.dial(gateway) {
            trace!("LP dialer: not dialing {gateway}: {err}");
        }
    }

    /// Resolve to the address `gateway` takes data on, dialing first if there is no session yet.
    ///
    /// Returns immediately when a session already exists. Concurrent callers for one gateway
    /// coalesce onto a single handshake and all receive its address, and a call sharing a gateway
    /// with a hint-driven dial joins that one.
    ///
    /// A gateway that is backing off after a failure is waited out, so this can block for as long
    /// as [`LewesProtocol::dial_backoff_max`]. A caller that cannot wait that long should use
    /// [`Self::request`], or wrap this in a timeout.
    pub async fn ensure_session(&self, gateway: NodeIdentity) -> DialResult {
        let mut outcome = self.dial(gateway)?;

        loop {
            if let Some(result) = outcome.borrow_and_update().clone() {
                return result;
            }

            // the only sender is the dial task, so a closed channel means it died without
            // publishing - which happens on shutdown
            if outcome.changed().await.is_err() {
                return Err(LpControlError::ShuttingDown);
            }
        }
    }

    /// Start a handshake with `gateway` unless one is already running or unnecessary.
    ///
    /// The returned channel carries the address this gateway takes data on, whether that comes
    /// from an existing session, a handshake already in flight, or one started here.
    fn dial(&self, gateway: NodeIdentity) -> Result<DialResultReceiver, LpControlError> {
        // a session already exists, so the answer is known: hand back a resolved channel. The
        // sender is dropped immediately, which is fine - the value is readable without it.
        if let Some(data_address) = self.sessions.data_address(gateway) {
            let (_, resolved) = watch::channel(Some(Ok(data_address)));
            return Ok(resolved);
        }

        // a gateway topology does not describe cannot be dialled, which is also what keeps an
        // identity from anywhere else from making this client open a handshake
        let details = self
            .gateway_details(gateway)
            .map_err(|err| LpControlError::UnreachableGateway(err.to_string()))?;

        let mut entry = self.dials.entry(gateway).or_default();

        // this is the whole of the coalescing: a handshake is already running, so join it
        if let Some(in_flight) = &entry.in_flight {
            return Ok(in_flight.clone());
        }

        let (dial_tx, dial_rx) = watch::channel(None);
        entry.in_flight = Some(dial_rx.clone());

        // only the part of the interval still outstanding, so a gateway left alone for longer than
        // its backoff asked for is dialled immediately
        let backoff = entry
            .retry_at
            .map(|retry_at| retry_at.saturating_duration_since(Instant::now()))
            .unwrap_or_default();

        // the guard must not be held across the spawn, and nothing below needs it
        drop(entry);

        tokio::spawn(self.clone().run_dial(gateway, details, backoff, dial_tx));

        Ok(dial_rx)
    }

    /// How to reach a gateway over LP, as topology currently describes it.
    ///
    /// The counterpart of `SelectedGateway::from_topology_node` for the LP listener: the topology
    /// says which node, the node says how to reach that listener.
    fn gateway_details(
        &self,
        gateway: NodeIdentity,
    ) -> Result<LpConnectionDetails, ClientCoreError> {
        // for now, let's use 'old' behaviour, the same as `SelectedGateway::from_topology_node`
        let prefer_ipv6 = false;

        // holding none at all is the same answer as holding one this gateway is not in: there is
        // nothing that says how to reach it
        let topology = self
            .topology_accessor
            .current_route_provider()
            .ok_or(NymTopologyError::EmptyNetworkTopology)?;

        let node = topology.egress_by_identity(gateway)?;

        Ok(LpConnectionDetails::for_node(node, prefer_ipv6)?)
    }

    /// Dial `gateway`, record what happened, and publish the outcome to everyone waiting on it.
    async fn run_dial(
        self,
        gateway: NodeIdentity,
        details: LpConnectionDetails,
        backoff: Duration,
        result_tx: DialResultSender,
    ) {
        // the only place a dial's failure is reported: a caller that awaited one gets it back, but
        // `request` has nobody to hand it to
        let result = self
            .attempt_dial(gateway, details, backoff)
            .await
            .inspect_err(|err| warn!("LP dialer: dialing gateway {gateway} failed: {err}"));

        // Record the outcome before publishing, so a waiter that is woken immediately sees a
        // consistent entry.
        if let Some(mut entry) = self.dials.get_mut(&gateway) {
            entry.in_flight = None;
            match &result {
                Ok(_) => {
                    entry.consecutive_failures = 0;
                    entry.retry_at = None;
                }
                Err(_) => {
                    let now = Instant::now();

                    // A failure arriving long after the previous interval expired starts a fresh
                    // streak: the gateway went untried for longer than the backoff asked for, so
                    // the old streak says nothing about its reachability. Only a success clears
                    // the count otherwise, and the wait runs *before* the attempt that could
                    // produce one - so a single bad spell would keep imposing its final interval
                    // on every later attempt.
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

    /// Wait out `backoff`, take a handshake permit, then handshake with the gateway and register
    /// for mixnet use over the same connection.
    ///
    /// Registration is not decoration. It is what binds our [`ClientAddress`] to the session on the
    /// gateway; without it the gateway holds a session it can decrypt but cannot address, so
    /// nothing can ever be sent back to us.
    ///
    /// [`ClientAddress`]: nym_sphinx_addressing::ClientAddress
    async fn attempt_dial(
        &self,
        gateway: NodeIdentity,
        details: LpConnectionDetails,
        backoff: Duration,
    ) -> Result<SocketAddr, LpControlError> {
        if !backoff.is_zero() {
            debug!("LP dialer: holding off {backoff:?} before redialing {gateway}");
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = self.shutdown.cancelled() => return Err(LpControlError::ShuttingDown),
            }
        }

        // Queue for a permit rather than giving up when the pool is saturated: waiting is not a
        // dial failure, and treating it as one would arm this gateway's backoff over a purely
        // local condition.
        let Ok(_permit) = self.permits.acquire().await else {
            return Err(LpControlError::ShuttingDown);
        };

        info!(
            "establishing an LP session with gateway {gateway} at {}",
            details.control_address
        );

        let mut channel = LpGatewayControlClient::<S>::new_with_default_config();

        let session = channel
            .handshake(
                details.control_address,
                LpLocalPeer::new(details.ciphersuite, self.lp_keypair.clone()),
                details.peer.clone(),
                details.protocol_version,
                HandshakeMode::OneWayEntry,
            )
            .await
            .map_err(|err| LpControlError::HandshakeFailed(err.to_string()))?;

        let session =
            LpMixnetRegistrationClient::new(&mut channel, details.control_address, session)
                .register(*self.identity_keys.public_key())
                .await
                .map_err(|err| LpControlError::RegistrationFailed(err.to_string()))?;

        // the control connection has done its job; the data plane carries on with the session
        channel.disconnect(details.control_address);

        self.sessions.insert(LpGatewaySession {
            gateway,
            session,
            data_address: details.data_address,
        });

        info!("LP session with gateway {gateway} is registered and ready");

        Ok(details.data_address)
    }
}

/// The interval a streak of `consecutive_failures` earns, measured from the failure that ended it.
///
/// Exponential in the failure count and capped, with jitter: gateways go down and come back for
/// everyone at once, so un-jittered retries would have every client redialing in lockstep. Zero for
/// an unblemished gateway, so the common case is not delayed at all.
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

#[cfg(test)]
mod tests {
    use super::*;
    use nym_api_requests::models::described::type_translation::{
        LPHashFunction, LPKEM, LewesProtocolDetailsDataV1,
    };
    use nym_crypto::asymmetric::x25519;
    use nym_kkt_ciphersuite::{HashFunction, KEM};
    use nym_lp::peer::mock_peers;
    use nym_test_utils::mocks::async_read_write::MockIOStream;
    use nym_topology::{
        CachedEpochRewardedSet, NodeId, NymTopology, NymTopologyMetadata, RoutingNode,
        SupportedRoles,
    };
    use std::collections::BTreeMap;
    use time::OffsetDateTime;

    /// What a gateway would publish about its LP listeners, for a peer whose keys we hold.
    ///
    /// Derived from a mock peer rather than written out, so what a dial resolves is what a real
    /// handshake would have been offered.
    fn published_lp_details() -> LewesProtocolDetailsDataV1 {
        let (_, gateway) = mock_peers();
        let gateway = gateway.as_remote();

        let kem_keys = gateway
            .kem_key_digests()
            .iter()
            .map(|(kem, digests)| {
                let kem = match kem {
                    KEM::MlKem768 => LPKEM::MlKem768,
                    KEM::McEliece => LPKEM::McEliece,
                };

                let digests = digests
                    .iter()
                    .map(|(hash_function, digest)| {
                        let hash_function = match hash_function {
                            HashFunction::Blake3 => LPHashFunction::Blake3,
                            HashFunction::Shake256 => LPHashFunction::Shake256,
                            HashFunction::Shake128 => LPHashFunction::Shake128,
                            HashFunction::SHA256 => LPHashFunction::Sha256,
                        };

                        (hash_function, hex::encode(digest))
                    })
                    .collect();

                (kem, digests)
            })
            .collect::<BTreeMap<_, _>>();

        LewesProtocolDetailsDataV1 {
            enabled: true,
            control_port: 41264,
            data_port: 51264,
            x25519: *gateway.x25519(),
            kem_keys,
        }
    }

    /// One gateway, on an address of its own, publishing LP details or not.
    fn gateway_node(node_id: NodeId, lp: bool) -> RoutingNode {
        RoutingNode {
            node_id,
            mix_host: format!("10.0.0.{node_id}:1789").parse().unwrap(),
            ip_addresses: vec![format!("10.0.0.{node_id}").parse().unwrap()],
            entry: None,
            identity_key: *ed25519::KeyPair::new(&mut OsRng).public_key(),
            sphinx_key: *x25519::KeyPair::new(&mut OsRng).public_key(),
            supported_roles: SupportedRoles {
                mixnode: false,
                mixnet_entry: true,
                mixnet_exit: true,
            },
            lp: lp.then(published_lp_details),
            build_version: Some(semver::Version::new(1, 39, 0)),
        }
    }

    /// A dialer whose topology contains exactly `gateways`, and the identities of those gateways.
    ///
    /// Instantiated over [`MockIOStream`] so nothing here touches a socket. The mock's `connect`
    /// hands back an unpaired stream, so these tests assert on what `dial` records; a completing
    /// handshake needs a mock that pairs the two halves.
    fn dialer_over(gateways: &[bool]) -> (LpGatewayDialer<MockIOStream>, Vec<NodeIdentity>) {
        let nodes: Vec<_> = gateways
            .iter()
            .enumerate()
            .map(|(i, lp)| gateway_node(i as NodeId + 1, *lp))
            .collect();

        let identities = nodes.iter().map(|node| node.identity_key).collect();

        let mut rewarded_set = CachedEpochRewardedSet::default();
        for node in &nodes {
            rewarded_set.entry_gateways.insert(node.node_id);
            rewarded_set.exit_gateways.insert(node.node_id);
        }

        let topology_accessor = TopologyAccessor::new(true);
        topology_accessor.manually_change_topology(NymTopology::new(
            NymTopologyMetadata::new(0, 1, OffsetDateTime::now_utc()),
            rewarded_set,
            nodes,
        ));

        let dialer = LpGatewayDialer::new(
            topology_accessor,
            LpGatewaySessions::default(),
            Arc::new(ed25519::KeyPair::new(&mut OsRng)),
            &LewesProtocol::default(),
            ShutdownToken::new(),
        );

        (dialer, identities)
    }

    /// A flood of requests for one gateway produces exactly one handshake.
    ///
    /// This is what keeps a gateway that is down from turning every message addressed to it into a
    /// dial attempt.
    #[tokio::test]
    async fn requests_for_one_gateway_are_coalesced() {
        let (dialer, gateways) = dialer_over(&[true]);

        for _ in 0..1_000 {
            dialer.request(gateways[0]);
        }

        assert_eq!(dialer.dials.len(), 1);
        assert!(dialer.dials.get(&gateways[0]).unwrap().in_flight.is_some());
    }

    /// Coalescing is per gateway, so one busy gateway does not hold up others.
    ///
    /// The whole point of the scaffolding: nothing in production names a second gateway yet, but
    /// asking for one establishes a session of its own.
    #[tokio::test]
    async fn different_gateways_dial_independently() {
        let (dialer, gateways) = dialer_over(&[true, true]);

        dialer.request(gateways[0]);
        dialer.request(gateways[1]);

        assert_eq!(dialer.dials.len(), 2);
    }

    /// An identity topology does not describe cannot make this client open a handshake.
    #[tokio::test]
    async fn an_unknown_gateway_is_not_dialled() {
        let (dialer, _) = dialer_over(&[true]);
        let stranger = *ed25519::KeyPair::new(&mut OsRng).public_key();

        dialer.request(stranger);

        assert!(
            dialer.dials.is_empty(),
            "an unrecognised gateway must not even be recorded"
        );
        assert!(matches!(
            dialer.dial(stranger),
            Err(LpControlError::UnreachableGateway(_))
        ));
    }

    /// A gateway that is in topology but publishes no LP details is refused the same way, so the
    /// guard above is about what a gateway offers rather than about membership alone.
    #[tokio::test]
    async fn a_gateway_without_lp_details_is_not_dialled() {
        let (dialer, gateways) = dialer_over(&[false]);

        dialer.request(gateways[0]);

        assert!(dialer.dials.is_empty());
    }

    /// A gateway we already hold a session with is not dialled again.
    #[tokio::test]
    async fn an_existing_session_short_circuits() {
        use nym_lp::SessionsMock;

        let (dialer, gateways) = dialer_over(&[true]);
        let data_address = "10.0.0.1:51264".parse().unwrap();

        dialer.sessions.insert(LpGatewaySession {
            gateway: gateways[0],
            session: SessionsMock::mock_seeded_post_handshake(1, KEM::MlKem768).initiator,
            data_address,
        });

        // resolved without dialing, and carrying the address the session already sends to
        let outcome = dialer.dial(gateways[0]).unwrap();
        assert!(matches!(*outcome.borrow(), Some(Ok(address)) if address == data_address));
        assert!(dialer.dials.is_empty(), "no dial should have been recorded");
    }

    /// The first attempt is immediate; later ones grow and stay under the ceiling.
    ///
    /// The escalation is the part worth pinning: it only works because the failure count outlives
    /// the attempt that incremented it.
    #[test]
    fn backoff_is_zero_then_grows_and_is_capped() {
        let config = LewesProtocol::default();
        let mut rng = OsRng;

        assert_eq!(
            backoff_delay(
                0,
                config.dial_backoff_initial,
                config.dial_backoff_max,
                &mut rng
            ),
            Duration::ZERO,
            "a first attempt must not be delayed"
        );

        let mut previous = Duration::ZERO;
        for failures in 1..20 {
            let delay = backoff_delay(
                failures,
                config.dial_backoff_initial,
                config.dial_backoff_max,
                &mut rng,
            );

            assert!(
                delay <= config.dial_backoff_max,
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
