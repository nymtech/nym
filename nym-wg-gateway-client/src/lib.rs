// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub mod deprecated;
mod error;

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

pub use error::{Error, ErrorMessage};
use nym_authenticator_client::{
    AuthenticatorClient, AuthenticatorMixnetClient, AuthenticatorResponse, AuthenticatorVersion,
    ClientMessage,
};
use nym_authenticator_requests::{v2, v3, v4, v5};
use nym_bandwidth_controller::PreparedCredential;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{encryption, x25519::KeyPair};
use nym_gateway_directory::{NodeIdentity, Recipient};
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::PeerPublicKey;
use nym_pemstore::KeyPairPath;
use nym_sdk::mixnet::CredentialStorage;
use nym_validator_client::QueryHttpRpcNyxdClient;
use nym_wg_go::PublicKey;
use rand::{rngs::OsRng, CryptoRng, RngCore};
use tracing::{debug, error, trace};

use crate::error::Result;

pub const DEFAULT_PRIVATE_ENTRY_WIREGUARD_KEY_FILENAME: &str = "free_private_entry_wireguard.pem";
pub const DEFAULT_PUBLIC_ENTRY_WIREGUARD_KEY_FILENAME: &str = "free_public_entry_wireguard.pem";
pub const DEFAULT_PRIVATE_EXIT_WIREGUARD_KEY_FILENAME: &str = "free_private_exit_wireguard.pem";
pub const DEFAULT_PUBLIC_EXIT_WIREGUARD_KEY_FILENAME: &str = "free_public_exit_wireguard.pem";

pub const TICKETS_TO_SPEND: u32 = 1;

#[derive(Clone, Debug)]
pub struct GatewayData {
    pub public_key: PublicKey,
    pub endpoint: SocketAddr,
    pub private_ipv4: Ipv4Addr,
    pub private_ipv6: Ipv6Addr,
}

pub struct WgGatewayClient {
    keypair: encryption::KeyPair,
    auth_client: AuthenticatorClient,
}

impl WgGatewayClient {
    fn new_type(
        data_path: &Option<PathBuf>,
        auth_mix_client: AuthenticatorMixnetClient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
        private_file_name: &str,
        public_file_name: &str,
    ) -> Self {
        let mut rng = OsRng;
        let auth_client = AuthenticatorClient::new(auth_mix_client, auth_recipient, auth_version);
        if let Some(data_path) = data_path {
            let paths = KeyPairPath::new(
                data_path.join(private_file_name),
                data_path.join(public_file_name),
            );
            let keypair = load_or_generate_keypair(&mut rng, paths);
            WgGatewayClient {
                keypair,
                auth_client,
            }
        } else {
            WgGatewayClient {
                keypair: KeyPair::new(&mut rng),
                auth_client,
            }
        }
    }

    pub fn new_entry(
        data_path: &Option<PathBuf>,
        auth_mix_client: AuthenticatorMixnetClient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
    ) -> Self {
        Self::new_type(
            data_path,
            auth_mix_client,
            auth_recipient,
            auth_version,
            DEFAULT_PRIVATE_ENTRY_WIREGUARD_KEY_FILENAME,
            DEFAULT_PUBLIC_ENTRY_WIREGUARD_KEY_FILENAME,
        )
    }

    pub fn new_exit(
        data_path: &Option<PathBuf>,
        auth_mix_client: AuthenticatorMixnetClient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
    ) -> Self {
        Self::new_type(
            data_path,
            auth_mix_client,
            auth_recipient,
            auth_version,
            DEFAULT_PRIVATE_EXIT_WIREGUARD_KEY_FILENAME,
            DEFAULT_PUBLIC_EXIT_WIREGUARD_KEY_FILENAME,
        )
    }

    pub fn keypair(&self) -> &encryption::KeyPair {
        &self.keypair
    }

    pub fn auth_recipient(&self) -> Recipient {
        self.auth_client.auth_recipient()
    }

    pub fn auth_version(&self) -> AuthenticatorVersion {
        self.auth_client.auth_version()
    }

