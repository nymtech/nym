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
use nym_credentials::ecash::utils::ecash_date_offset;
use nym_credentials_interface::{Bandwidth, BandwidthCredential, CredentialSpendingData};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_free_tier_check::{validate_free_tier_jwt, FreeTierPurpose, CREDENTIAL_PROXY_JWT_ISSUER};
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::models::{FreeTierRecord, PersistedBandwidth};
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_network_defaults::constants::{FREE_TIER_CLAIM_WINDOW, FREE_TIER_TRIAL_TIME_CAP};
use nym_node_metrics::prometheus_wrapper::{PrometheusMetric, PROMETHEUS_METRICS};
use nym_registration_common::dvpn::{
    LpDvpnRegistrationFinalisation, LpDvpnRegistrationInitialRequest,
};
use nym_registration_common::LpRegistrationResponse;
use nym_sdk::mixnet::Recipient;
use nym_service_provider_requests_common::Protocol;
use nym_task::ShutdownToken;
use nym_wireguard::WireguardConfig;
use nym_wireguard_types::PeerPublicKey;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{interval_at, Instant};
use tracing::trace;

mod authenticator;
mod helpers;
mod lp;
mod pending;

use lp::ExistingPeerOutcome;

#[derive(Clone)]
pub struct PeerRegistrator {
    /// Handle for the structure managing verification of the ecash credentials for the bandwidth control
    pub(crate) ecash_verifier: Arc<dyn EcashManager + Send + Sync>,

    /// Handle for communication with the [`nym_wireguard::peer_controller::PeerController`]
    pub(crate) peer_manager: PeerManager,

    /// Information about the current state of the upgrade mode as well as a handle
    /// to remotely trigger the recheck
    pub(crate) upgrade_mode: UpgradeModeDetails,

    pub(crate) free_tier_config: FreeTierRegistrationConfig,

    /// Registrations in progress
    pub(crate) pending_registrations: PendingRegistrations,
}

/// Free-tier settings the registrator needs, bundled so callers pass one value.
#[derive(Clone, Copy)]
pub struct FreeTierRegistrationConfig {
    /// Signer public key; `Some` iff the free tier is enabled.
    pub signer: Option<ed25519::PublicKey>,

    /// Byte allowance seeded for a new trial.
    pub allowance_bytes: u64,
}

/// Outcome of evaluating a free-tier token against the peer's existing record.
/// The single `granted_at` timestamp defines two nested windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FreeTierClaimOutcome {
    /// No prior record: grant a fresh allowance and record `granted_at = now`.
    FirstClaim,

    /// Trial still active (`elapsed < time_cap`) but the peer row is gone (a reaped
    /// peer re-presenting its token). v1 grants no fresh allowance - the row-present
    /// resume keeps its remaining bytes via the existing-peer short-circuit, and once
    /// the row is gone those bytes are unrecoverable, so re-seeding here would be a
    /// reconnect-refill vector. Task 5 routes this into the walled garden with the
    /// trial time still counting from `granted_at`.
    Resume,

    /// Spent but still within the claim window (`time_cap <= elapsed < claim_window`):
    /// the single-claim guard rejects a fresh grant (v1; task 5 routes this to the garden).
    GuardBlocked,

    /// Claim window elapsed (`elapsed >= claim_window`): grant a fresh allowance and
    /// reset `granted_at = now`.
    FreshReclaim,
}

/// Classify a free-tier claim from the peer's existing record (if any) and the
/// wall-clock elapsed since its grant. Pure; windows are passed in whole seconds.
fn classify_free_tier_claim(
    record: Option<&FreeTierRecord>,
    now: OffsetDateTime,
    time_cap_secs: i64,
    claim_window_secs: i64,
) -> FreeTierClaimOutcome {
    let Some(record) = record else {
        return FreeTierClaimOutcome::FirstClaim;
    };
    let elapsed_secs = (now - record.granted_at).whole_seconds();
    if elapsed_secs < time_cap_secs {
        FreeTierClaimOutcome::Resume
    } else if elapsed_secs < claim_window_secs {
        FreeTierClaimOutcome::GuardBlocked
    } else {
        FreeTierClaimOutcome::FreshReclaim
    }
}

