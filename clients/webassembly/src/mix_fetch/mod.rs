// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::client::MixFetchClient;
use crate::mix_fetch::config::MixFetchConfig;
use crate::mix_fetch::error::MixFetchError;
use crate::mix_fetch::mix_http_requests::{
    http_request_to_mixnet_request_to_vec_u8, socks5_connect_request,
};
use crate::mix_fetch::request_adapter::WebSysRequestAdapter;
use crate::mix_fetch::request_correlator::{goWasmMixFetch, ActiveRequests, RequestId, Response};
use crate::mix_fetch::response_adapter::FetchResponse;
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use httpcodec::{Request as HttpCodecRequest, RequestTarget};
use js_sys::Promise;
use nym_client_core::client::base_client::{ClientInput, ClientOutput};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReconstructedMessagesReceiver,
};
use nym_http_requests::error::MixHttpRequestError;
use nym_http_requests::socks::MixHttpResponse;
use nym_service_providers_common::interface::{
    ProviderInterfaceVersion, ResponseContent, Serializable,
};
use nym_socks5_requests::{
    ConnectionId, RemoteAddress, SocketData, Socks5ProtocolVersion, Socks5ProviderRequest,
    Socks5ProviderResponse, Socks5Request, Socks5ResponseContent,
};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::connections::TransmissionLane;
use nym_validator_client::client::IdentityKey;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use url::Origin;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{future_to_promise, spawn_local, JsFuture};
use wasm_timer::Delay;
use wasm_utils::{console_error, console_log, simple_js_error, PromisableResult};
use web_sys::RequestInit;

mod client;
mod config;
pub mod error;
pub mod mix_http_requests;
pub(crate) mod request_adapter;
mod request_correlator;
mod response_adapter;

pub const MIX_FETCH_CLIENT_ID_PREFIX: &str = "mix-fetch-";

pub(crate) const PROVIDER_INTERFACE_VERSION: ProviderInterfaceVersion =
    ProviderInterfaceVersion::new_current();
pub(crate) const SOCKS5_PROTOCOL_VERSION: Socks5ProtocolVersion =
    Socks5ProtocolVersion::new_current();

static MIX_FETCH: OnceLock<MixFetchClient> = OnceLock::new();

fn set_mix_fetch_client(mix_fetch_client: MixFetchClient) -> Result<(), MixFetchError> {
    MIX_FETCH
        .set(mix_fetch_client)
        .map_err(|_| MixFetchError::AlreadyInitialised)
}

fn mix_fetch_client() -> Result<&'static MixFetchClient, MixFetchError> {
    MIX_FETCH.get().ok_or(MixFetchError::Uninitialised)
}

#[wasm_bindgen(js_name = setupMixFetch)]
pub fn setup_mix_fetch(
    config: MixFetchConfig,
    preferred_gateway: Option<IdentityKey>,
    storage_passphrase: Option<String>,
) -> Promise {
    if MIX_FETCH.get().is_some() {
        MixFetchError::AlreadyInitialised.into_rejected_promise()
    } else {
        future_to_promise(async move {
            let client =
                MixFetchClient::new_async(config, preferred_gateway, storage_passphrase).await?;
            set_mix_fetch_client(client)?;
            Ok(JsValue::undefined())
        })
    }
}

#[derive(Debug)]
pub enum Resource {
    Url(url::Url),
    Request(web_sys::Request),
}

impl TryFrom<JsValue> for Resource {
    type Error = MixFetchError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if value.is_string() {
            let string = value
                .as_string()
                .ok_or(MixFetchError::NotStringMixFetchUrl)?;
            Ok(Resource::Url(string.parse()?))
        } else {
            Ok(Resource::Request(web_sys::Request::from(value)))
        }
    }
}

async fn mix_fetch_async(
    resource: JsValue,
    opts: Option<web_sys::RequestInit>,
) -> Result<web_sys::Response, MixFetchError> {
    let resource = Resource::try_from(resource)?;
    console_log!("mix fetch with {resource:?} and {opts:?}");

    let mix_fetch_client = mix_fetch_client()?;
    mix_fetch_client.fetch_async(resource, opts).await
}

// https://developer.mozilla.org/en-US/docs/Web/API/fetch#syntax
#[wasm_bindgen(js_name = mixFetch)]
pub fn mix_fetch(resource: JsValue, opts: Option<web_sys::RequestInit>) -> Promise {
    future_to_promise(async move { mix_fetch_async(resource, opts).await.into_promise_result() })
}

#[derive(Clone)]
struct Fetcher {
    fetch_provider: Recipient,
    self_address: Recipient,
    client_input: Arc<ClientInput>,
    requests: ActiveRequests,
}

