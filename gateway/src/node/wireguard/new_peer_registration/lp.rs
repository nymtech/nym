// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::wireguard::new_peer_registration::pending::{
    PendingRegistration, PendingRegistrationData,
};
use crate::node::wireguard::{GatewayWireguardError, PeerRegistrator};
use defguard_wireguard_rs::key::Key;
use nym_gateway_storage::models::FreeTierRecord;
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_network_defaults::constants::{FREE_TIER_CLAIM_WINDOW, FREE_TIER_TRIAL_TIME_CAP};
use nym_registration_common::{LpRegistrationResponse, WireguardRegistrationData};
use nym_wireguard::ip_pool::{allocated_ip_pair, IpPair};
use nym_wireguard_types::PeerPublicKey;
use std::time::Instant;
use time::OffsetDateTime;

/// How a returning, already-registered peer should be handled at the LP initial request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturningPeerOutcome {
    /// Resume the existing session. `restricted` marks a spent-but-within-window free peer
    /// confined to the purchase garden (working tunnel, purchase-only); `false` is a normal
    /// unrestricted resume (active trial, or a non-free / upgraded peer).
    Resume { restricted: bool },

    /// The claim window has elapsed: the peer is eligible for a fresh trial, so ask for a
    /// credential (a fresh allowance is never auto-granted without a token).
    RequiresReclaim,
}

/// Classify a returning existing peer from its free-tier record + remaining bytes. Pure;
/// windows are passed in whole seconds. A peer with no record, or one that has upgraded to
/// paid (`is_free = false`), resumes unrestricted exactly as before.
fn classify_returning_free_peer(
    record: Option<&FreeTierRecord>,
    available_bandwidth: i64,
    now: OffsetDateTime,
    time_cap_secs: i64,
    claim_window_secs: i64,
) -> ReturningPeerOutcome {
    let Some(record) = record else {
        return ReturningPeerOutcome::Resume { restricted: false };
    };
    if !record.is_free {
        return ReturningPeerOutcome::Resume { restricted: false };
    }
    let elapsed_secs = (now - record.granted_at).whole_seconds();
    if elapsed_secs >= claim_window_secs {
        return ReturningPeerOutcome::RequiresReclaim;
    }
    // within the claim window: spent by either limit -> confined to the garden.
    let spent = elapsed_secs >= time_cap_secs || available_bandwidth <= 0;
    ReturningPeerOutcome::Resume { restricted: spent }
}

/// Actionable outcome for a returning existing peer, returned by `check_existing_lp_peer`.
/// The caller applies the PSK-consuming side effects, so `check_existing_lp_peer` needs no
/// `psk` (nothing to clone): `Resume` -> refresh PSK + return the config; `Reclaim` -> build
/// a pending (which carries the PSK) and ask for a credential.
pub(super) enum ExistingPeerOutcome {
    /// Resume the existing session. `restricted` marks a spent-but-within-window free peer
    /// confined to the purchase garden (working tunnel, purchase-only).
    Resume {
        config: WireguardRegistrationData,
        restricted: bool,
    },
    /// Claim window elapsed: re-claim needed. Carries the existing IPs to reuse for the
    /// pending registration.
    Reclaim { allocated_ips: IpPair },
}

impl PeerRegistrator {
    /// In the case of an already registered WG peer, update its PSK.
    ///
    /// The peer controller keeps the active config and the on-disk PSK in sync.
    pub(super) async fn update_peer_psk(
        &self,
        peer: PeerPublicKey,
        psk: Key,
    ) -> Result<(), GatewayWireguardError> {
        self.peer_manager.update_peer_psk(peer, psk).await
    }

    /// Build the WireGuard registration data returned to a resuming peer.
    fn wg_registration_data(&self, allocated_ips: IpPair) -> WireguardRegistrationData {
        WireguardRegistrationData {
            public_key: *self.keypair().public_key(),
            port: self.wg_port(),
            private_ipv4: allocated_ips.ipv4,
            private_ipv6: allocated_ips.ipv6,
        }
    }

