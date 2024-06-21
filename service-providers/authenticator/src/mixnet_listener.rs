// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::error::AuthenticatorError;
use futures::StreamExt;
use nym_authenticator_requests::v1::{
    self,
    request::{AuthenticatorRequest, AuthenticatorRequestData},
    response::AuthenticatorResponse,
};
use nym_sdk::mixnet::{InputMessage, MixnetMessageSender, Recipient, TransmissionLane};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use nym_wireguard::WireguardGatewayData;
use nym_wireguard_types::{
    registration::{PendingRegistrations, PrivateIPs, RegistrationData},
    GatewayClient, InitMessage, PeerPublicKey,
};
use rand::{prelude::IteratorRandom, thread_rng};

use crate::{config::Config, error::*};

type AuthenticatorHandleResult = Result<AuthenticatorResponse>;

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

    pub(crate) free_private_network_ips: Arc<PrivateIPs>,
}

impl MixnetListener {
    pub fn new(
        config: Config,
        wireguard_gateway_data: WireguardGatewayData,
        mixnet_client: nym_sdk::mixnet::MixnetClient,
        task_handle: TaskHandle,
    ) -> Self {
        MixnetListener {
            config,
            mixnet_client,
            task_handle,
            registration_in_progres: Default::default(),
            wireguard_gateway_data,
            free_private_network_ips: Default::default(),
        }
    }

    fn remove_from_registry(
        &self,
        remote_public: &PeerPublicKey,
        gateway_client: &GatewayClient,
    ) -> Result<()> {
        self.wireguard_gateway_data
            .remove_peer(gateway_client)
            .map_err(|err| {
                AuthenticatorError::InternalError(format!("could not remove peer: {:?}", err))
            })?;
        self.wireguard_gateway_data
            .client_registry()
            .remove(remote_public);
        Ok(())
    }

    fn on_initial_request(
        &mut self,
        init_message: InitMessage,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        let remote_public = init_message.pub_key();
        let nonce: u64 = fastrand::u64(..);
        if let Some(registration_data) = self.registration_in_progres.get(&remote_public) {
            return Ok(AuthenticatorResponse::new_pending_registration_success(
                registration_data.value().clone(),
                reply_to,
            ));
        }
        let gateway_client_opt = if let Some(gateway_client) = self
            .wireguard_gateway_data
            .client_registry()
            .get(&remote_public)
        {
            let mut private_ip_ref = self
                .free_private_network_ips
                .get_mut(&gateway_client.private_ip)
                .ok_or(AuthenticatorError::InternalError(String::from(
                    "could not find private IP",
                )))?;
            *private_ip_ref = true;
            Some(gateway_client.clone())
        } else {
            None
        };
        if let Some(gateway_client) = gateway_client_opt {
            self.remove_from_registry(&remote_public, &gateway_client)?;
        }
        let mut private_ip_ref = self
            .free_private_network_ips
            .iter_mut()
            .filter(|r| **r)
            .choose(&mut thread_rng())
            .ok_or(AuthenticatorError::NoFreeIp)?;
        // mark it as used, even though it's not final
        *private_ip_ref = false;
        let gateway_data = GatewayClient::new(
            self.wireguard_gateway_data.keypair().private_key(),
            remote_public.inner(),
            *private_ip_ref.key(),
            nonce,
        );
        let registration_data = RegistrationData {
            nonce,
            gateway_data,
            wg_port: self.config.authenticator.binding_port,
        };
        self.registration_in_progres
            .insert(remote_public, registration_data.clone());

        Ok(AuthenticatorResponse::new_pending_registration_success(
            registration_data,
            reply_to,
        ))
    }

    fn on_final_request(
        &mut self,
        gateway_client: GatewayClient,
        reply_to: Recipient,
    ) -> AuthenticatorHandleResult {
        Ok(AuthenticatorResponse::new_registered(reply_to))
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
                self.on_initial_request(init_msg, request.reply_to)
            }
            AuthenticatorRequestData::Final(client) => {
                self.on_final_request(client, request.reply_to)
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

        let input_message = InputMessage::new_regular_with_custom_hops(
            recipient,
            response_packet,
            TransmissionLane::General,
            None,
            None,
        );
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
