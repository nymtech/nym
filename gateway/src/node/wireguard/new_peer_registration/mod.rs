// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Unification of Nym registration flow
//! In general the registration has the following structure:
//! 1. Initial request message is received
//!    1.1. We check if the peer has already registered before -> if so, we returned the past information
//!    1.2. We check if the peer already has a pending registration -> if so, we return the past information
//!    1.3. We pre-allocated [`nym_wireguard::ip_pool::IpPair`] and save time-sensitive pending registration.
//!    If it does not complete within specified time interval, the information is going to get removed.
//! 2. Finalisation request message is received, where credential has to be attached is verified.
//!    Upon successful completion, pending registration is transformed into a properly inserted peer.

use crate::node::wireguard::new_peer_registration::pending::{
    PendingRegistration, PendingRegistrations,
};
use crate::node::wireguard::{GatewayWireguardError, PeerManager};
use defguard_wireguard_rs::host::Peer;
use defguard_wireguard_rs::key::Key;
use defguard_wireguard_rs::net::IpAddrMask;
use nym_authenticator_requests::models::BandwidthClaim;
use nym_authenticator_requests::response::SerialisedResponse;
use nym_authenticator_requests::traits::{FinalMessage, InitMessage};
use nym_credential_verification::bandwidth_storage_manager::BandwidthStorageManager;
use nym_credential_verification::ecash::traits::EcashManager;
use nym_credential_verification::upgrade_mode::UpgradeModeDetails;
use nym_credential_verification::{
    BandwidthFlushingBehaviourConfig, ClientBandwidth, CredentialVerifier,
};
use nym_credentials_interface::{BandwidthCredential, CredentialSpendingData};
use nym_crypto::asymmetric::x25519;
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::models::PersistedBandwidth;
use nym_registration_common::dvpn::{
    LpDvpnRegistrationFinalisation, LpDvpnRegistrationInitialRequest,
};
use nym_registration_common::LpRegistrationResponse;
use nym_sdk::mixnet::Recipient;
use nym_service_provider_requests_common::Protocol;
use nym_task::ShutdownToken;
use nym_wireguard::WireguardConfig;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval_at, Instant};
use tracing::trace;

mod authenticator;
mod helpers;
mod lp;
mod pending;

#[derive(Clone)]
pub struct PeerRegistrator {
    /// Handle for the structure managing verification of the ecash credentials for the bandwidth control
    pub(crate) ecash_verifier: Arc<dyn EcashManager + Send + Sync>,

    /// Handle for communication with the [`nym_wireguard::peer_controller::PeerController`]
    pub(crate) peer_manager: PeerManager,

    /// Information about the current state of the upgrade mode as well as a handle
    /// to remotely trigger the recheck
    pub(crate) upgrade_mode: UpgradeModeDetails,

    /// Registrations in progress
    pub(crate) pending_registrations: PendingRegistrations,
}

impl PeerRegistrator {
    pub fn new(
        ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
        peer_manager: PeerManager,
        upgrade_mode: UpgradeModeDetails,
    ) -> Self {
        PeerRegistrator {
            ecash_verifier,
            peer_manager,
            upgrade_mode,
            pending_registrations: Default::default(),
        }
    }

    pub fn cleanup_task(&self, shutdown_token: ShutdownToken) -> StaleRegistrationRemover {
        StaleRegistrationRemover {
            pending_registrations: self.pending_registrations.clone(),
            shutdown_token,
        }
    }

    fn upgrade_mode_enabled(&self) -> bool {
        self.upgrade_mode.enabled()
    }

    fn keypair(&self) -> &Arc<x25519::KeyPair> {
        self.peer_manager.wireguard_gateway_data.keypair()
    }

    fn wireguard_config(&self) -> WireguardConfig {
        self.peer_manager.wireguard_gateway_data.config()
    }

    fn wg_port(&self) -> u16 {
        self.wireguard_config().announced_tunnel_port
    }

    pub async fn credential_storage_preparation(
        &self,
        client_id: i64,
    ) -> Result<PersistedBandwidth, GatewayWireguardError> {
        self.ecash_verifier
            .storage()
            .create_bandwidth_entry(client_id)
            .await?;

        self.ecash_verifier
            .storage()
            .get_available_bandwidth(client_id)
            .await?
            .ok_or(GatewayWireguardError::internal(
                "missing bandwidth entry after it has just been created",
            ))
    }

