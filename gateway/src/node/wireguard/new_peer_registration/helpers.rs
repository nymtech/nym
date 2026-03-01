// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::wireguard::GatewayWireguardError;
use nym_authenticator_requests::response::SerialisedResponse;
use nym_authenticator_requests::{v1, v2, v3, v4, v5, v6, AuthenticatorVersion};
use nym_crypto::asymmetric::x25519;
use nym_sdk::mixnet::Recipient;
use nym_wireguard::ip_pool::IpPair;
use nym_wireguard_types::PeerPublicKey;

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_pending_authenticator_response(
    ip_allocation: IpPair,
    wg_port: u16,
    local_key: &x25519::PrivateKey,
    peer_key: PeerPublicKey,
    upgrade_mode_enabled: bool,
    nonce: u64,
    request_id: u64,
    version: AuthenticatorVersion,
    reply_to: Option<Recipient>,
) -> Result<SerialisedResponse, GatewayWireguardError> {
    let private_ipv4 = ip_allocation.ipv4;
    let private_ipv6 = ip_allocation.ipv6;

    let bytes = match version {
        AuthenticatorVersion::V1 => Err(GatewayWireguardError::UnsupportedAuthenticatorVersion),
        AuthenticatorVersion::V2 => {
            v2::response::AuthenticatorResponse::new_pending_registration_success(
                v2::registration::RegistrationData {
                    nonce,
                    gateway_data: v2::registration::GatewayClient::new(
                        local_key,
                        peer_key.inner(),
                        private_ipv4.into(),
                        nonce,
                    ),
                    wg_port,
                },
                request_id,
                reply_to.ok_or(GatewayWireguardError::MissingReplyToForOldClient)?,
            )
            .to_bytes()
            .map_err(GatewayWireguardError::authenticator_response_serialisation)
        }
        AuthenticatorVersion::V3 => {
            v3::response::AuthenticatorResponse::new_pending_registration_success(
                v3::registration::RegistrationData {
                    nonce,
                    gateway_data: v3::registration::GatewayClient::new(
                        local_key,
                        peer_key.inner(),
                        private_ipv4.into(),
                        nonce,
                    ),
                    wg_port,
                },
                request_id,
                reply_to.ok_or(GatewayWireguardError::MissingReplyToForOldClient)?,
            )
            .to_bytes()
            .map_err(GatewayWireguardError::authenticator_response_serialisation)
        }
        AuthenticatorVersion::V4 => {
            v4::response::AuthenticatorResponse::new_pending_registration_success(
                v4::registration::RegistrationData {
                    nonce,
                    gateway_data: v4::registration::GatewayClient::new(
                        local_key,
                        peer_key.inner(),
                        v4::registration::IpPair::new(private_ipv4, private_ipv6),
                        nonce,
                    ),
                    wg_port,
                },
                request_id,
                reply_to.ok_or(GatewayWireguardError::MissingReplyToForOldClient)?,
            )
            .to_bytes()
            .map_err(GatewayWireguardError::authenticator_response_serialisation)
        }
        AuthenticatorVersion::V5 => {
            v5::response::AuthenticatorResponse::new_pending_registration_success(
                v5::registration::RegistrationData {
                    nonce,
                    gateway_data: v5::registration::GatewayClient::new(
                        local_key,
                        peer_key.inner(),
                        v5::registration::IpPair::new(private_ipv4, private_ipv6),
                        nonce,
                    ),
                    wg_port,
                },
                request_id,
            )
            .to_bytes()
            .map_err(GatewayWireguardError::authenticator_response_serialisation)
        }
        AuthenticatorVersion::V6 => {
            v6::response::AuthenticatorResponse::new_pending_registration_success(
                v6::registration::RegistrationData {
                    nonce,
                    gateway_data: v6::registration::GatewayClient::new(
                        local_key,
                        peer_key.inner(),
                        v6::registration::IpPair::new(private_ipv4, private_ipv6),
                        nonce,
                    ),
                    wg_port,
                },
                request_id,
                upgrade_mode_enabled,
            )
            .to_bytes()
            .map_err(GatewayWireguardError::authenticator_response_serialisation)
        }
        AuthenticatorVersion::UNKNOWN => {
            return Err(GatewayWireguardError::UnknownAuthenticatorVersion)
        }
    }?;

    Ok(nym_authenticator_requests::response::SerialisedResponse::new(bytes, reply_to))
}

pub(crate) fn build_final_authenticator_response(
    ip_allocation: IpPair,
    wg_port: u16,
    pub_key: PeerPublicKey,
    upgrade_mode_enabled: bool,
    request_id: u64,
    version: AuthenticatorVersion,
    reply_to: Option<Recipient>,
) -> Result<SerialisedResponse, GatewayWireguardError> {
    let private_ipv4 = ip_allocation.ipv4;
    let private_ipv6 = ip_allocation.ipv6;

    let bytes = match version {
        AuthenticatorVersion::V1 => v1::response::AuthenticatorResponse::new_registered(
            v1::registration::RegisteredData {
                pub_key,
                private_ip: private_ipv4.into(),
                wg_port,
            },
            reply_to.ok_or(GatewayWireguardError::MissingReplyToForOldClient)?,
            request_id,
        )
        .to_bytes()
        .map_err(GatewayWireguardError::authenticator_response_serialisation)?,
        AuthenticatorVersion::V2 => v2::response::AuthenticatorResponse::new_registered(
            v2::registration::RegisteredData {
                pub_key,
                private_ip: private_ipv4.into(),
                wg_port,
            },
            reply_to.ok_or(GatewayWireguardError::MissingReplyToForOldClient)?,
            request_id,
        )
        .to_bytes()
        .map_err(GatewayWireguardError::authenticator_response_serialisation)?,
        AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_registered(
            v3::registration::RegisteredData {
                pub_key,
                private_ip: private_ipv4.into(),
                wg_port,
            },
            reply_to.ok_or(GatewayWireguardError::MissingReplyToForOldClient)?,
            request_id,
        )
        .to_bytes()
        .map_err(GatewayWireguardError::authenticator_response_serialisation)?,
        AuthenticatorVersion::V4 => v4::response::AuthenticatorResponse::new_registered(
            v4::registration::RegisteredData {
                pub_key,
                private_ips: v4::registration::IpPair::new(private_ipv4, private_ipv6),
                wg_port,
            },
            reply_to.ok_or(GatewayWireguardError::MissingReplyToForOldClient)?,
            request_id,
        )
        .to_bytes()
        .map_err(GatewayWireguardError::authenticator_response_serialisation)?,
        AuthenticatorVersion::V5 => v5::response::AuthenticatorResponse::new_registered(
            v5::registration::RegisteredData {
                pub_key,
                private_ips: v5::registration::IpPair::new(private_ipv4, private_ipv6),
                wg_port,
            },
            request_id,
        )
        .to_bytes()
        .map_err(GatewayWireguardError::authenticator_response_serialisation)?,
        AuthenticatorVersion::V6 => v6::response::AuthenticatorResponse::new_registered(
            v6::registration::RegisteredData {
                pub_key,
                private_ips: v6::registration::IpPair::new(private_ipv4, private_ipv6),
                wg_port,
            },
            request_id,
            upgrade_mode_enabled,
        )
        .to_bytes()
        .map_err(GatewayWireguardError::authenticator_response_serialisation)?,
        AuthenticatorVersion::UNKNOWN => {
            return Err(GatewayWireguardError::UnknownAuthenticatorVersion)
        }
    };
    Ok(SerialisedResponse::new(bytes, reply_to))
}
