// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// LP (Lewes Protocol) Metrics Documentation
//
// This module implements comprehensive metrics collection for LP operations using nym-metrics macros.
// All metrics are automatically prefixed with the package name (nym_gateway) when registered.
//
// ## Connection Metrics (via NetworkStats in nym-node-metrics)
// - active_lp_connections: Gauge tracking current active LP connections (incremented on accept, decremented on close)
//
// ## Handler Metrics (in client_handler)
// - lp_connections_total: Counter for total LP connections handled
// - lp_client_hello_failed: Counter for ClientHello failures (timestamp validation, protocol errors)
// - lp_handshakes_success: Counter for successful handshake completions
// - lp_handshakes_failed: Counter for failed handshakes
// - lp_handshake_duration_seconds: Histogram of handshake durations (buckets: 10ms to 10s)
// - lp_timestamp_validation_accepted: Counter for timestamp validations that passed
// - lp_timestamp_validation_rejected: Counter for timestamp validations that failed
// - lp_errors_handshake: Counter for handshake errors
// - lp_errors_send_response: Counter for errors sending registration responses
// - lp_errors_timestamp_too_old: Counter for ClientHello timestamps that are too old
// - lp_errors_timestamp_too_far_future: Counter for ClientHello timestamps that are too far in the future
//
// ## Registration Metrics (in registration.rs)
// - lp_registration_attempts_total: Counter for all registration attempts
// - lp_registration_success_total: Counter for successful registrations (any mode)
// - lp_registration_failed_total: Counter for failed registrations (any mode)
// - lp_registration_failed_timestamp: Counter for registrations rejected due to invalid timestamp
// - lp_registration_duration_seconds: Histogram of registration durations (buckets: 100ms to 30s)
//
// ## Mode-Specific Registration Metrics (in registration.rs)
// - lp_registration_dvpn_attempts: Counter for dVPN mode registration attempts
// - lp_registration_dvpn_success: Counter for successful dVPN registrations
// - lp_registration_dvpn_failed: Counter for failed dVPN registrations
// - lp_registration_mixnet_attempts: Counter for Mixnet mode registration attempts
// - lp_registration_mixnet_success: Counter for successful Mixnet registrations
// - lp_registration_mixnet_failed: Counter for failed Mixnet registrations
//
// ## Credential Verification Metrics (in registration.rs)
// - lp_credential_verification_attempts: Counter for credential verification attempts
// - lp_credential_verification_success: Counter for successful credential verifications
// - lp_credential_verification_failed: Counter for failed credential verifications
// - lp_bandwidth_allocated_bytes_total: Counter for total bandwidth allocated (in bytes)
//
// ## Error Categorization Metrics
// - lp_errors_wg_peer_registration: Counter for WireGuard peer registration failures
//
// ## Connection Lifecycle Metrics (in client_handler)
// - lp_connection_duration_seconds: Histogram of connection duration from start to end (buckets: 1s to 24h)
// - lp_connection_bytes_received_total: Counter for total bytes received including protocol framing
// - lp_connection_bytes_sent_total: Counter for total bytes sent including protocol framing
// - lp_connections_completed_gracefully: Counter for connections that completed successfully
// - lp_connections_completed_with_error: Counter for connections that terminated with an error
//
// ## State Cleanup Metrics (in cleanup task)
// - lp_states_cleanup_handshake_removed: Counter for stale handshakes removed by cleanup task
// - lp_states_cleanup_session_removed: Counter for stale sessions removed by cleanup task
// - lp_states_cleanup_demoted_removed: Counter for demoted (read-only) sessions removed by cleanup task
//
// ## Subsession/Rekeying Metrics (in client_handler)
// - lp_subsession_kk2_sent: Counter for SubsessionKK2 responses sent (indicates client initiated rekeying)
// - lp_subsession_complete: Counter for successful subsession promotions
// - lp_subsession_receiver_index_collision: Counter for subsession receiver_index collisions
//
// ## Usage Example
// To view metrics, the nym-metrics registry automatically collects all metrics.
// They can be exported via Prometheus format using the metrics endpoint.

