// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration message types shared between client and gateway.

use crate::WireguardConfiguration;
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
use nym_credentials_interface::TicketType;
use serde::{Deserialize, Serialize};
use tracing::error;

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
        data: LpMixnetRegistrationRequestMessage,
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

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .inspect_err(|_| error!("the current timestamp predates unix epoch!"))
        .unwrap_or_default()
        .as_secs()
}

impl LpRegistrationRequest {
    /// Helper wrapping timestamp extraction
    fn new(registration_data: LpRegistrationRequestData) -> LpRegistrationRequest {
        Self {
            registration_data,
            timestamp: current_timestamp(),
        }
    }

    /// Create new dVPN registration initialisation request
    pub fn new_initial_dvpn(
        wg_public_key: nym_wireguard_types::PeerPublicKey,
        psk: [u8; 32],
        ticket_type: TicketType,
    ) -> Self {
        Self::new(LpRegistrationRequestData::Dvpn {
            data: Box::new(LpDvpnRegistrationRequestMessage {
                content: LpDvpnRegistrationRequestMessageContent::InitialRequest(
                    LpDvpnRegistrationInitialRequest {
                        wg_public_key,
                        psk,
                        ticket_type,
                    },
                ),
            }),
        })
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

    /// Validate the request timestamp is within acceptable bounds
    pub fn validate_timestamp(&self, max_skew_secs: u64) -> bool {
        let now = current_timestamp();

        (now as i64 - self.timestamp as i64).abs() <= max_skew_secs as i64
    }

    /// Attempt to serialise this `LpRegistrationRequest` into bytes.
    pub fn serialise(&self) -> Result<Vec<u8>, BincodeError> {
        lp_bincode_serializer().serialize(self)
    }

    /// Attempt to deserialise a `LpRegistrationRequest` from bytes.
    pub fn try_deserialise(b: &[u8]) -> Result<Self, BincodeError> {
        lp_bincode_serializer().deserialize(b)
    }
}

impl LpRegistrationResponse {
    /// Create a success response with GatewayData (for dVPN mode)
    pub fn success_dvpn(config: WireguardConfiguration, available_bandwidth: i64) -> Self {
        Self {
            status: RegistrationStatus::Completed,
            response_data: LpRegistrationResponseData::Dvpn {
                data: LpDvpnRegistrationResponseMessage {
                    content: LpDvpnRegistrationResponseMessageContent::CompletedRegistration(
                        dvpn::CompletedRegistrationResponse {
                            config,
                            available_bandwidth,
                        },
                    ),
                },
            },
        }
    }

    pub fn success_mixnet(config: LpMixnetGatewayData, available_bandwidth: i64) -> Self {
        Self {
            status: RegistrationStatus::Completed,
            response_data: LpRegistrationResponseData::Mixnet {
                data: LpMixnetRegistrationResponseMessage {
                    content: LpMixnetRegistrationResponseMessageContent::CompletedRegistration(
                        mixnet::CompletedRegistrationResponse {
                            config,
                            available_bandwidth,
                        },
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
    use crate::WireguardConfiguration;
    use nym_authenticator_requests::models::BandwidthClaim;
    use nym_credentials_interface::TicketType;
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

        /// Type of the ticket/gateway we're going to register with
        pub ticket_type: TicketType,
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
        pub config: WireguardConfiguration,

        /// The bandwidth available to this client,
        pub available_bandwidth: i64,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct RequiresCredentialResponse;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RegistrationFailureResponse {
        pub error: String,
    }
}

pub mod mixnet {
    use nym_crypto::asymmetric::ed25519;
    use serde::{Deserialize, Serialize};

    // client
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpMixnetRegistrationRequestMessage {
        pub content: LpMixnetRegistrationRequestContent,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LpMixnetRegistrationRequestContent {
        /// Client's ed25519 public key (identity)
        ///
        /// Used to derive DestinationAddressBytes for ActiveClientsStore lookup.
        pub client_ed25519_pubkey: ed25519::PublicKey,
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
        CompletedRegistration(CompletedRegistrationResponse),
        RegistrationFailure(RegistrationFailureResponse),
    }

    /// Gateway data for mixnet mode registration
    ///
    /// Contains the gateway's identity and sphinx key needed for the client
    /// to construct its full nym Recipient address.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct LpMixnetGatewayData {
        /// Gateway's ed25519 identity public key
        ///
        /// Forms part of the client's nym Recipient address.
        pub gateway_identity: ed25519::PublicKey,
        // TODO: what we really need in here is the address of internal IPR
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CompletedRegistrationResponse {
        /// Gateway data for mixnet mode
        ///
        /// Contains gateway identity and sphinx key needed for nym address construction.
        pub config: LpMixnetGatewayData,

        /// The bandwidth available to this client,
        pub available_bandwidth: i64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RegistrationFailureResponse {
        pub error: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::ed25519;
    use nym_test_utils::helpers::deterministic_rng;
    use std::net::{Ipv4Addr, Ipv6Addr};
    // ==================== Helper Functions ====================

    fn create_test_gateway_data() -> WireguardConfiguration {
        WireguardConfiguration {
            public_key: nym_crypto::asymmetric::x25519::PublicKey::from(
                nym_sphinx::PublicKey::from([1u8; 32]),
            ),
            psk: Some([42u8; 32]),
            private_ipv4: Ipv4Addr::new(10, 0, 0, 1),
            private_ipv6: Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1),
            endpoint: "192.168.1.1:8080".parse().expect("Valid test endpoint"),
        }
    }

    // ==================== LpRegistrationRequest Tests ====================

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
        let cfg = create_test_gateway_data();
        let allocated_bandwidth = 500_000_000;

        let response = LpRegistrationResponse::success_dvpn(cfg, allocated_bandwidth);
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
        assert_eq!(complete.available_bandwidth, allocated_bandwidth);
    }

    #[test]
    fn test_lp_registration_response_success_mixnet() {
        let mut rng = deterministic_rng();
        let valid_key = ed25519::KeyPair::new(&mut rng);

        let lp_gateway_data = LpMixnetGatewayData {
            gateway_identity: *valid_key.public_key(),
        };
        let allocated_bandwidth = 500_000_000;

        let response =
            LpRegistrationResponse::success_mixnet(lp_gateway_data.clone(), allocated_bandwidth);
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
        assert_eq!(complete.available_bandwidth, allocated_bandwidth);
    }
}
