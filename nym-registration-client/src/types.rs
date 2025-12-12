// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_authenticator_client::{AuthClientMixnetListenerHandle, AuthenticatorClient};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_registration_common::{AssignedAddresses, GatewayData};
use nym_sdk::mixnet::{EventReceiver, MixnetClient};

pub enum RegistrationResult {
    Mixnet(Box<MixnetRegistrationResult>),
    Wireguard(Box<WireguardRegistrationResult>),
    Lp(Box<LpRegistrationResult>),
}

pub struct MixnetRegistrationResult {
    pub assigned_addresses: AssignedAddresses,
    pub mixnet_client: MixnetClient,
    pub event_rx: EventReceiver,
}

pub struct WireguardRegistrationResult {
    pub entry_gateway_client: AuthenticatorClient,
    pub exit_gateway_client: AuthenticatorClient,
    pub entry_gateway_data: GatewayData,
    pub exit_gateway_data: GatewayData,
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
/// * `bw_controller` - Bandwidth ticket provider for credential management
pub struct LpRegistrationResult {
    /// Gateway configuration data from entry gateway
    pub entry_gateway_data: GatewayData,

    /// Gateway configuration data from exit gateway
    pub exit_gateway_data: GatewayData,

    /// Bandwidth controller for credential management
    pub bw_controller: Box<dyn BandwidthTicketProvider>,
}