use crate::config::LpConfig;
use crate::error::NymNodeError;
use crate::node::lp::cleanup::CleanupTask;
use crate::node::lp::data::listener::LpDataListener;
use control::ingress::listener::LpControlListener;
use nym_gateway::node::wireguard::PeerRegistrator;
use nym_lp::peer::LpLocalPeer;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownTracker;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::error;

use crate::node::lp::directory::LpNodes;
use crate::node::lp::state::{ActiveLpSessions, SharedLpNodeControlState};
pub use nym_mixnet_client::forwarder::{MixForwardingReceiver, mix_forwarding_channels};
pub use state::{SharedLpClientControlState, SharedLpDataState, SharedLpState};

mod cleanup;
pub mod control;
mod data;
pub mod directory;
pub mod error;
mod registration;
pub mod state;

pub struct LpSetup {
    control_listener: LpControlListener,
    data_listener: LpDataListener,
    cleanup_task: CleanupTask,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl LpSetup {
    pub async fn new(
        local_lp_peer: LpLocalPeer,
        lp_config: LpConfig,
        metrics: NymNodeMetrics,
        peer_registrator: Option<PeerRegistrator>,
        network_nodes: LpNodes,
        mix_packet_sender: MixForwardingSender,
        shutdown: ShutdownTracker,
    ) -> Result<Self, NymNodeError> {
        // TODO: this will require loading old states from disk in the future
        let session_states = ActiveLpSessions::new();

        let shared_lp_state = SharedLpState {
            metrics,
            lp_config,
            session_states: session_states.clone(),
        };

        let client_control_state = SharedLpClientControlState {
            local_lp_peer: local_lp_peer.clone(),
            peer_registrator,
            forward_semaphore: Arc::new(Semaphore::new(lp_config.debug.max_concurrent_forwards)),
            shared: shared_lp_state.clone(),
        };

        let nodes_control_state = SharedLpNodeControlState {
            local_lp_peer,
            nodes: network_nodes,
            shared: shared_lp_state.clone(),
        };

        let data_state = SharedLpDataState {
            outbound_mix_sender: mix_packet_sender,
            shared: shared_lp_state,
        };

        let control_listener = LpControlListener::new(
            lp_config.control_bind_address,
            client_control_state,
            nodes_control_state,
            shutdown.clone(),
        );
        let data_listener = LpDataListener::new(
            lp_config.data_bind_address,
            data_state,
            shutdown.clone_shutdown_token(),
        );
        let cleanup_task = CleanupTask::new(
            session_states,
            lp_config.debug,
            shutdown.clone_shutdown_token(),
        );

        Ok(LpSetup {
            control_listener,
            data_listener,
            cleanup_task,
            shutdown,
        })
    }

    pub fn start_tasks(mut self) {
        // control listener
        let shutdown_token = self.shutdown.clone_shutdown_token();
        self.shutdown.try_spawn_named(
            async move {
                if let Err(err) = self.control_listener.run().await {
                    shutdown_token.cancel();
                    error!("LP control listener error: {err}");
                }
            },
            "LP::LpControlListener",
        );

        // Spawn the UDP data handler for LP data plane
        // The data handler listens on UDP port 51264 and processes LP-wrapped Sphinx packets
        // from registered clients. It decrypts the LP layer and forwards the Sphinx packets
        let shutdown_token = self.shutdown.clone_shutdown_token();
        self.shutdown.try_spawn_named(
            async move {
                if let Err(err) = self.data_listener.run().await {
                    shutdown_token.cancel();
                    error!("LP data listener error: {err}");
                }
            },
            "LP::LpDataListener",
        );

        // cleanup task
        self.shutdown.try_spawn_named(
            async move { self.cleanup_task.run().await },
            "LP::CleanupTask",
        );
    }
}
