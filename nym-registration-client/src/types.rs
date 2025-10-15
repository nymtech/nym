// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_authenticator_client::{
    AuthenticatorClient, mixnet_listener::AuthClientMixnetListenerHandle,
};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_registration_common::{AssignedAddresses, GatewayData};
use nym_sdk::mixnet::MixnetClient;

pub enum RegistrationResult {
    Mixnet(Box<MixnetRegistrationResult>),
    Wireguard(Box<WireguardRegistrationResult>),
}

pub struct MixnetRegistrationResult {
    pub assigned_addresses: AssignedAddresses,
    pub mixnet_client: MixnetClient,
}

pub struct WireguardRegistrationResult {
    pub entry_gateway_client: AuthenticatorClient,
    pub exit_gateway_client: AuthenticatorClient,
    pub entry_gateway_data: GatewayData,
    pub exit_gateway_data: GatewayData,
    pub authenticator_listener_handle: AuthClientMixnetListenerHandle,
    pub bw_controller: Box<dyn BandwidthTicketProvider>,
}
