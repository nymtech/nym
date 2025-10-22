// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_registration_common::GatewayData;
use nym_wireguard_types::PeerPublicKey;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Registration request sent by client after LP handshake
/// Aligned with existing authenticator registration flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpRegistrationRequest {
    /// Client's WireGuard public key (for dVPN mode)
    pub wg_public_key: PeerPublicKey,

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
        client_id: [u8; 32]
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
        wg_public_key: PeerPublicKey,
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
            .unwrap()
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