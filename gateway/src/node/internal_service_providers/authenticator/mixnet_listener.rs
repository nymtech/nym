// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::internal_service_providers::authenticator::{
    config::Config, error::AuthenticatorError, seen_credential_cache::SeenCredentialCache,
};
use crate::node::wireguard::{PeerManager, PeerRegistrator};
use futures::StreamExt;
use nym_authenticator_requests::traits::UpgradeModeMessage;
use nym_authenticator_requests::{
    request::AuthenticatorRequest,
    traits::{FinalMessage, InitMessage, QueryBandwidthMessage, TopUpMessage},
    v1, v2, v3, v4, v5, v6, AuthenticatorVersion, CURRENT_VERSION,
};
use nym_credential_verification::upgrade_mode::UpgradeModeDetails;
use nym_sdk::mixnet::{
    AnonymousSenderTag, InputMessage, MixnetMessageSender, Recipient, TransmissionLane,
};
use nym_service_provider_requests_common::{Protocol, ServiceProviderTypeExt};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::ShutdownToken;
use nym_wireguard::WireguardGatewayData;
use nym_wireguard_types::PeerPublicKey;
use std::cmp::max;
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;

type AuthenticatorHandleResult = Result<(Vec<u8>, Option<Recipient>), AuthenticatorError>;
const DEFAULT_CREDENTIAL_TIMEOUT_CHECK: Duration = Duration::from_secs(60); // 1 minute

// we need to be above MINIMUM_REMAINING_BANDWIDTH (500MB) plus we also have to trick the client
// its depletion is low enough to not require sending new tickets
const DEFAULT_WG_CLIENT_BANDWIDTH_THRESHOLD: i64 = 1024 * 1024 * 1024;

pub(crate) struct MixnetListener {
    // The configuration for the mixnet listener
    pub(crate) _config: Config,

    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    pub(crate) peer_manager: PeerManager,

    pub(crate) upgrade_mode: UpgradeModeDetails,

    pub(crate) peer_registrator: PeerRegistrator,

    pub(crate) timeout_check_interval: IntervalStream,

    pub(crate) seen_credential_cache: SeenCredentialCache,
}

impl MixnetListener {
    pub fn new(
        config: Config,
        wireguard_gateway_data: WireguardGatewayData,
        mixnet_client: nym_sdk::mixnet::MixnetClient,
        peer_registrator: PeerRegistrator,
        upgrade_mode: UpgradeModeDetails,
    ) -> Self {
        let timeout_check_interval =
            IntervalStream::new(tokio::time::interval(DEFAULT_CREDENTIAL_TIMEOUT_CHECK));
        MixnetListener {
            _config: config,
            mixnet_client,
            peer_manager: PeerManager::new(wireguard_gateway_data),
            upgrade_mode,
            peer_registrator,
            timeout_check_interval,
            seen_credential_cache: SeenCredentialCache::new(),
        }
    }

    fn upgrade_mode_enabled(&self) -> bool {
        self.upgrade_mode.enabled()
    }

    async fn upgrade_mode_bandwidth(&self, peer: PeerPublicKey) -> Result<i64, AuthenticatorError> {
        // if we're undergoing upgrade mode, we don't meter bandwidth,
        // we simply return MAX of clients current bandwidth and minimum bandwidth before default
        // client would have attempted to send new ticket (hopefully)
        // the latter is to support older clients that will ignore `upgrade_mode` field in the response
        // as they're not aware of its existence
        let available_bandwidth = self.peer_manager.query_bandwidth(peer).await?;
        Ok(max(
            DEFAULT_WG_CLIENT_BANDWIDTH_THRESHOLD,
            available_bandwidth,
        ))
    }

    async fn on_initial_request(
        &mut self,
        init_message: Box<dyn InitMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let response = self
            .peer_registrator
            .on_initial_authenticator_request(init_message, protocol, request_id, reply_to)
            .await?;

        Ok((response.bytes, response.reply_to))
    }

    async fn on_final_request(
        &mut self,
        final_message: Box<dyn FinalMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let response = self
            .peer_registrator
            .on_final_authenticator_request(final_message, protocol, request_id, reply_to)
            .await?;

        Ok((response.bytes, response.reply_to))
    }

