// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration message types shared between client and gateway.

use crate::WireguardRegistrationData;
use crate::dvpn::{
    LpDvpnRegistrationFinalisation, LpDvpnRegistrationInitialRequest,
    LpDvpnRegistrationRequestMessage, LpDvpnRegistrationRequestMessageContent,
    LpDvpnRegistrationResponseMessage, LpDvpnRegistrationResponseMessageContent,
    RequiresCredentialResponse,
};
use crate::mixnet::{
    LpMixnetGatewayData, LpMixnetRegistrationRequestMessage, LpMixnetRegistrationResponseMessage,
    LpMixnetRegistrationResponseMessageContent,
};
use crate::serialisation::{BincodeError, BincodeOptions, lp_bincode_serializer};
use nym_authenticator_requests::models::BandwidthClaim;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp_data::packet::LpFrame;
use nym_lp_data::packet::frame::LpFrameKind;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

/// Why a registration message could not be put into, or taken out of, an LP frame.
#[derive(Debug, Error)]
pub enum LpRegistrationFrameError {
    #[error("failed to serialise the registration message: {0}")]
    Serialisation(BincodeError),

    #[error("expected a registration frame, got {0:?}")]
    UnexpectedFrameKind(LpFrameKind),

    #[error("received registration data was malformed: {0}")]
    Deserialisation(BincodeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationMode {
    /// dVPN mode - register as WireGuard peer (most common)
    Dvpn,

    /// Mixnet mode - register for mixnet routing via IPR
    Mixnet,
}

/// Registration request sent by client after LP handshake
/// Aligned with existing authenticator registration flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRegistrationRequest {
    /// Mode specific registration data
    pub registration_data: LpRegistrationRequestData,

    /// Unix timestamp for replay protection
    pub timestamp: u64,
}

impl LpRegistrationRequest {
    pub fn mode(&self) -> RegistrationMode {
        match self.registration_data {
            LpRegistrationRequestData::Dvpn { .. } => RegistrationMode::Dvpn,
            LpRegistrationRequestData::Mixnet { .. } => RegistrationMode::Mixnet,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LpRegistrationRequestData {
    /// dVPN mode - register as WireGuard peer (most common)
    Dvpn {
        data: Box<LpDvpnRegistrationRequestMessage>,
    },

    /// Mixnet mode - register for mixnet routing via IPR
    Mixnet {
        data: Box<LpMixnetRegistrationRequestMessage>,
    },
}

/// Registration response from gateway
/// Contains GatewayData for compatibility with existing client code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRegistrationResponse {
    /// The status of this registration after the last received client message
    pub status: RegistrationStatus,

    /// Mode specific registration response
    pub response_data: LpRegistrationResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LpRegistrationResponseData {
    /// dVPN mode - register as WireGuard peer (most common)
    Dvpn {
        data: LpDvpnRegistrationResponseMessage,
    },

    /// Mixnet mode - register for mixnet routing via IPR
    Mixnet {
        data: LpMixnetRegistrationResponseMessage,
    },
}

/// Represents the registration status after the last received client message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationStatus {
    /// The registration has been completed successfully
    Completed,

    /// The registration has failed
    Failed,

    /// To complete registration the client needs to send additional data,
    /// e.g. a credential. it is context dependent.
    PendingMoreData,
}

impl RegistrationStatus {
    pub fn is_successful(&self) -> bool {
        matches!(self, RegistrationStatus::Completed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, RegistrationStatus::Failed)
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, RegistrationStatus::PendingMoreData)
    }
}

/// `timestamp` moved back by a positive `skew`, or forward by a negative one.
fn skewed_timestamp(timestamp: u64, skew: time::Duration) -> u64 {
    let stamped = std::time::Duration::from_secs(timestamp);
    let abs = skew.unsigned_abs();
    if skew.is_negative() {
        stamped.saturating_add(abs)
    } else {
        stamped.saturating_sub(abs)
    }
    .as_secs()
}

/// Whether `timestamp`, in unix seconds, is within `max_skew_secs` of now, either way.
fn timestamp_is_recent(timestamp: u64, max_skew_secs: u64) -> bool {
    let now = current_timestamp().as_secs();
    now.abs_diff(timestamp) <= max_skew_secs
}

fn current_timestamp() -> std::time::Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .inspect_err(|_| error!("the current timestamp predates unix epoch!"))
        .unwrap_or_default()
}

impl LpRegistrationRequest {
    fn new(registration_data: LpRegistrationRequestData) -> LpRegistrationRequest {
        Self {
            registration_data,
            timestamp: current_timestamp().as_secs(),
        }
    }

