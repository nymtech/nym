// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::error::AuthenticatorError;
use futures::StreamExt;
use ipnetwork::IpNetwork;
use nym_authenticator_requests::v1::{
    self,
    request::{AuthenticatorRequest, AuthenticatorRequestData},
    response::AuthenticatorResponse,
};
use nym_sdk::mixnet::{InputMessage, MixnetMessageSender, Recipient, TransmissionLane};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use nym_wireguard::{peer_controller::PeerControlResponse, WireguardGatewayData};
use nym_wireguard_types::{
    registration::{PendingRegistrations, PrivateIPs, RegistrationData, RegistredData},
    GatewayClient, InitMessage, PeerPublicKey,
};
use rand::{prelude::IteratorRandom, thread_rng};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_stream::wrappers::IntervalStream;

use crate::{config::Config, error::*};

type AuthenticatorHandleResult = Result<AuthenticatorResponse>;
const DEFAULT_REGISTRATION_TIMEOUT_CHECK: Duration = Duration::from_secs(60); // 1 minute

pub(crate) struct MixnetListener {
    // The configuration for the mixnet listener
    pub(crate) config: Config,

    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    // The task handle for the main loop
    pub(crate) task_handle: TaskHandle,

    // Registrations awaiting confirmation
    pub(crate) registration_in_progres: Arc<PendingRegistrations>,

    pub(crate) wireguard_gateway_data: WireguardGatewayData,

    pub(crate) response_rx: UnboundedReceiver<PeerControlResponse>,

    pub(crate) free_private_network_ips: Arc<PrivateIPs>,

    pub(crate) timeout_check_interval: IntervalStream,
}

impl MixnetListener {
    pub fn new(
        config: Config,
        private_ip_network: IpNetwork,
        wireguard_gateway_data: WireguardGatewayData,
        response_rx: UnboundedReceiver<PeerControlResponse>,
        mixnet_client: nym_sdk::mixnet::MixnetClient,
        task_handle: TaskHandle,
    ) -> Self {
        let timeout_check_interval =
            IntervalStream::new(tokio::time::interval(DEFAULT_REGISTRATION_TIMEOUT_CHECK));
        MixnetListener {
            config,
            mixnet_client,
            task_handle,
            registration_in_progres: Default::default(),
            wireguard_gateway_data,
            response_rx,
            free_private_network_ips: Arc::new(
                private_ip_network.iter().map(|ip| (ip, None)).collect(),
            ),
            timeout_check_interval,
        }
    }

    fn remove_stale_registrations(&self) -> Result<()> {
        for reg in self.registration_in_progres.iter().map(|reg| reg.clone()) {
            let mut ip = self
                .free_private_network_ips
                .get_mut(&reg.gateway_data.private_ip)
                .ok_or(AuthenticatorError::InternalDataCorruption(format!(
                    "IP {} should be present",
                    reg.gateway_data.private_ip
                )))?;

            let timestamp = ip.ok_or(AuthenticatorError::InternalDataCorruption(format!(
                "timestamp should be set for IP {}",
                ip.key()
            )))?;
            let duration = SystemTime::now().duration_since(timestamp).map_err(|_| {
                AuthenticatorError::InternalDataCorruption(
                    "set timestamp shouldn't have been set in the future".to_string(),
                )
            })?;
            if duration > DEFAULT_REGISTRATION_TIMEOUT_CHECK {
                *ip = None;
                self.registration_in_progres
                    .remove(&reg.gateway_data.pub_key());
                log::debug!(
                    "Removed stale registration of {}",
                    reg.gateway_data.pub_key()
                );
            }
        }
        Ok(())
    }

    async fn on_initial_request(
        &mut self,
        init_message: InitMessage,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let remote_public = init_message.pub_key();
        let nonce: u64 = fastrand::u64(..);
        if let Some(registration_data) = self.registration_in_progres.get(&remote_public) {
            return Ok(AuthenticatorResponse::new_pending_registration_success(
                registration_data.value().clone(),
                request_id,
                reply_to,
            ));
        }
        if let Some(gateway_client) = self
            .wireguard_gateway_data
            .client_registry()
            .get(&remote_public)
        {
            return Ok(AuthenticatorResponse::new_registered(
                RegistredData {
                    pub_key: PeerPublicKey::new(
                        self.wireguard_gateway_data
                            .keypair()
                            .public_key()
                            .to_bytes()
                            .into(),
                    ),
                    private_ip: gateway_client.private_ip,
                    wg_port: self.config.authenticator.announced_port,
                },
                reply_to,
                request_id,
            ));
        }
        let mut private_ip_ref = self
            .free_private_network_ips
            .iter_mut()
            .filter(|r| r.is_none())
            .choose(&mut thread_rng())
            .ok_or(AuthenticatorError::NoFreeIp)?;
        // mark it as used, even though it's not final
        *private_ip_ref = Some(SystemTime::now());
        let gateway_data = GatewayClient::new(
            self.wireguard_gateway_data.keypair().private_key(),
            remote_public.inner(),
            *private_ip_ref.key(),
            nonce,
        );
        let registration_data = RegistrationData {
            nonce,
            gateway_data,
            wg_port: self.config.authenticator.announced_port,
        };
        self.registration_in_progres
            .insert(remote_public, registration_data.clone());

        Ok(AuthenticatorResponse::new_pending_registration_success(
            registration_data,
            request_id,
            reply_to,
        ))
    }

