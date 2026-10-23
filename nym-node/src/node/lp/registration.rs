// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::active_sessions::LpPeer;
use crate::node::lp::state::SharedGatewayLpControlState;
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_metrics::{add_histogram_obs, inc};
use nym_registration_common::dvpn::{
    LpDvpnRegistrationFinalisation, LpDvpnRegistrationInitialRequest,
    LpDvpnRegistrationRequestMessage, LpDvpnRegistrationRequestMessageContent,
};
use nym_registration_common::mixnet::{
    LpMixnetGatewayData, LpMixnetRegistrationFinalisation, LpMixnetRegistrationInitialRequest,
    LpMixnetRegistrationRequestMessage, LpMixnetRegistrationRequestMessageContent,
};
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

impl SharedGatewayLpControlState {
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

    /// Answer the opening of a mixnet registration with the challenge that closes it.
    ///
    /// Nothing is bound here and nothing is trusted: the request is unsigned, and all this does is
    /// mint a nonce the finalisation will have to sign.
    fn process_mixnet_initial_registration(
        &self,
        request: LpMixnetRegistrationInitialRequest,
    ) -> LpRegistrationResponse {
        inc!("lp_registration_mixnet_attempts");

        if request.node_ed25519_pubkey != self.identity {
            warn!(
                "LP mixnet registration meant for another node: {}",
                request.node_ed25519_pubkey
            );
            inc!("lp_registration_mixnet_failed");
            return LpRegistrationResponse::error("wrong node", RegistrationMode::Mixnet);
        }

        LpRegistrationResponse::challenge_mixnet(self.registration_nonces.issue())
    }

    /// Name the session this request arrived on.
    ///
    /// The handshake that created the session is anonymous - a client has no KEM identity to
    /// authenticate with - so the session exists before anything knows whose it is. This request
    /// carries the client's keys, and the [`ClientAddress`] derived from them is how the rest of
    /// the network addresses that client. Binding the two here is what makes the session usable
    /// for sending: before this, the session can only decrypt.
    ///
    /// The request names the node it is meant for, the nonce that node issued, and when it was
    /// made, all signed with the identity key - so only the holder of the key can bind it, only
    /// here, and only once.
    fn process_mixnet_registration_finalisation(
        &self,
        sender: LpReceiverIndex,
        request: LpMixnetRegistrationFinalisation,
    ) -> LpRegistrationResponse {
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

        // last, because it is the only check with a side effect: a request refused above must not
        // cost the client the nonce it would need to try again
        if !self.registration_nonces.redeem(&request.content.nonce) {
            warn!(
                "LP mixnet registration with a nonce this node did not issue, or issued too long ago"
            );
            inc!("lp_registration_mixnet_failed");
            return LpRegistrationResponse::error("unknown nonce", RegistrationMode::Mixnet);
        }

        let client = ClientAddress::from_keys(
            &request.content.client_ed25519_pubkey,
            &request.content.client_x25519_pubkey,
        );
        self.sessions.bind_peer(LpPeer::client(client), sender);

        LpRegistrationResponse::success_mixnet(LpMixnetGatewayData {})
    }

