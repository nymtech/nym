// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::messages::{LpRegistrationRequest, LpRegistrationResponse, RegistrationMode};
use super::LpHandlerState;
use crate::error::GatewayError;
use defguard_wireguard_rs::host::Peer;
use defguard_wireguard_rs::key::Key;
use futures::channel::oneshot;
use nym_credential_verification::ecash::traits::EcashManager;
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth, CredentialVerifier,
};
use nym_credentials_interface::CredentialSpendingData;
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::models::PersistedBandwidth;
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_metrics::{add_histogram_obs, inc, inc_by};
use nym_registration_common::GatewayData;
use nym_wireguard::PeerControlRequest;
use std::sync::Arc;
use tracing::*;

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

// Histogram buckets for WireGuard peer controller channel latency
// Measures time to send request and receive response from peer controller
// Expected: 1ms-100ms for normal operations, up to 2s for slow conditions
const WG_CONTROLLER_LATENCY_BUCKETS: &[f64] = &[
    0.001, // 1ms
    0.005, // 5ms
    0.01,  // 10ms
    0.05,  // 50ms
    0.1,   // 100ms
    0.25,  // 250ms
    0.5,   // 500ms
    1.0,   // 1s
    2.0,   // 2s
];

/// Prepare bandwidth storage for a client
async fn credential_storage_preparation(
    ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
    client_id: i64,
) -> Result<PersistedBandwidth, GatewayError> {
    // Check if bandwidth entry already exists (idempotent)
    let existing_bandwidth = ecash_verifier
        .storage()
        .get_available_bandwidth(client_id)
        .await?;

    // Only create if it doesn't exist
    if existing_bandwidth.is_none() {
        ecash_verifier
            .storage()
            .create_bandwidth_entry(client_id)
            .await?;
    }

    let bandwidth = ecash_verifier
        .storage()
        .get_available_bandwidth(client_id)
        .await?
        .ok_or_else(|| GatewayError::InternalError("bandwidth entry should exist".to_string()))?;
    Ok(bandwidth)
}

/// Verify credential and allocate bandwidth using CredentialVerifier
async fn credential_verification(
    ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
    credential: CredentialSpendingData,
    client_id: i64,
) -> Result<i64, GatewayError> {
    let bandwidth = credential_storage_preparation(ecash_verifier.clone(), client_id).await?;
    let client_bandwidth = ClientBandwidth::new(bandwidth.into());
    let mut verifier = CredentialVerifier::new(
        CredentialSpendingRequest::new(credential),
        ecash_verifier.clone(),
        BandwidthStorageManager::new(
            ecash_verifier.storage(),
            client_bandwidth,
            client_id,
            BandwidthFlushingBehaviourConfig::default(),
            true,
        ),
    );

    // Track credential verification attempts
    inc!("lp_credential_verification_attempts");

    // For mock ecash mode (local testing), skip cryptographic verification
    // and just return a dummy bandwidth value since we don't have blockchain access
    let allocated = if ecash_verifier.is_mock() {
        // Return a reasonable test bandwidth value (e.g., 1GB in bytes)
        const MOCK_BANDWIDTH: i64 = 1024 * 1024 * 1024;
        inc!("lp_credential_verification_success");
        inc_by!("lp_bandwidth_allocated_bytes_total", MOCK_BANDWIDTH);
        Ok::<i64, GatewayError>(MOCK_BANDWIDTH)
    } else {
        match verifier.verify().await {
            Ok(allocated) => {
                inc!("lp_credential_verification_success");
                // Track allocated bandwidth
                inc_by!("lp_bandwidth_allocated_bytes_total", allocated);
                Ok(allocated)
            }
            Err(e) => {
                inc!("lp_credential_verification_failed");
                Err(e.into())
            }
        }
    }?;

    Ok(allocated)
}

