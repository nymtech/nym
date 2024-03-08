// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Socks5ClientCoreError, Socks5ClientCoreStatusMessage};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nym_client_core::client::received_buffer::ReconstructedMessagesReceiver;
use nym_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReceivedBufferRequestSender,
};
use nym_service_providers_common::interface::{ControlResponse, ResponseContent};
use nym_socks5_proxy_helpers::connection_controller::{ControllerCommand, ControllerSender};
use nym_socks5_requests::{Socks5ProviderResponse, Socks5Response, Socks5ResponseContent};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskClient;

pub(crate) struct MixnetResponseListener {
    buffer_requester: ReceivedBufferRequestSender,
    mix_response_receiver: ReconstructedMessagesReceiver,
    controller_sender: ControllerSender,
    shutdown: TaskClient,
}

impl Drop for MixnetResponseListener {
    fn drop(&mut self) {
        if let Err(err) = self
            .buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverDisconnect)
        {
            if self.shutdown.is_shutdown_poll() {
                log::debug!("The buffer request failed: {err}");
            } else {
                log::error!("The buffer request failed: {err}");
            }
        }
    }
}

impl MixnetResponseListener {
    pub(crate) fn new(
        buffer_requester: ReceivedBufferRequestSender,
        controller_sender: ControllerSender,
        shutdown: TaskClient,
    ) -> Self {
        let (mix_response_sender, mix_response_receiver) = mpsc::unbounded();
        buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(mix_response_sender))
            .unwrap();

        MixnetResponseListener {
            buffer_requester,
            mix_response_receiver,
            controller_sender,
            shutdown,
        }
    }

    fn on_control_response(
        &self,
        control_response: ControlResponse,
    ) -> Result<(), Socks5ClientCoreError> {
        error!("received a control response which we don't know how to handle yet!");
        error!("got: {:?}", control_response);

        // I guess we'd need another channel here to forward those to where they need to go

        Ok(())
    }

    fn on_provider_data_response(
        &self,
        provider_response: Socks5Response,
    ) -> Result<(), Socks5ClientCoreError> {
        match provider_response.content {
            Socks5ResponseContent::ConnectionError(err_response) => {
                error!(
                    "Network requester failed on connection id {} with error: {}",
                    err_response.connection_id, err_response.network_requester_error
                );
                Err(err_response.into())
            }
            Socks5ResponseContent::NetworkData { content } => {
                self.controller_sender
                    .unbounded_send(ControllerCommand::new_send(content))
                    .unwrap();
                Ok(())
            }
            Socks5ResponseContent::Query(response) => {
                error!("received a query response which we don't know how to handle yet!");
                error!("got: {:?}", response);

                // I guess we'd need another channel here to forward those to where they need to go

                Ok(())
            }
        }
    }

    fn on_message(
        &self,
        reconstructed_message: ReconstructedMessage,
    ) -> Result<(), Socks5ClientCoreError> {
        let raw_message = reconstructed_message.message;
        if reconstructed_message.sender_tag.is_some() {
            warn!("this message was sent anonymously - it couldn't have come from the service provider");
        }
        match Socks5ProviderResponse::try_from_bytes(&raw_message) {
            Err(err) => {
                warn!("failed to parse received response: {err}");
                Ok(())
            }
            Ok(response) => {
                // as long as the client used the same (or older) interface than the service provider,
                // the response should have used exactly the same version
                trace!(
                    "the received response was sent with {:?} interface version",
                    response.interface_version
                );
                match response.content {
                    ResponseContent::Control(control_response) => {
                        self.on_control_response(control_response)
                    }
                    ResponseContent::ProviderData(provider_response) => {
                        self.on_provider_data_response(provider_response)
                    }
                }
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                received_responses = self.mix_response_receiver.next() => {
                    if let Some(received_responses) = received_responses {
                        for reconstructed_message in received_responses {
                            if let Err(err) = self.on_message(reconstructed_message) {
                                let msg = Socks5ClientCoreStatusMessage::from(err);
                                self.shutdown.send_status_msg(Box::new(msg));
                            }
                        }
                    } else {
                        log::trace!("MixnetResponseListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = self.shutdown.recv() => {
                    log::trace!("MixnetResponseListener: Received shutdown");
                }
            }
        }
        self.shutdown.recv_timeout().await;
        log::debug!("MixnetResponseListener: Exiting");
    }
}
