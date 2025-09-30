// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use rand::rngs::OsRng;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, error, trace};

use crate::mixnet_listener::{MixnetMessageBroadcastReceiver, MixnetMessageInputSender};
use nym_authenticator_requests::{
    client_message::ClientMessage, response::AuthenticatorResponse, traits::Id, v2, v3, v4, v5,
    AuthenticatorVersion,
};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::x25519::{KeyPair, PublicKey};
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::PeerPublicKey;
use nym_pemstore::KeyPairPath;
use nym_sdk::mixnet::{IncludedSurbs, Recipient};
use nym_service_provider_requests_common::{Protocol, ServiceProviderTypeExt};

mod error;
mod helpers;
mod legacy;
mod mixnet_listener;

pub use crate::error::{Error, Result};
pub use crate::legacy::LegacyAuthenticatorClient;
pub use crate::mixnet_listener::{AuthClientMixnetListener, AuthClientMixnetListenerHandle};

// that should be somewhere else imo
pub const DEFAULT_PRIVATE_ENTRY_WIREGUARD_KEY_FILENAME: &str = "free_private_entry_wireguard.pem";
pub const DEFAULT_PUBLIC_ENTRY_WIREGUARD_KEY_FILENAME: &str = "free_public_entry_wireguard.pem";
pub const DEFAULT_PRIVATE_EXIT_WIREGUARD_KEY_FILENAME: &str = "free_private_exit_wireguard.pem";
pub const DEFAULT_PUBLIC_EXIT_WIREGUARD_KEY_FILENAME: &str = "free_public_exit_wireguard.pem";

#[derive(Clone, Debug)]
pub struct GatewayData {
    pub public_key: PublicKey,
    pub endpoint: SocketAddr,
    pub private_ipv4: Ipv4Addr,
    pub private_ipv6: Ipv6Addr,
}

pub struct AuthenticatorClient {
    mixnet_listener: MixnetMessageBroadcastReceiver,
    mixnet_sender: MixnetMessageInputSender,
    our_nym_address: Recipient,
    pub auth_recipient: Recipient,
    auth_version: AuthenticatorVersion,

    keypair: KeyPair,
    ip_addr: IpAddr,
}

impl AuthenticatorClient {
    #[allow(clippy::too_many_arguments)]
    fn new_type(
        data_path: &Option<PathBuf>,
        mixnet_listener: MixnetMessageBroadcastReceiver,
        mixnet_sender: MixnetMessageInputSender,
        our_nym_address: Recipient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
        private_file_name: &str,
        public_file_name: &str,
        ip_addr: IpAddr,
    ) -> Self {
        let mut rng = OsRng;

        let keypair = if let Some(data_path) = data_path {
            let paths = KeyPairPath::new(
                data_path.join(private_file_name),
                data_path.join(public_file_name),
            );
            helpers::load_or_generate_keypair(&mut rng, paths)
        } else {
            KeyPair::new(&mut rng)
        };
        Self {
            mixnet_listener,
            mixnet_sender,
            our_nym_address,
            auth_recipient,
            auth_version,
            keypair,
            ip_addr,
        }
    }

    pub fn new_entry(
        data_path: &Option<PathBuf>,
        mixnet_listener: MixnetMessageBroadcastReceiver,
        mixnet_sender: MixnetMessageInputSender,
        our_nym_address: Recipient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
        ip_addr: IpAddr,
    ) -> Self {
        Self::new_type(
            data_path,
            mixnet_listener,
            mixnet_sender,
            our_nym_address,
            auth_recipient,
            auth_version,
            DEFAULT_PRIVATE_ENTRY_WIREGUARD_KEY_FILENAME,
            DEFAULT_PUBLIC_ENTRY_WIREGUARD_KEY_FILENAME,
            ip_addr,
        )
    }

    pub fn new_exit(
        data_path: &Option<PathBuf>,
        mixnet_listener: MixnetMessageBroadcastReceiver,
        mixnet_sender: MixnetMessageInputSender,
        our_nym_address: Recipient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
        ip_addr: IpAddr,
    ) -> Self {
        Self::new_type(
            data_path,
            mixnet_listener,
            mixnet_sender,
            our_nym_address,
            auth_recipient,
            auth_version,
            DEFAULT_PRIVATE_EXIT_WIREGUARD_KEY_FILENAME,
            DEFAULT_PUBLIC_EXIT_WIREGUARD_KEY_FILENAME,
            ip_addr,
        )
    }

    pub async fn send_and_wait_for_response(
        &mut self,
        message: &ClientMessage,
    ) -> Result<AuthenticatorResponse> {
        let request_id = self.send_request(message).await?;

        debug!("Waiting for reply...");
        self.listen_for_response(request_id).await
    }

