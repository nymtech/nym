// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::{LpHandlerState, ReceiverIndex, TimestampedState};
use crate::error::GatewayError;
use defguard_wireguard_rs::host::Peer;
use defguard_wireguard_rs::key::Key;
use nym_authenticator_requests::models::BandwidthClaim;
use nym_credential_verification::ecash::traits::EcashManager;
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth, CredentialVerifier,
};
use nym_credentials_interface::{BandwidthCredential, CredentialSpendingData, TicketType};
use nym_crypto::asymmetric::encryption::KeyPair;
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::models::PersistedBandwidth;
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_metrics::{add_histogram_obs, inc, inc_by};
use nym_registration_common::dvpn::{
    LpDvpnRegistrationFinalisation, LpDvpnRegistrationInitialRequest,
    LpDvpnRegistrationRequestMessage, LpDvpnRegistrationRequestMessageContent,
};
use nym_registration_common::mixnet::LpMixnetRegistrationRequestMessage;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationRequestData, LpRegistrationResponse, RegistrationMode,
    RegistrationStatus, WireguardConfiguration,
};
use nym_wireguard::WireguardConfig;
use nym_wireguard_types::PeerPublicKey;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
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

#[derive(Clone, Copy)]
pub struct PendingRegistrationState {
    client_id: i64,
    peer_key: PeerPublicKey,
    ticket_type: TicketType,
    wireguard_config: WireguardConfiguration,
}

#[derive(Clone, Default)]
pub struct RegistrationsInProgress {
    /// Wrapped in TimestampedState for TTL-based cleanup of stale data.
    inner: Arc<Mutex<HashMap<ReceiverIndex, TimestampedState<PendingRegistrationState>>>>,
}

impl RegistrationsInProgress {
    pub async fn lock(
        &self,
    ) -> MutexGuard<'_, HashMap<ReceiverIndex, TimestampedState<PendingRegistrationState>>> {
        self.inner.lock().await
    }
}

impl LpHandlerState {
    fn upgrade_mode_enabled(&self) -> bool {
        self.upgrade_mode.enabled()
    }

    fn keypair(&self) -> &Arc<KeyPair> {
        self.peer_manager.wireguard_gateway_data.keypair()
    }

    fn wireguard_config(&self) -> WireguardConfig {
        self.peer_manager.wireguard_gateway_data.config()
    }

    fn successful_dvpn_registration(
        &self,
        peer_private_ipv4: Ipv4Addr,
        peer_private_ipv6: Ipv6Addr,
        bandwidth: i64,
    ) -> LpRegistrationResponse {
        LpRegistrationResponse::success_dvpn(
            WireguardConfiguration {
                public_key: *self.keypair().public_key(),
                // TODO: according to @SW this is most likely very wrong
                endpoint: self.wireguard_config().bind_address,
                private_ipv4: peer_private_ipv4,
                private_ipv6: peer_private_ipv6,
            },
            bandwidth,
        )
    }

    /// Check if WG peer already registered, return cached response if so.
    ///
    /// This enables idempotent registration: if a client retries registration
    /// with the same WG public key (e.g., after network failure), we return
    /// the existing registration data instead of re-processing. This prevents
    /// wasting credentials on network issues.
    async fn check_existing_dvpn_registration(
        &self,
        public_key: PeerPublicKey,
    ) -> Option<LpRegistrationResponse> {
        // Look up existing peer
        let Ok(maybe_peer) = self.peer_manager.query_peer(public_key).await else {
            return Some(LpRegistrationResponse::error(
                "iternal failure: failed to resolve peer information",
                RegistrationMode::Dvpn,
            ));
        };

        let peer = maybe_peer?;

        // Extract IPv4 and IPv6 from allowed_ips
        let mut private_ipv4 = None;
        let mut private_ipv6 = None;
        for ip_mask in &peer.allowed_ips {
            match ip_mask.address {
                IpAddr::V4(v4) => private_ipv4 = Some(v4),
                IpAddr::V6(v6) => private_ipv6 = Some(v6),
            }
            if private_ipv4.is_some() && private_ipv6.is_some() {
                break;
            }
        }

        // Incomplete data, treat as new registration
        let (Some(private_ipv4), Some(private_ipv6)) = (private_ipv4, private_ipv6) else {
            return None;
        };

        // Get current bandwidth
        let Ok(bandwidth) = self.peer_manager.query_client_bandwidth(public_key).await else {
            return Some(LpRegistrationResponse::error(
                "iternal failure: failed to resolve peer bandwidth",
                RegistrationMode::Dvpn,
            ));
        };

        Some(self.successful_dvpn_registration(
            private_ipv4,
            private_ipv6,
            bandwidth.available().await,
        ))
    }

