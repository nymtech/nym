// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::state::SharedLpClientControlState;
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_metrics::{add_histogram_obs, inc};
use nym_registration_common::dvpn::{
    LpDvpnRegistrationFinalisation, LpDvpnRegistrationInitialRequest,
    LpDvpnRegistrationRequestMessage, LpDvpnRegistrationRequestMessageContent,
};
use nym_registration_common::mixnet::{LpMixnetGatewayData, LpMixnetRegistrationRequestMessage};
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationRequestData, LpRegistrationResponse, RegistrationMode,
    RegistrationStatus,
};
use nym_sphinx_addressing::ClientAddress;
use tracing::*;

/// How far a registration timestamp may be from this node's clock, either way.
const MAX_REGISTRATION_TIMESTAMP_SKEW_SECS: u64 = 30;

// Histogram buckets for LP registration duration tracking
// Registration includes credential verification, DB operations, and potentially WireGuard peer setup
// Expected durations: 100ms - 5s for normal operations, up to 30s for slow DB or network issues
const LP_REGISTRATION_DURATION_BUCKETS: &[f64] = &[
    0.1,  // 100ms
    0.25, // 250ms
    0.5,  // 500ms
    1.0,  // 1s
    2.5,  // 2.5s
    5.0,  // 5s
    10.0, // 10s
    30.0, // 30s
];

impl SharedLpClientControlState {
    async fn process_dvpn_initial_registration(
        &self,
        sender: LpReceiverIndex,
        request: LpDvpnRegistrationInitialRequest,
    ) -> LpRegistrationResponse {
        let Some(registrator) = self.peer_registrator.as_ref() else {
            return LpRegistrationResponse::error(
                "dVPN via LP is not enabled on this node",
                RegistrationMode::Dvpn,
            );
        };

        registrator
            .on_initial_lp_request(request, sender)
            .await
            .unwrap_or_else(|err| {
                LpRegistrationResponse::error(
                    format!("LP registration has failed: {err}"),
                    RegistrationMode::Dvpn,
                )
            })
    }

    async fn process_dvpn_registration_finalisation(
        &self,
        sender: LpReceiverIndex,
        request: LpDvpnRegistrationFinalisation,
    ) -> LpRegistrationResponse {
        let Some(registrator) = self.peer_registrator.as_ref() else {
            return LpRegistrationResponse::error(
                "dVPN via LP is not enabled on this node",
                RegistrationMode::Dvpn,
            );
        };

        registrator
            .on_final_lp_request(request, sender)
            .await
            .unwrap_or_else(|err| {
                LpRegistrationResponse::error(
                    format!("LP registration has failed: {err}"),
                    RegistrationMode::Dvpn,
                )
            })
    }

    async fn process_dvpn_registration(
        &self,
        sender: LpReceiverIndex,
        request: Box<LpDvpnRegistrationRequestMessage>,
    ) -> LpRegistrationResponse {
        // Track dVPN registration attempts
        inc!("lp_registration_dvpn_attempts");

        match request.content {
            LpDvpnRegistrationRequestMessageContent::InitialRequest(req) => {
                self.process_dvpn_initial_registration(sender, req).await
            }
            LpDvpnRegistrationRequestMessageContent::Finalisation(req) => {
                self.process_dvpn_registration_finalisation(sender, req)
                    .await
            }
        }
    }

    /// Name the session this request arrived on.
    ///
    /// The handshake that created the session is anonymous - a client has no KEM identity to
    /// authenticate with - so the session exists before anything knows whose it is. This request
    /// carries the client's identity key, and the [`ClientAddress`] derived from it is how the rest
    /// of the network addresses that client. Binding the two here is what makes the session usable
    /// for sending: before this, the session can only decrypt.
    ///
    /// The request names the node it is meant for and when it was made, signed with the identity
    /// key, so only the holder of the key can bind it, only here, and only while it is recent.
    async fn process_mixnet_registration(
        &self,
        sender: LpReceiverIndex,
        request: LpMixnetRegistrationRequestMessage,
    ) -> LpRegistrationResponse {
        inc!("lp_registration_mixnet_attempts");

        if let Err(err) = request.verify() {
            warn!("LP mixnet registration with an invalid signature: {err}");
            inc!("lp_registration_mixnet_failed");
            return LpRegistrationResponse::error("invalid signature", RegistrationMode::Mixnet);
        }

        if request.content.node_ed25519_pubkey != self.identity {
            warn!(
                "LP mixnet registration meant for another node: {}",
                request.content.node_ed25519_pubkey
            );
            inc!("lp_registration_mixnet_failed");
            return LpRegistrationResponse::error("wrong node", RegistrationMode::Mixnet);
        }

        if !request
            .content
            .is_recent(MAX_REGISTRATION_TIMESTAMP_SKEW_SECS)
        {
            warn!("LP mixnet registration with a signed timestamp too old or too far in future");
            inc!("lp_registration_mixnet_failed");
            return LpRegistrationResponse::error("invalid timestamp", RegistrationMode::Mixnet);
        }

        let client = ClientAddress::from_identity(&request.content.client_ed25519_pubkey);
        self.shared
            .sessions
            .bind_peer(LpPeer::client(client), sender);

        LpRegistrationResponse::success_mixnet(LpMixnetGatewayData {})
    }