    async fn send_request(&self, message: &ClientMessage) -> Result<u64> {
        let (data, request_id) = message.bytes(self.our_nym_address)?;

        // We use 20 surbs for the connect request because typically the
        // authenticator mixnet client on the nym-node is configured to have a min
        // threshold of 10 surbs that it reserves for itself to request additional
        // surbs.
        let surbs = if message.use_surbs() {
            match &message {
                ClientMessage::Initial(_) => IncludedSurbs::new(20),
                _ => IncludedSurbs::new(1),
            }
        } else {
            IncludedSurbs::ExposeSelfAddress
        };
        let input_message = helpers::create_input_message(self.auth_recipient, data, surbs);

        self.mixnet_sender
            .send(input_message)
            .await
            .map_err(|e| Error::SendMixnetMessage(Box::new(e)))?;

        Ok(request_id)
    }

    async fn listen_for_response(&mut self, request_id: u64) -> Result<AuthenticatorResponse> {
        let timeout = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    error!("Timed out waiting for reply to connect request");
                    return Err(Error::TimeoutWaitingForConnectResponse);
                }
                msg = self.mixnet_listener.recv() => match msg {
                    Err(_) => {
                        return Err(Error::NoMixnetMessagesReceived);
                    }
                    Ok(msg) => {
                        let Some(header) = msg.message.first_chunk::<2>() else {
                            debug!("received too short message that couldn't have been from the authenticator while waiting for connect response");
                            continue;
                        };

                        let Ok(protocol) = Protocol::try_from(header) else {
                            debug!("received a message not meant to any service provider while waiting for connect response");
                            continue;
                        };

                        if !protocol.service_provider_type.is_authenticator() {
                            debug!("Received non-authenticator message while waiting for connect response");
                            continue;
                        }
                        // Confirm that the version is correct
                        let version = AuthenticatorVersion::from(protocol.version);

                        // Then we deserialize the message
                        debug!("AuthClient: got message while waiting for connect response with version {version:?}");
                        let ret: Result<AuthenticatorResponse> = match version {
                            AuthenticatorVersion::V1 => Err(Error::UnsupportedVersion),
                            AuthenticatorVersion::V2 => v2::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::V4 => v4::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::V5 => v5::response::AuthenticatorResponse::from_reconstructed_message(&msg).map(Into::into).map_err(Into::into),
                            AuthenticatorVersion::UNKNOWN => Err(Error::UnknownVersion),
                        };
                        let Ok(response) = ret else {
                            // This is ok, it's likely just one of our self-pings
                            debug!("Failed to deserialize reconstructed message");
                            continue;
                        };

                        if response.id() == request_id {
                            debug!("Got response with matching id");
                            return Ok(response);
                        }
                    }
                }
            }
        }
    }

    pub async fn register_wireguard(
        &mut self,
        controller: &dyn BandwidthTicketProvider,
        ticketbook_type: TicketType,
    ) -> Result<GatewayData> {
        debug!("Registering with the wg gateway...");
        let init_message = match self.auth_version {
            AuthenticatorVersion::V1 => return Err(Error::UnsupportedAuthenticatorVersion),
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
        trace!("sending init msg to {}: {:?}", &self.ip_addr, &init_message);
        let response = self.send_and_wait_for_response(&init_message).await?;
        let registered_data = match response {
            AuthenticatorResponse::PendingRegistration(pending_registration_response) => {
                // Unwrap since we have already checked that we have the keypair.
                debug!("Verifying data");
                if let Err(e) = pending_registration_response.verify(self.keypair.private_key()) {
                    return Err(Error::VerificationFailed(e));
                }

                trace!(
                    "received \"pending-registration\" msg from {}: {:?}",
                    &self.ip_addr,
                    &pending_registration_response
                );

                let credential = Some(
                    controller
                        .get_ecash_ticket(
                            ticketbook_type,
                            self.auth_recipient.gateway(),
                            DEFAULT_TICKETS_TO_SPEND,
                        )
                        .await
                        .map_err(|source| Error::GetTicket {
                            ticketbook_type,
                            source,
                        })?
                        .data,
                );

                let finalized_message = match self.auth_version {
                    AuthenticatorVersion::V1 => return Err(Error::UnsupportedAuthenticatorVersion),
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
                    &self.ip_addr,
                    &finalized_message
                );

                let response = self.send_and_wait_for_response(&finalized_message).await?;
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
            &self.ip_addr,
            &registered_data
        );

        let gateway_data = GatewayData {
            public_key: registered_data.pub_key().inner().into(),
            endpoint: SocketAddr::from_str(&format!(
                "{}:{}",
                self.ip_addr,
                registered_data.wg_port()
            ))
            .map_err(Error::FailedToParseEntryGatewaySocketAddr)?,
            private_ipv4: registered_data.private_ips().ipv4,
            private_ipv6: registered_data.private_ips().ipv6,
        };

        Ok(gateway_data)
    }
}
