// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::active_requests::ActiveRequests;
use futures::channel::mpsc;
use futures::StreamExt;
use nym_service_providers_common::interface::ResponseContent;
use nym_socks5_requests::{Socks5ProviderResponse, Socks5ResponseContent};
use wasm_bindgen_futures::spawn_local;
use wasm_client_core::client::base_client::ClientOutput;
use wasm_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReceivedBufferRequestSender, ReconstructedMessagesReceiver,
};
use wasm_client_core::ReconstructedMessage;
use wasm_utils::console_error;

pub(crate) struct RequestWriter {
    reconstructed_receiver: ReconstructedMessagesReceiver,

    // we need to keep that channel alive as not to trigger the shutdown
    _received_buffer_request_sender: ReceivedBufferRequestSender,

    requests: ActiveRequests,
}

impl RequestWriter {
    pub(crate) fn new(client_output: ClientOutput, requests: ActiveRequests) -> Self {
        // register our output
        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();

        // tell the buffer to start sending stuff to us
        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .expect("the buffer request failed!");

        RequestWriter {
            reconstructed_receiver,
            _received_buffer_request_sender: client_output.received_buffer_request_sender,
            requests,
        }
    }

    async fn handle_reconstructed(&mut self, reconstructed_message: ReconstructedMessage) {
        let (msg, tag) = reconstructed_message.into_inner();

        if tag.is_some() {
            console_error!(
                "received a response with an anonymous sender tag - this is highly unexpected!"
            );
        }

        match Socks5ProviderResponse::try_from_bytes(&msg) {
            Err(err) => {
                console_error!("failed to deserialize provider response. it was most likely not a response to our fetch: {err}")
            }
            Ok(provider_response) => match provider_response.content {
                ResponseContent::Control(control) => {
                    console_error!("received a provider control response even though we didnt send any requests! - {control:#?}")
                }
                ResponseContent::ProviderData(data_response) => match data_response.content {
                    Socks5ResponseContent::ConnectionError(err) => {
                        self.requests.reject(err.connection_id, err.into()).await;
                    }
                    Socks5ResponseContent::Query(query) => {
                        console_error!("received a provider query response even though we didn't send any queries! - {query:#?}")
                    }
                    Socks5ResponseContent::NetworkData { content } => {
                        self.requests.try_send_data_to_go(content).await;
                    }
                },
            },
        }
    }

    pub(crate) fn start(mut self) {
        spawn_local(async move {
            while let Some(reconstructed) = self.reconstructed_receiver.next().await {
                for reconstructed_msg in reconstructed {
                    self.handle_reconstructed(reconstructed_msg).await
                }
            }
            console_error!("we stopped receiving reconstructed messages!")
        })
    }
}