    /// Create new dVPN registration initialisation request
    pub fn new_initial_dvpn(
        wg_public_key: nym_wireguard_types::PeerPublicKey,
        psk: [u8; 32],
    ) -> Self {
        Self::new(LpRegistrationRequestData::Dvpn {
            data: Box::new(LpDvpnRegistrationRequestMessage {
                content: LpDvpnRegistrationRequestMessageContent::InitialRequest(
                    LpDvpnRegistrationInitialRequest { wg_public_key, psk },
                ),
            }),
        })
    }

    /// Open a mixnet registration: name the keys, and ask for the nonce to sign.
    ///
    /// Both keys, because the name the node binds covers both - see
    /// [`ClientAddress::from_keys`](nym_sphinx_addressing::ClientAddress::from_keys). Finish with
    /// [`Self::finalise_mixnet`] once the node has answered.
    pub fn new_initial_mixnet(
        client_identity: &ed25519::PublicKey,
        client_encryption_key: &x25519::PublicKey,
        node_identity: &ed25519::PublicKey,
    ) -> Self {
        Self::new(LpRegistrationRequestData::Mixnet {
            data: Box::new(LpMixnetRegistrationRequestMessage::initial(
                *client_identity,
                *client_encryption_key,
                *node_identity,
            )),
        })
    }

    /// Close a mixnet registration by signing the nonce the node answered with.
    ///
    /// The signed timestamp is the request's own, so [`Self::with_spend_time_skew`] must not be
    /// applied afterwards: use [`Self::finalise_mixnet_with_skew`] instead.
    pub fn new_finalise_mixnet(
        identity: &ed25519::KeyPair,
        client_encryption_key: &x25519::PublicKey,
        node_identity: &ed25519::PublicKey,
        nonce: mixnet::RegistrationNonce,
        bandwidth_claim: Option<BandwidthClaim>,
    ) -> Self {
        Self::new_finalise_mixnet_with_skew(
            identity,
            client_encryption_key,
            node_identity,
            nonce,
            bandwidth_claim,
            time::Duration::ZERO,
        )
    }

    /// [`Self::finalise_mixnet`], with the timestamp corrected by `skew` before it is signed.
    ///
    /// `skew` means the same as in [`Self::with_spend_time_skew`].
    pub fn new_finalise_mixnet_with_skew(
        identity: &ed25519::KeyPair,
        client_encryption_key: &x25519::PublicKey,
        node_identity: &ed25519::PublicKey,
        nonce: mixnet::RegistrationNonce,
        bandwidth_claim: Option<BandwidthClaim>,
        skew: time::Duration,
    ) -> Self {
        let timestamp = skewed_timestamp(current_timestamp().as_secs(), skew);
        LpRegistrationRequest {
            registration_data: LpRegistrationRequestData::Mixnet {
                data: Box::new(LpMixnetRegistrationRequestMessage::finalisation(
                    mixnet::LpMixnetRegistrationFinalisation::new(
                        identity,
                        *client_encryption_key,
                        *node_identity,
                        nonce,
                        timestamp,
                        bandwidth_claim,
                    ),
                )),
            },
            timestamp,
        }
    }

    pub fn new_finalise_dvpn(credential: BandwidthClaim) -> Self {
        Self::new(LpRegistrationRequestData::Dvpn {
            data: Box::new(LpDvpnRegistrationRequestMessage {
                content: LpDvpnRegistrationRequestMessageContent::Finalisation(
                    LpDvpnRegistrationFinalisation { credential },
                ),
            }),
        })
    }

    /// Positive `skew` is how far the device is ahead of the VPN API; the stamp is moved back (negative moves it forward).
    pub fn with_spend_time_skew(mut self, skew: time::Duration) -> Self {
        self.timestamp = skewed_timestamp(self.timestamp, skew);
        self
    }

    /// Validate the request timestamp is within acceptable bounds
    pub fn validate_timestamp(&self, max_skew_secs: u64) -> bool {
        timestamp_is_recent(self.timestamp, max_skew_secs)
    }

    /// Attempt to serialise this `LpRegistrationRequest` into bytes.
    pub fn serialise(&self) -> Result<Vec<u8>, BincodeError> {
        lp_bincode_serializer().serialize(self)
    }