    /// Remaining free-tier bytes for an existing peer, by public key. Returns 0 when there
    /// is no bandwidth row (treated as spent). Distinguishes an active trial from a
    /// byte-exhausted one when classifying a returning peer.
    async fn available_bandwidth_for_peer(
        &self,
        public_key: &str,
    ) -> Result<i64, GatewayWireguardError> {
        let Some(peer) = self
            .ecash_verifier
            .storage()
            .get_wireguard_peer(public_key)
            .await?
        else {
            return Ok(0);
        };
        Ok(self
            .ecash_verifier
            .storage()
            .get_available_bandwidth(peer.client_id)
            .await?
            .map(|b| b.available)
            .unwrap_or(0))
    }

    pub(super) async fn check_pending_lp_registration(
        &self,
        receiver_index: LpReceiverIndex,
    ) -> Result<Option<LpRegistrationResponse>, GatewayWireguardError> {
        let Some(pending_registration) = self.pending_registrations.check_lp(receiver_index).await
        else {
            return Ok(None);
        };

        Ok(Some(pending_registration.to_pending_lp_response()))
    }

    /// Classify a returning peer at the LP initial request from its free-tier state.
    /// Returns `None` only when there is no existing (fully-allocated) peer - the caller
    /// then does a fresh registration. Otherwise the caller acts on the [`ExistingPeerOutcome`]
    /// (which is why this takes no `psk`: the PSK-consuming work stays in the caller).
    pub(super) async fn check_existing_lp_peer(
        &self,
        remote_public: PeerPublicKey,
    ) -> Result<Option<ExistingPeerOutcome>, GatewayWireguardError> {
        let Some(peer) = self.peer_manager.query_peer(remote_public).await? else {
            // no existing peer -> caller falls through to a fresh registration
            return Ok(None);
        };

        // Incomplete allocation -> treat as a fresh registration.
        let Some(allocated_ips) = allocated_ip_pair(&peer) else {
            return Ok(None);
        };

        let public_key = peer.public_key.to_string();
        let record = self
            .ecash_verifier
            .storage()
            .get_free_tier_record(&public_key)
            .await?;
        let available_bandwidth = self.available_bandwidth_for_peer(&public_key).await?;

        let outcome = classify_returning_free_peer(
            record.as_ref(),
            available_bandwidth,
            OffsetDateTime::now_utc(),
            FREE_TIER_TRIAL_TIME_CAP.as_secs() as i64,
            FREE_TIER_CLAIM_WINDOW.as_secs() as i64,
        );

        Ok(Some(match outcome {
            // Active trial, or a non-free / upgraded peer -> resume unrestricted; spent within
            // the claim window -> resume but marked purchase-only.
            ReturningPeerOutcome::Resume { restricted } => ExistingPeerOutcome::Resume {
                config: self.wg_registration_data(allocated_ips),
                restricted,
            },
            // Claim window elapsed -> re-claim (the caller reuses these IPs for the pending).
            ReturningPeerOutcome::RequiresReclaim => ExistingPeerOutcome::Reclaim { allocated_ips },
        }))
    }

    /// Build + register a re-claim pending for a returning window-elapsed peer, reusing its
    /// existing IPs, and return the credential prompt. The finalisation upserts (same
    /// `client_id`), `FreshReclaim` re-seeds the allowance, and `admit` moves the peer
    /// garden->pool - no removal, IPs stable. The pending carries the PSK (applied at
    /// finalisation), so the caller need not update it separately.
    pub(super) async fn start_lp_reclaim(
        &self,
        remote_public: PeerPublicKey,
        psk: Key,
        allocated_ips: IpPair,
        receiver_index: LpReceiverIndex,
    ) -> LpRegistrationResponse {
        let pending = self.new_pending_lp(remote_public, psk, allocated_ips);
        self.pending_registrations
            .lp
            .write()
            .await
            .insert(receiver_index, pending);
        LpRegistrationResponse::request_dvpn_credential()
    }

    pub(super) fn new_pending_lp(
        &self,
        peer: PeerPublicKey,
        psk: Key,
        ip_allocation: IpPair,
    ) -> PendingRegistration {
        let nonce: u64 = fastrand::u64(..);

        PendingRegistration {
            requested_on: Instant::now(),
            data: PendingRegistrationData {
                nonce,
                peer_key: peer,
                psk: Some(psk),
                wireguard_config: WireguardRegistrationData {
                    public_key: *self.keypair().public_key(),
                    port: self.wg_port(),
                    private_ipv4: ip_allocation.ipv4,
                    private_ipv6: ip_allocation.ipv6,
                },
            },
        }
    }

