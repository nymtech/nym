use client_core::client::received_buffer::ReconstructedMessagesReceiver;
use client_core::client::received_buffer::{ReceivedBufferMessage, ReceivedBufferRequestSender};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nymsphinx::receiver::ReconstructedMessage;
use proxy_helpers::connection_controller::{ControllerCommand, ControllerSender};
use socks5_requests::Message;
use task::ShutdownListener;

pub(crate) struct MixnetResponseListener {
    buffer_requester: ReceivedBufferRequestSender,
    mix_response_receiver: ReconstructedMessagesReceiver,
    controller_sender: ControllerSender,
    shutdown: ShutdownListener,
}

impl Drop for MixnetResponseListener {
    fn drop(&mut self) {
        self.buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverDisconnect)
            .expect("the buffer request failed!")
    }
}

impl MixnetResponseListener {
    pub(crate) fn new(
        buffer_requester: ReceivedBufferRequestSender,
        controller_sender: ControllerSender,
        shutdown: ShutdownListener,
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

    async fn on_message(&self, reconstructed_message: ReconstructedMessage) {
        let raw_message = reconstructed_message.message;
        if reconstructed_message.reply_surb.is_some() {
            warn!("this message had a surb - we didn't do anything with it");
        }

        let response = match Message::try_from_bytes(&raw_message) {
            Err(err) => {
                warn!("failed to parse received response - {:?}", err);
                return;
            }
            Ok(Message::Request(_)) => {
                warn!("unexpected request");
                return;
            }
            Ok(Message::Response(data)) => data,
            Ok(Message::NetworkRequesterResponse(r)) => {
                error!(
                    "Network requester failed on connection id {} with error: {}",
                    r.connection_id, r.network_requester_error
                );
                return;
            }
        };

        self.controller_sender
            .unbounded_send(ControllerCommand::Send(
                response.connection_id,
                response.data,
                response.is_closed,
            ))
            .unwrap();
    }

    pub(crate) async fn run(&mut self) {
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                Some(received_responses) = self.mix_response_receiver.next() => {
                    for reconstructed_message in received_responses {
                        self.on_message(reconstructed_message).await;
                    }
                },
                _ = self.shutdown.recv() => {
                    log::trace!("MixnetResponseListener: Received shutdown");
                }
            }
        }
        let a = crate::client::SHUTDOWN_IS_SIGNALLED.load(std::sync::atomic::Ordering::Relaxed);
        log::debug!("MixnetResponseListener: Exiting");
    }
}