    /// Attempt to deserialise a `LpRegistrationRequest` from bytes.
    pub fn try_deserialise(b: &[u8]) -> Result<Self, BincodeError> {
        lp_bincode_serializer().deserialize(b)
    }

    /// Serialise this request into the frame that carries it.
    pub fn to_lp_frame(&self) -> Result<LpFrame, LpRegistrationFrameError> {
        tracing::debug!(
            "Serializing registration request ({} bytes)",
            request_bytes.len()
        );
        let request_bytes = self
            .serialise()
            .map_err(LpRegistrationFrameError::Serialisation)?;

        Ok(LpFrame::new_registration(request_bytes))
    }
}

impl LpRegistrationResponse {
    /// Recover a response from the frame that carried it.
    pub fn from_lp_frame(frame: LpFrame) -> Result<Self, LpRegistrationFrameError> {
        if frame.kind() != LpFrameKind::Registration {
            return Err(LpRegistrationFrameError::UnexpectedFrameKind(frame.kind()));
        }

        Self::try_deserialise(&frame.content).map_err(LpRegistrationFrameError::Deserialisation)
    }

    /// Create a success response with GatewayData (for dVPN mode)
    pub fn success_dvpn(config: WireguardRegistrationData, upgrade_mode: bool) -> Self {
        Self {
            status: RegistrationStatus::Completed,
            response_data: LpRegistrationResponseData::Dvpn {
                data: LpDvpnRegistrationResponseMessage {
                    content: LpDvpnRegistrationResponseMessageContent::CompletedRegistration(
                        dvpn::CompletedRegistrationResponse {
                            config,
                            upgrade_mode,
                        },
                    ),
                },
            },
        }
    }

    pub fn success_mixnet(config: LpMixnetGatewayData) -> Self {
        Self {
            status: RegistrationStatus::Completed,
            response_data: LpRegistrationResponseData::Mixnet {
                data: LpMixnetRegistrationResponseMessage {
                    content: LpMixnetRegistrationResponseMessageContent::CompletedRegistration(
                        mixnet::CompletedRegistrationResponse { config },
                    ),
                },
            },
        }
    }

    /// Answer an initial mixnet request with the nonce its finalisation must sign.
    pub fn challenge_mixnet(nonce: mixnet::RegistrationNonce) -> Self {
        Self {
            status: RegistrationStatus::PendingMoreData,
            response_data: LpRegistrationResponseData::Mixnet {
                data: LpMixnetRegistrationResponseMessage {
                    content: LpMixnetRegistrationResponseMessageContent::Challenge(
                        mixnet::MixnetChallengeResponse { nonce },
                    ),
                },
            },
        }
    }

    /// Create an error response
    pub fn error(error: impl Into<String>, mode: RegistrationMode) -> Self {
        let response_data = match mode {
            RegistrationMode::Dvpn => LpRegistrationResponseData::Dvpn {
                data: LpDvpnRegistrationResponseMessage::error(error),
            },
            RegistrationMode::Mixnet => LpRegistrationResponseData::Mixnet {
                data: LpMixnetRegistrationResponseMessage::error(error),
            },
        };
        LpRegistrationResponse {
            status: RegistrationStatus::Failed,
            response_data,
        }
    }

    pub fn request_dvpn_credential() -> Self {
        LpRegistrationResponse {
            status: RegistrationStatus::PendingMoreData,
            response_data: LpRegistrationResponseData::Dvpn {
                data: LpDvpnRegistrationResponseMessage {
                    content: LpDvpnRegistrationResponseMessageContent::RequiresCredential(
                        RequiresCredentialResponse,
                    ),
                },
            },
        }
    }

    pub fn into_dvpn_response(self) -> Option<LpDvpnRegistrationResponseMessage> {
        match self.response_data {
            LpRegistrationResponseData::Dvpn { data } => Some(data),
            LpRegistrationResponseData::Mixnet { .. } => None,
        }
    }