    pub async fn request_bandwidth<St: CredentialStorage>(
        gateway_id: NodeIdentity,
        controller: &nym_bandwidth_controller::BandwidthController<QueryHttpRpcNyxdClient, St>,
        ticketbook_type: TicketType,
    ) -> Result<PreparedCredential>
    where
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        let credential = controller
            .prepare_ecash_ticket(ticketbook_type, gateway_id.to_bytes(), TICKETS_TO_SPEND)
            .await
            .map_err(|source| Error::GetTicket {
                ticketbook_type,
                source,
            })?;
        Ok(credential)
    }

    pub async fn register_wireguard<St: CredentialStorage>(
        &mut self,
        gateway_host: IpAddr,
        controller: &nym_bandwidth_controller::BandwidthController<QueryHttpRpcNyxdClient, St>,
        ticketbook_type: TicketType,
    ) -> Result<GatewayData>
    where
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        debug!("Registering with the wg gateway...");
        let init_message = match self.auth_version() {
            AuthenticatorVersion::V2 => {
                ClientMessage::Initial(Box::new(v2::registration::InitMessage {
                    pub_key: PeerPublicKey::new(self.keypair.public_key().to_bytes().into()),
                }))
            }
            AuthenticatorVersion::V3 => {
                ClientMessage::Initial(Box::new(v3::registration::InitMessage {
                    pub_key: PeerPublicKey::new(self.keypair.public_key().to_bytes().into()),
                }))
            }
            AuthenticatorVersion::V4 => {
                ClientMessage::Initial(Box::new(v4::registration::InitMessage {
                    pub_key: PeerPublicKey::new(self.keypair.public_key().to_bytes().into()),
                }))
            }
            AuthenticatorVersion::V5 => {
                ClientMessage::Initial(Box::new(v5::registration::InitMessage {
                    pub_key: PeerPublicKey::new(self.keypair.public_key().to_bytes().into()),
                }))
            }
            AuthenticatorVersion::UNKNOWN => return Err(Error::UnsupportedAuthenticatorVersion),
        };
        trace!("sending init msg to {}: {:?}", &gateway_host, &init_message);
        let response = self.auth_client.send(&init_message).await?;
        let registered_data = match response {
            AuthenticatorResponse::PendingRegistration(pending_registration_response) => {
                // Unwrap since we have already checked that we have the keypair.
                debug!("Verifying data");
                if let Err(e) = pending_registration_response.verify(self.keypair.private_key()) {
                    return Err(Error::VerificationFailed(e));
                }

                trace!(
                    "received \"pending-registration\" msg from {}: {:?}",
                    &gateway_host,
                    &pending_registration_response
                );

                let credential = Some(
                    Self::request_bandwidth(
                        self.auth_recipient().gateway(),
                        controller,
                        ticketbook_type,
                    )
                    .await?
                    .data,
                );

                let finalized_message = match self.auth_version() {
                    AuthenticatorVersion::V2 => {
                        ClientMessage::Final(Box::new(v2::registration::FinalMessage {
                            gateway_client: v2::registration::GatewayClient::new(
                                self.keypair.private_key(),
                                pending_registration_response.pub_key().inner(),
                                pending_registration_response.private_ips().ipv4.into(),
                                pending_registration_response.nonce(),
                            ),
                            credential,
                        }))
                    }
                    AuthenticatorVersion::V3 => {
                        ClientMessage::Final(Box::new(v3::registration::FinalMessage {
                            gateway_client: v3::registration::GatewayClient::new(
                                self.keypair.private_key(),
                                pending_registration_response.pub_key().inner(),
                                pending_registration_response.private_ips().ipv4.into(),
                                pending_registration_response.nonce(),
                            ),
                            credential,
                        }))
                    }
                    AuthenticatorVersion::V4 => {
                        ClientMessage::Final(Box::new(v4::registration::FinalMessage {
                            gateway_client: v4::registration::GatewayClient::new(
                                self.keypair.private_key(),
                                pending_registration_response.pub_key().inner(),
                                pending_registration_response.private_ips().into(),
                                pending_registration_response.nonce(),
                            ),
                            credential,
                        }))
                    }
                    AuthenticatorVersion::V5 => {
                        ClientMessage::Final(Box::new(v5::registration::FinalMessage {
                            gateway_client: v5::registration::GatewayClient::new(
                                self.keypair.private_key(),
                                pending_registration_response.pub_key().inner(),
                                pending_registration_response.private_ips(),
                                pending_registration_response.nonce(),
                            ),
                            credential,
                        }))
                    }
                    AuthenticatorVersion::UNKNOWN => {
                        return Err(Error::UnsupportedAuthenticatorVersion);
                    }
                };
                trace!(
                    "sending final msg to {}: {:?}",
                    &gateway_host,
                    &finalized_message
                );

                let response = self.auth_client.send(&finalized_message).await?;
                let AuthenticatorResponse::Registered(registered_response) = response else {
                    return Err(Error::InvalidGatewayAuthResponse);
                };
                registered_response
            }
            AuthenticatorResponse::Registered(registered_response) => registered_response,
            _ => return Err(Error::InvalidGatewayAuthResponse),
        };

        trace!(
            "received \"registered\" msg from {}: {:?}",
            &gateway_host,
            &registered_data
        );

        let gateway_data = GatewayData {
            public_key: PublicKey::from(registered_data.pub_key().to_bytes()),
            endpoint: SocketAddr::from_str(&format!(
                "{}:{}",
                gateway_host,
                registered_data.wg_port()
            ))
            .map_err(Error::FailedToParseEntryGatewaySocketAddr)?,
            private_ipv4: registered_data.private_ips().ipv4,
            private_ipv6: registered_data.private_ips().ipv6,
        };

        Ok(gateway_data)
    }
}

fn load_or_generate_keypair<R: RngCore + CryptoRng>(rng: &mut R, paths: KeyPairPath) -> KeyPair {
    match nym_pemstore::load_keypair(&paths) {
        Ok(keypair) => keypair,
        Err(_) => {
            let keypair = KeyPair::new(rng);
            if let Err(e) = nym_pemstore::store_keypair(&keypair, &paths) {
                error!(
                    "could not store generated keypair at {:?} - {:?}; will use ephemeral keys",
                    paths, e
                );
            }
            keypair
        }
    }
}
