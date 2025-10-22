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
use nym_registration_common::GatewayData;
use nym_wireguard::PeerControlRequest;
use rand::RngCore;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tracing::*;

/// Prepare bandwidth storage for a client
async fn credential_storage_preparation(
    ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
    client_id: i64,
) -> Result<PersistedBandwidth, GatewayError> {
    ecash_verifier
        .storage()
        .create_bandwidth_entry(client_id)
        .await?;
    let bandwidth = ecash_verifier
        .storage()
        .get_available_bandwidth(client_id)
        .await?
        .ok_or_else(|| {
            GatewayError::InternalError(
                "bandwidth entry should have just been created".to_string(),
            )
        })?;
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
    Ok(verifier.verify().await?)
}

/// Process an LP registration request
pub async fn process_registration(
    request: LpRegistrationRequest,
    state: &LpHandlerState,
) -> LpRegistrationResponse {
    let session_id = rand::random::<u32>();

    // 1. Validate timestamp for replay protection
    if !request.validate_timestamp(30) {
        warn!("LP registration failed: timestamp too old or too far in future");
        return LpRegistrationResponse::error(
            session_id,
            "Invalid timestamp".to_string(),
        );
    }

    // 2. Process based on mode
    match request.mode {
        RegistrationMode::Dvpn => {
            // Register as WireGuard peer first to get client_id
            let (gateway_data, client_id) = match register_wg_peer(
                request.wg_public_key.inner().as_ref(),
                request.client_ip,
                request.ticket_type,
                state,
            ).await {
                Ok(result) => result,
                Err(e) => {
                    error!("LP WireGuard peer registration failed: {}", e);
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
            ).await {
                Ok(bandwidth) => bandwidth,
                Err(e) => {
                    // Credential verification failed, remove the peer
                    warn!("LP credential verification failed for client {}: {}", client_id, e);
                    if let Err(remove_err) = state.storage
                        .remove_wireguard_peer(&request.wg_public_key.to_string())
                        .await
                    {
                        error!("Failed to remove peer after credential verification failure: {}", remove_err);
                    }
                    return LpRegistrationResponse::error(
                        session_id,
                        format!("Credential verification failed: {}", e),
                    );
                }
            };

            info!("LP dVPN registration successful for session {} (client_id: {})", session_id, client_id);
            LpRegistrationResponse::success(
                session_id,
                allocated_bandwidth,
                gateway_data,
            )
        }
        RegistrationMode::Mixnet { client_id: client_id_bytes } => {
            // Generate i64 client_id from the [u8; 32] in the request
            let client_id = i64::from_be_bytes(client_id_bytes[0..8].try_into().unwrap());

            info!("LP Mixnet registration for client_id {}, session {}", client_id, session_id);

            // Verify credential with CredentialVerifier
            let allocated_bandwidth = match credential_verification(
                state.ecash_verifier.clone(),
                request.credential,
                client_id,
            ).await {
                Ok(bandwidth) => bandwidth,
                Err(e) => {
                    warn!("LP Mixnet credential verification failed for client {}: {}", client_id, e);
                    return LpRegistrationResponse::error(
                        session_id,
                        format!("Credential verification failed: {}", e),
                    );
                }
            };

            // For mixnet mode, we don't have WireGuard data
            // In the future, this would set up mixnet-specific state
            info!("LP Mixnet registration successful for session {} (client_id: {})", session_id, client_id);
            LpRegistrationResponse {
                success: true,
                error: None,
                gateway_data: None,
                allocated_bandwidth,
                session_id,
            }
        }
    }
}

/// Register a WireGuard peer and return gateway data along with the client_id
async fn register_wg_peer(
    public_key_bytes: &[u8],
    client_ip: IpAddr,
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
            "Invalid WireGuard public key length".to_string()
        ));
    }
    key_bytes.copy_from_slice(public_key_bytes);
    let peer_key = Key::new(key_bytes);

    // Allocate IP addresses for the client
    // TODO: Proper IP pool management - for now use random in private range
    let last_octet = {
        let mut rng = rand::thread_rng();
        (rng.next_u32() % 254 + 1) as u8
    };

    let client_ipv4 = Ipv4Addr::new(10, 1, 0, last_octet);
    let client_ipv6 = Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, last_octet as u16);

    // Create WireGuard peer
    let mut peer = Peer::new(peer_key.clone());
    peer.preshared_key = Some(Key::new(state.local_identity.public_key().to_bytes()));
    peer.endpoint = Some(format!("{}:51820", client_ip).parse().unwrap_or_else(|_| {
        SocketAddr::from_str("0.0.0.0:51820").unwrap()
    }));
    peer.allowed_ips = vec![
        format!("{}/32", client_ipv4).parse().unwrap(),
        format!("{}/128", client_ipv6).parse().unwrap(),
    ];
    peer.persistent_keepalive_interval = Some(25);

    // Send to WireGuard peer controller
    let (tx, rx) = oneshot::channel();
    wg_controller
        .send(PeerControlRequest::AddPeer {
            peer: peer.clone(),
            response_tx: tx,
        })
        .await
        .map_err(|e| GatewayError::InternalError(format!("Failed to send peer request: {}", e)))?;

    rx.await
        .map_err(|e| GatewayError::InternalError(format!("Failed to receive peer response: {}", e)))?
        .map_err(|e| GatewayError::InternalError(format!("Failed to add peer: {:?}", e)))?;

    // Store bandwidth allocation and get client_id
    let client_id = state.storage
        .insert_wireguard_peer(&peer, ticket_type.into())
        .await
        .map_err(|e| {
            error!("Failed to store WireGuard peer in database: {}", e);
            GatewayError::InternalError(format!("Failed to store peer: {}", e))
        })?;

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

// Helper function to convert bandwidth to ClientBandwidth if needed
// This would integrate with the actual bandwidth controller
#[allow(dead_code)]
async fn store_client_bandwidth(
    client_id: String,
    bandwidth: i64,
    storage: &nym_gateway_storage::GatewayStorage,
) -> Result<(), GatewayError> {
    // This would integrate with the actual bandwidth storage
    // For now, just log it
    info!("Storing bandwidth {} for client {}", bandwidth, client_id);
    Ok(())
}