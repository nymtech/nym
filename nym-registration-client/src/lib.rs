// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use tokio_util::sync::CancellationToken;

use nym_authenticator_client::{AuthClientMixnetListener, AuthenticatorClient};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_ip_packet_client::IprClientConnect;
use nym_registration_common::AssignedAddresses;
use nym_sdk::mixnet::{EventReceiver, MixnetClient, Recipient};
use std::sync::Arc;

use crate::config::RegistrationClientConfig;
use crate::lp_client::{LpClientError, LpTransport};

mod builder;
mod config;
mod error;
mod lp_client;
mod types;

pub use builder::RegistrationClientBuilder;
pub use builder::config::{
    BuilderConfig as RegistrationClientBuilderConfig, MixnetClientConfig,
    NymNodeWithKeys as RegistrationNymNode,
};
pub use config::RegistrationMode;
pub use error::RegistrationClientError;
pub use lp_client::{LpConfig, LpRegistrationClient};
pub use types::{
    LpRegistrationResult, MixnetRegistrationResult, RegistrationResult, WireguardRegistrationResult,
};

pub struct RegistrationClient {
    mixnet_client: MixnetClient,
    config: RegistrationClientConfig,
    mixnet_client_address: Recipient,
    bandwidth_controller: Box<dyn BandwidthTicketProvider>,
    cancel_token: CancellationToken,
    event_rx: EventReceiver,
}

impl RegistrationClient {
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

    async fn register_lp(self) -> Result<RegistrationResult, RegistrationClientError> {
        // Extract and validate LP addresses
        let entry_lp_address = self.config.entry.node.lp_address.ok_or(
            RegistrationClientError::LpRegistrationNotPossible {
                node_id: self.config.entry.node.identity.to_base58_string(),
            },
        )?;

        let exit_lp_address = self.config.exit.node.lp_address.ok_or(
            RegistrationClientError::LpRegistrationNotPossible {
                node_id: self.config.exit.node.identity.to_base58_string(),
            },
        )?;

        tracing::debug!("Entry gateway LP address: {}", entry_lp_address);
        tracing::debug!("Exit gateway LP address: {}", exit_lp_address);

        // Generate fresh Ed25519 keypairs for LP registration
        // These are ephemeral and used only for the LP handshake protocol
        use nym_crypto::asymmetric::ed25519;
        use rand::rngs::OsRng;
        let entry_lp_keypair = Arc::new(ed25519::KeyPair::new(&mut OsRng));
        let exit_lp_keypair = Arc::new(ed25519::KeyPair::new(&mut OsRng));

        // Register entry gateway via LP
        let entry_fut = {
            let bandwidth_controller = &self.bandwidth_controller;
            let entry_keys = self.config.entry.keys.clone();
            let entry_identity = self.config.entry.node.identity;
            let entry_ip = self.config.entry.node.ip_address;
            let entry_lp_keys = entry_lp_keypair.clone();

            async move {
                let mut client = LpRegistrationClient::new_with_default_psk(
                    entry_lp_keys,
                    entry_identity,
                    entry_lp_address,
                    entry_ip,
                );

                // Connect
                client.connect().await?;

                // Perform handshake
                client.perform_handshake().await?;

                // Send registration request
                client
                    .send_registration_request(
                        &entry_keys,
                        &entry_identity,
                        &**bandwidth_controller,
                        TicketType::V1WireguardEntry,
                    )
                    .await?;

                // Receive registration response
                let gateway_data = client.receive_registration_response().await?;

                // Convert to transport for ongoing communication
                let transport = client.into_transport()?;

                Ok::<(LpTransport, _), LpClientError>((transport, gateway_data))
            }
        };

        // Register exit gateway via LP
        let exit_fut = {
            let bandwidth_controller = &self.bandwidth_controller;
            let exit_keys = self.config.exit.keys.clone();
            let exit_identity = self.config.exit.node.identity;
            let exit_ip = self.config.exit.node.ip_address;
            let exit_lp_keys = exit_lp_keypair;

            async move {
                let mut client = LpRegistrationClient::new_with_default_psk(
                    exit_lp_keys,
                    exit_identity,
                    exit_lp_address,
                    exit_ip,
                );

                // Connect
                client.connect().await?;

                // Perform handshake
                client.perform_handshake().await?;

                // Send registration request
                client
                    .send_registration_request(
                        &exit_keys,
                        &exit_identity,
                        &**bandwidth_controller,
                        TicketType::V1WireguardExit,
                    )
                    .await?;

                // Receive registration response
                let gateway_data = client.receive_registration_response().await?;

                // Convert to transport for ongoing communication
                let transport = client.into_transport()?;

                Ok::<(LpTransport, _), LpClientError>((transport, gateway_data))
            }
        };

        // Execute registrations in parallel
        let (entry_result, exit_result) =
            Box::pin(async { tokio::join!(entry_fut, exit_fut) }).await;

        // Handle entry gateway result
        // Note: entry_transport is dropped here, closing the LP connection
        let (_entry_transport, entry_gateway_data) =
            entry_result.map_err(|source| RegistrationClientError::EntryGatewayRegisterLp {
                gateway_id: self.config.entry.node.identity.to_base58_string(),
                lp_address: entry_lp_address,
                source: Box::new(source),
            })?;

        // Handle exit gateway result
        // Note: exit_transport is dropped here, closing the LP connection
        let (_exit_transport, exit_gateway_data) =
            exit_result.map_err(|source| RegistrationClientError::ExitGatewayRegisterLp {
                gateway_id: self.config.exit.node.identity.to_base58_string(),
                lp_address: exit_lp_address,
                source: Box::new(source),
            })?;

        tracing::info!(
            "LP registration successful for both gateways (LP connections will be closed)"
        );

        // LP is registration-only. All data flows through WireGuard after this point.
        // The LP transports have been dropped, automatically closing TCP connections.
        Ok(RegistrationResult::Lp(Box::new(LpRegistrationResult {
            entry_gateway_data,
            exit_gateway_data,
            bw_controller: self.bandwidth_controller,
        })))
    }

    pub async fn register(self) -> Result<RegistrationResult, RegistrationClientError> {
        self.cancel_token
            .clone()
            .run_until_cancelled(async {
                match self.config.mode {
                    RegistrationMode::Mixnet => self.register_mix_exit().await,
                    RegistrationMode::Wireguard => self.register_wg().await,
                    RegistrationMode::Lp => self.register_lp().await,
                }
            })
            .await
            .ok_or(RegistrationClientError::Cancelled)?
    }
}