    async fn on_final_request(
        &mut self,
        gateway_client: GatewayClient,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let registration_data = self
            .registration_in_progres
            .get(&gateway_client.pub_key())
            .ok_or(AuthenticatorError::RegistrationNotInProgress)?
            .value()
            .clone();

        if gateway_client
            .verify(
                self.wireguard_gateway_data.keypair().private_key(),
                registration_data.nonce,
            )
            .is_ok()
        {
            self.wireguard_gateway_data
                .add_peer(&gateway_client)
                .map_err(|err| {
                    AuthenticatorError::InternalError(format!("could not add peer: {:?}", err))
                })?;

            let PeerControlResponse::AddPeer { success } =
                self.response_rx
                    .recv()
                    .await
                    .ok_or(AuthenticatorError::InternalError(
                        "no response for add peer".to_string(),
                    ))?
            else {
                return Err(AuthenticatorError::InternalError(
                    "unexpected response type".to_string(),
                ));
            };
            if !success {
                return Err(AuthenticatorError::InternalError(
                    "adding peer could not be performed".to_string(),
                ));
            }
            self.registration_in_progres
                .remove(&gateway_client.pub_key());

            Ok(AuthenticatorResponse::new_registered(
                RegistredData {
                    pub_key: registration_data.gateway_data.pub_key,
                    private_ip: registration_data.gateway_data.private_ip,
                    wg_port: registration_data.wg_port,
                },
                reply_to,
                request_id,
            ))
        } else {
            Err(AuthenticatorError::MacVerificationFailure)
        }
    }

    async fn on_query_bandwidth_request(
        &mut self,
        peer_public_key: PeerPublicKey,
        request_id: u64,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        self.wireguard_gateway_data
            .query_bandwidth(peer_public_key)
            .map_err(|err| {
                AuthenticatorError::InternalError(format!(
                    "could not query peer bandwidth: {:?}",
                    err
                ))
            })?;
        let PeerControlResponse::QueryBandwidth { bandwidth_data } = self
            .response_rx
            .recv()
            .await
            .ok_or(AuthenticatorError::InternalError(
                "no response for query".to_string(),
            ))?
        else {
            return Err(AuthenticatorError::InternalError(
                "unexpected response type".to_string(),
            ));
        };
        Ok(AuthenticatorResponse::new_remaining_bandwidth(
            bandwidth_data,
            reply_to,
            request_id,
        ))
    }

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> AuthenticatorHandleResult {
        log::debug!(
            "Received message with sender_tag: {:?}",
            reconstructed.sender_tag
        );

        let request = match deserialize_request(&reconstructed) {
            Err(AuthenticatorError::InvalidPacketVersion(version)) => {
                return self.on_version_mismatch(version, &reconstructed);
            }
            req => req,
        }?;

        match request.data {
            AuthenticatorRequestData::Initial(init_msg) => {
                self.on_initial_request(init_msg, request.request_id, request.reply_to)
                    .await
            }
            AuthenticatorRequestData::Final(client) => {
                self.on_final_request(client, request.request_id, request.reply_to)
                    .await
            }
            AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                self.on_query_bandwidth_request(
                    peer_public_key,
                    request.request_id,
                    request.reply_to,
                )
                .await
            }
        }
    }

    fn on_version_mismatch(
        &self,
        version: u8,
        _reconstructed: &ReconstructedMessage,
    ) -> AuthenticatorHandleResult {
        // If it's possible to parse, do so and return back a response, otherwise just drop
        Err(AuthenticatorError::InvalidPacketVersion(version))
    }

    // When an incoming mixnet message triggers a response that we send back.
    async fn handle_response(&self, response: AuthenticatorResponse) -> Result<()> {
        let recipient = response.recipient();

        let response_packet = response.to_bytes().map_err(|err| {
            log::error!("Failed to serialize response packet");
            AuthenticatorError::FailedToSerializeResponsePacket { source: err }
        })?;

        let input_message =
            InputMessage::new_regular(recipient, response_packet, TransmissionLane::General, None);
        self.mixnet_client
            .send(input_message)
            .await
            .map_err(|err| AuthenticatorError::FailedToSendPacketToMixnet { source: err })
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        let mut task_client = self.task_handle.fork("main_loop");

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("Authenticator [main loop]: received shutdown");
                },
                _ = self.timeout_check_interval.next() => {
                    if let Err(e) = self.remove_stale_registrations() {
                        log::error!("Could not clear stale registrations. The registration process might get jammed soon - {:?}", e);
                    }
                }
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok(response) => {
                                if let Err(err) = self.handle_response(response).await {
                                    log::error!("Mixnet listener failed to handle response: {err}");
                                }
                            }
                            Err(err) => {
                                log::error!("Error handling reconstructed mixnet message: {err}");
                            }

                        };
                    } else {
                        log::trace!("Authenticator [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        log::debug!("Authenticator: stopping");
        Ok(())
    }
}

fn deserialize_request(reconstructed: &ReconstructedMessage) -> Result<AuthenticatorRequest> {
    let request_version = *reconstructed
        .message
        .first()
        .ok_or(AuthenticatorError::EmptyPacket)?;

    // Check version of the request and convert to the latest version if necessary
    match request_version {
        1 => v1::request::AuthenticatorRequest::from_reconstructed_message(reconstructed)
            .map_err(|err| AuthenticatorError::FailedToDeserializeTaggedPacket { source: err }),
        _ => {
            log::info!("Received packet with invalid version: v{request_version}");
            Err(AuthenticatorError::InvalidPacketVersion(request_version))
        }
    }
}