impl Fetcher {
    pub(crate) fn new(
        fetch_provider: Recipient,
        self_address: Recipient,
        client_input: ClientInput,
        requests: ActiveRequests,
    ) -> Self {
        Fetcher {
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
                .await?
                .try_into_fetch_response()
                .map(Into::into)
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

    async fn send_request_data_new(
        &self,
        request_id: RequestId,
        local_closed: bool,
        message_sequence: u64,
        data: Vec<u8>,
    ) -> Result<(), MixFetchError> {
        console_log!("sending {data:?} to {}", self.fetch_provider);

        // TODO: clean this up...
        let request_content = nym_socks5_requests::request::Socks5Request::new_send(
            SOCKS5_PROTOCOL_VERSION,
            SocketData::new(message_sequence, request_id, local_closed, data),
        );
        let socks_req =
            Socks5ProviderRequest::new_provider_data(PROVIDER_INTERFACE_VERSION, request_content);

        let lane = TransmissionLane::ConnectionId(request_id);
        let input =
            InputMessage::new_regular(self.fetch_provider, socks_req.into_bytes(), lane, None);

        self.client_input
            .input_sender
            .send(input)
            .await
            .expect("TODO: error handling");

        Ok(())
    }

    #[deprecated]
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

    pub(crate) async fn fetch_async_new(
        &self,
        resource: Resource,
        opts: Option<RequestInit>,
    ) -> Result<web_sys::Response, MixFetchError> {
        // TODO: make it user configurable
        const TIMEOUT: Duration = Duration::from_secs(5);

        if let Some(opts) = opts {
            console_error!("attempted to mix fetch with extra request options: {opts:?}");
            unimplemented!()
        }

        let url = match resource {
            Resource::Url(url) => url,
            Resource::Request(request) => {
                console_error!("attempted to mix fetch with request object: {request:?}");
                unimplemented!()
            }
        };

        // required for the 'connect' request
        let origin = url.origin();
        let target = match origin {
            Origin::Opaque(_) => unimplemented!(),
            Origin::Tuple(ref _scheme, ref host, port) => format!("{}:{}", host, port),
        };

        let (tx, rx) = oneshot::channel();
        let request_id = self.requests.insert_new(tx).await;
        let _unused = rx;
        self.send_connect(request_id, target).await?;

        let go_fut: JsFuture = goWasmMixFetch(request_id.to_string(), url.to_string()).into();

        let timeout = Delay::new(TIMEOUT);

        tokio::select! {
            _ = timeout => {
                console_error!("timed out while waiting for response");
                self.requests.abort(request_id).await;
                Err(MixFetchError::Timeout {
                    id: request_id, timeout: TIMEOUT
                })
            }
            go_res = go_fut => {
                match go_res {
                    Ok(res) => {
                        console_log!("received response to our fetch request: {res:?}");
                        return Ok(res.into())
                    },
                    Err(err) => {
                        console_error!("go request failure: {err:?}");
                        todo!()
                    }
                }
            }
        }
    }

    pub(crate) async fn fetch_async(
        &self,
        local_closed: bool,
        ordered_message_index: u64,
        req: WebSysRequestAdapter,
    ) -> Result<httpcodec::Response<Vec<u8>>, MixFetchError> {
        // TODO: make it user configurable
        const TIMEOUT: Duration = Duration::from_secs(5);

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

        let timeout = Delay::new(TIMEOUT);

        tokio::select! {
            _ = timeout => {
                console_error!("timed out while waiting for response");
                self.requests.abort(request_id).await;
                Err(MixFetchError::Timeout {
                    id: request_id, timeout: TIMEOUT
                })
            }
            res = rx => {
                let Ok(res) = res else {
                    // we can't do anything more than abort here. this situation should have never occurred
                    // in the first place
                    panic!("our response channel has been dropped.");
                };

                console_log!("received response to our fetch request");
                res
            }
        }
    }
}

struct RequestResolver {
    reconstructed_receiver: ReconstructedMessagesReceiver,
    requests: ActiveRequests,
}

impl RequestResolver {
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

        RequestResolver {
            reconstructed_receiver,
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
                        console_log!("received raw: content: {content:?}");

                        // self.requests.attempt_resolve(content).await;
                        self.requests.try_send_data_to_go(content).await;
                    }
                },
            },
        }
    }

    pub(crate) fn start(mut self) {
        spawn_local(async move {
            while let Some(reconstructed) = self.reconstructed_receiver.next().await {
                console_log!("reconstructed something!");
                for reconstructed_msg in reconstructed {
                    self.handle_reconstructed(reconstructed_msg).await
                }
            }
            console_error!("we stopped receiving reconstructed messages!")
        })
    }
}