    /// In the case of an already registered WG peer, update its PSK.
    async fn update_peer_psk(&self, peer: PeerPublicKey, psk: Key) -> Result<(), GatewayError> {
        let encoded_psk = psk.to_lower_hex();
        self.storage
            .update_peer_psk(&peer.to_string(), Some(&encoded_psk))
            .await?;

        // TODO: do we have to go through a peer manager to also update PSK if a peer is currently active?
        // seems like an edge case. maybe we should force disconnect here?
        Ok(())
    }

    async fn process_dvpn_initial_registration(
        &self,
        sender: ReceiverIndex,
        request: LpDvpnRegistrationInitialRequest,
    ) -> LpRegistrationResponse {
        let wg_key_str = request.wg_public_key.to_string();

        // check for an existing registration (same WG key already registered)
        // This allows clients to retry registration after network failures
        // or to re-use gateway without spending additional bandwidth
        if let Some(existing_response) = self
            .check_existing_dvpn_registration(request.wg_public_key)
            .await
        {
            // if there already exists registration for this client, update the psk and return the peer data
            if let Err(err) = self
                .update_peer_psk(request.wg_public_key, Key::new(request.psk))
                .await
            {
                return LpRegistrationResponse::error(
                    format!("WireGuard peer PSK update failed: {err}"),
                    RegistrationMode::Dvpn,
                );
            }
            info!("LP dVPN re-registration for existing peer {wg_key_str} (idempotent)",);
            inc!("lp_registration_dvpn_idempotent");
            return existing_response;
        }

        // TODO: this could be a source of some issue as we pre-allocate ip before validating credentials
        // (but we do the same in the authenticator anyway...)
        if let Err(err) = self
            .register_wg_peer(
                sender,
                request.wg_public_key,
                request.ticket_type,
                Key::new(request.psk),
            )
            .await
        {
            return LpRegistrationResponse::error(
                format!("WireGuard peer IP allocation failed: {err}"),
                RegistrationMode::Dvpn,
            );
        }

        LpRegistrationResponse::request_dvpn_credential()
    }

    // TODO: dedup
    async fn handle_final_credential_claim(
        &self,
        claim: BandwidthClaim,
        client_id: i64,
    ) -> Result<i64, GatewayError> {
        match claim.credential {
            BandwidthCredential::ZkNym(zk_nym) => {
                // if we got zk-nym, we just try to verify it
                let bandwidth =
                    credential_verification(self.ecash_verifier.clone(), *zk_nym, client_id)
                        .await?;
                Ok(bandwidth)
            }
            BandwidthCredential::UpgradeModeJWT { token } => {
                // TODO: move
                const UM_BANDWIDTH: i64 = 1024 * 1024 * 1024;

                // if we're already in the upgrade mode, don't bother validating the token
                if self.upgrade_mode_enabled() {
                    return Ok(UM_BANDWIDTH);
                }

                self.upgrade_mode.try_enable_via_received_jwt(token).await?;
                Ok(UM_BANDWIDTH)
            }
        }
    }

    async fn process_dvpn_registration_finalisation(
        &self,
        sender: ReceiverIndex,
        request: LpDvpnRegistrationFinalisation,
    ) -> LpRegistrationResponse {
        // see if we still have the pending registration
        // (e.g. it's illegal for client to request registration and only finalise it,
        // for example the next day; we can't keep the data forever)
        let Some(pending) = self
            .registrations_in_progress
            .lock()
            .await
            .get(&sender)
            .map(|pending| pending.state)
        else {
            return LpRegistrationResponse::error(
                "no pending registration",
                RegistrationMode::Dvpn,
            );
        };

        if pending.ticket_type != request.credential.kind {
            return LpRegistrationResponse::error(
                format!(
                    "inconsistent ticket type. used {} for initial request and {} for finalisation",
                    pending.ticket_type, request.credential.kind
                ),
                RegistrationMode::Dvpn,
            );
        }

        let client_id = pending.client_id;

        let allocated_bandwidth = match self
            .handle_final_credential_claim(request.credential, client_id)
            .await
        {
            Ok(bandwidth) => bandwidth,
            Err(err) => {
                // Credential verification failed, remove the peer
                warn!("LP credential verification failed for client {client_id}: {err}");
                inc!("lp_registration_dvpn_failed");
                if let Err(remove_err) = self
                    .storage
                    .remove_wireguard_peer(&pending.peer_key.to_string())
                    .await
                {
                    error!(
                        "Failed to remove peer after credential verification failure: {remove_err}"
                    );
                }
                self.registrations_in_progress.lock().await.remove(&sender);
                return LpRegistrationResponse::error(
                    format!("Credential verification failed: {err}"),
                    RegistrationMode::Dvpn,
                );
            }
        };

        info!("LP dVPN registration successful (client_id: {client_id})");
        inc!("lp_registration_dvpn_success");
        LpRegistrationResponse::success_dvpn(pending.wireguard_config, allocated_bandwidth)
    }