    fn process_mixnet_registration(
        &self,
        sender: LpReceiverIndex,
        request: LpMixnetRegistrationRequestMessage,
    ) -> LpRegistrationResponse {
        match request.content {
            LpMixnetRegistrationRequestMessageContent::Initial(request) => {
                self.process_mixnet_initial_registration(request)
            }
            LpMixnetRegistrationRequestMessageContent::Finalisation(request) => {
                self.process_mixnet_registration_finalisation(sender, request)
            }
        }
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
                self.process_mixnet_registration(sender, *data)
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
    use crate::node::lp::active_sessions::ActiveLpSessions;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use nym_registration_common::LpRegistrationResponseData;
    use nym_registration_common::mixnet::{
        LpMixnetRegistrationResponseMessageContent, RegistrationNonce,
    };
    use nym_test_utils::helpers::deterministic_rng;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// The receiver index every request in these tests arrives on.
    const SENDER: LpReceiverIndex = 42;

    /// This node's client-facing state, a client, and the identity of some other node.
    fn parties() -> (SharedGatewayLpControlState, Client, ed25519::PublicKey) {
        let mut rng = deterministic_rng();
        let node = ed25519::KeyPair::new(&mut rng);
        let client = Client {
            identity: ed25519::KeyPair::new(&mut rng),
            encryption_key: *x25519::KeyPair::new(&mut rng).public_key(),
        };
        let other_node = *ed25519::KeyPair::new(&mut rng).public_key();

        let state = SharedGatewayLpControlState::new(
            *node.public_key(),
            ActiveLpSessions::new(),
            None,
            &LpConfig::default(),
        );
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

    /// Open a registration and take the nonce the node answered with.
    async fn challenge(state: &SharedGatewayLpControlState, client: &Client) -> RegistrationNonce {
        let response = state
            .process_registration(
                SENDER,
                LpRegistrationRequest::new_initial_mixnet(
                    client.identity.public_key(),
                    &client.encryption_key,
                    &state.identity,
                ),
            )
            .await;

        assert!(response.status.is_pending());

        let LpRegistrationResponseData::Mixnet { data } = response.response_data else {
            panic!("unexpected response mode")
        };
        match data.content {
            LpMixnetRegistrationResponseMessageContent::Challenge(challenge) => challenge.nonce,
            other => panic!("expected a challenge, got {other:?}"),
        }
    }

    /// Everything a client needs to register, as a pair of keys.
    struct Client {
        identity: ed25519::KeyPair,
        encryption_key: x25519::PublicKey,
    }

    impl Client {
        fn address(&self) -> ClientAddress {
            ClientAddress::from_keys(self.identity.public_key(), &self.encryption_key)
        }

        fn finalise(
            &self,
            node: &ed25519::PublicKey,
            nonce: RegistrationNonce,
        ) -> LpRegistrationRequest {
            LpRegistrationRequest::new_finalise_mixnet(
                &self.identity,
                &self.encryption_key,
                node,
                nonce,
                None,
            )
        }
    }

    #[tokio::test]
    async fn a_valid_registration_binds_the_client() {
        let (state, client, _) = parties();
        let nonce = challenge(&state, &client).await;

        let response = state
            .process_registration(SENDER, client.finalise(&state.identity, nonce))
            .await;

        assert!(response.status.is_successful());
        assert_eq!(
            state.sessions.peer_for(SENDER),
            Some(LpPeer::client(client.address()))
        );
    }

    /// The point of the whole exchange: a finalisation is good exactly once.
    #[tokio::test]
    async fn a_replayed_finalisation_binds_nothing() {
        let (state, client, _) = parties();
        let nonce = challenge(&state, &client).await;
        let finalisation = client.finalise(&state.identity, nonce);

        assert!(
            state
                .process_registration(SENDER, finalisation.clone())
                .await
                .status
                .is_successful()
        );

        // someone else's session, presenting the same blob
        const OTHER_SENDER: LpReceiverIndex = SENDER + 1;
        let response = state.process_registration(OTHER_SENDER, finalisation).await;

        assert!(response.status.is_failed());
        assert!(state.sessions.peer_for(OTHER_SENDER).is_none());
    }

    #[tokio::test]
    async fn a_finalisation_with_a_nonce_this_node_never_issued_binds_nothing() {
        let (state, client, _) = parties();

        let response = state
            .process_registration(SENDER, client.finalise(&state.identity, [3u8; 32]))
            .await;

        assert!(response.status.is_failed());
        assert!(state.sessions.peer_for(SENDER).is_none());
    }

    /// A refusal must not cost the client the nonce it would need to try again.
    #[tokio::test]
    async fn a_nonce_survives_a_registration_refused_for_another_reason() {
        let (state, client, other_node) = parties();
        let nonce = challenge(&state, &client).await;

        assert!(
            state
                .process_registration(SENDER, client.finalise(&other_node, nonce))
                .await
                .status
                .is_failed()
        );

        // the same nonce, now used properly
        assert!(
            state
                .process_registration(SENDER, client.finalise(&state.identity, nonce))
                .await
                .status
                .is_successful()
        );
    }

    #[tokio::test]
    async fn a_registration_for_another_node_binds_nothing() {
        let (state, client, other_node) = parties();
        let nonce = challenge(&state, &client).await;

        let response = state
            .process_registration(SENDER, client.finalise(&other_node, nonce))
            .await;

        assert!(response.status.is_failed());
        assert!(state.sessions.peer_for(SENDER).is_none());
    }

    #[tokio::test]
    async fn an_initial_request_for_another_node_is_refused() {
        let (state, client, other_node) = parties();

        let response = state
            .process_registration(
                SENDER,
                LpRegistrationRequest::new_initial_mixnet(
                    client.identity.public_key(),
                    &client.encryption_key,
                    &other_node,
                ),
            )
            .await;

        assert!(response.status.is_failed());
    }

    #[tokio::test]
    async fn a_registration_with_a_stale_signed_timestamp_binds_nothing() {
        let (state, client, _) = parties();
        let nonce = challenge(&state, &client).await;

        let stale = LpMixnetRegistrationFinalisation::new(
            &client.identity,
            client.encryption_key,
            state.identity,
            nonce,
            now() - 120,
            None,
        );

        let response = state
            .process_registration(
                SENDER,
                in_fresh_request(LpMixnetRegistrationRequestMessage::finalisation(stale)),
            )
            .await;

        assert!(response.status.is_failed());
        assert!(state.sessions.peer_for(SENDER).is_none());
    }

    #[tokio::test]
    async fn a_registration_claiming_another_identity_binds_nothing() {
        let (state, attacker, _) = parties();
        let victim = ed25519::KeyPair::new(&mut nym_test_utils::helpers::seeded_rng([1; 32]));
        let nonce = challenge(&state, &attacker).await;

        // the attacker's signature, under the victim's name
        let mut finalisation = match mixnet_message(attacker.finalise(&state.identity, nonce))
            .content
        {
            LpMixnetRegistrationRequestMessageContent::Finalisation(finalisation) => finalisation,
            other => panic!("expected a finalisation, got {other:?}"),
        };
        finalisation.content.client_ed25519_pubkey = *victim.public_key();

        let response = state
            .process_registration(
                SENDER,
                in_fresh_request(LpMixnetRegistrationRequestMessage::finalisation(
                    finalisation,
                )),
            )
            .await;

        assert!(response.status.is_failed());
        assert!(state.sessions.peer_for(SENDER).is_none());
    }
}