    pub fn into_mixnet_response(self) -> Option<LpMixnetRegistrationResponseMessage> {
        match self.response_data {
            LpRegistrationResponseData::Mixnet { data } => Some(data),
            LpRegistrationResponseData::Dvpn { .. } => None,
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match &self.response_data {
            LpRegistrationResponseData::Dvpn { data } => match &data.content {
                LpDvpnRegistrationResponseMessageContent::RegistrationFailure(response) => {
                    Some(&response.error)
                }
                _ => None,
            },
            LpRegistrationResponseData::Mixnet { data } => match &data.content {
                LpMixnetRegistrationResponseMessageContent::RegistrationFailure(response) => {
                    Some(&response.error)
                }
                _ => None,
            },
        }
    }

    /// Attempt to serialise this `LpRegistrationResponse` into bytes.
    pub fn serialise(&self) -> Result<Vec<u8>, BincodeError> {
        lp_bincode_serializer().serialize(self)
    }

    /// Attempt to deserialise a `LpRegistrationResponse` from bytes.
    pub fn try_deserialise(b: &[u8]) -> Result<Self, BincodeError> {
        lp_bincode_serializer().deserialize(b)
    }
}

pub mod dvpn {
    use crate::WireguardRegistrationData;
    use nym_authenticator_requests::models::BandwidthClaim;
    use serde::{Deserialize, Serialize};

