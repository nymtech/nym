use std::time::Duration;

use futures::channel::mpsc;
use futures::StreamExt;
use log::*;

use client_core::client::received_buffer::ReconstructedMessagesReceiver;
use client_core::client::received_buffer::{ReceivedBufferMessage, ReceivedBufferRequestSender};
use nymsphinx::receiver::ReconstructedMessage;
use proxy_helpers::connection_controller::{ControllerCommand, ControllerSender};
use socks5_requests::Message;
use task::TaskClient;

use crate::error::Socks5ClientError;

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

    fn on_message(
        &self,
        reconstructed_message: ReconstructedMessage,
    ) -> Result<(), Socks5ClientError> {
        let raw_message = reconstructed_message.message;
        if reconstructed_message.sender_tag.is_some() {
            warn!("this message was sent anonymously - it couldn't have come from the service provider");
        }

        let response = match Message::try_from_bytes(&raw_message) {
            Err(err) => {
                warn!("failed to parse received response - {err}");
                return Ok(());
            }
            Ok(Message::Request(_)) => {
                warn!("unexpected request");
                return Ok(());
            }
            Ok(Message::Response(data)) => data,
            Ok(Message::NetworkRequesterResponse(r)) => {
                error!(
                    "Network requester failed on connection id {} with error: {}",
                    r.connection_id, r.network_requester_error
                );
                return Err(Socks5ClientError::NetworkRequesterError {
                    connection_id: r.connection_id,
                    error: r.network_requester_error,
                });
            }
        };

        self.controller_sender
            .unbounded_send(ControllerCommand::Send(
                response.connection_id,
                response.data,
                response.is_closed,
            ))
            .unwrap();

        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                received_responses = self.mix_response_receiver.next() => {
                    if let Some(received_responses) = received_responses {
                        for reconstructed_message in received_responses {
                            if let Err(err) = self.on_message(reconstructed_message) {
                                self.shutdown.send_status_msg(Box::new(err));
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
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::timeout(Duration::from_secs(5), self.shutdown.recv())
            .await
            .expect("Task stopped without shutdown called");
        log::debug!("MixnetResponseListener: Exiting");
    }
}
