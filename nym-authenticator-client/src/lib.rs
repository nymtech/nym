// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_authenticator_requests::client_message::QueryMessageImpl;
use nym_bandwidth_controller::{BandwidthTicketProvider, DEFAULT_TICKETS_TO_SPEND};
use nym_crypto::asymmetric::x25519::KeyPair;
use nym_registration_common::GatewayData;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, trace};

use crate::mixnet_listener::{MixnetMessageBroadcastReceiver, MixnetMessageInputSender};
use nym_authenticator_requests::{
    AuthenticatorVersion, client_message::ClientMessage, response::AuthenticatorResponse,
    traits::Id, v2, v3, v4, v5, v6,
};
use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_sdk::mixnet::{IncludedSurbs, Recipient, ReconstructedMessage};
use nym_service_provider_requests_common::{Protocol, ServiceProviderTypeExt};
use nym_wireguard_types::PeerPublicKey;

mod error;
mod helpers;
mod mixnet_listener;

pub use crate::error::{Error, Result};
pub use crate::mixnet_listener::{AuthClientMixnetListener, AuthClientMixnetListenerHandle};

pub struct AuthenticatorClient {
    mixnet_listener: MixnetMessageBroadcastReceiver,
    mixnet_sender: MixnetMessageInputSender,
    our_nym_address: Recipient,
    pub auth_recipient: Recipient,
    auth_version: AuthenticatorVersion,

    keypair: Arc<KeyPair>,
    ip_addr: IpAddr,
}

