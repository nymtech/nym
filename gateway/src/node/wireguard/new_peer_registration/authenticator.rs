// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::wireguard::new_peer_registration::helpers::build_final_authenticator_response;
use crate::node::wireguard::new_peer_registration::pending::{
    PendingRegistration, PendingRegistrationData,
};
use crate::node::wireguard::{GatewayWireguardError, PeerRegistrator};
use defguard_wireguard_rs::host::Peer;
use nym_authenticator_requests::authenticator_ipv4_to_ipv6;
use nym_authenticator_requests::response::SerialisedResponse;
use nym_registration_common::WireguardRegistrationData;
use nym_sdk::mixnet::Recipient;
use nym_service_provider_requests_common::Protocol;
use nym_wireguard::peer_controller::IpPair;
use nym_wireguard_types::PeerPublicKey;
use std::net::IpAddr;
use std::time::Instant;

impl PeerRegistrator {
    fn authenticator_peer_to_final_response(
        &self,
        peer: Peer,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> Result<SerialisedResponse, GatewayWireguardError> {
        let allowed_ipv4 = peer
            .allowed_ips
            .iter()
            .find_map(|ip_mask| match ip_mask.address {
                IpAddr::V4(ipv4_addr) => Some(ipv4_addr),
                _ => None,
            })
            .ok_or(GatewayWireguardError::internal(
                "there should be one private IPv4 in the list",
            ))?;
        let allowed_ipv6 = peer
            .allowed_ips
            .iter()
            .find_map(|ip_mask| match ip_mask.address {
                IpAddr::V6(ipv6_addr) => Some(ipv6_addr),
                _ => None,
            })
            .unwrap_or(authenticator_ipv4_to_ipv6(allowed_ipv4));

        let ip_allocation = IpPair::new(allowed_ipv4, allowed_ipv6);
        let wg_port = self.wg_port();
        let local_pub_key = (*self.keypair().public_key()).into();
        let upgrade_mode_enabled = self.upgrade_mode_enabled();

        build_final_authenticator_response(
            ip_allocation,
            wg_port,
            local_pub_key,
            upgrade_mode_enabled,
            request_id,
            protocol.into(),
            reply_to,
        )
    }

    pub(super) async fn check_pending_authenticator_registration(
        &self,
        protocol: Protocol,
        request_id: u64,
        remote_public: PeerPublicKey,
        reply_to: Option<Recipient>,
    ) -> Result<Option<SerialisedResponse>, GatewayWireguardError> {
        let Some(pending_registration) = self
            .pending_registrations
            .check_authenticator(&remote_public)
            .await
        else {
            return Ok(None);
        };

        Ok(Some(
            pending_registration.to_pending_authenticator_response(
                self.keypair().private_key(),
                self.upgrade_mode_enabled(),
                request_id,
                protocol.into(),
                reply_to,
            )?,
        ))
    }

    pub(super) async fn check_existing_authenticator_peer(
        &self,
        protocol: Protocol,
        request_id: u64,
        remote_public: PeerPublicKey,
        reply_to: Option<Recipient>,
    ) -> Result<Option<SerialisedResponse>, GatewayWireguardError> {
        let Some(peer) = self.peer_manager.query_peer(remote_public).await? else {
            return Ok(None);
        };
        Ok(Some(self.authenticator_peer_to_final_response(
            peer, protocol, request_id, reply_to,
        )?))
    }

    pub(super) fn new_pending_authenticator(
        &self,
        peer: PeerPublicKey,
        ip_allocation: IpPair,
    ) -> PendingRegistration {
        let nonce: u64 = fastrand::u64(..);

        PendingRegistration {
            requested_on: Instant::now(),
            data: PendingRegistrationData {
                nonce,
                peer_key: peer,
                psk: None,
                wireguard_config: WireguardRegistrationData {
                    public_key: *self.keypair().public_key(),
                    port: self.wg_port(),
                    private_ipv4: ip_allocation.ipv4,
                    private_ipv6: ip_allocation.ipv6,
                },
            },
        }
    }

    pub(super) async fn process_fresh_initial_authenticator_registration(
        &self,
        protocol: Protocol,
        request_id: u64,
        remote_public: PeerPublicKey,
        reply_to: Option<Recipient>,
    ) -> Result<SerialisedResponse, GatewayWireguardError> {
        // 1. allocate ip pair
        let ip_allocation = self.peer_manager.preallocate_peer_ip_pair().await?;

        let pending = self.new_pending_authenticator(remote_public, ip_allocation);

        // 2. construct response
        let response = pending.to_pending_authenticator_response(
            self.keypair().private_key(),
            self.upgrade_mode_enabled(),
            request_id,
            protocol.into(),
            reply_to,
        )?;

        // 3. insert pending data into cache
        self.pending_registrations
            .authenticator
            .write()
            .await
            .insert(remote_public, pending);

        Ok(response)
    }
}
