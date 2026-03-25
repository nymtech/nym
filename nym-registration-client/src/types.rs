// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_authenticator_client::{AuthClientMixnetListenerHandle, AuthenticatorClient};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_lp::peer::DHKeyPair;
use nym_registration_common::{AssignedAddresses, WireguardConfiguration};
use nym_sdk::mixnet::{EventReceiver, MixnetClient};
use std::sync::Arc;

pub enum RegistrationResult {
    Mixnet(Box<MixnetRegistrationResult>),
    Wireguard(Box<WireguardRegistrationResult>),
}

impl RegistrationResult {
    pub fn mixnet(
        mixnet_client: MixnetClient,
        assigned_addresses: AssignedAddresses,
        event_rx: EventReceiver,
    ) -> Self {
        RegistrationResult::Mixnet(Box::new(MixnetRegistrationResult {
            assigned_addresses,
            mixnet_client,
            event_rx,
        }))
    }

    pub fn wireguard_legacy(
        entry_gateway_client: AuthenticatorClient,
        exit_gateway_client: AuthenticatorClient,
        entry_gateway_data: WireguardConfiguration,
        exit_gateway_data: WireguardConfiguration,
        authenticator_listener_handle: AuthClientMixnetListenerHandle,
        bw_controller: Box<dyn BandwidthTicketProvider>,
    ) -> Self {
        RegistrationResult::Wireguard(Box::new(WireguardRegistrationResult::Legacy(Box::new(
            AuthenticatorRegistrationResult {
                entry_gateway_client,
                exit_gateway_client,
                entry_gateway_data,
                exit_gateway_data,
                authenticator_listener_handle,
                bw_controller,
            },
        ))))
    }

    pub fn wireguard_lp(
        entry_gateway_data: WireguardConfiguration,
        exit_gateway_data: WireguardConfiguration,
        entry_lp_keypair: Arc<DHKeyPair>,
        exit_lp_keypair: Arc<DHKeyPair>,
        bw_controller: Box<dyn BandwidthTicketProvider>,
    ) -> Self {
        RegistrationResult::Wireguard(Box::new(WireguardRegistrationResult::LewesProtocol(
            Box::new(LpRegistrationResult {
                entry_gateway_data,
                exit_gateway_data,
                entry_lp_keypair,
                exit_lp_keypair,
                bw_controller,
            }),
        )))
    }
}

pub struct MixnetRegistrationResult {
    pub assigned_addresses: AssignedAddresses,
    pub mixnet_client: MixnetClient,
    pub event_rx: EventReceiver,
}

pub enum WireguardRegistrationResult {
    Legacy(Box<AuthenticatorRegistrationResult>),
    LewesProtocol(Box<LpRegistrationResult>),
}

impl WireguardRegistrationResult {
    pub fn entry_gateway_data(&self) -> &WireguardConfiguration {
        match self {
            Self::Legacy(res) => &res.entry_gateway_data,
            Self::LewesProtocol(res) => &res.entry_gateway_data,
        }
    }

    pub fn exit_gateway_data(&self) -> &WireguardConfiguration {
        match self {
            Self::Legacy(res) => &res.exit_gateway_data,
            Self::LewesProtocol(res) => &res.exit_gateway_data,
        }
    }
}

pub struct AuthenticatorRegistrationResult {
    pub entry_gateway_client: AuthenticatorClient,
    pub exit_gateway_client: AuthenticatorClient,
    pub entry_gateway_data: WireguardConfiguration,
    pub exit_gateway_data: WireguardConfiguration,
    pub authenticator_listener_handle: AuthClientMixnetListenerHandle,
    pub bw_controller: Box<dyn BandwidthTicketProvider>,
}

/// Result of LP (Lewes Protocol) registration with entry and exit gateways.
///
/// LP is used only for registration. After successful registration, all data flows
/// through WireGuard tunnels established using the returned gateway configuration.
/// The LP connections are automatically closed after registration completes.
///
/// # Fields
/// * `entry_gateway_data` - WireGuard configuration from entry gateway
/// * `exit_gateway_data` - WireGuard configuration from exit gateway
/// * `entry_lp_keypair` - x25519 keypair used on the entry LP channel (persist to resume a pre-established session)
/// * `exit_lp_keypair` - x25519 keypair used on the exit LP channel (persist to resume a pre-established session)
/// * `bw_controller` - Bandwidth ticket provider for credential management
pub struct LpRegistrationResult {
    /// Gateway configuration data from entry gateway
    pub entry_gateway_data: WireguardConfiguration,

    /// Gateway configuration data from exit gateway
    pub exit_gateway_data: WireguardConfiguration,

    /// x25519 keypair used on the entry channel.
    /// the purpose of persisting those keys is to be able to resume the pre-established session
    pub entry_lp_keypair: Arc<DHKeyPair>,

    /// x25519 keypair used on the exit channel
    /// the purpose of persisting those keys is to be able to resume the pre-established session
    pub exit_lp_keypair: Arc<DHKeyPair>,

    /// Bandwidth controller for credential management
    pub bw_controller: Box<dyn BandwidthTicketProvider>,
}
