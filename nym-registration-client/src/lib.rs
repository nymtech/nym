// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use std::net::IpAddr;

use nym_authenticator_client::{
    AuthClientMixnetListener, AuthClientMixnetListenerHandle, AuthenticatorClient,
};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_sdk::mixnet::{MixnetClient, Recipient};
use nym_sdk::ShutdownManager;
use tokio_util::sync::CancellationToken;

use nym_ip_packet_client::IprClientConnect;
use nym_ip_packet_requests::IpPair;

use crate::config::RegistrationClientConfig;

mod builder;
mod config;
mod error;

pub use builder::config::{BuilderConfig as RegistrationClientBuilderConfig, MixnetClientConfig};
pub use builder::RegistrationClientBuilder;
pub use config::NymNode as RegistrationClientNymNode;
pub use error::RegistrationClientError;

pub use nym_authenticator_client::{
    GatewayData, DEFAULT_PRIVATE_ENTRY_WIREGUARD_KEY_FILENAME,
    DEFAULT_PRIVATE_EXIT_WIREGUARD_KEY_FILENAME, DEFAULT_PUBLIC_ENTRY_WIREGUARD_KEY_FILENAME,
    DEFAULT_PUBLIC_EXIT_WIREGUARD_KEY_FILENAME,
};

pub struct RegistrationClient {
    mixnet_client: MixnetClient,
    config: RegistrationClientConfig,
    mixnet_client_address: Recipient,
    bandwidth_controller: Box<dyn BandwidthTicketProvider>,
    mixnet_shutdown_manager: ShutdownManager,
    cancel_token: CancellationToken,
}

impl RegistrationClient {
    async fn register_mix_exit(self) -> Result<RegistrationResult, RegistrationClientError> {
        let entry_mixnet_gateway_ip = self.config.entry.ip_address;

        let exit_mixnet_gateway_ip = self.config.exit.ip_address;

        let ipr_address = self.config.exit.ipr_address.ok_or(
            RegistrationClientError::NoIpPacketRouterAddress {
                node_id: self.config.exit.identity.to_base58_string(),
            },
        )?;
        let mut ipr_client =
            IprClientConnect::new(self.mixnet_client, self.cancel_token.clone()).await;
        let interface_addresses = ipr_client
            .connect(ipr_address)
            .await
            .map_err(RegistrationClientError::ConnectToIpPacketRouter)?;

        Ok(RegistrationResult::Mixnet(Box::new(
            MixnetRegistrationResult {
                mixnet_client: ipr_client.into_mixnet_client(),
                mixnet_shutdown_manager: self.mixnet_shutdown_manager,
                assigned_addresses: AssignedAddresses {
                    interface_addresses,
                    exit_mix_address: ipr_address,
                    mixnet_client_address: self.mixnet_client_address,
                    entry_mixnet_gateway_ip,
                    exit_mixnet_gateway_ip,
                },
            },
        )))
    }

    async fn register_wg(self) -> Result<RegistrationResult, RegistrationClientError> {
        let entry_auth_address = self.config.entry.authenticator_address.ok_or(
            RegistrationClientError::AuthenticationNotPossible {
                node_id: self.config.entry.identity.to_base58_string(),
            },
        )?;

        let exit_auth_address = self.config.exit.authenticator_address.ok_or(
            RegistrationClientError::AuthenticationNotPossible {
                node_id: self.config.exit.identity.to_base58_string(),
            },
        )?;

        let entry_version = self.config.entry.version;
        tracing::debug!("Entry gateway version: {entry_version}");
        let exit_version = self.config.exit.version;
        tracing::debug!("Exit gateway version: {exit_version}");

        // Start the auth client mixnet listener, which will listen for incoming messages from the
        // mixnet and rebroadcast them to the auth clients.
        let mixnet_listener = AuthClientMixnetListener::new(
            self.mixnet_client,
            self.cancel_token.clone(),
            self.mixnet_shutdown_manager,
        )
        .start();

        let mut entry_auth_client = AuthenticatorClient::new_entry(
            &self.config.data_path,
            mixnet_listener.subscribe(),
            mixnet_listener.mixnet_sender(),
            self.mixnet_client_address,
            entry_auth_address,
            entry_version,
            self.config.entry.ip_address,
        );

        let mut exit_auth_client = AuthenticatorClient::new_exit(
            &self.config.data_path,
            mixnet_listener.subscribe(),
            mixnet_listener.mixnet_sender(),
            self.mixnet_client_address,
            exit_auth_address,
            exit_version,
            self.config.exit.ip_address,
        );

        let entry_fut = entry_auth_client
            .register_wireguard(&*self.bandwidth_controller, TicketType::V1WireguardEntry);
        let exit_fut = exit_auth_client
            .register_wireguard(&*self.bandwidth_controller, TicketType::V1WireguardExit);

        let (entry, exit) = Box::pin(async { tokio::try_join!(entry_fut, exit_fut) })
            .await
            .map_err(Box::new)?;

        Ok(RegistrationResult::Wireguard(Box::new(
            WireguardRegistrationResult {
                entry_gateway_client: entry_auth_client,
                exit_gateway_client: exit_auth_client,
                entry_gateway_data: entry,
                exit_gateway_data: exit,
                authenticator_listener_handle: mixnet_listener,
                bw_controller: self.bandwidth_controller,
            },
        )))
    }

    pub async fn register(self) -> Result<RegistrationResult, RegistrationClientError> {
        self.cancel_token
            .clone()
            .run_until_cancelled(async {
                if self.config.two_hops {
                    self.register_wg().await
                } else {
                    self.register_mix_exit().await
                }
            })
            .await
            .ok_or(RegistrationClientError::Cancelled)?
    }
}

pub enum RegistrationResult {
    Mixnet(Box<MixnetRegistrationResult>),
    Wireguard(Box<WireguardRegistrationResult>),
}

#[derive(Clone, Copy, Debug)]
pub struct AssignedAddresses {
    pub entry_mixnet_gateway_ip: IpAddr,
    pub exit_mixnet_gateway_ip: IpAddr,
    pub mixnet_client_address: Recipient,
    pub exit_mix_address: Recipient,
    pub interface_addresses: IpPair,
}

pub struct MixnetRegistrationResult {
    pub assigned_addresses: AssignedAddresses,
    pub mixnet_client: nym_sdk::mixnet::MixnetClient,
    pub mixnet_shutdown_manager: nym_sdk::ShutdownManager,
}

pub struct WireguardRegistrationResult {
    pub entry_gateway_client: AuthenticatorClient,
    pub exit_gateway_client: AuthenticatorClient,
    pub entry_gateway_data: GatewayData,
    pub exit_gateway_data: GatewayData,
    pub authenticator_listener_handle: AuthClientMixnetListenerHandle,
    pub bw_controller: Box<dyn BandwidthTicketProvider>,
}
