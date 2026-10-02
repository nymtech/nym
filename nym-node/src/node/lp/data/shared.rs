// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::error::LpHandlerError;
use crate::{
    config::{Config, LpConfig},
    node::{
        key_rotation::active_keys::{ActiveSphinxKeys, SphinxKeyGuard},
        lp::active_sessions::{ActiveLpSessions, LpPeer},
        lp::data::handler::error::LpDataHandlerError,
        replay_protection::bloomfilter::ReplayProtectionBloomfilters,
        routing_filter::network_filter::NetworkRoutingFilter,
        shared_network::CachedFullTopology,
    },
};
use nym_gateway::node::{ClientRegistry, EmbeddedServiceProviders};
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_lp_data::{PipelinePayload, fragmentation::reconstruction::MessageReconstructor};
use nym_node_metrics::{NymNodeMetrics, mixnet::PacketKind};
use nym_sphinx_addressing::{ClientAddress, nodes::NymNodeRoutingAddress};
use nym_sphinx_framing::processing::PacketProcessingError;
use nym_sphinx_params::SphinxKeyRotation;
use nym_task::ShutdownToken;
#[cfg(feature = "mix-sim")]
use std::collections::HashMap;
use std::{net::SocketAddr, time::Duration};
use tracing::{Span, warn};

#[derive(Clone, Copy)]
pub struct ProcessingConfig {
    pub(crate) maximum_packet_delay: Duration,
    pub(crate) client_forwarding_enabled: bool,
}

impl ProcessingConfig {
    pub(crate) fn new(config: &Config) -> Self {
        ProcessingConfig {
            maximum_packet_delay: config.mixnet.debug.maximum_forward_packet_delay,
            client_forwarding_enabled: config.modes.expects_client_traffic(),
        }
    }
}

/// Shared state for LP data connections
pub struct SharedLpDataState {
    /// LP configuration (for timestamp validation, etc.)
    pub(crate) lp_config: LpConfig,

    pub(crate) processing_config: ProcessingConfig,

    pub(crate) sphinx_keys: ActiveSphinxKeys,

    pub(crate) replay_protection_filter: ReplayProtectionBloomfilters,

    pub(crate) message_reconstructor: MessageReconstructor,

    pub(crate) routing_filter: NetworkRoutingFilter,

    /// Every established LP session, clients and nodes alike, established by the control plane and
    /// consumed here: the outbound wrap resolves one by peer, the inbound unwrap by receiver index.
    pub sessions: ActiveLpSessions,

    /// Where each registered client currently is.
    pub(crate) clients: ClientRegistry,

    /// Metrics collection
    pub(crate) metrics: NymNodeMetrics,

    pub(crate) shutdown_token: ShutdownToken,
}