    /// Process an LP registration request
    pub async fn process_registration(
        &self,
        sender: LpReceiverIndex,
        request: LpRegistrationRequest,
    ) -> LpRegistrationResponse {
        let registration_start = std::time::Instant::now();

        // Track total registration attempts
        inc!("lp_registration_attempts_total");

        // 1. Validate timestamp for replay protection
        if !request.validate_timestamp(MAX_REGISTRATION_TIMESTAMP_SKEW_SECS) {
            warn!("LP registration failed: timestamp too old or too far in future");
            inc!("lp_registration_failed_timestamp");
            return LpRegistrationResponse::error("invalid timestamp", request.mode());
        }

        // 2. Process based on mode
        let result = match request.registration_data {
            LpRegistrationRequestData::Dvpn { data } => {
                self.process_dvpn_registration(sender, data).await
            }
            LpRegistrationRequestData::Mixnet { data } => {
                self.process_mixnet_registration(sender, *data).await
            }
        };

        // Track registration duration
        let duration = registration_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "lp_registration_duration_seconds",
            duration,
            LP_REGISTRATION_DURATION_BUCKETS
        );

        // Track overall success/failure
        match result.status {
            RegistrationStatus::Completed => {
                inc!("lp_registration_success_total");
            }
            RegistrationStatus::Failed => {
                inc!("lp_registration_failed_total");
            }
            RegistrationStatus::PendingMoreData => {
                inc!("lp_registration_pending_more_data");
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LpConfig;
    use crate::node::lp::SharedLpState;
    use crate::node::lp::active_sessions::ActiveLpSessions;
    use nym_crypto::asymmetric::ed25519;
    use nym_test_utils::helpers::deterministic_rng;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// The receiver index every request in these tests arrives on.
    const SENDER: LpReceiverIndex = 42;

    /// This node's state, a client, and the identity of some other node.
    fn parties() -> (
        SharedLpClientControlState,
        ed25519::KeyPair,
        ed25519::PublicKey,
    ) {
        let mut rng = deterministic_rng();
        let node = ed25519::KeyPair::new(&mut rng);
        let client = ed25519::KeyPair::new(&mut rng);
        let other_node = *ed25519::KeyPair::new(&mut rng).public_key();

        let lp_config = LpConfig::default();
        let state = SharedLpClientControlState {
            local_lp_peer: nym_lp::peer::mock_peers().0,
            identity: *node.public_key(),
            peer_registrator: None,
            forward_semaphore: Arc::new(tokio::sync::Semaphore::new(
                lp_config.debug.max_concurrent_forwards,
            )),
            shared: SharedLpState {
                sessions: ActiveLpSessions::new(),
                metrics: Default::default(),
                lp_config,
            },
        };
        (state, client, other_node)
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// `message` in a request whose own timestamp is current, so only the message is on trial.
    fn in_fresh_request(message: LpMixnetRegistrationRequestMessage) -> LpRegistrationRequest {
        LpRegistrationRequest {
            registration_data: LpRegistrationRequestData::Mixnet {
                data: Box::new(message),
            },
            timestamp: now(),
        }
    }

    fn mixnet_message(request: LpRegistrationRequest) -> LpMixnetRegistrationRequestMessage {
        let LpRegistrationRequestData::Mixnet { data } = request.registration_data else {
            panic!("unexpected request")
        };
        *data
    }

    #[tokio::test]
    async fn a_valid_registration_binds_the_client() {
        let (state, client, _) = parties();
        let request = LpRegistrationRequest::new_mixnet(&client, &state.identity);

        let response = state.process_registration(SENDER, request).await;

        assert!(response.status.is_successful());
        assert_eq!(
            state.shared.sessions.peer_for(SENDER),
            Some(LpPeer::client(ClientAddress::from_identity(
                client.public_key()
            )))
        );
    }

    #[tokio::test]
    async fn a_registration_for_another_node_binds_nothing() {
        let (state, client, other_node) = parties();
        let request = LpRegistrationRequest::new_mixnet(&client, &other_node);

        let response = state.process_registration(SENDER, request).await;

        assert!(response.status.is_failed());
        assert!(state.shared.sessions.peer_for(SENDER).is_none());
    }

    #[tokio::test]
    async fn a_registration_with_a_stale_signed_timestamp_binds_nothing() {
        let (state, client, _) = parties();
        let stale = LpMixnetRegistrationRequestMessage::new(&client, state.identity, now() - 120);

        let response = state
            .process_registration(SENDER, in_fresh_request(stale))
            .await;

        assert!(response.status.is_failed());
        assert!(state.shared.sessions.peer_for(SENDER).is_none());
    }

    #[tokio::test]
    async fn a_registration_claiming_another_identity_binds_nothing() {
        let (state, attacker, _) = parties();
        let victim = ed25519::KeyPair::new(&mut nym_test_utils::helpers::seeded_rng([1; 32]));

        // the attacker's signature, under the victim's name
        let mut message = mixnet_message(LpRegistrationRequest::new_mixnet(
            &attacker,
            &state.identity,
        ));
        message.content.client_ed25519_pubkey = *victim.public_key();

        let response = state
            .process_registration(SENDER, in_fresh_request(message))
            .await;

        assert!(response.status.is_failed());
        assert!(state.shared.sessions.peer_for(SENDER).is_none());
    }
}