    // client
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpDvpnRegistrationRequestMessage {
        pub content: LpDvpnRegistrationRequestMessageContent,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum LpDvpnRegistrationRequestMessageContent {
        InitialRequest(LpDvpnRegistrationInitialRequest),
        Finalisation(LpDvpnRegistrationFinalisation),
        // in theory, we could also extend it with Bandwidth-related messages,
        // but that shouldn't really be the responsibility of a Registration client.
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpDvpnRegistrationInitialRequest {
        /// Client's WireGuard public key (for dVPN mode)
        pub wg_public_key: nym_wireguard_types::PeerPublicKey,

        /// Preshared key to be used for the connection
        pub psk: [u8; 32],
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpDvpnRegistrationFinalisation {
        /// Ecash credential
        pub credential: BandwidthClaim,
    }

    // gateway
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpDvpnRegistrationResponseMessage {
        pub content: LpDvpnRegistrationResponseMessageContent,
    }

    impl LpDvpnRegistrationResponseMessage {
        pub fn error(error: impl Into<String>) -> Self {
            LpDvpnRegistrationResponseMessage {
                content: LpDvpnRegistrationResponseMessageContent::RegistrationFailure(
                    RegistrationFailureResponse {
                        error: error.into(),
                    },
                ),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum LpDvpnRegistrationResponseMessageContent {
        RequiresCredential(RequiresCredentialResponse),
        CompletedRegistration(CompletedRegistrationResponse),
        RegistrationFailure(RegistrationFailureResponse),
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct CompletedRegistrationResponse {
        /// Gateway configuration data for dVPN mode (WireGuard)
        /// This matches what WireguardRegistrationResult expects
        pub config: WireguardRegistrationData,

        /// Flag indicating whether the gateway has detected the system is undergoing the upgrade
        /// (thus it will not meter bandwidth)
        pub upgrade_mode: bool,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct RequiresCredentialResponse;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RegistrationFailureResponse {
        pub error: String,
    }
}

pub mod mixnet {
    use nym_authenticator_requests::models::BandwidthClaim;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use serde::{Deserialize, Serialize};

    const MIXNET_REGISTRATION_DOMAIN: &[u8] = b"NYM-LP-MIXNET-REGISTRATION-V1";

    /// The challenge a node issues, and the client signs to prove the exchange is live.
    pub type RegistrationNonce = [u8; 32];

    // client

    /// A mixnet registration, in whichever of its two steps it is at.
    ///
    /// Two steps rather than one because a signature over the client's own material alone says
    /// nothing about *when* it was made: a copy of it, presented by anyone, binds the same name.
    /// The node answers the first step with a nonce, and signing that nonce is what makes the
    /// second step good exactly once.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpMixnetRegistrationRequestMessage {
        pub content: LpMixnetRegistrationRequestMessageContent,
    }

    /// New steps are added as variants rather than as fields of the existing ones: bincode cannot
    /// tolerate a struct growing a field, but a reader meeting an unknown variant fails cleanly
    /// rather than misreading what follows.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum LpMixnetRegistrationRequestMessageContent {
        Initial(LpMixnetRegistrationInitialRequest),
        Finalisation(LpMixnetRegistrationFinalisation),
    }

    impl LpMixnetRegistrationRequestMessage {
        pub fn initial(
            client_ed25519_pubkey: ed25519::PublicKey,
            client_x25519_pubkey: x25519::PublicKey,
            node_ed25519_pubkey: ed25519::PublicKey,
        ) -> Self {
            LpMixnetRegistrationRequestMessage {
                content: LpMixnetRegistrationRequestMessageContent::Initial(
                    LpMixnetRegistrationInitialRequest {
                        client_ed25519_pubkey,
                        client_x25519_pubkey,
                        node_ed25519_pubkey,
                    },
                ),
            }
        }

        pub fn finalisation(finalisation: LpMixnetRegistrationFinalisation) -> Self {
            LpMixnetRegistrationRequestMessage {
                content: LpMixnetRegistrationRequestMessageContent::Finalisation(finalisation),
            }
        }
    }

    /// Names the keys to be registered and asks for a nonce.
    ///
    /// Unsigned: possession of the identity key is proven by the finalisation, and issuing a nonce
    /// costs the node nothing worth protecting.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpMixnetRegistrationInitialRequest {
        /// Client's ed25519 public key (identity).
        pub client_ed25519_pubkey: ed25519::PublicKey,

        /// Client's x25519 public key, which traffic to it is encrypted under.
        ///
        /// Part of the registration because the name a client is given covers both keys - see
        /// [`ClientAddress::from_keys`](nym_sphinx_addressing::ClientAddress::from_keys).
        pub client_x25519_pubkey: x25519::PublicKey,

        /// The ed25519 identity of the node the request is meant for, so it cannot be presented at
        /// any other.
        pub node_ed25519_pubkey: ed25519::PublicKey,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpMixnetRegistrationFinalisation {
        pub content: LpMixnetRegistrationFinalisationContent,

        /// The client's identity key over [`Self::content`].
        pub signature: ed25519::Signature,
    }

    impl LpMixnetRegistrationFinalisation {
        pub fn new(
            identity: &ed25519::KeyPair,
            client_x25519_pubkey: x25519::PublicKey,
            node_ed25519_pubkey: ed25519::PublicKey,
            nonce: RegistrationNonce,
            timestamp: u64,
            bandwidth_claim: Option<BandwidthClaim>,
        ) -> Self {
            let content = LpMixnetRegistrationFinalisationContent {
                client_ed25519_pubkey: *identity.public_key(),
                client_x25519_pubkey,
                node_ed25519_pubkey,
                nonce,
                timestamp,
                bandwidth_claim,
            };
            let signature = identity.private_key().sign(content.signed_plaintext());
            LpMixnetRegistrationFinalisation { content, signature }
        }

        /// Check the content was signed by the client identity it names.
        ///
        /// This says nothing about whether the content is acceptable: that the node it names is
        /// the one checking, that its nonce is live, and that its timestamp is recent, are all the
        /// receiving node's checks.
        pub fn verify(&self) -> Result<(), ed25519::SignatureError> {
            self.content
                .client_ed25519_pubkey
                .verify(self.content.signed_plaintext(), &self.signature)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpMixnetRegistrationFinalisationContent {
        /// Client's ed25519 public key (identity).
        pub client_ed25519_pubkey: ed25519::PublicKey,

        /// Client's x25519 public key, repeated from the initial request so that the signature
        /// covers it: the name the node binds derives from both keys, so both must be attested.
        pub client_x25519_pubkey: x25519::PublicKey,

        /// The ed25519 identity of the node the request is meant for, so it cannot be presented at
        /// any other.
        pub node_ed25519_pubkey: ed25519::PublicKey,

        /// The nonce the node issued in answer to the initial request. Good once.
        pub nonce: RegistrationNonce,

        /// When the request was made, in unix seconds, so a leaked copy stops being accepted.
        pub timestamp: u64,

        /// Bandwidth to claim alongside the registration.
        ///
        /// Carried from the outset and unused for now, so that the day it is needed costs no
        /// change to the layout. Outside the signature: a credential carries its own proof, so
        /// signing it would attest nothing the node does not already check for itself.
        pub bandwidth_claim: Option<BandwidthClaim>,
    }

    impl LpMixnetRegistrationFinalisationContent {
        /// Whether [`Self::timestamp`] is within `max_skew_secs` of now, either way.
        pub fn is_recent(&self, max_skew_secs: u64) -> bool {
            super::timestamp_is_recent(self.timestamp, max_skew_secs)
        }

        /// What the signature covers: everything the node binds a name on the strength of.
        ///
        /// Built by hand rather than from the serialised form, so the signature stays independent
        /// of how the message happens to be encoded. [`Self::bandwidth_claim`] is deliberately
        /// absent - see its documentation.
        fn signed_plaintext(&self) -> Vec<u8> {
            MIXNET_REGISTRATION_DOMAIN
                .iter()
                .chain(&self.client_ed25519_pubkey.to_bytes())
                .chain(&self.client_x25519_pubkey.to_bytes())
                .chain(&self.node_ed25519_pubkey.to_bytes())
                .chain(&self.nonce)
                .chain(&self.timestamp.to_be_bytes())
                .copied()
                .collect()
        }
    }

    // gateway

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpMixnetRegistrationResponseMessage {
        pub content: LpMixnetRegistrationResponseMessageContent,
    }

    impl LpMixnetRegistrationResponseMessage {
        pub fn error(error: impl Into<String>) -> Self {
            LpMixnetRegistrationResponseMessage {
                content: LpMixnetRegistrationResponseMessageContent::RegistrationFailure(
                    RegistrationFailureResponse {
                        error: error.into(),
                    },
                ),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum LpMixnetRegistrationResponseMessageContent {
        /// Answer to an initial request: sign this and come back.
        Challenge(MixnetChallengeResponse),
        CompletedRegistration(CompletedRegistrationResponse),
        RegistrationFailure(RegistrationFailureResponse),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MixnetChallengeResponse {
        pub nonce: RegistrationNonce,
    }

    /// Gateway data for mixnet mode registration.
    // TODO: what we really need in here is the address of internal IPR. Is it though?
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct LpMixnetGatewayData {}

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CompletedRegistrationResponse {
        pub config: LpMixnetGatewayData,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RegistrationFailureResponse {
        pub error: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    // ==================== Helper Functions ====================

    fn create_test_wg_config() -> WireguardRegistrationData {
        WireguardRegistrationData {
            public_key: nym_crypto::asymmetric::x25519::PublicKey::from(
                nym_sphinx::PublicKey::from([1u8; 32]),
            ),
            port: 1234,
            private_ipv4: Ipv4Addr::new(10, 0, 0, 1),
            private_ipv6: Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1),
        }
    }

    // ==================== LpRegistrationRequest Tests ====================

    fn test_wg_peer_key() -> nym_wireguard_types::PeerPublicKey {
        nym_crypto::asymmetric::x25519::PublicKey::from(nym_sphinx::PublicKey::from([1u8; 32]))
            .into()
    }

    #[test]
    fn lp_registration_request_new_initial_dvpn_stamps_local_now() {
        let req = LpRegistrationRequest::new_initial_dvpn(test_wg_peer_key(), [0u8; 32]);
        let delta = current_timestamp().as_secs().abs_diff(req.timestamp);
        assert!(delta <= 1, "delta={delta}");
    }

    #[test]
    fn lp_registration_request_spend_time_skew_subtracts_ahead_clock() {
        let req = LpRegistrationRequest::new_initial_dvpn(test_wg_peer_key(), [0u8; 32])
            .with_spend_time_skew(time::Duration::seconds(38));
        let expected = current_timestamp().saturating_sub(std::time::Duration::from_secs(38));
        let delta = expected.as_secs().abs_diff(req.timestamp);
        assert!(delta <= 1, "delta={delta}");
    }

    #[test]
    fn lp_registration_request_spend_time_skew_adds_behind_clock() {
        let req = LpRegistrationRequest::new_initial_dvpn(test_wg_peer_key(), [0u8; 32])
            .with_spend_time_skew(time::Duration::seconds(-38));
        let expected = current_timestamp().saturating_add(std::time::Duration::from_secs(38));
        let delta = expected.as_secs().abs_diff(req.timestamp);
        assert!(delta <= 1, "delta={delta}");
    }

    // ==================== Mixnet registration signature ====================

    const NONCE: mixnet::RegistrationNonce = [9u8; 32];

    /// A client's keys, the node it registers with, and another node.
    fn mixnet_parties() -> (
        ed25519::KeyPair,
        x25519::PublicKey,
        ed25519::PublicKey,
        ed25519::PublicKey,
    ) {
        let mut rng = nym_test_utils::helpers::deterministic_rng();
        let client = ed25519::KeyPair::new(&mut rng);
        let client_encryption = *x25519::KeyPair::new(&mut rng).public_key();
        let node = *ed25519::KeyPair::new(&mut rng).public_key();
        let other_node = *ed25519::KeyPair::new(&mut rng).public_key();
        (client, client_encryption, node, other_node)
    }

    fn mixnet_message(request: LpRegistrationRequest) -> LpMixnetRegistrationRequestMessage {
        let LpRegistrationRequestData::Mixnet { data } = request.registration_data else {
            panic!("unexpected request")
        };
        *data
    }

    fn mixnet_finalisation(
        request: LpRegistrationRequest,
    ) -> mixnet::LpMixnetRegistrationFinalisation {
        match mixnet_message(request).content {
            mixnet::LpMixnetRegistrationRequestMessageContent::Finalisation(finalisation) => {
                finalisation
            }
            other => panic!("expected a finalisation, got {other:?}"),
        }
    }

    /// A finalisation for the given parties, signed over [`NONCE`].
    fn finalisation_for(
        client: &ed25519::KeyPair,
        client_encryption: &x25519::PublicKey,
        node: &ed25519::PublicKey,
    ) -> mixnet::LpMixnetRegistrationFinalisation {
        mixnet_finalisation(LpRegistrationRequest::new_finalise_mixnet(
            client,
            client_encryption,
            node,
            NONCE,
            None,
        ))
    }

    #[test]
    fn an_initial_mixnet_request_names_both_client_keys() {
        let (client, client_encryption, node, _) = mixnet_parties();
        let request = LpRegistrationRequest::new_initial_mixnet(
            client.public_key(),
            &client_encryption,
            &node,
        );

        match mixnet_message(request).content {
            mixnet::LpMixnetRegistrationRequestMessageContent::Initial(initial) => {
                assert_eq!(initial.client_ed25519_pubkey, *client.public_key());
                assert_eq!(initial.client_x25519_pubkey, client_encryption);
                assert_eq!(initial.node_ed25519_pubkey, node);
            }
            other => panic!("expected an initial request, got {other:?}"),
        }
    }

    #[test]
    fn a_finalisation_names_its_node_nonce_keys_and_timestamp() {
        let (client, client_encryption, node, _) = mixnet_parties();
        let request = LpRegistrationRequest::new_finalise_mixnet(
            &client,
            &client_encryption,
            &node,
            NONCE,
            None,
        );
        let timestamp = request.timestamp;
        let finalisation = mixnet_finalisation(request);

        assert_eq!(
            finalisation.content.client_ed25519_pubkey,
            *client.public_key()
        );
        assert_eq!(finalisation.content.client_x25519_pubkey, client_encryption);
        assert_eq!(finalisation.content.node_ed25519_pubkey, node);
        assert_eq!(finalisation.content.nonce, NONCE);
        assert_eq!(finalisation.content.timestamp, timestamp);
        assert!(finalisation.content.bandwidth_claim.is_none());
        assert!(finalisation.verify().is_ok());
    }

    #[test]
    fn a_finalisation_redirected_to_another_node_does_not_verify() {
        let (client, client_encryption, node, other_node) = mixnet_parties();
        let mut finalisation = finalisation_for(&client, &client_encryption, &node);
        finalisation.content.node_ed25519_pubkey = other_node;

        assert!(finalisation.verify().is_err());
    }

    /// Presenting the same signature against a different nonce is what a replay would have to do.
    #[test]
    fn a_finalisation_carrying_another_nonce_does_not_verify() {
        let (client, client_encryption, node, _) = mixnet_parties();
        let mut finalisation = finalisation_for(&client, &client_encryption, &node);
        finalisation.content.nonce = [1u8; 32];

        assert!(finalisation.verify().is_err());
    }

    /// The encryption key is in the signature because the name the node binds derives from it.
    #[test]
    fn a_finalisation_with_a_substituted_encryption_key_does_not_verify() {
        let (client, client_encryption, node, _) = mixnet_parties();
        let mut finalisation = finalisation_for(&client, &client_encryption, &node);
        finalisation.content.client_x25519_pubkey =
            x25519::PublicKey::from_bytes(&[7u8; 32]).unwrap();

        assert!(finalisation.verify().is_err());
    }

    #[test]
    fn a_finalisation_with_an_edited_timestamp_does_not_verify() {
        let (client, client_encryption, node, _) = mixnet_parties();
        let mut finalisation = finalisation_for(&client, &client_encryption, &node);
        finalisation.content.timestamp += 1;

        assert!(finalisation.verify().is_err());
    }

    #[test]
    fn a_finalisation_is_recent_only_within_the_skew() {
        let (client, client_encryption, node, _) = mixnet_parties();
        let mut finalisation = finalisation_for(&client, &client_encryption, &node);
        assert!(finalisation.content.is_recent(30));

        finalisation.content.timestamp -= 60;
        assert!(!finalisation.content.is_recent(30));

        finalisation.content.timestamp += 120;
        assert!(!finalisation.content.is_recent(30));
    }

    #[test]
    fn a_finalisation_signs_the_skewed_timestamp() {
        let (client, client_encryption, node, _) = mixnet_parties();
        let request = LpRegistrationRequest::new_finalise_mixnet_with_skew(
            &client,
            &client_encryption,
            &node,
            NONCE,
            None,
            time::Duration::seconds(38),
        );

        let expected = current_timestamp().saturating_sub(std::time::Duration::from_secs(38));
        let delta = expected.as_secs().abs_diff(request.timestamp);
        assert!(delta <= 1, "delta={delta}");

        let timestamp = request.timestamp;
        let finalisation = mixnet_finalisation(request);
        assert_eq!(finalisation.content.timestamp, timestamp);
        assert!(finalisation.verify().is_ok());
    }

    #[test]
    fn a_finalisation_does_not_verify_for_a_claimed_identity() {
        let (attacker, client_encryption, node, _) = mixnet_parties();
        let victim = ed25519::KeyPair::new(&mut nym_test_utils::helpers::seeded_rng([1; 32]));

        // the attacker's signature, under the victim's name
        let mut finalisation = finalisation_for(&attacker, &client_encryption, &node);
        finalisation.content.client_ed25519_pubkey = *victim.public_key();

        assert!(finalisation.verify().is_err());
    }

    #[test]
    fn a_mixnet_registration_survives_the_wire() {
        let (client, client_encryption, node, _) = mixnet_parties();

        let initial = LpRegistrationRequest::new_initial_mixnet(
            client.public_key(),
            &client_encryption,
            &node,
        );
        let decoded =
            LpRegistrationRequest::try_deserialise(&initial.serialise().unwrap()).unwrap();
        assert!(matches!(
            mixnet_message(decoded).content,
            mixnet::LpMixnetRegistrationRequestMessageContent::Initial(_)
        ));

        let request = LpRegistrationRequest::new_finalise_mixnet(
            &client,
            &client_encryption,
            &node,
            NONCE,
            None,
        );
        let decoded =
            LpRegistrationRequest::try_deserialise(&request.serialise().unwrap()).unwrap();
        assert!(mixnet_finalisation(decoded).verify().is_ok());
    }

    // ==================== LpRegistrationResponse Tests ====================

    #[test]
    fn test_lp_registration_response_error() {
        let error_msg = String::from("Insufficient bandwidth");

        let response_mixnet =
            LpRegistrationResponse::error(error_msg.clone(), RegistrationMode::Mixnet);
        let response_dvpn =
            LpRegistrationResponse::error(error_msg.clone(), RegistrationMode::Dvpn);

        assert!(response_mixnet.status.is_failed());
        assert!(response_dvpn.status.is_failed());

        // check mixnet
        let LpRegistrationResponseData::Mixnet { data } = response_mixnet.response_data else {
            panic!("unexpected response")
        };

        let LpMixnetRegistrationResponseMessageContent::RegistrationFailure(failure) = data.content
        else {
            panic!("unexpected response")
        };
        assert_eq!(failure.error, error_msg);

        // check dvpn
        let LpRegistrationResponseData::Dvpn { data } = response_dvpn.response_data else {
            panic!("unexpected response")
        };

        let LpDvpnRegistrationResponseMessageContent::RegistrationFailure(failure) = data.content
        else {
            panic!("unexpected response")
        };
        assert_eq!(failure.error, error_msg);
    }

    #[test]
    fn test_lp_registration_response_success_dvpn() {
        let cfg = create_test_wg_config();

        let response = LpRegistrationResponse::success_dvpn(cfg, false);
        assert!(response.status.is_successful());

        let LpRegistrationResponseData::Dvpn { data } = response.response_data else {
            panic!("unexpected response")
        };

        let LpDvpnRegistrationResponseMessageContent::CompletedRegistration(complete) =
            data.content
        else {
            panic!("unexpected response")
        };
        assert_eq!(complete.config, cfg);
        assert!(!complete.upgrade_mode);
    }

    #[test]
    fn test_lp_registration_response_success_mixnet() {
        let lp_gateway_data = LpMixnetGatewayData {};
        let response = LpRegistrationResponse::success_mixnet(lp_gateway_data.clone());
        assert!(response.status.is_successful());

        let LpRegistrationResponseData::Mixnet { data } = response.response_data else {
            panic!("unexpected response")
        };

        let LpMixnetRegistrationResponseMessageContent::CompletedRegistration(complete) =
            data.content
        else {
            panic!("unexpected response")
        };
        assert_eq!(complete.config, lp_gateway_data);
    }
}