impl AuthenticatorClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mixnet_listener: MixnetMessageBroadcastReceiver,
        mixnet_sender: MixnetMessageInputSender,
        our_nym_address: Recipient,
        auth_recipient: Recipient,
        auth_version: AuthenticatorVersion,
        keypair: Arc<KeyPair>,
        ip_addr: IpAddr,
    ) -> Self {
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

    fn peer_public_key(&self) -> PeerPublicKey {
        PeerPublicKey::from(self.keypair.public_key().inner())
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
        let serialised = message.bytes(self.our_nym_address)?;
        let data = serialised.bytes;
        let request_id = serialised.request_id;

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

    fn handle_response(
        &self,
        msg: Arc<ReconstructedMessage>,
        request_id: u64,
    ) -> Option<Result<AuthenticatorResponse>> {
        let Some(header) = msg.message.first_chunk::<2>() else {
            debug!(
                "received too short message that couldn't have been from the authenticator while waiting for connect response"
            );
            return None;
        };

        let Ok(protocol) = Protocol::try_from(header) else {
            debug!(
                "received a message not meant to any service provider while waiting for connect response"
            );
            return None;
        };

        if !protocol.service_provider_type.is_authenticator() {
            debug!("Received non-authenticator message while waiting for connect response");
            return None;
        }
        // Confirm that the version is correct
        let version = AuthenticatorVersion::from(protocol.version);

        // Then we deserialize the message
        debug!(
            "AuthClient: got message while waiting for connect response with version {version:?}"
        );
        let ret: Result<AuthenticatorResponse> = match version {
            AuthenticatorVersion::V1 | AuthenticatorVersion::UNKNOWN => {
                return Some(Err(Error::UnsupportedAuthenticatorVersion));
            }
            AuthenticatorVersion::V2 => {
                v2::response::AuthenticatorResponse::from_reconstructed_message(&msg)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            AuthenticatorVersion::V3 => {
                v3::response::AuthenticatorResponse::from_reconstructed_message(&msg)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            AuthenticatorVersion::V4 => {
                v4::response::AuthenticatorResponse::from_reconstructed_message(&msg)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            AuthenticatorVersion::V5 => {
                v5::response::AuthenticatorResponse::from_reconstructed_message(&msg)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            AuthenticatorVersion::V6 => {
                v6::response::AuthenticatorResponse::from_reconstructed_message(&msg)
                    .map(Into::into)
                    .map_err(Into::into)
            }
        };
        let Ok(response) = ret else {
            // This is ok, it's likely just one of our self-pings
            debug!("Failed to deserialize reconstructed message");
            return None;
        };

        if response.id() == request_id {
            debug!("Got response with matching id");
            return Some(Ok(response));
        }
        None
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
                        match self.handle_response(msg, request_id) {
                            None => continue,
                            Some(res) => return res,
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
        let pub_key = self.peer_public_key();

        let init_message = match self.auth_version {
            AuthenticatorVersion::V1 | AuthenticatorVersion::UNKNOWN => {
                return Err(Error::UnsupportedAuthenticatorVersion);
            }
            AuthenticatorVersion::V2 => {
                ClientMessage::Initial(Box::new(v2::registration::InitMessage { pub_key }))
            }
            AuthenticatorVersion::V3 => {
                ClientMessage::Initial(Box::new(v3::registration::InitMessage { pub_key }))
            }
            AuthenticatorVersion::V4 => {
                ClientMessage::Initial(Box::new(v4::registration::InitMessage { pub_key }))
            }
            AuthenticatorVersion::V5 => {
                ClientMessage::Initial(Box::new(v5::registration::InitMessage { pub_key }))
            }
            AuthenticatorVersion::V6 => {
                ClientMessage::Initial(Box::new(v6::registration::InitMessage { pub_key }))
            }
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
                    &self.ip_addr, &pending_registration_response
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
                let credential = credential
                .map(TryInto::try_into)
                    .transpose()
                    .inspect_err(|err| error!("failed to convert {ticketbook_type} ticket to a valid BandwidthClaim: {err}"))
                    .map_err(|_| Error::InternalError)?;

                let finalized_message = pending_registration_response
                    .finalise_registration(self.keypair.private_key(), credential);
                let client_message = ClientMessage::Final(finalized_message);

                trace!("sending final msg to {}: {client_message:?}", &self.ip_addr);

                let response = self.send_and_wait_for_response(&client_message).await?;
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
            &self.ip_addr, &registered_data
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

    pub async fn query_bandwidth(&mut self) -> Result<Option<i64>> {
        let pub_key = self.peer_public_key();
        let version = self.auth_version;

        let query_message = match self.auth_version {
            AuthenticatorVersion::V1 | AuthenticatorVersion::UNKNOWN => {
                return Err(Error::UnsupportedAuthenticatorVersion);
            }
            AuthenticatorVersion::V2
            | AuthenticatorVersion::V3
            | AuthenticatorVersion::V4
            | AuthenticatorVersion::V5
            | AuthenticatorVersion::V6 => {
                ClientMessage::Query(Box::new(QueryMessageImpl { pub_key, version }))
            }
        };
        let response = self.send_and_wait_for_response(&query_message).await?;

        let available_bandwidth = match response {
            AuthenticatorResponse::RemainingBandwidth(remaining_bandwidth_response) => {
                if let Some(available_bandwidth) =
                    remaining_bandwidth_response.available_bandwidth()
                {
                    available_bandwidth
                } else {
                    return Ok(None);
                }
            }
            _ => return Err(Error::InvalidGatewayAuthResponse),
        };

        let remaining_pretty = if available_bandwidth > 1024 * 1024 {
            format!("{:.2} MB", available_bandwidth as f64 / 1024.0 / 1024.0)
        } else {
            format!("{} KB", available_bandwidth / 1024)
        };
        tracing::debug!(
            "Remaining wireguard bandwidth with gateway {} for today: {}",
            self.auth_recipient.gateway(),
            remaining_pretty
        );
        if available_bandwidth < 1024 * 1024 {
            tracing::warn!(
                "Remaining bandwidth is under 1 MB. The wireguard mode will get suspended after that until tomorrow, UTC time. The client might shutdown with timeout soon"
            );
        }
        Ok(Some(available_bandwidth))
    }

    pub async fn top_up(&mut self, credential: CredentialSpendingData) -> Result<i64> {
        let pub_key = self.peer_public_key();

        let top_up_message = match self.auth_version {
            AuthenticatorVersion::V3 => ClientMessage::TopUp(Box::new(v3::topup::TopUpMessage {
                pub_key,
                credential,
            })),
            // NOTE: looks like a bug here using v3. But we're leaving it as is since it's working
            // and V4 is deprecated in favour of V5
            AuthenticatorVersion::V4 => ClientMessage::TopUp(Box::new(v4::topup::TopUpMessage {
                pub_key,
                credential,
            })),
            AuthenticatorVersion::V5 => ClientMessage::TopUp(Box::new(v5::topup::TopUpMessage {
                pub_key,
                credential,
            })),
            AuthenticatorVersion::V6 => ClientMessage::TopUp(Box::new(v6::topup::TopUpMessage {
                pub_key,
                credential,
            })),
            AuthenticatorVersion::V1 | AuthenticatorVersion::V2 | AuthenticatorVersion::UNKNOWN => {
                return Err(Error::UnsupportedAuthenticatorVersion);
            }
        };
        let response = self.send_and_wait_for_response(&top_up_message).await?;

        let remaining_bandwidth = match response {
            AuthenticatorResponse::TopUpBandwidth(top_up_bandwidth_response) => {
                top_up_bandwidth_response.available_bandwidth()
            }
            _ => return Err(Error::InvalidGatewayAuthResponse),
        };

        Ok(remaining_bandwidth)
    }
}