    async fn credential_verification(
        &self,
        credential: CredentialSpendingData,
        client_id: i64,
    ) -> Result<i64, GatewayWireguardError> {
        let bandwidth = self.credential_storage_preparation(client_id).await?;
        let client_bandwidth = ClientBandwidth::new(bandwidth.into());
        let mut verifier = CredentialVerifier::new(
            CredentialSpendingRequest::new(credential),
            self.ecash_verifier.clone(),
            BandwidthStorageManager::new(
                self.ecash_verifier.storage(),
                client_bandwidth,
                client_id,
                BandwidthFlushingBehaviourConfig::default(),
                true,
            ),
        );

        Ok(verifier.verify().await?)
    }

    async fn handle_final_credential_claim(
        &self,
        claim: BandwidthClaim,
        client_id: i64,
    ) -> Result<(), GatewayWireguardError> {
        match claim.credential {
            BandwidthCredential::ZkNym(zk_nym) => {
                // if we got zk-nym, we just try to verify it
                self.credential_verification(*zk_nym, client_id).await?;
                Ok(())
            }
            BandwidthCredential::UpgradeModeJWT { token } => {
                // if we're already in the upgrade mode, don't bother validating the token
                if self.upgrade_mode_enabled() {
                    return Ok(());
                }

                self.upgrade_mode.try_enable_via_received_jwt(token).await?;
                Ok(())
            }
        }
    }

    /// Attempt to process new peer by:
    /// 1. retrieving previous IP allocation
    /// 2. inserting it into the storage
    /// 3. verifying bandwidth claim and increasing the allowance
    /// 4. spawning the peer handler
    async fn process_new_peer(
        &self,
        pending: PendingRegistration,
        credential: BandwidthClaim,
    ) -> Result<(), GatewayWireguardError> {
        // 1. create peer based on the cached registration information
        let defguard_key = Key::new(pending.data.peer_key.to_bytes());
        let mut peer = Peer::new(defguard_key);
        if let Some(psk) = pending.data.psk {
            peer.preshared_key = Some(psk);
        }
        let private_ipv4 = pending.data.wireguard_config.private_ipv4;
        let private_ipv6 = pending.data.wireguard_config.private_ipv6;
        peer.allowed_ips = vec![
            IpAddrMask::new(private_ipv4.into(), 32),
            IpAddrMask::new(private_ipv6.into(), 128),
        ];

        let typ = credential.kind;

        // 2. attempt to pre-insert peer into the storage
        let client_id = self
            .ecash_verifier
            .storage()
            .insert_wireguard_peer(&peer, typ.into())
            .await?;

        // 3. verify the credential
        if let Err(err) = self
            .handle_final_credential_claim(credential, client_id)
            .await
        {
            // 3.1. on failure -> remove the inserted peer
            self.ecash_verifier
                .storage()
                .remove_wireguard_peer(&peer.public_key.to_string())
                .await?;
            return Err(err);
        }

        // 4. attempt to start the actual handle for the peer
        let public_key = peer.public_key.to_string();
        if let Err(err) = self.peer_manager.add_peer(peer).await {
            // 4.1. on failure -> remove the inserted peer (from the storage)
            self.ecash_verifier
                .storage()
                .remove_wireguard_peer(&public_key)
                .await?;
            return Err(err);
        }

        Ok(())
    }