/// Process an LP registration request
pub async fn process_registration(
    request: LpRegistrationRequest,
    state: &LpHandlerState,
) -> LpRegistrationResponse {
    let session_id = rand::random::<u32>();
    let registration_start = std::time::Instant::now();

    // Track total registration attempts
    inc!("lp_registration_attempts_total");

    // 1. Validate timestamp for replay protection
    if !request.validate_timestamp(30) {
        warn!("LP registration failed: timestamp too old or too far in future");
        inc!("lp_registration_failed_timestamp");
        return LpRegistrationResponse::error(session_id, "Invalid timestamp".to_string());
    }

    // 2. Process based on mode
    let result = match request.mode {
        RegistrationMode::Dvpn => {
            // Track dVPN registration attempts
            inc!("lp_registration_dvpn_attempts");
            // Register as WireGuard peer first to get client_id
            let (gateway_data, client_id) = match register_wg_peer(
                request.wg_public_key.inner().as_ref(),
                request.ticket_type,
                state,
            )
            .await
            {
                Ok(result) => result,
                Err(e) => {
                    error!("LP WireGuard peer registration failed: {}", e);
                    inc!("lp_registration_dvpn_failed");
                    inc!("lp_errors_wg_peer_registration");
                    return LpRegistrationResponse::error(
                        session_id,
                        format!("WireGuard peer registration failed: {}", e),
                    );
                }
            };

            // Verify credential with CredentialVerifier (handles double-spend, storage, etc.)
            let allocated_bandwidth = match credential_verification(
                state.ecash_verifier.clone(),
                request.credential,
                client_id,
            )
            .await
            {
                Ok(bandwidth) => bandwidth,
                Err(e) => {
                    // Credential verification failed, remove the peer
                    warn!(
                        "LP credential verification failed for client {}: {}",
                        client_id, e
                    );
                    inc!("lp_registration_dvpn_failed");
                    if let Err(remove_err) = state
                        .storage
                        .remove_wireguard_peer(&request.wg_public_key.to_string())
                        .await
                    {
                        error!(
                            "Failed to remove peer after credential verification failure: {}",
                            remove_err
                        );
                    }
                    return LpRegistrationResponse::error(
                        session_id,
                        format!("Credential verification failed: {}", e),
                    );
                }
            };

            info!(
                "LP dVPN registration successful for session {} (client_id: {})",
                session_id, client_id
            );
            inc!("lp_registration_dvpn_success");
            LpRegistrationResponse::success(session_id, allocated_bandwidth, gateway_data)
        }
        RegistrationMode::Mixnet {
            client_id: client_id_bytes,
        } => {
            // Track mixnet registration attempts
            inc!("lp_registration_mixnet_attempts");

            // Generate i64 client_id from the [u8; 32] in the request
            #[allow(clippy::expect_used)]
            let client_id = i64::from_be_bytes(
                client_id_bytes[0..8]
                    .try_into()
                    .expect("This cannot fail, since the id is 32 bytes long"),
            );

            info!(
                "LP Mixnet registration for client_id {}, session {}",
                client_id, session_id
            );

            // Verify credential with CredentialVerifier
            let allocated_bandwidth = match credential_verification(
                state.ecash_verifier.clone(),
                request.credential,
                client_id,
            )
            .await
            {
                Ok(bandwidth) => bandwidth,
                Err(e) => {
                    warn!(
                        "LP Mixnet credential verification failed for client {}: {}",
                        client_id, e
                    );
                    inc!("lp_registration_mixnet_failed");
                    return LpRegistrationResponse::error(
                        session_id,
                        format!("Credential verification failed: {}", e),
                    );
                }
            };

            // For mixnet mode, we don't have WireGuard data
            // In the future, this would set up mixnet-specific state
            info!(
                "LP Mixnet registration successful for session {} (client_id: {})",
                session_id, client_id
            );
            inc!("lp_registration_mixnet_success");
            LpRegistrationResponse {
                success: true,
                error: None,
                gateway_data: None,
                allocated_bandwidth,
                session_id,
            }
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
    if result.success {
        inc!("lp_registration_success_total");
    } else {
        inc!("lp_registration_failed_total");
    }

    result
}

/// Register a WireGuard peer and return gateway data along with the client_id
async fn register_wg_peer(
    public_key_bytes: &[u8],
    ticket_type: nym_credentials_interface::TicketType,
    state: &LpHandlerState,
) -> Result<(GatewayData, i64), GatewayError> {
    let Some(wg_controller) = &state.wg_peer_controller else {
        return Err(GatewayError::ServiceProviderNotRunning {
            service: "WireGuard".to_string(),
        });
    };

    let Some(wg_data) = &state.wireguard_data else {
        return Err(GatewayError::ServiceProviderNotRunning {
            service: "WireGuard".to_string(),
        });
    };

    // Convert public key bytes to WireGuard Key
    let mut key_bytes = [0u8; 32];
    if public_key_bytes.len() != 32 {
        return Err(GatewayError::LpProtocolError(
            "Invalid WireGuard public key length".to_string(),
        ));
    }
    key_bytes.copy_from_slice(public_key_bytes);
    let peer_key = Key::new(key_bytes);

    // Allocate IPs from centralized pool managed by PeerController
    let registration_data = nym_wireguard::PeerRegistrationData::new(peer_key.clone());

    // Request IP allocation from PeerController
    let (tx, rx) = oneshot::channel();
    wg_controller
        .send(PeerControlRequest::RegisterPeer {
            registration_data,
            response_tx: tx,
        })
        .await
        .map_err(|e| {
            GatewayError::InternalError(format!("Failed to send IP allocation request: {}", e))
        })?;

    // Wait for IP allocation from pool
    let ip_pair = rx
        .await
        .map_err(|e| {
            GatewayError::InternalError(format!("Failed to receive IP allocation: {}", e))
        })?
        .map_err(|e| {
            error!("Failed to allocate IPs from pool: {}", e);
            GatewayError::InternalError(format!("Failed to allocate IPs: {:?}", e))
        })?;

    let client_ipv4 = ip_pair.ipv4;
    let client_ipv6 = ip_pair.ipv6;

    info!(
        "Allocated IPs for peer {}: {} / {}",
        peer_key, client_ipv4, client_ipv6
    );

    // Create WireGuard peer with allocated IPs
    let mut peer = Peer::new(peer_key.clone());
    peer.preshared_key = Some(Key::new(state.local_identity.public_key().to_bytes()));
    peer.endpoint = None;
    peer.allowed_ips = vec![
        format!("{client_ipv4}/32").parse()?,
        format!("{client_ipv6}/128").parse()?,
    ];
    peer.persistent_keepalive_interval = Some(25);

    // Store peer in database FIRST (before adding to controller)
    // This ensures bandwidth storage exists when controller's generate_bandwidth_manager() is called
    let client_id = state
        .storage
        .insert_wireguard_peer(&peer, ticket_type.into())
        .await
        .map_err(|e| {
            error!("Failed to store WireGuard peer in database: {}", e);
            GatewayError::InternalError(format!("Failed to store peer: {}", e))
        })?;

    // Create bandwidth entry for the client
    // This must happen BEFORE AddPeer because generate_bandwidth_manager() expects it to exist
    credential_storage_preparation(state.ecash_verifier.clone(), client_id).await?;

    // Now send peer to WireGuard controller and track latency
    let controller_start = std::time::Instant::now();
    let (tx, rx) = oneshot::channel();
    wg_controller
        .send(PeerControlRequest::AddPeer {
            peer: peer.clone(),
            response_tx: tx,
        })
        .await
        .map_err(|e| GatewayError::InternalError(format!("Failed to send peer request: {}", e)))?;

    let result = rx
        .await
        .map_err(|e| {
            GatewayError::InternalError(format!("Failed to receive peer response: {}", e))
        })?
        .map_err(|e| GatewayError::InternalError(format!("Failed to add peer: {:?}", e)));

    // Record peer controller channel latency
    let latency = controller_start.elapsed().as_secs_f64();
    add_histogram_obs!(
        "wg_peer_controller_channel_latency_seconds",
        latency,
        WG_CONTROLLER_LATENCY_BUCKETS
    );

    result?;

    // Get gateway's actual WireGuard public key
    let gateway_pubkey = *wg_data.keypair().public_key();

    // Get gateway's WireGuard endpoint from config
    let gateway_endpoint = wg_data.config().bind_address;

    // Create GatewayData response (matching authenticator response format)
    Ok((
        GatewayData {
            public_key: gateway_pubkey,
            endpoint: gateway_endpoint,
            private_ipv4: client_ipv4,
            private_ipv6: client_ipv6,
        },
        client_id,
    ))
}