    pub(super) async fn process_fresh_initial_lp_registration(
        &self,
        receiver_index: LpReceiverIndex,
        remote_public: PeerPublicKey,
        psk: Key,
    ) -> Result<LpRegistrationResponse, GatewayWireguardError> {
        // 1. allocate ip pair
        let ip_allocation = self.peer_manager.preallocate_peer_ip_pair().await?;

        let pending = self.new_pending_lp(remote_public, psk, ip_allocation);

        // 2. construct response
        let response = pending.to_pending_lp_response();

        // 3. insert pending data into cache
        self.pending_registrations
            .lp
            .write()
            .await
            .insert(receiver_index, pending);

        Ok(response)
    }
}

#[cfg(test)]
mod returning_peer_tests {
    use super::*;
    use time::Duration as TimeDuration;

    const TIME_CAP: i64 = 600; // 10 minutes
    const CLAIM_WINDOW: i64 = 86_400; // 24 hours
    const HAS_BYTES: i64 = 100_000_000;

    fn base_now() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + TimeDuration::days(1)
    }

    fn free_record_granted_secs_ago(now: OffsetDateTime, ago_secs: i64) -> FreeTierRecord {
        FreeTierRecord {
            public_key: "pk".to_string(),
            granted_at: now - TimeDuration::seconds(ago_secs),
            is_free: true,
        }
    }

    #[test]
    fn no_record_resumes_unrestricted() {
        assert_eq!(
            classify_returning_free_peer(None, HAS_BYTES, base_now(), TIME_CAP, CLAIM_WINDOW),
            ReturningPeerOutcome::Resume { restricted: false }
        );
    }

    #[test]
    fn upgraded_paid_peer_resumes_unrestricted() {
        // is_free == false wins even if the timestamp/bytes look spent.
        let mut rec = free_record_granted_secs_ago(base_now(), TIME_CAP + 1);
        rec.is_free = false;
        assert_eq!(
            classify_returning_free_peer(Some(&rec), 0, base_now(), TIME_CAP, CLAIM_WINDOW),
            ReturningPeerOutcome::Resume { restricted: false }
        );
    }

    #[test]
    fn active_trial_with_bytes_resumes_unrestricted() {
        let rec = free_record_granted_secs_ago(base_now(), TIME_CAP - 1);
        assert_eq!(
            classify_returning_free_peer(Some(&rec), HAS_BYTES, base_now(), TIME_CAP, CLAIM_WINDOW),
            ReturningPeerOutcome::Resume { restricted: false }
        );
    }

    #[test]
    fn byte_exhausted_within_time_cap_is_restricted() {
        // well within the time cap, but out of bytes -> confined to the garden.
        let rec = free_record_granted_secs_ago(base_now(), 10);
        assert_eq!(
            classify_returning_free_peer(Some(&rec), 0, base_now(), TIME_CAP, CLAIM_WINDOW),
            ReturningPeerOutcome::Resume { restricted: true }
        );
    }

    #[test]
    fn time_spent_within_window_is_restricted() {
        // past the time cap but within the claim window, bytes remaining -> still restricted.
        let rec = free_record_granted_secs_ago(base_now(), TIME_CAP + 1);
        assert_eq!(
            classify_returning_free_peer(Some(&rec), HAS_BYTES, base_now(), TIME_CAP, CLAIM_WINDOW),
            ReturningPeerOutcome::Resume { restricted: true }
        );
    }

    #[test]
    fn window_elapsed_requires_reclaim() {
        // exactly at the window boundary is already eligible to re-claim.
        let rec = free_record_granted_secs_ago(base_now(), CLAIM_WINDOW);
        assert_eq!(
            classify_returning_free_peer(Some(&rec), HAS_BYTES, base_now(), TIME_CAP, CLAIM_WINDOW),
            ReturningPeerOutcome::RequiresReclaim
        );
    }
}
