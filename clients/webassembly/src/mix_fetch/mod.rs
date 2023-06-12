// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::error::MixFetchError;
use crate::mix_fetch::mix_http_requests::{
    http_request_to_mixnet_request_to_vec_u8, socks5_connect_request,
};
use crate::mix_fetch::request_adapter::WebSysRequestAdapter;
use crate::mix_fetch::request_correlator::{ActiveRequests, Response};
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use httpcodec::{Request as HttpCodecRequest, RequestTarget};
use js_sys::Promise;
use nym_client_core::client::base_client::{ClientInput, ClientOutput};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReconstructedMessagesReceiver,
};
use nym_http_requests::socks::MixHttpResponse;
use nym_service_providers_common::interface::{
    ProviderInterfaceVersion, ResponseContent, Serializable,
};
use nym_socks5_requests::{
    ConnectionId, RemoteAddress, Socks5ProtocolVersion, Socks5ProviderResponse,
    Socks5ResponseContent,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::TransmissionLane;
use rand::{thread_rng, RngCore};
use std::sync::Arc;
use wasm_bindgen_futures::{future_to_promise, spawn_local};
use wasm_utils::{console_error, console_log, PromisableResult};

mod client;
mod config;
pub mod error;
pub mod mix_http_requests;
pub(crate) mod request_adapter;
mod request_correlator;

pub const MIX_FETCH_CLIENT_ID_PREFIX: &str = "mix-fetch-";

pub(crate) const PROVIDER_INTERFACE_VERSION: ProviderInterfaceVersion =
    ProviderInterfaceVersion::new_current();
pub(crate) const SOCKS5_PROTOCOL_VERSION: Socks5ProtocolVersion =
    Socks5ProtocolVersion::new_current();

#[derive(Clone)]
struct Placeholder {
    fetch_provider: Recipient,
    self_address: Recipient,
    client_input: Arc<ClientInput>,
    requests: ActiveRequests,
}

impl Placeholder {
    pub(crate) fn new(
        fetch_provider: Recipient,
        self_address: Recipient,
        client_input: ClientInput,
        requests: ActiveRequests,
    ) -> Self {
        Placeholder {
            fetch_provider,
            self_address,
            client_input: Arc::new(client_input),
            requests,
        }
    }

    pub(crate) fn fetch(
        &self,
        local_closed: bool,
        ordered_message_index: u64,
        req: WebSysRequestAdapter,
    ) -> Promise {
        console_log!("started fetch");
        let this = self.clone();
        future_to_promise(async move {
            this.fetch_async(local_closed, ordered_message_index, req)
                .await
                .map(|res| {
                    console_log!("raw response: {:?}", res);
                    "placeholder return value".to_string()
                })
                .into_promise_result()
        })
    }

    async fn send_connect(
        &self,
        request_id: ConnectionId,
        target: RemoteAddress,
    ) -> Result<(), MixFetchError> {
        // TODO: regenerate id in case of collisions
        // (though realistically the chance is 1 in ~ 2^63 so do we have to bother?)
        let raw_conn_req = socks5_connect_request(request_id, target, self.self_address);
        let lane = TransmissionLane::ConnectionId(request_id);
        let input = InputMessage::new_regular(self.fetch_provider, raw_conn_req, lane, None);

        self.client_input
            .input_sender
            .send(input)
            .await
            .expect("TODO: error handling");

        Ok(())
    }

    async fn send_request_data(
        &self,
        request_id: ConnectionId,
        local_closed: bool,
        ordered_message_index: u64,
        content: HttpCodecRequest<Vec<u8>>,
    ) -> Result<(), MixFetchError> {
        // TODO: regenerate id in case of collisions
        // (though realistically the chance is 1 in ~ 2^63 so do we have to bother?)
        let raw_send_request = match http_request_to_mixnet_request_to_vec_u8(
            request_id,
            local_closed,
            ordered_message_index,
            content,
        ) {
            Ok(ok) => ok,
            Err(_) => {
                panic!("TODO: error handling");
            }
        };
        let lane = TransmissionLane::ConnectionId(request_id);
        let input = InputMessage::new_regular(self.fetch_provider, raw_send_request, lane, None);

        self.client_input
            .input_sender
            .send(input)
            .await
            .expect("TODO: error handling");

        Ok(())
    }

    pub(crate) async fn fetch_async(
        &self,
        local_closed: bool,
        ordered_message_index: u64,
        req: WebSysRequestAdapter,
    ) -> Result<Response, MixFetchError> {
        console_log!("started fetch async for {:?}", req.target);

        // TODO: regenerate id in case of collisions
        // (though realistically the chance is 1 in ~ 2^63 so do we have to bother?)
        let mut rng = thread_rng();
        let request_id = rng.next_u64();

        console_log!("raw id: {:?}", request_id.to_be_bytes());

        self.send_connect(request_id, req.target).await?;
        self.send_request_data(request_id, local_closed, ordered_message_index, req.request)
            .await?;

        let (tx, rx) = oneshot::channel();
        self.requests.start_new(request_id, tx).await;
        console_log!("waiting for response");
        let res = rx.await.expect("TODO: error handling for closed channel");
        console_log!("got response!");

        Ok(res)
    }
}

struct Placeholder2 {
    reconstructed_receiver: ReconstructedMessagesReceiver,
    requests: ActiveRequests,
}

impl Placeholder2 {
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

        Placeholder2 {
            reconstructed_receiver,
            requests,
        }
    }

    // TODO: obviously collapse all ifs and clean it up
    pub(crate) fn start(mut self) {
        spawn_local(async move {
            while let Some(reconstructed) = self.reconstructed_receiver.next().await {
                console_log!("got reply!");
                for reconstructed_msg in reconstructed {
                    let (msg, tag) = reconstructed_msg.into_inner();
                    let raw = String::from_utf8_lossy(&msg);
                    console_log!("raw: {raw}");

                    if tag.is_some() {
                        panic!("TODO: error handling for set tag")
                    }

                    match Socks5ProviderResponse::try_from_bytes(&msg) {
                        Err(err) => {
                            console_error!("failed to deserialize provider response: {err}")
                        }
                        Ok(provider_response) => {
                            if let ResponseContent::ProviderData(response) =
                                provider_response.content
                            {
                                match MixHttpResponse::try_from(response.clone()) {
                                    Ok(mix_http_response) => {
                                        self.requests
                                            .resolve(
                                                mix_http_response.connection_id,
                                                mix_http_response.http_response,
                                            )
                                            .await
                                    }
                                    Err(err) => {
                                        console_error!("this wasn't mix http response...: {err}");
                                        console_log!("{:?}", response);
                                    }
                                }
                            } else {
                                console_error!("received data was not a provider data response")
                            }
                        }
                    }

                    // if let Ok(socks5_response) =
                    //     nym_socks5_requests::Socks5Response::try_from_bytes(&msg)
                    // {
                    //     if let Socks5ResponseContent::NetworkData { content } =
                    //         &socks5_response.content
                    //     {
                    //         console_log!("raw res: {:?}", content.header);
                    //         console_log!(
                    //             "raw res: {:?}",
                    //             content.header.connection_id.to_be_bytes()
                    //         );
                    //         console_log!("raw res: {:?}", content.header.seq.to_be_bytes());
                    //     }
                    //
                    //     match MixHttpResponse::try_from(socks5_response.clone()) {
                    //         Ok(mix_http_response) => {
                    //             self.requests
                    //                 .resolve(
                    //                     mix_http_response.connection_id,
                    //                     mix_http_response.http_response,
                    //                 )
                    //                 .await
                    //         }
                    //         Err(err) => {
                    //             console_error!("this wasn't mix http response...: {err}");
                    //             console_log!("{:?}", socks5_response);
                    //         }
                    //     }
                    // } else {
                    //     panic!("TODO: error handling for receiving something that's not socks5 response")
                    // }
                }
            }

            console_error!("we stopped receiving reconstructed messages!")
        })
    }
}
