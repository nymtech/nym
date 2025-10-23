// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! LP (Lewes Protocol) registration message types shared between client and gateway.

use nym_credentials_interface::{CredentialSpendingData, TicketType};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

use crate::GatewayData;

/// Registration request sent by client after LP handshake
/// Aligned with existing authenticator registration flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRegistrationRequest {
    /// Client's WireGuard public key (for dVPN mode)
    pub wg_public_key: nym_wireguard_types::PeerPublicKey,

    /// Bandwidth credential for payment
    pub credential: CredentialSpendingData,

    /// Ticket type for bandwidth allocation
    pub ticket_type: TicketType,

    /// Registration mode
    pub mode: RegistrationMode,

    /// Client's IP address (for tracking/metrics)
    pub client_ip: IpAddr,

    /// Unix timestamp for replay protection
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationMode {
    /// dVPN mode - register as WireGuard peer (most common)
    Dvpn,

    /// Mixnet mode - register for mixnet usage (future)
    Mixnet {
        /// Client identifier for mixnet mode
        client_id: [u8; 32],
    },
}

/// Registration response from gateway
/// Contains GatewayData for compatibility with existing client code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRegistrationResponse {
    /// Whether registration succeeded
    pub success: bool,

    /// Error message if registration failed
    pub error: Option<String>,

    /// Gateway configuration data (same as returned by authenticator)
    /// This matches what WireguardRegistrationResult expects
    pub gateway_data: Option<GatewayData>,

    /// Allocated bandwidth in bytes
    pub allocated_bandwidth: i64,

    /// Session identifier for future reference
    pub session_id: u32,
}

impl LpRegistrationRequest {
    /// Create a new dVPN registration request
    pub fn new_dvpn(
        wg_public_key: nym_wireguard_types::PeerPublicKey,
        credential: CredentialSpendingData,
        ticket_type: TicketType,
        client_ip: IpAddr,
    ) -> Self {
        Self {
            wg_public_key,
            credential,
            ticket_type,
            mode: RegistrationMode::Dvpn,
            client_ip,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
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
}

impl LpRegistrationResponse {
    /// Create a success response with GatewayData
    pub fn success(
        session_id: u32,
        allocated_bandwidth: i64,
        gateway_data: GatewayData,
    ) -> Self {
        Self {
            success: true,
            error: None,
            gateway_data: Some(gateway_data),
            allocated_bandwidth,
            session_id,
        }
    }

    /// Create an error response
    pub fn error(session_id: u32, error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            gateway_data: None,
            allocated_bandwidth: 0,
            session_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    // ==================== Helper Functions ====================

    fn create_test_gateway_data() -> GatewayData {
        use std::net::Ipv6Addr;

        GatewayData {
            public_key: nym_crypto::asymmetric::x25519::PublicKey::from(nym_sphinx::PublicKey::from([1u8; 32])),
            private_ipv4: Ipv4Addr::new(10, 0, 0, 1),
            private_ipv6: Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1),
            endpoint: "192.168.1.1:8080".parse().unwrap(),
        }
    }


    // ==================== LpRegistrationRequest Tests ====================

    // ==================== LpRegistrationResponse Tests ====================

    #[test]
    fn test_lp_registration_response_success() {
        let gateway_data = create_test_gateway_data();
        let session_id = 12345;
        let allocated_bandwidth = 1_000_000_000;

        let response = LpRegistrationResponse::success(session_id, allocated_bandwidth, gateway_data.clone());

        assert!(response.success);
        assert!(response.error.is_none());
        assert!(response.gateway_data.is_some());
        assert_eq!(response.allocated_bandwidth, allocated_bandwidth);
        assert_eq!(response.session_id, session_id);

        let returned_gw_data = response.gateway_data.unwrap();
        assert_eq!(returned_gw_data.public_key, gateway_data.public_key);
        assert_eq!(returned_gw_data.private_ipv4, gateway_data.private_ipv4);
        assert_eq!(returned_gw_data.private_ipv6, gateway_data.private_ipv6);
        assert_eq!(returned_gw_data.endpoint, gateway_data.endpoint);
    }

    #[test]
    fn test_lp_registration_response_error() {
        let session_id = 54321;
        let error_msg = String::from("Insufficient bandwidth");

        let response = LpRegistrationResponse::error(session_id, error_msg.clone());

        assert!(!response.success);
        assert_eq!(response.error, Some(error_msg));
        assert!(response.gateway_data.is_none());
        assert_eq!(response.allocated_bandwidth, 0);
        assert_eq!(response.session_id, session_id);
    }

    #[test]
    fn test_lp_registration_response_serialize_deserialize_success() {
        let gateway_data = create_test_gateway_data();
        let original = LpRegistrationResponse::success(999, 5_000_000_000, gateway_data);

        // Serialize
        let serialized = bincode::serialize(&original).expect("Failed to serialize response");

        // Deserialize
        let deserialized: LpRegistrationResponse =
            bincode::deserialize(&serialized).expect("Failed to deserialize response");

        assert_eq!(deserialized.success, original.success);
        assert_eq!(deserialized.error, original.error);
        assert_eq!(deserialized.allocated_bandwidth, original.allocated_bandwidth);
        assert_eq!(deserialized.session_id, original.session_id);
        assert!(deserialized.gateway_data.is_some());
    }

    #[test]
    fn test_lp_registration_response_serialize_deserialize_error() {
        let original = LpRegistrationResponse::error(777, String::from("Test error message"));

        // Serialize
        let serialized = bincode::serialize(&original).expect("Failed to serialize response");

        // Deserialize
        let deserialized: LpRegistrationResponse =
            bincode::deserialize(&serialized).expect("Failed to deserialize response");

        assert_eq!(deserialized.success, original.success);
        assert_eq!(deserialized.error, original.error);
        assert_eq!(deserialized.allocated_bandwidth, 0);
        assert_eq!(deserialized.session_id, original.session_id);
        assert!(deserialized.gateway_data.is_none());
    }

    #[test]
    fn test_lp_registration_response_malformed_deserialize() {
        // Create invalid bincode data
        let invalid_data = vec![0xFF; 100];

        // Attempt to deserialize
        let result: Result<LpRegistrationResponse, _> = bincode::deserialize(&invalid_data);

        assert!(result.is_err(), "Expected deserialization to fail for malformed data");
    }

    // ==================== RegistrationMode Tests ====================

    #[test]
    fn test_registration_mode_serialize_dvpn() {
        let mode = RegistrationMode::Dvpn;

        let serialized = bincode::serialize(&mode).expect("Failed to serialize mode");
        let deserialized: RegistrationMode =
            bincode::deserialize(&serialized).expect("Failed to deserialize mode");

        assert!(matches!(deserialized, RegistrationMode::Dvpn));
    }

    #[test]
    fn test_registration_mode_serialize_mixnet() {
        let client_id = [99u8; 32];
        let mode = RegistrationMode::Mixnet { client_id };

        let serialized = bincode::serialize(&mode).expect("Failed to serialize mode");
        let deserialized: RegistrationMode =
            bincode::deserialize(&serialized).expect("Failed to deserialize mode");

        match deserialized {
            RegistrationMode::Mixnet { client_id: id } => {
                assert_eq!(id, client_id);
            }
            _ => panic!("Expected Mixnet mode"),
        }
    }
}