impl SharedLpDataState {
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: &Config,
        sphinx_keys: ActiveSphinxKeys,
        replay_protection_filter: ReplayProtectionBloomfilters,
        routing_filter: NetworkRoutingFilter,
        sessions: ActiveLpSessions,
        clients: ClientRegistry,
        metrics: NymNodeMetrics,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SharedLpDataState {
            processing_config: ProcessingConfig::new(config),
            lp_config: config.lp,
            sphinx_keys,
            replay_protection_filter,
            message_reconstructor: Default::default(),
            routing_filter,
            sessions,
            clients,
            metrics,
            shutdown_token,
        }
    }

    /// Whether a session exists to encrypt towards `dst`.
    ///
    /// Checked before wrapping, since the wrap consumes the frame and a peer that has simply not
    /// been dialled yet is routine.
    pub(crate) fn has_session_for(&self, dst: NymNodeRoutingAddress) -> bool {
        match dst {
            NymNodeRoutingAddress::Node(addr) => {
                self.sessions.has_session_for(LpPeer::node(addr.ip()))
            }
            NymNodeRoutingAddress::Client(client) => {
                self.sessions.has_session_for(LpPeer::client(client))
                    && self.clients.last_seen(client).is_some()
            }
        }
    }

    /// Encrypt `frame` on the session that reaches `dst`, and say where it goes on the wire.
    ///
    /// Each kind of peer is addressed the only way it can be: a node by the IP the control plane
    /// dialled, a client by the [`ClientAddress`] it registered under. A client's socket address is
    /// merely where it was last seen - it identifies nothing, and is never used to pick a session.
    pub(crate) fn send_frame(
        &self,
        dst: NymNodeRoutingAddress,
        frame: LpFrame,
    ) -> Result<(EncryptedLpPacket, SocketAddr), LpHandlerError> {
        match dst {
            NymNodeRoutingAddress::Node(addr) => {
                let packet = self.sessions.send_frame(LpPeer::node(addr.ip()), frame)?;
                Ok((packet, addr))
            }
            NymNodeRoutingAddress::Client(client) => {
                // resolved here rather than at routing time, so a client that moved while the
                // frame waited for its release time is still reached
                let seen_at = self.clients.last_seen(client).ok_or_else(|| {
                    LpHandlerError::NoSessionForPeer {
                        peer: client.to_string(),
                    }
                })?;
                let packet = self.sessions.send_frame(LpPeer::client(client), frame)?;
                Ok((packet, seen_at))
            }
        }
    }

    /// Decrypt a packet on the session its outer header names.
    pub(crate) fn receive_packet(
        &self,
        packet: EncryptedLpPacket,
    ) -> Result<LpFrame, LpHandlerError> {
        self.sessions.receive_packet(packet)
    }

    /// Record that the session named by `receiver_index` was last heard from at `src`.
    ///
    /// A client's address is not stable - it is only ever whatever the last packet came from - so
    /// this is what keeps node→client traffic deliverable. Node peers are addressed by the IP the
    /// control plane dialled and need no refresh.
    pub(crate) fn refresh_client_address(&self, receiver_index: LpReceiverIndex, src: SocketAddr) {
        if let Some(LpPeer::Client(client)) = self.sessions.peer_for(receiver_index) {
            self.clients.refresh(client, src);
        }
    }

    /// Resolve the sphinx key for the given rotation, recording the rotation
    /// label on the current tracing span.  Returns `ExpiredKey` if the requested
    /// odd/even key has already been rotated out.
    pub(crate) fn resolve_rotation_key(
        &self,
        rotation: SphinxKeyRotation,
    ) -> Result<SphinxKeyGuard, PacketProcessingError> {
        let rotation_label = match rotation {
            SphinxKeyRotation::Unknown => "unknown",
            SphinxKeyRotation::OddRotation => "odd",
            SphinxKeyRotation::EvenRotation => "even",
        };
        Span::current().record("key_rotation", rotation_label);

        match rotation {
            SphinxKeyRotation::Unknown => Ok(self.sphinx_keys.primary()),
            SphinxKeyRotation::OddRotation => self.sphinx_keys.odd().ok_or_else(|| {
                warn!(
                    event = "packet.dropped.expired_key",
                    key_rotation = "odd",
                    "dropping packet: odd key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
            SphinxKeyRotation::EvenRotation => self.sphinx_keys.even().ok_or_else(|| {
                warn!(
                    event = "packet.dropped.expired_key",
                    key_rotation = "even",
                    "dropping packet: even key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
        }
    }

    pub(super) fn malformed_packet(&self) {
        self.metrics.mixnet.lp_malformed_packet()
    }

    pub(super) fn message_received(&self, message_kind: impl Into<PacketKind>) {
        self.metrics.mixnet.lp_message_received(message_kind.into())
    }

    pub(super) fn packet_forwarded(&self, dst: SocketAddr) {
        self.metrics.mixnet.lp_packet_forwarded(dst)
    }

    pub(super) fn packet_received(&self, src: SocketAddr) {
        self.metrics.mixnet.lp_packet_received(src)
    }

    pub(super) fn egress_overloaded_packet_dropped(&self) {
        self.metrics
            .mixnet
            .lp_egress_overloaded_packets_dropped_packets()
    }
    pub(super) fn pipeline_overloaded_packet_dropped(&self) {
        self.metrics.mixnet.lp_pipeline_overloaded_dropped_packets()
    }

    pub(super) fn worker_pool_overloaded_packet_dropped(&self) {
        self.metrics
            .mixnet
            .lp_worker_pool_overloaded_dropped_packets()
    }

    pub(super) fn excessive_delay_packet(&self) {
        self.metrics.mixnet.lp_excessive_delay_packet()
    }

    pub(super) fn routing_filter_dropped(&self, dst: SocketAddr) {
        self.metrics.mixnet.lp_routing_filter_dropped(dst)
    }

    pub(super) fn client_forwarding_disabled_dropped(&self) {
        self.metrics.mixnet.lp_client_forwarding_disabled_dropped()
    }

    pub(super) fn internal_sp_routed(&self) {
        self.metrics.mixnet.lp_internal_sp_routed()
    }

    pub(super) fn update_processing_metrics(
        &self,
        processing_result: &Result<
            PipelinePayload<impl Clone + Into<PacketKind>, NymNodeRoutingAddress>,
            LpDataHandlerError,
        >,
    ) {
        match processing_result {
            Ok(packet) => {
                self.metrics
                    .mixnet
                    .lp_processed_message(packet.options.clone().into());
            }
            Err(LpDataHandlerError::PacketProcessingError(PacketProcessingError::PacketReplay)) => {
                self.metrics.mixnet.lp_processing_replayed_packet();
            }
            Err(LpDataHandlerError::FinalHop) => {
                self.metrics.mixnet.lp_processing_dropped_final_hop_packet();
            }
            Err(_) => {
                self.metrics.mixnet.lp_processing_misc_error();
            }
        }
    }
}

/// Gateway-only shared state for LP data processing.
///
/// Only constructed and consumed when the node operates in a client-forwarding
/// role (entry/exit).
pub struct SharedGatewayLpDataState {
    pub(crate) cached_topology: CachedFullTopology,

    /// Service providers this node runs itself, reached by channel rather than over the wire.
    pub(crate) service_providers: EmbeddedServiceProviders,
}

impl SharedGatewayLpDataState {
    pub(crate) fn new(
        cached_topology: CachedFullTopology,
        service_providers: EmbeddedServiceProviders,
    ) -> Self {
        Self {
            cached_topology,
            service_providers,
        }
    }

    pub(super) fn is_internal_service_provider(&self, client_address: ClientAddress) -> bool {
        self.service_providers.hosts(client_address)
    }
}

#[cfg(feature = "mix-sim")]
impl SharedLpDataState {
    /// Build a [`SharedLpDataState`] for use in a discrete simulator.
    ///
    /// Initialises the state with:
    /// - the provided x25519 private key as the only sphinx key (rotation 0),
    /// - replay protection disabled,
    /// - the testnet routing filter (allows all destinations),
    /// - client forwarding enabled,
    /// - empty session stores and client registry, which the driver fills during setup,
    /// - fresh metrics and a never-firing shutdown token.
    pub fn new_for_simulation(
        sphinx_private_key: nym_crypto::asymmetric::x25519::PrivateKey,
        clients: HashMap<ClientAddress, SocketAddr>,
    ) -> Self {
        use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
        use crate::node::key_rotation::key::SphinxPrivateKey;
        use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
        use crate::node::routing_filter::network_filter::NetworkRoutingFilter;

        let primary = SphinxPrivateKey::import(sphinx_private_key, 0);
        SharedLpDataState {
            lp_config: LpConfig::default(),
            processing_config: ProcessingConfig {
                maximum_packet_delay: Duration::from_secs(10),
                client_forwarding_enabled: true,
            },
            sphinx_keys: ActiveSphinxKeys::new_loaded(primary, None),
            replay_protection_filter: ReplayProtectionBloomfilters::new_disabled(),
            message_reconstructor: MessageReconstructor::default(),
            routing_filter: NetworkRoutingFilter::new_empty(true),
            sessions: ActiveLpSessions::new(),
            clients: ClientRegistry::from_iter(clients),
            metrics: NymNodeMetrics::default(),
            shutdown_token: ShutdownToken::new(),
        }
    }
}

#[cfg(feature = "mix-sim")]
impl SharedGatewayLpDataState {
    pub fn new_for_simulation(topology: nym_topology::NymTopology) -> Self {
        Self {
            cached_topology: CachedFullTopology::from_topology(topology),
            service_providers: Default::default(),
        }
    }
}