    pub async fn on_initial_authenticator_request(
        &mut self,
        init_message: Box<dyn InitMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> Result<SerialisedResponse, GatewayWireguardError> {
        let remote_public = init_message.pub_key();

        // 1. check if there's any pending registration already in progress,
        // if so, return the same data again without additional processing
        if let Some(pending_registration) = self
            .check_pending_authenticator_registration(protocol, request_id, remote_public, reply_to)
            .await?
        {
            return Ok(pending_registration);
        }

        // 2. check if there is already a peer associated with this sender,
        // if so, retrieve the "final" data without additional processing
        if let Some(existing_registration) = self
            .check_existing_authenticator_peer(protocol, request_id, remote_public, reply_to)
            .await?
        {
            return Ok(existing_registration);
        }

        // 3. process fresh registration request
        self.process_fresh_initial_authenticator_registration(
            protocol,
            request_id,
            remote_public,
            reply_to,
        )
        .await
    }

    pub async fn on_final_authenticator_request(
        &mut self,
        final_message: Box<dyn FinalMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> Result<SerialisedResponse, GatewayWireguardError> {
        let peer = final_message.gateway_client_pub_key();
        // 1. check if there's any pending registration associated with this peer
        let pending_data = self
            .pending_registrations
            .check_authenticator(&peer)
            .await
            .ok_or(GatewayWireguardError::RegistrationNotInProgress)?
            .clone();

        // 2. verify the correctness of the received request based on the prior nonce
        if final_message
            .verify(self.keypair().private_key(), pending_data.data.nonce)
            .is_err()
        {
            return Err(GatewayWireguardError::AuthenticatorMacVerificationFailure);
        }

        // 3. ensure we have received a credential
        let Some(credential) = final_message.credential() else {
            return Err(GatewayWireguardError::MissingAuthenticatorCredential);
        };

        // 4. prepare new peer information and verify the credential
        self.process_new_peer(pending_data.clone(), credential)
            .await?;

        // 5. remove pending registration
        self.pending_registrations.remove_authenticator(&peer).await;

        // 6. construct and return the response
        pending_data.to_registered_authenticator_response(
            self.upgrade_mode_enabled(),
            request_id,
            protocol.into(),
            reply_to,
        )
    }

    pub async fn on_initial_lp_request(
        &self,
        init_msg: LpDvpnRegistrationInitialRequest,
        receiver_index: u64,
    ) -> Result<LpRegistrationResponse, GatewayWireguardError> {
        let remote_public = init_msg.wg_public_key;
        let psk = Key::new(init_msg.psk);

        // 1. check if there's any pending registration already in progress,
        // if so, return the same data again without additional processing,
        // but update stored PSK
        if let Some(pending_registration) =
            self.check_pending_lp_registration(receiver_index).await?
        {
            self.update_peer_psk(remote_public, psk).await?;
            return Ok(pending_registration);
        }

        // 2. check if there is already a peer associated with this sender,
        // if so, retrieve the "final" data without additional processing,
        // but do update stored PSK
        if let Some(existing_registration) = self.check_existing_lp_peer(remote_public).await? {
            self.update_peer_psk(remote_public, psk).await?;
            return Ok(existing_registration);
        }

        // 3. process fresh registration request
        self.process_fresh_initial_lp_registration(receiver_index, remote_public, psk)
            .await
    }

    pub async fn on_final_lp_request(
        &self,
        final_msg: LpDvpnRegistrationFinalisation,
        receiver_index: u64,
    ) -> Result<LpRegistrationResponse, GatewayWireguardError> {
        // 1. check if there's any pending registration associated with this peer
        let pending_data = self
            .pending_registrations
            .check_lp(receiver_index)
            .await
            .ok_or(GatewayWireguardError::RegistrationNotInProgress)?
            .clone();

        let credential = final_msg.credential;

        // 2. prepare new peer information and verify the credential
        self.process_new_peer(pending_data.clone(), credential)
            .await?;

        // 3 remove pending registration
        self.pending_registrations.remove_lp(receiver_index).await;

        // 4. construct and return the response
        Ok(pending_data.to_registered_lp_response(self.upgrade_mode_enabled()))
    }
}

pub struct StaleRegistrationRemover {
    pending_registrations: PendingRegistrations,
    shutdown_token: ShutdownToken,
}

impl StaleRegistrationRemover {
    // TODO: make it configurable
    const STALE_REG_CHECK_INTERVAL: Duration = Duration::from_secs(60);

    pub async fn run(&self) {
        let start = Instant::now() + Self::STALE_REG_CHECK_INTERVAL;
        let mut interval = interval_at(start, Self::STALE_REG_CHECK_INTERVAL);
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("StaleRegistrationRemover: received shutdown");
                    break
                }
                _ = interval.tick() => {
                    self.pending_registrations.remove_stale_registrations().await
                }
            }
        }
    }
}
