// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::RegistrationClientConfig;
use crate::config::RegistrationMode;
use crate::error::RegistrationClientError;
use crate::types::{MixnetRegistrationResult, RegistrationResult, WireguardRegistrationResult};
use nym_authenticator_client::{AuthClientMixnetListener, AuthenticatorClient};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_ip_packet_client::IprClientConnect;
use nym_registration_common::AssignedAddresses;
use nym_sdk::mixnet::{EventReceiver, MixnetClient, Recipient};
use tokio_util::sync::CancellationToken;

pub struct MixnetBasedRegistrationClient {
    pub(crate) mixnet_client: MixnetClient,
    pub(crate) config: RegistrationClientConfig,
    pub(crate) mixnet_client_address: Recipient,
    pub(crate) bandwidth_controller: Box<dyn BandwidthTicketProvider>,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) event_rx: EventReceiver,
}

impl MixnetBasedRegistrationClient {
    async fn register_mix_exit(self) -> Result<RegistrationResult, RegistrationClientError> {
        let entry_mixnet_gateway_ip = self.config.entry.node.ip_address;

        let exit_mixnet_gateway_ip = self.config.exit.node.ip_address;

        let ipr_address = self.config.exit.node.ipr_address.ok_or(
            RegistrationClientError::NoIpPacketRouterAddress {
                node_id: self.config.exit.node.identity.to_base58_string(),
            },
        )?;
        let mut ipr_client = IprClientConnect::new(self.mixnet_client, self.cancel_token.clone());
        let interface_addresses = ipr_client
            .connect(ipr_address)
            .await
            .map_err(RegistrationClientError::ConnectToIpPacketRouter)?;

        Ok(RegistrationResult::Mixnet(Box::new(
            MixnetRegistrationResult {
                mixnet_client: ipr_client.into_mixnet_client(),
                assigned_addresses: AssignedAddresses {
                    interface_addresses,
                    exit_mix_address: ipr_address,
                    mixnet_client_address: self.mixnet_client_address,
                    entry_mixnet_gateway_ip,
                    exit_mixnet_gateway_ip,
                },
                event_rx: self.event_rx,
            },
        )))
    }

    async fn register_wg(self) -> Result<RegistrationResult, RegistrationClientError> {
        let entry_auth_address = self.config.entry.node.authenticator_address.ok_or(
            RegistrationClientError::AuthenticationNotPossible {
                node_id: self.config.entry.node.identity.to_base58_string(),
            },
        )?;

        let exit_auth_address = self.config.exit.node.authenticator_address.ok_or(
            RegistrationClientError::AuthenticationNotPossible {
                node_id: self.config.exit.node.identity.to_base58_string(),
            },
        )?;

        let entry_version = self.config.entry.node.version;
        tracing::debug!("Entry gateway version: {entry_version}");
        let exit_version = self.config.exit.node.version;
        tracing::debug!("Exit gateway version: {exit_version}");

        // Start the auth client mixnet listener, which will listen for incoming messages from the
        // mixnet and rebroadcast them to the auth clients.
        let mixnet_listener =
            AuthClientMixnetListener::new(self.mixnet_client, self.cancel_token.clone()).start();

        let mut entry_auth_client = AuthenticatorClient::new(
            mixnet_listener.subscribe(),
            mixnet_listener.mixnet_sender(),
            self.mixnet_client_address,
            entry_auth_address,
            entry_version,
            self.config.entry.keys,
            self.config.entry.node.ip_address,
        );

        let mut exit_auth_client = AuthenticatorClient::new(
            mixnet_listener.subscribe(),
            mixnet_listener.mixnet_sender(),
            self.mixnet_client_address,
            exit_auth_address,
            exit_version,
            self.config.exit.keys,
            self.config.exit.node.ip_address,
        );

        let entry_fut = entry_auth_client
            .register_wireguard(&*self.bandwidth_controller, TicketType::V1WireguardEntry);
        let exit_fut = exit_auth_client
            .register_wireguard(&*self.bandwidth_controller, TicketType::V1WireguardExit);

        let (entry, exit) = Box::pin(async { tokio::join!(entry_fut, exit_fut) }).await;

        let entry = entry.map_err(|source| {
            RegistrationClientError::from_authenticator_error(
                source,
                self.config.entry.node.identity.to_base58_string(),
                entry_auth_address,
                true, // is entry
            )
        })?;
        let exit = exit.map_err(|source| {
            RegistrationClientError::from_authenticator_error(
                source,
                self.config.exit.node.identity.to_base58_string(),
                exit_auth_address,
                false, // is exit (not entry)
            )
        })?;

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

    pub(crate) async fn register(self) -> Result<RegistrationResult, RegistrationClientError> {
        self.cancel_token
            .clone()
            .run_until_cancelled(async {
                match self.config.mode {
                    RegistrationMode::Mixnet => self.register_mix_exit().await,
                    RegistrationMode::Wireguard => self.register_wg().await,
                }
            })
            .await
            .ok_or(RegistrationClientError::Cancelled)?
    }
}
