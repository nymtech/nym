// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use tokio_util::sync::CancellationToken;

use nym_authenticator_client::{AuthClientMixnetListener, AuthenticatorClient};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_ip_packet_client::IprClientConnect;
use nym_registration_common::AssignedAddresses;
use nym_sdk::mixnet::{EventReceiver, MixnetClient, Recipient};
use tracing::debug;

use crate::config::RegistrationClientConfig;

mod builder;
mod config;
mod error;
mod types;

pub use builder::RegistrationClientBuilder;
pub use builder::config::{
    BuilderConfig as RegistrationClientBuilderConfig, MixnetClientConfig,
    NymNodeWithKeys as RegistrationNymNode,
};
pub use error::RegistrationClientError;
pub use types::{MixnetRegistrationResult, RegistrationResult, WireguardRegistrationResult};

pub struct RegistrationClient {
    mixnet_client: MixnetClient,
    config: RegistrationClientConfig,
    mixnet_client_address: Recipient,
    bandwidth_controller: Box<dyn BandwidthTicketProvider>,
    cancel_token: CancellationToken,
    event_rx: EventReceiver,
}

// Bundle of an actual error and the underlying mixnet client so it can be shutdown correctly if needed
struct RegistrationError {
    mixnet_client: Option<MixnetClient>,
    source: crate::RegistrationClientError,
}

impl RegistrationClient {
    async fn register_mix_exit(self) -> Result<RegistrationResult, RegistrationError> {
        let entry_mixnet_gateway_ip = self.config.entry.node.ip_address;

        let exit_mixnet_gateway_ip = self.config.exit.node.ip_address;

        let Some(ipr_address) = self.config.exit.node.ipr_address else {
            return Err(RegistrationError {
                mixnet_client: Some(self.mixnet_client),
                source: RegistrationClientError::NoIpPacketRouterAddress {
                    node_id: self.config.exit.node.identity.to_base58_string(),
                },
            });
        };

        let mut ipr_client =
            IprClientConnect::new(self.mixnet_client, self.cancel_token.child_token());

        let interface_addresses = match self
            .cancel_token
            .run_until_cancelled(ipr_client.connect(ipr_address))
            .await
        {
            Some(Ok(addr)) => addr,
            Some(Err(e)) => {
                return Err(RegistrationError {
                    mixnet_client: Some(ipr_client.into_mixnet_client()),
                    source: RegistrationClientError::ConnectToIpPacketRouter(e),
                });
            }
            None => {
                return Err(RegistrationError {
                    mixnet_client: Some(ipr_client.into_mixnet_client()),
                    source: RegistrationClientError::Cancelled,
                });
            }
        };

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

    async fn register_wg(self) -> Result<RegistrationResult, RegistrationError> {
        let Some(entry_auth_address) = self.config.entry.node.authenticator_address else {
            return Err(RegistrationError {
                mixnet_client: Some(self.mixnet_client),
                source: RegistrationClientError::AuthenticationNotPossible {
                    node_id: self.config.entry.node.identity.to_base58_string(),
                },
            });
        };

        let Some(exit_auth_address) = self.config.exit.node.authenticator_address else {
            return Err(RegistrationError {
                mixnet_client: Some(self.mixnet_client),
                source: RegistrationClientError::AuthenticationNotPossible {
                    node_id: self.config.exit.node.identity.to_base58_string(),
                },
            });
        };

        let entry_version = self.config.entry.node.version;
        tracing::debug!("Entry gateway version: {entry_version}");
        let exit_version = self.config.exit.node.version;
        tracing::debug!("Exit gateway version: {exit_version}");

        // Start the auth client mixnet listener, which will listen for incoming messages from the
        // mixnet and rebroadcast them to the auth clients.
        // From this point on, we don't need to care about the mixnet client anymore
        let mixnet_listener =
            AuthClientMixnetListener::new(self.mixnet_client, self.cancel_token.child_token())
                .start();

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

        let (entry, exit) = Box::pin(
            self.cancel_token
                .run_until_cancelled(async { tokio::join!(entry_fut, exit_fut) }),
        )
        .await
        .ok_or(RegistrationError {
            mixnet_client: None,
            source: RegistrationClientError::Cancelled,
        })?;

        let entry = entry.map_err(|source| RegistrationError {
            mixnet_client: None,
            source: RegistrationClientError::EntryGatewayRegisterWireguard {
                gateway_id: self.config.entry.node.identity.to_base58_string(),
                authenticator_address: Box::new(entry_auth_address),
                source: Box::new(source),
            },
        })?;

        let exit = exit.map_err(|source| RegistrationError {
            mixnet_client: None,
            source: RegistrationClientError::EntryGatewayRegisterWireguard {
                gateway_id: self.config.exit.node.identity.to_base58_string(),
                authenticator_address: Box::new(exit_auth_address),
                source: Box::new(source),
            },
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

    pub async fn register(self) -> Result<RegistrationResult, RegistrationClientError> {
        let registration_result = if self.config.two_hops {
            self.register_wg().await
        } else {
            self.register_mix_exit().await
        };

        // If we failed to register, and we were the owner of the mixnet client, shut it down
        match registration_result {
            Ok(result) => Ok(result),
            Err(error) => {
                debug!("Registration failed");
                if let Some(mixnet_client) = error.mixnet_client {
                    debug!("Shutting down mixnet client");
                    mixnet_client.disconnect().await;
                }
                Err(error.source)
            }
        }
    }
}