    async fn on_query_bandwidth_request(
        &mut self,
        msg: Box<dyn QueryBandwidthMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let available_bandwidth = if self.upgrade_mode_enabled() {
            self.upgrade_mode_bandwidth(msg.pub_key()).await?
        } else {
            self.peer_manager.query_bandwidth(msg.pub_key()).await?
        };

        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V1 => {
                v1::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v1::registration::RemainingBandwidthData {
                        available_bandwidth: available_bandwidth as u64,
                        suspended: false,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(AuthenticatorError::response_serialisation)?
            }
            AuthenticatorVersion::V2 => {
                v2::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v2::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(AuthenticatorError::response_serialisation)?
            }
            AuthenticatorVersion::V3 => {
                v3::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v3::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(AuthenticatorError::response_serialisation)?
            }
            AuthenticatorVersion::V4 => {
                v4::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v4::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                    request_id,
                )
                .to_bytes()
                .map_err(AuthenticatorError::response_serialisation)?
            }
            AuthenticatorVersion::V5 => {
                v5::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v5::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    request_id,
                )
                .to_bytes()
                .map_err(AuthenticatorError::response_serialisation)?
            }
            AuthenticatorVersion::V6 => {
                v6::response::AuthenticatorResponse::new_remaining_bandwidth(
                    Some(v6::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    request_id,
                    self.upgrade_mode_enabled(),
                )
                .to_bytes()
                .map_err(AuthenticatorError::response_serialisation)?
            }
            AuthenticatorVersion::UNKNOWN => return Err(AuthenticatorError::UnknownVersion),
        };
        Ok((bytes, reply_to))
    }

    // if we received a topup request, don't do anything with the upgrade mode
    async fn on_topup_bandwidth_request(
        &mut self,
        msg: Box<dyn TopUpMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
        reply_to: Option<Recipient>,
    ) -> AuthenticatorHandleResult {
        let available_bandwidth = if self.received_retry(msg.as_ref()) {
            // don't process the credential and just return the current bandwidth
            self.peer_manager.query_bandwidth(msg.pub_key()).await?
        } else {
            let mut verifier = self
                .peer_manager
                .query_verifier_by_key(msg.pub_key(), msg.credential())
                .await?;
            let available_bandwidth = verifier.verify().await?;
            self.seen_credential_cache
                .insert_credential(msg.credential(), msg.pub_key());
            available_bandwidth
        };

        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::V6 => v6::response::AuthenticatorResponse::new_topup_bandwidth(
                v6::registration::RemainingBandwidthData {
                    available_bandwidth,
                },
                request_id,
                self.upgrade_mode_enabled(),
            )
            .to_bytes()
            .map_err(AuthenticatorError::response_serialisation)?,
            AuthenticatorVersion::V5 => v5::response::AuthenticatorResponse::new_topup_bandwidth(
                v5::registration::RemainingBandwidthData {
                    available_bandwidth,
                },
                request_id,
            )
            .to_bytes()
            .map_err(AuthenticatorError::response_serialisation)?,
            AuthenticatorVersion::V4 => v4::response::AuthenticatorResponse::new_topup_bandwidth(
                v4::registration::RemainingBandwidthData {
                    available_bandwidth,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(AuthenticatorError::response_serialisation)?,
            AuthenticatorVersion::V3 => v3::response::AuthenticatorResponse::new_topup_bandwidth(
                v3::registration::RemainingBandwidthData {
                    available_bandwidth,
                },
                reply_to.ok_or(AuthenticatorError::MissingReplyToForOldClient)?,
                request_id,
            )
            .to_bytes()
            .map_err(AuthenticatorError::response_serialisation)?,
            AuthenticatorVersion::V1 | AuthenticatorVersion::V2 | AuthenticatorVersion::UNKNOWN => {
                return Err(AuthenticatorError::UnknownVersion)
            }
        };

        Ok((bytes, reply_to))
    }

    async fn on_upgrade_mode_check(
        &mut self,
        msg: Box<dyn UpgradeModeMessage + Send + Sync + 'static>,
        protocol: Protocol,
        request_id: u64,
    ) -> AuthenticatorHandleResult {
        // if upgrade mode is already enabled, we don't need to perform any additional checks
        if !self.upgrade_mode_enabled() {
            // currently upgrade mode JWT is the only type of emergency credentials supported
            if let Some(upgrade_mode_jwt) = msg.upgrade_mode_global_attestation_jwt() {
                self.upgrade_mode
                    .try_enable_via_received_jwt(upgrade_mode_jwt)
                    .await?;
            }
        }

        let bytes = match AuthenticatorVersion::from(protocol) {
            AuthenticatorVersion::UNKNOWN
            | AuthenticatorVersion::V1
            | AuthenticatorVersion::V2
            | AuthenticatorVersion::V3
            | AuthenticatorVersion::V4
            | AuthenticatorVersion::V5 => {
                // pre v6 this message hasn't existed
                return Err(AuthenticatorError::UnknownVersion);
            }
            AuthenticatorVersion::V6 => {
                v6::response::AuthenticatorResponse::new_upgrade_mode_check(
                    request_id,
                    self.upgrade_mode_enabled(),
                )
                .to_bytes()
                .map_err(AuthenticatorError::response_serialisation)?
            }
        };

        // no need to support reply_to, as this is never set in v6 and older versions do not include this message
        Ok((bytes, None))
    }

    fn received_retry(&self, msg: &(dyn TopUpMessage + Send + Sync + 'static)) -> bool {
        if let Some(peer_pub_key) = self
            .seen_credential_cache
            .get_peer_pub_key(&msg.credential())
        {
            // check if the same peer sent the same credential twice, probably because of a retry
            peer_pub_key == msg.pub_key()
        } else {
            false
        }
    }

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> AuthenticatorHandleResult {
        tracing::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

        let request = deserialize_request(&reconstructed)?;

        match request {
            AuthenticatorRequest::Initial {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_initial_request(msg, protocol, request_id, reply_to)
                    .await
            }
            AuthenticatorRequest::Final {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_final_request(msg, protocol, request_id, reply_to)
                    .await
            }
            AuthenticatorRequest::QueryBandwidth {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_query_bandwidth_request(msg, protocol, request_id, reply_to)
                    .await
            }
            AuthenticatorRequest::TopUpBandwidth {
                msg,
                reply_to,
                request_id,
                protocol,
            } => {
                self.on_topup_bandwidth_request(msg, protocol, request_id, reply_to)
                    .await
            }
            AuthenticatorRequest::CheckUpgradeMode {
                msg,
                protocol,
                request_id,
            } => self.on_upgrade_mode_check(msg, protocol, request_id).await,
        }
    }

    // When an incoming mixnet message triggers a response that we send back.
    async fn handle_response(
        &mut self,
        response: Vec<u8>,
        recipient: Option<Recipient>,
        sender_tag: Option<AnonymousSenderTag>,
    ) -> Result<(), AuthenticatorError> {
        let input_message = create_input_message(recipient, sender_tag, response)?;
        self.mixnet_client.send(input_message).await.map_err(|err| {
            AuthenticatorError::FailedToSendPacketToMixnet {
                source: Box::new(err),
            }
        })
    }

    pub(crate) async fn run(
        mut self,
        shutdown_token: ShutdownToken,
    ) -> Result<(), AuthenticatorError> {
        tracing::info!("Using authenticator version {CURRENT_VERSION}");

        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    tracing::debug!("Authenticator [main loop]: received shutdown");
                    break;
                },
                _ = self.timeout_check_interval.next() => {
                    self.seen_credential_cache.remove_stale();
                }
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        let sender_tag = msg.sender_tag;
                        match self.on_reconstructed_message(msg).await {
                            Ok((response, recipient)) => {
                                if let Err(err) = self.handle_response(response, recipient, sender_tag).await {
                                    tracing::error!("Mixnet listener failed to handle response: {err}");
                                }
                            }
                            Err(err) => {
                                tracing::error!("Error handling reconstructed mixnet message: {err}");
                            }

                        };
                    } else {
                        tracing::trace!("Authenticator [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        tracing::debug!("Authenticator: stopping");
        Ok(())
    }
}

fn deserialize_request(
    reconstructed: &ReconstructedMessage,
) -> Result<AuthenticatorRequest, AuthenticatorError> {
    let header = reconstructed
        .message
        .first_chunk::<2>()
        .ok_or(AuthenticatorError::ShortPacket)?;

    let version = header[0];

    // special case for v1 request where service provider information hasn't been exposed in the header
    if version == v1::VERSION {
        return v1::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
            .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err })
            .map(Into::into);
    }

    let protocol = Protocol::try_from(header)?;

    if !protocol.service_provider_type.is_authenticator() {
        return Err(AuthenticatorError::InvalidPacketType(
            protocol.service_provider_type as u8,
        ));
    }

    let version = AuthenticatorVersion::from(protocol.version);

    // Check version of the request and convert to the latest version if necessary
    match version {
        AuthenticatorVersion::V1 => {
            // this branch should be unreachable as v1 has already been handled independently
            Err(AuthenticatorError::UnknownVersion)
        }
        AuthenticatorVersion::V2 => {
            v2::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err })
                .map(Into::<v3::request::AuthenticatorRequest>::into)
                .map(Into::into)
        }
        AuthenticatorVersion::V3 => {
            v3::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err })
                .map(Into::into)
        }
        AuthenticatorVersion::V4 => {
            v4::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err })
                .map(Into::into)
        }
        AuthenticatorVersion::V5 => {
            v5::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err })
                .map(Into::into)
        }
        AuthenticatorVersion::V6 => {
            v6::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
                .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err })
                .map(Into::into)
        }
        AuthenticatorVersion::UNKNOWN => {
            tracing::info!(
                "Received packet with invalid version: v{}",
                protocol.version
            );
            Err(AuthenticatorError::InvalidPacketVersion(protocol.version))
        }
    }
}

fn create_input_message(
    nym_address: Option<Recipient>,
    reply_to_tag: Option<AnonymousSenderTag>,
    response_packet: Vec<u8>,
) -> Result<InputMessage, AuthenticatorError> {
    let lane = TransmissionLane::General;
    let packet_type = None;
    if let Some(reply_to_tag) = reply_to_tag {
        tracing::debug!("Creating message using SURB");
        Ok(InputMessage::new_reply(
            reply_to_tag,
            response_packet,
            lane,
            packet_type,
        ))
    } else if let Some(nym_address) = nym_address {
        tracing::debug!("Creating message using nym_address");
        Ok(InputMessage::new_regular(
            nym_address,
            response_packet,
            lane,
            packet_type,
        ))
    } else {
        tracing::error!("No nym-address or sender tag provided");
        Err(AuthenticatorError::MissingReplyToForOldClient)
    }
}