impl PeerRegistrator {
    pub fn new(
        ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
        peer_manager: PeerManager,
        upgrade_mode: UpgradeModeDetails,
        free_tier_config: FreeTierRegistrationConfig,
    ) -> Self {
        PeerRegistrator {
            ecash_verifier,
            peer_manager,
            upgrade_mode,
            free_tier_config,
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

    fn free_tier_enabled(&self) -> bool {
        self.free_tier_config.signer.is_some()
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
        let _metric_timer = PROMETHEUS_METRICS
            .start_timer(PrometheusMetric::PeerRegistrationCredentialVerification);

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

    /// Seed the fixed free-tier byte allowance for a newly-registered free peer,
    /// reusing the existing bandwidth accounting (mirrors the testnet free path).
    async fn seed_free_tier_bandwidth(&self, client_id: i64) -> Result<(), GatewayWireguardError> {
        let bandwidth = self.credential_storage_preparation(client_id).await?;
        let client_bandwidth = ClientBandwidth::new(bandwidth.into());
        let mut manager = BandwidthStorageManager::new(
            self.ecash_verifier.storage(),
            client_bandwidth,
            client_id,
            BandwidthFlushingBehaviourConfig::default(),
            true,
        );

        manager
            .set_bandwidth_to(
                Bandwidth::new_unchecked(self.free_tier_config.allowance_bytes),
                // use offset of 1 to avoid immediately expiring all bandwidth if claimed at 23:59
                ecash_date_offset(1),
            )
            .await?;
        Ok(())
    }

    /// Apply the free-tier claim guard and resume logic for a verified free-tier
    /// token, keyed by the WireGuard peer public key. See [`FreeTierClaimOutcome`].
    ///
    /// Only reached when the peer is not already registered (a brand-new peer, or
    /// one whose row was removed): an existing peer short-circuits to
    /// `CompletedRegistration` before finalisation and never re-seeds.
    async fn grant_or_resume_free_tier(
        &self,
        public_key: &str,
        client_id: i64,
    ) -> Result<(), GatewayWireguardError> {
        let record = self
            .ecash_verifier
            .storage()
            .get_free_tier_record(public_key)
            .await?;
        let now = OffsetDateTime::now_utc();

        match classify_free_tier_claim(
            record.as_ref(),
            now,
            FREE_TIER_TRIAL_TIME_CAP.as_secs() as i64,
            FREE_TIER_CLAIM_WINDOW.as_secs() as i64,
        ) {
            // First-ever claim or an eligible re-claim (the claim window has elapsed):
            // grant a fresh allowance and (re)stamp granted_at.
            FreeTierClaimOutcome::FirstClaim | FreeTierClaimOutcome::FreshReclaim => {
                self.seed_free_tier_bandwidth(client_id).await?;
                self.ecash_verifier
                    .storage()
                    .set_free_tier_record(public_key, now, true)
                    .await?;
                Ok(())
            }
            // Within the claim window with the peer row gone: NO fresh allowance.
            // A genuine mid-session resume keeps its remaining bytes via the
            // existing-peer short-circuit (row present) and never reaches here; once
            // the row is gone those bytes cannot be restored across the new client_id,
            // so re-seeding would let a peer refill its allowance simply by reconnecting
            // after exhaustion. v1 rejects both cases; task 5 will instead route them
            // into the walled garden (Resume = trial time still remaining,
            // GuardBlocked = trial spent), and a proper remaining-byte resume arrives
            // once the metering path persists remaining bytes to the record.
            FreeTierClaimOutcome::Resume | FreeTierClaimOutcome::GuardBlocked => {
                Err(GatewayWireguardError::FreeTierClaimGuardActive)
            }
        }
    }

    /// If this peer still carries an active free-tier record, treat a paid credential as a
    /// reconnect-to-upgrade (task 5.6): flip the record to paid and release the peer from ALL
    /// free-tier enforcement (rate-limit pool + walled garden), restoring full unrestricted
    /// access. Releasing is best-effort - a failure is logged, not fatal: `is_free` is now
    /// false, so a restart's reconcile would drop it from enforcement anyway.
    async fn upgrade_free_tier_peer_if_needed(
        &self,
        public_key: &str,
        peer_public_key: PeerPublicKey,
    ) -> Result<(), GatewayWireguardError> {
        let was_free = self
            .ecash_verifier
            .storage()
            .get_free_tier_record(public_key)
            .await?
            .map(|r| r.is_free)
            .unwrap_or(false);
        if !was_free {
            return Ok(());
        }

        self.ecash_verifier
            .storage()
            .set_free_tier_is_free(public_key, false)
            .await?;

        // Release keyed by the peer's public key; the peer controller resolves its tunnel
        // IPs. Best-effort - a failure is logged, not fatal (`is_free` is now false, so a
        // restart's reconcile drops it from enforcement regardless).
        if let Err(e) = self.peer_manager.release_free_tier(peer_public_key).await {
            tracing::warn!(
                "failed to release upgraded peer {public_key} from free-tier enforcement: {e}"
            );
        }
        Ok(())
    }

    /// Handle a renewal free-tier token (task 5.7): grant NO bandwidth. Ensure the bandwidth
    /// row exists (at zero, so the peer handle can be built) and record the peer as a
    /// free-tier peer with no allowance, so it classifies into - and is reconciled into - the
    /// walled garden. The peer controller confines it when the peer is added; it is never
    /// pooled or per-IP limited.
    async fn confine_renewal_peer(
        &self,
        public_key: &str,
        client_id: i64,
    ) -> Result<(), GatewayWireguardError> {
        self.credential_storage_preparation(client_id).await?;
        self.ecash_verifier
            .storage()
            .set_free_tier_record(public_key, OffsetDateTime::now_utc(), true)
            .await?;
        Ok(())
    }

    async fn handle_final_credential_claim(
        &self,
        claim: BandwidthClaim,
        client_id: i64,
        public_key: &str,
        peer_public_key: PeerPublicKey,
    ) -> Result<(), GatewayWireguardError> {
        match claim.credential {
            BandwidthCredential::ZkNym(zk_nym) => {
                // if we got zk-nym, we just try to verify it
                self.credential_verification(*zk_nym, client_id).await?;
                // reconnect-to-upgrade: a formerly-free peer presenting a paid credential
                // clears its free-tier flag + enforcement (task 5.6).
                self.upgrade_free_tier_peer_if_needed(public_key, peer_public_key)
                    .await?;
                Ok(())
            }
            BandwidthCredential::UpgradeModeJWT { token } => {
                // if we're already in the upgrade mode, don't bother validating the token
                // (we have already received valid information about the upgrade mode,
                // so even if we received total rubbish now, it wouldn't influence the current state)
                if self.upgrade_mode_enabled() {
                    return Ok(());
                }

                self.upgrade_mode.try_enable_via_received_jwt(token).await?;
                Ok(())
            }
            BandwidthCredential::FreeTier { token } => {
                let Some(signer) = self.free_tier_config.signer else {
                    return Err(GatewayWireguardError::FreeTierDisabled);
                };

                // verify the capability token offline against the configured
                // free-tier signer key (the credential proxy)
                let claims =
                    validate_free_tier_jwt(&token, &signer, Some(CREDENTIAL_PROXY_JWT_ISSUER))?;

                // renewal tokens grant NO free bandwidth (task 5.7): the peer is recorded as
                // a free-tier peer with no allowance and confined straight to the purchase
                // walled garden (by the peer controller when it is added). new-user tokens go
                // through the grant/claim path.
                if claims.purpose == FreeTierPurpose::Renewal {
                    return self.confine_renewal_peer(public_key, client_id).await;
                }

                self.grant_or_resume_free_tier(public_key, client_id).await
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
        let public_key = peer.public_key.to_string();

        // 2. attempt to pre-insert peer into the storage
        let client_id = self
            .ecash_verifier
            .storage()
            .insert_wireguard_peer(&peer, typ.into())
            .await?;

        // 3. verify the credential
        if let Err(err) = self
            .handle_final_credential_claim(
                credential,
                client_id,
                &public_key,
                pending.data.peer_key,
            )
            .await
        {
            // 3.1. on failure -> remove the inserted peer
            self.ecash_verifier
                .storage()
                .remove_wireguard_peer(&public_key)
                .await?;
            return Err(err);
        }

        // 4. attempt to start the actual handle for the peer
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
        let _metric_timer = PROMETHEUS_METRICS
            .start_timer(PrometheusMetric::DvpnAuthenticatorClientRegistrationMsg1);

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
        let _metric_timer = PROMETHEUS_METRICS
            .start_timer(PrometheusMetric::DvpnAuthenticatorClientRegistrationMsg2);

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
        receiver_index: LpReceiverIndex,
    ) -> Result<LpRegistrationResponse, GatewayWireguardError> {
        let _metric_timer =
            PROMETHEUS_METRICS.start_timer(PrometheusMetric::DvpnLpClientRegistrationMsg1);

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

        // 2. check if there is already a peer associated with this sender; if so, branch on
        // its free-tier state. `psk` is consumed exactly once per path (no clone): resume
        // refreshes the stored PSK, re-claim hands it to the pending (applied at finalisation).
        if let Some(outcome) = self.check_existing_lp_peer(remote_public).await? {
            return match outcome {
                ExistingPeerOutcome::Resume { config, restricted } => {
                    self.update_peer_psk(remote_public, psk).await?;
                    Ok(if restricted {
                        LpRegistrationResponse::restricted_dvpn(config)
                    } else {
                        LpRegistrationResponse::success_dvpn(config, self.upgrade_mode_enabled())
                    })
                }
                ExistingPeerOutcome::Reclaim { allocated_ips } => Ok(self
                    .start_lp_reclaim(remote_public, psk, allocated_ips, receiver_index)
                    .await),
            };
        }

        // 3. process fresh registration request
        self.process_fresh_initial_lp_registration(receiver_index, remote_public, psk)
            .await
    }

    pub async fn on_final_lp_request(
        &self,
        final_msg: LpDvpnRegistrationFinalisation,
        receiver_index: LpReceiverIndex,
    ) -> Result<LpRegistrationResponse, GatewayWireguardError> {
        let _metric_timer =
            PROMETHEUS_METRICS.start_timer(PrometheusMetric::DvpnLpClientRegistrationMsg2);

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

#[cfg(test)]
mod free_tier_claim_tests {
    use super::*;
    use time::Duration as TimeDuration;

    const TIME_CAP: i64 = 600; // 10 minutes
    const CLAIM_WINDOW: i64 = 86_400; // 24 hours

    fn record_granted_secs_ago(now: OffsetDateTime, ago_secs: i64) -> FreeTierRecord {
        FreeTierRecord {
            public_key: "pk".to_string(),
            granted_at: now - TimeDuration::seconds(ago_secs),
            is_free: true,
        }
    }

    #[test]
    fn no_record_is_a_first_claim() {
        let now = OffsetDateTime::UNIX_EPOCH + TimeDuration::days(1);
        assert_eq!(
            classify_free_tier_claim(None, now, TIME_CAP, CLAIM_WINDOW),
            FreeTierClaimOutcome::FirstClaim
        );
    }

    #[test]
    fn within_the_time_cap_resumes() {
        let now = OffsetDateTime::UNIX_EPOCH + TimeDuration::days(1);
        let rec = record_granted_secs_ago(now, TIME_CAP - 1);
        assert_eq!(
            classify_free_tier_claim(Some(&rec), now, TIME_CAP, CLAIM_WINDOW),
            FreeTierClaimOutcome::Resume
        );
    }

    #[test]
    fn spent_within_the_claim_window_is_guard_blocked() {
        let now = OffsetDateTime::UNIX_EPOCH + TimeDuration::days(1);
        // just past the time cap
        let rec = record_granted_secs_ago(now, TIME_CAP + 1);
        assert_eq!(
            classify_free_tier_claim(Some(&rec), now, TIME_CAP, CLAIM_WINDOW),
            FreeTierClaimOutcome::GuardBlocked
        );
        // just shy of the claim window
        let rec = record_granted_secs_ago(now, CLAIM_WINDOW - 1);
        assert_eq!(
            classify_free_tier_claim(Some(&rec), now, TIME_CAP, CLAIM_WINDOW),
            FreeTierClaimOutcome::GuardBlocked
        );
    }

    #[test]
    fn after_the_claim_window_allows_a_fresh_reclaim() {
        let now = OffsetDateTime::UNIX_EPOCH + TimeDuration::days(2);
        // exactly at the window boundary is already eligible (elapsed >= claim_window)
        let rec = record_granted_secs_ago(now, CLAIM_WINDOW);
        assert_eq!(
            classify_free_tier_claim(Some(&rec), now, TIME_CAP, CLAIM_WINDOW),
            FreeTierClaimOutcome::FreshReclaim
        );
    }

    #[test]
    fn a_future_grant_from_clock_skew_resumes() {
        let now = OffsetDateTime::UNIX_EPOCH + TimeDuration::days(1);
        // granted_at in the future -> negative elapsed -> treated as active
        let rec = record_granted_secs_ago(now, -60);
        assert_eq!(
            classify_free_tier_claim(Some(&rec), now, TIME_CAP, CLAIM_WINDOW),
            FreeTierClaimOutcome::Resume
        );
    }
}