    async fn process_dvpn_registration(
        &self,
        sender: ReceiverIndex,
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

    async fn process_mixnet_registration(
        &self,
        request: LpMixnetRegistrationRequestMessage,
    ) -> LpRegistrationResponse {
        let _ = request;
        LpRegistrationResponse::error(
            "mixnet registration is not yet supported",
            RegistrationMode::Mixnet,
        )
    }

    /// Process an LP registration request
    pub async fn process_registration(
        &self,
        sender: ReceiverIndex,
        request: LpRegistrationRequest,
    ) -> LpRegistrationResponse {
        let registration_start = std::time::Instant::now();

        // Track total registration attempts
        inc!("lp_registration_attempts_total");

        // 1. Validate timestamp for replay protection
        if !request.validate_timestamp(30) {
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
                self.process_mixnet_registration(data).await
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

    /// Register a WireGuard peer and return gateway data along with the client_id
    async fn register_wg_peer(
        &self,
        sender: ReceiverIndex,
        peer_key: PeerPublicKey,
        ticket_type: nym_credentials_interface::TicketType,
        psk: Key,
    ) -> Result<(), GatewayError> {
        // Allocate IPs from centralized pool managed by PeerController
        let defguard_key = Key::new(peer_key.to_bytes());

        let registration_data = nym_wireguard::PeerRegistrationData::new(defguard_key.clone(), psk);

        let psk = registration_data.preshared_key.clone();

        // Request IP allocation from PeerController
        let ip_pair = self.peer_manager.register_peer(registration_data).await?;

        let client_ipv4 = ip_pair.ipv4;
        let client_ipv6 = ip_pair.ipv6;

        info!("Allocated IPs for peer {peer_key}: {client_ipv4} / {client_ipv6}");

        // Create WireGuard peer with allocated IPs
        let mut peer = Peer::new(defguard_key);
        peer.endpoint = None;
        peer.allowed_ips = vec![
            format!("{client_ipv4}/32").parse()?,
            format!("{client_ipv6}/128").parse()?,
        ];
        peer.persistent_keepalive_interval = Some(25);
        peer.preshared_key = Some(psk);

        // Store peer in database FIRST (before adding to controller)
        // This ensures bandwidth storage exists when controller's generate_bandwidth_manager() is called
        let client_id = self
            .storage
            .insert_wireguard_peer(&peer, ticket_type.into())
            .await
            .map_err(|e| {
                error!("Failed to store WireGuard peer in database: {}", e);
                GatewayError::InternalError(format!("Failed to store peer: {}", e))
            })?;

        // Create bandwidth entry for the client
        // This must happen BEFORE AddPeer because generate_bandwidth_manager() expects it to exist
        credential_storage_preparation(self.ecash_verifier.clone(), client_id).await?;

        // Now send peer to WireGuard controller and track latency
        let controller_start = std::time::Instant::now();
        let result = self.peer_manager.add_peer(peer).await;

        // Record peer controller channel latency
        let latency = controller_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "wg_peer_controller_channel_latency_seconds",
            latency,
            WG_CONTROLLER_LATENCY_BUCKETS
        );

        result?;

        // Get gateway's actual WireGuard public key
        let gateway_pubkey = *self.keypair().public_key();

        // Get gateway's WireGuard endpoint from config
        let gateway_endpoint = self.wireguard_config().bind_address;
        self.registrations_in_progress.lock().await.insert(
            sender,
            TimestampedState::new(PendingRegistrationState {
                client_id,
                peer_key,
                ticket_type,
                wireguard_config: WireguardConfiguration {
                    public_key: gateway_pubkey,
                    endpoint: gateway_endpoint,
                    private_ipv4: client_ipv4,
                    private_ipv6: client_ipv6,
                },
            }),
        );
        Ok(())
    }
}

// TODO: dedup
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

// TODO: dedup
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
