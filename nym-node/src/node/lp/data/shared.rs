// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    config::{Config, LpConfig},
    node::{
        key_rotation::active_keys::{ActiveSphinxKeys, SphinxKeyGuard},
        lp::active_sessions::ActiveLpSessions,
        lp::data::handler::error::LpDataHandlerError,
        replay_protection::bloomfilter::ReplayProtectionBloomfilters,
        routing_filter::network_filter::NetworkRoutingFilter,
        shared_network::CachedFullTopology,
    },
};
use dashmap::DashMap;
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

    /// Node-to-node sessions, established by the control plane and consumed here: the outbound
    /// wrap resolves one by next-hop address, the inbound unwrap by receiver index.
    pub node_sessions: ActiveLpSessions,

    /// Metrics collection
    pub(crate) metrics: NymNodeMetrics,

    pub(crate) shutdown_token: ShutdownToken,
}

impl SharedLpDataState {
    pub(crate) fn new(
        config: &Config,
        sphinx_keys: ActiveSphinxKeys,
        replay_protection_filter: ReplayProtectionBloomfilters,
        routing_filter: NetworkRoutingFilter,
        node_sessions: ActiveLpSessions,
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
            node_sessions,
            metrics,
            shutdown_token,
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
    pub(crate) client_map: DashMap<ClientAddress, SocketAddr>, // SW tmp until proper client wiring is done, something akin to ActiveClientsStore
}

impl SharedGatewayLpDataState {
    pub(crate) fn new(cached_topology: CachedFullTopology) -> Self {
        Self {
            cached_topology,
            client_map: Default::default(),
        }
    }

    // SW Placeholder for SP routing while we don't have gateway state
    pub(super) fn is_internal_service_provider(&self, _client_address: ClientAddress) -> bool {
        false
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
    /// - an empty node-session store, so the simulator drives framing and mixing rather than
    ///   transport encryption,
    /// - fresh metrics and a never-firing shutdown token.
    pub fn new_for_simulation(
        sphinx_private_key: nym_crypto::asymmetric::x25519::PrivateKey,
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
            node_sessions: ActiveLpSessions::new(),
            metrics: NymNodeMetrics::default(),
            shutdown_token: ShutdownToken::new(),
        }
    }
}

#[cfg(feature = "mix-sim")]
impl SharedGatewayLpDataState {
    pub fn new_for_simulation(
        topology: nym_topology::NymTopology,
        client_map: HashMap<ClientAddress, SocketAddr>,
    ) -> Self {
        Self {
            cached_topology: CachedFullTopology::from_topology(topology),
            client_map: DashMap::from_iter(client_map),
        }
    }
}
