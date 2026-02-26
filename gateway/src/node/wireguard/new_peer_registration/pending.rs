// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::wireguard::new_peer_registration::helpers::{
    build_final_authenticator_response, build_pending_authenticator_response,
};
use crate::node::wireguard::GatewayWireguardError;
use defguard_wireguard_rs::key::Key;
use nym_authenticator_requests::AuthenticatorVersion;
use nym_crypto::asymmetric::x25519;
use nym_registration_common::{LpRegistrationResponse, WireguardRegistrationData};
use nym_sdk::mixnet::Recipient;
use nym_wireguard::ip_pool::IpPair;
use nym_wireguard_types::PeerPublicKey;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const DEFAULT_PENDING_REGISTRATION_TTL: Duration = Duration::from_secs(120); // 2 minutes

#[derive(Clone)]
pub struct PendingRegistration {
    pub(super) requested_on: Instant,
    pub(super) data: PendingRegistrationData,
}

#[derive(Clone)]
pub struct PendingRegistrationData {
    pub(super) nonce: u64,

    pub(super) peer_key: PeerPublicKey,

    // will not be set if registering via the Authenticator
    pub(super) psk: Option<Key>,

    pub(super) wireguard_config: WireguardRegistrationData,
}

impl PendingRegistration {
    pub(crate) fn to_pending_authenticator_response(
        &self,
        local_key: &x25519::PrivateKey,
        upgrade_mode_enabled: bool,
        request_id: u64,
        version: AuthenticatorVersion,
        reply_to: Option<Recipient>,
    ) -> Result<nym_authenticator_requests::response::SerialisedResponse, GatewayWireguardError>
    {
        let nonce = self.data.nonce;
        let remote_public = self.data.peer_key;
        let wg_port = self.data.wireguard_config.port;
        let ip_allocation = IpPair::new(
            self.data.wireguard_config.private_ipv4,
            self.data.wireguard_config.private_ipv6,
        );

        build_pending_authenticator_response(
            ip_allocation,
            wg_port,
            local_key,
            remote_public,
            upgrade_mode_enabled,
            nonce,
            request_id,
            version,
            reply_to,
        )
    }

    pub(crate) fn to_registered_authenticator_response(
        &self,
        upgrade_mode_enabled: bool,
        request_id: u64,
        version: AuthenticatorVersion,
        reply_to: Option<Recipient>,
    ) -> Result<nym_authenticator_requests::response::SerialisedResponse, GatewayWireguardError>
    {
        let wg_port = self.data.wireguard_config.port;
        let local_pub_key = self.data.wireguard_config.public_key.into();

        let ip_allocation = IpPair::new(
            self.data.wireguard_config.private_ipv4,
            self.data.wireguard_config.private_ipv6,
        );

        build_final_authenticator_response(
            ip_allocation,
            wg_port,
            local_pub_key,
            upgrade_mode_enabled,
            request_id,
            version,
            reply_to,
        )
    }

    pub(crate) fn to_pending_lp_response(&self) -> LpRegistrationResponse {
        LpRegistrationResponse::request_dvpn_credential()
    }

    pub(crate) fn to_registered_lp_response(
        &self,
        upgrade_mode_enabled: bool,
    ) -> LpRegistrationResponse {
        LpRegistrationResponse::success_dvpn(self.data.wireguard_config, upgrade_mode_enabled)
    }
}

#[derive(Clone, Default)]
pub(crate) struct PendingRegistrations {
    // TODO: unify those, somehow, later
    /// Registrations in progress received from the Authenticator service provider via the
    /// [`crate::node::internal_service_providers::authenticator::mixnet_listener::MixnetListener`]
    pub(crate) authenticator: Arc<RwLock<HashMap<PeerPublicKey, PendingRegistration>>>,

    /// Registrations in progress received from the LP Listener via the
    /// `LpConnectionHandler` and handle through `LpHandlerState`
    pub(crate) lp: Arc<RwLock<HashMap<u64, PendingRegistration>>>,
}

impl PendingRegistrations {
    pub(crate) async fn check_authenticator(
        &self,
        peer: &PeerPublicKey,
    ) -> Option<PendingRegistration> {
        self.authenticator.read().await.get(peer).cloned()
    }

    pub(crate) async fn remove_authenticator(&self, peer: &PeerPublicKey) {
        self.authenticator.write().await.remove(peer);
    }

    pub(crate) async fn remove_lp(&self, receiver_index: u64) {
        self.lp.write().await.remove(&receiver_index);
    }

    pub(crate) async fn check_lp(&self, receiver_index: u64) -> Option<PendingRegistration> {
        self.lp.read().await.get(&receiver_index).cloned()
    }

    pub(crate) async fn remove_stale_registrations(&self) {
        // note: `IpPool` will release stale pre-allocated addresses by itself during the cleanup,
        // so there's no need to send explicit messages over
        let now = Instant::now();
        self.authenticator.write().await.retain(|_, pending| {
            now.duration_since(pending.requested_on) < DEFAULT_PENDING_REGISTRATION_TTL
        });
        self.lp.write().await.retain(|_, pending| {
            now.duration_since(pending.requested_on) < DEFAULT_PENDING_REGISTRATION_TTL
        });
    }
}
