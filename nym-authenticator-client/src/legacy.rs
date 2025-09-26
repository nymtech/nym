// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use tracing::{debug, error};

use crate::mixnet_listener::{MixnetMessageBroadcastReceiver, MixnetMessageInputSender};
use crate::{helpers, ClientMessage, Error, Result};
use nym_authenticator_requests::{
    client_message::QueryMessageImpl, response::AuthenticatorResponse, traits::Id, v2, v3, v4, v5,
    AuthenticatorVersion,
};
use nym_credentials_interface::CredentialSpendingData;
use nym_crypto::asymmetric::x25519::{KeyPair, PublicKey};
use nym_node_requests::api::v1::gateway::client_interfaces::wireguard::models::PeerPublicKey;
use nym_sdk::mixnet::{IncludedSurbs, Recipient};
use nym_service_provider_requests_common::{Protocol, ServiceProviderTypeExt};

impl crate::AuthenticatorClient {
    pub fn into_legacy_and_keypair(self) -> (LegacyAuthenticatorClient, KeyPair) {
        (
            LegacyAuthenticatorClient {
                public_key: *self.keypair.public_key(),
                mixnet_listener: self.mixnet_listener,
                mixnet_sender: self.mixnet_sender,
                our_nym_address: self.our_nym_address,
                auth_recipient: self.auth_recipient,
                auth_version: self.auth_version,
            },
            self.keypair,
        )
    }
}

// This is the legacy Authenticator that has to be used to handle bandwidth top up for legacy gateaways
pub struct LegacyAuthenticatorClient {
    public_key: PublicKey,
    mixnet_listener: MixnetMessageBroadcastReceiver,
    mixnet_sender: MixnetMessageInputSender,
    our_nym_address: Recipient,
    pub auth_recipient: Recipient,
    auth_version: AuthenticatorVersion,
}

impl LegacyAuthenticatorClient {
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

    pub async fn query_bandwidth(&mut self) -> Result<Option<i64>> {
        let query_message = match self.auth_version {
            AuthenticatorVersion::V1 => return Err(Error::UnsupportedAuthenticatorVersion),
            AuthenticatorVersion::V2 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V2,
            })),
            AuthenticatorVersion::V3 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V3,
            })),
            AuthenticatorVersion::V4 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V4,
            })),
            AuthenticatorVersion::V5 => ClientMessage::Query(Box::new(QueryMessageImpl {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                version: AuthenticatorVersion::V5,
            })),
            AuthenticatorVersion::UNKNOWN => return Err(Error::UnsupportedAuthenticatorVersion),
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
        let top_up_message = match self.auth_version {
            AuthenticatorVersion::V3 => ClientMessage::TopUp(Box::new(v3::topup::TopUpMessage {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                credential,
            })),
            // NOTE: looks like a bug here using v3. But we're leaving it as is since it's working
            // and V4 is deprecated in favour of V5
            AuthenticatorVersion::V4 => ClientMessage::TopUp(Box::new(v4::topup::TopUpMessage {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
                credential,
            })),
            AuthenticatorVersion::V5 => ClientMessage::TopUp(Box::new(v5::topup::TopUpMessage {
                pub_key: PeerPublicKey::new(self.public_key.to_bytes().into()),
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
