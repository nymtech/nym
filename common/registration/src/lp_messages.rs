// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration message types shared between client and gateway.

use crate::WireguardConfiguration;
use crate::serialisation::{BincodeError, BincodeOptions, lp_bincode_serializer};
use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};

/// Registration request sent by client after LP handshake
/// Aligned with existing authenticator registration flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRegistrationRequest {
    /// Mode specific registration data
    pub registration_data: LpRegistrationData,

    /// Unix timestamp for replay protection
    pub timestamp: u64,
}

impl LpRegistrationRequest {
    pub fn mode(&self) -> RegistrationMode {
        match self.registration_data {
            LpRegistrationData::Dvpn { .. } => RegistrationMode::Dvpn,
            LpRegistrationData::Mixnet { .. } => RegistrationMode::Mixnet,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LpRegistrationData {
    /// dVPN mode - register as WireGuard peer (most common)
    Dvpn {
        data: Box<LpDvpnRegistrationRequest>,
    },

    /// Mixnet mode - register for mixnet routing via IPR
    Mixnet { data: LpMixnetRegistrationRequest },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpDvpnRegistrationRequest {
    /// Client's WireGuard public key (for dVPN mode)
    pub wg_public_key: nym_wireguard_types::PeerPublicKey,

    /// Bandwidth credential for payment
    pub credential: CredentialSpendingData,

    /// Ticket type for bandwidth allocation
    pub ticket_type: TicketType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpMixnetRegistrationRequest {
    /// Client's ed25519 public key (identity)
    ///
    /// Used to derive DestinationAddressBytes for ActiveClientsStore lookup.
    pub client_ed25519_pubkey: ed25519::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationMode {
    /// dVPN mode - register as WireGuard peer (most common)
    Dvpn,

    /// Mixnet mode - register for mixnet routing via IPR
    Mixnet,
}

/// Gateway data for mixnet mode registration
///
/// Contains the gateway's identity and sphinx key needed for the client
/// to construct its full nym Recipient address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpMixnetGatewayData {
    /// Gateway's ed25519 identity public key
    ///
    /// Forms part of the client's nym Recipient address.
    pub gateway_identity: ed25519::PublicKey,
    // TODO: what we really need in here is the address of internal IPR
}

/// Registration response from gateway
/// Contains GatewayData for compatibility with existing client code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRegistrationResponse {
    /// Whether registration succeeded
    pub success: bool,

    /// Error message if registration failed
    pub error: Option<String>,

    /// Gateway configuration data for dVPN mode (WireGuard)
    /// This matches what WireguardRegistrationResult expects
    pub gateway_data: Option<WireguardConfiguration>,

    /// Gateway data for mixnet mode
    ///
    /// Contains gateway identity and sphinx key needed for nym address construction.
    /// Only populated for Mixnet mode registrations.
    pub lp_gateway_data: Option<LpMixnetGatewayData>,

    /// Allocated bandwidth in bytes
    pub allocated_bandwidth: i64,
}

impl LpRegistrationRequest {
    /// Create a new dVPN registration request
    pub fn new_dvpn(
        wg_public_key: nym_wireguard_types::PeerPublicKey,
        credential: CredentialSpendingData,
        ticket_type: TicketType,
    ) -> Self {
        Self {
            registration_data: LpRegistrationData::Dvpn {
                data: Box::new(LpDvpnRegistrationRequest {
                    wg_public_key,
                    credential,
                    ticket_type,
                }),
            },
            #[allow(clippy::expect_used)]
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time before UNIX epoch")
                .as_secs(),
        }
    }

    /// Validate the request timestamp is within acceptable bounds
    pub fn validate_timestamp(&self, max_skew_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

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
    pub fn success(allocated_bandwidth: i64, gateway_data: WireguardConfiguration) -> Self {
        Self {
            success: true,
            error: None,
            gateway_data: Some(gateway_data),
            lp_gateway_data: None,
            allocated_bandwidth,
        }
    }

    /// Create a success response for mixnet mode with LpGatewayData
    pub fn success_mixnet(allocated_bandwidth: i64, lp_gateway_data: LpMixnetGatewayData) -> Self {
        Self {
            success: true,
            error: None,
            gateway_data: None,
            lp_gateway_data: Some(lp_gateway_data),
            allocated_bandwidth,
        }
    }

    /// Create an error response
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            gateway_data: None,
            lp_gateway_data: None,
            allocated_bandwidth: 0,
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

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::helpers::deterministic_rng;
    use std::net::Ipv4Addr;
    // ==================== Helper Functions ====================

    fn create_test_gateway_data() -> WireguardConfiguration {
        use std::net::Ipv6Addr;

        WireguardConfiguration {
            public_key: nym_crypto::asymmetric::x25519::PublicKey::from(
                nym_sphinx::PublicKey::from([1u8; 32]),
            ),
            private_ipv4: Ipv4Addr::new(10, 0, 0, 1),
            private_ipv6: Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1),
            endpoint: "192.168.1.1:8080".parse().expect("Valid test endpoint"),
        }
    }

    // ==================== LpRegistrationRequest Tests ====================

    // ==================== LpRegistrationResponse Tests ====================

    #[test]
    fn test_lp_registration_response_success() {
        let gateway_data = create_test_gateway_data();
        let allocated_bandwidth = 1_000_000_000;

        let response = LpRegistrationResponse::success(allocated_bandwidth, gateway_data.clone());

        assert!(response.success);
        assert!(response.error.is_none());
        assert!(response.gateway_data.is_some());
        assert_eq!(response.allocated_bandwidth, allocated_bandwidth);

        let returned_gw_data = response
            .gateway_data
            .expect("Gateway data should be present in success response");
        assert_eq!(returned_gw_data.public_key, gateway_data.public_key);
        assert_eq!(returned_gw_data.private_ipv4, gateway_data.private_ipv4);
        assert_eq!(returned_gw_data.private_ipv6, gateway_data.private_ipv6);
        assert_eq!(returned_gw_data.endpoint, gateway_data.endpoint);
    }

    #[test]
    fn test_lp_registration_response_error() {
        let error_msg = String::from("Insufficient bandwidth");

        let response = LpRegistrationResponse::error(error_msg.clone());

        assert!(!response.success);
        assert_eq!(response.error, Some(error_msg));
        assert!(response.gateway_data.is_none());
        assert_eq!(response.allocated_bandwidth, 0);
    }

    #[test]
    fn test_lp_registration_response_success_mixnet() {
        let mut rng = deterministic_rng();
        let valid_key = ed25519::KeyPair::new(&mut rng);

        let lp_gateway_data = LpMixnetGatewayData {
            gateway_identity: *valid_key.public_key(),
        };
        let allocated_bandwidth = 500_000_000;

        let response = LpRegistrationResponse::success_mixnet(allocated_bandwidth, lp_gateway_data);

        assert!(response.success);
        assert!(response.error.is_none());
        assert!(response.gateway_data.is_none());
        assert!(response.lp_gateway_data.is_some());
        assert_eq!(response.allocated_bandwidth, allocated_bandwidth);

        let gw_data = response
            .lp_gateway_data
            .expect("LpGatewayData should be present");
        assert_eq!(gw_data.gateway_identity, *valid_key.public_key());
    }
}
