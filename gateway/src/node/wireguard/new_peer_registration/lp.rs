// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::lp_listener::ReceiverIndex;
use crate::node::wireguard::new_peer_registration::pending::{
    PendingRegistration, PendingRegistrationData,
};
use crate::node::wireguard::{GatewayWireguardError, PeerRegistrator};
use defguard_wireguard_rs::host::Peer;
use defguard_wireguard_rs::key::Key;
use nym_registration_common::{LpRegistrationResponse, WireguardRegistrationData};
use nym_wireguard::ip_pool::{allocated_ip_pair, IpPair};
use nym_wireguard_types::PeerPublicKey;
use std::time::Instant;

impl PeerRegistrator {
    /// In the case of an already registered WG peer, update its PSK.
    pub(super) async fn update_peer_psk(
        &self,
        peer: PeerPublicKey,
        psk: Key,
    ) -> Result<(), GatewayWireguardError> {
        // 1. check if the peer is currently being handled
        if self.peer_manager.check_active_peer(peer).await? {
            // 2. if so, force disconnect it (as we're handling new request from the same peer)
            self.peer_manager.remove_peer(peer).await?;
        }

        // 3. update the on-disk PSK
        let encoded_psk = psk.to_lower_hex();
        self.ecash_verifier
            .storage()
            .update_peer_psk(&peer.to_string(), Some(&encoded_psk))
            .await?;

        Ok(())
    }

    fn lp_peer_to_final_response(
        &self,
        peer: Peer,
    ) -> Result<Option<LpRegistrationResponse>, GatewayWireguardError> {
        // Incomplete data, treat as new registration
        let Some(allocated_ips) = allocated_ip_pair(&peer) else {
            return Ok(None);
        };

        Ok(Some(LpRegistrationResponse::success_dvpn(
            WireguardRegistrationData {
                public_key: *self.keypair().public_key(),
                port: self.wg_port(),
                private_ipv4: allocated_ips.ipv4,
                private_ipv6: allocated_ips.ipv6,
            },
            self.upgrade_mode_enabled(),
        )))
    }

    pub(super) async fn check_pending_lp_registration(
        &self,
        sender: ReceiverIndex,
    ) -> Result<Option<LpRegistrationResponse>, GatewayWireguardError> {
        let Some(pending_registration) = self.pending_registrations.check_lp(sender).await else {
            return Ok(None);
        };

        Ok(Some(pending_registration.to_pending_lp_response()))
    }

    pub(super) async fn check_existing_lp_peer(
        &self,
        remote_public: PeerPublicKey,
    ) -> Result<Option<LpRegistrationResponse>, GatewayWireguardError> {
        let Some(peer) = self.peer_manager.query_peer(remote_public).await? else {
            return Ok(None);
        };

        self.lp_peer_to_final_response(peer)
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
        sender: ReceiverIndex,
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
            .insert(sender, pending);

        Ok(response)
    }
}
