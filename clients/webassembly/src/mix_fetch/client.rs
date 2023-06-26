// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use crate::helpers::{setup_gateway_from_api, setup_reply_surb_storage_backend};
use crate::mix_fetch::active_requests::ActiveRequests;
use crate::mix_fetch::config::MixFetchConfig;
use crate::mix_fetch::error::MixFetchError;
use crate::mix_fetch::go_bridge::goWasmMixFetch;
use crate::mix_fetch::go_bridge::goWasmMixFetch2;
use crate::mix_fetch::request_writer::RequestWriter;
use crate::mix_fetch::socks_helpers::{socks5_connect_request, socks5_data_request};
use crate::mix_fetch::{config, RequestId, Resource};
use crate::storage::traits::FullWasmClientStorage;
use crate::storage::ClientStorage;
use futures::channel::oneshot;
use js_sys::Promise;
use nym_bandwidth_controller::wasm_mockups::{Client as FakeClient, DirectSigningNyxdClient};
use nym_client_core::client::base_client::{BaseClientBuilder, ClientInput, ClientOutput};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_socks5_requests::RemoteAddress;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::connections::TransmissionLane;
use nym_task::TaskManager;
use nym_validator_client::client::IdentityKey;
use url::{Origin, Url};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use wasm_timer::Delay;
use wasm_utils::{console_error, console_log, PromisableResult};

fn socks5_target(raw_url: &str) -> Result<String, MixFetchError> {
    let url: Url = raw_url
        .parse()
        .map_err(MixFetchError::MalformedMixFetchUrl)?;

    let origin = url.origin();

    // TODO: there must be a better way for that...
    match origin {
        Origin::Opaque(_) => Err(MixFetchError::UnsupportedOrigin),

        // TODO: make a match on the scheme to reject requests we can't handle, like `ftp` or `file`
        Origin::Tuple(ref _scheme, ref host, port) => Ok(format!("{}:{}", host, port)),
    }
}

#[wasm_bindgen]
pub struct MixFetchClient {
    mix_fetch_config: config::MixFetch,

    self_address: Recipient,

    client_input: ClientInput,

    requests: ActiveRequests,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _task_manager: TaskManager,
}

#[wasm_bindgen]
pub struct MixFetchClientBuilder {
    config: MixFetchConfig,
    preferred_gateway: Option<IdentityKey>,

    storage_passphrase: Option<String>,
}

#[wasm_bindgen]
impl MixFetchClientBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        config: MixFetchConfig,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Self {
        MixFetchClientBuilder {
            config,
            preferred_gateway,
            storage_passphrase,
        }
    }

    fn initialise_storage(
        config: &MixFetchConfig,
        base_storage: ClientStorage,
    ) -> FullWasmClientStorage {
        FullWasmClientStorage {
            keys_and_gateway_store: base_storage,
            reply_storage: setup_reply_surb_storage_backend(config.base.debug.reply_surbs),
            credential_storage: EphemeralCredentialStorage::default(),
        }
    }

    fn start_reconstructor(client_output: ClientOutput, requests: ActiveRequests) {
        RequestWriter::new(client_output, requests).start()
    }

    // TODO: combine with normal wasm client
    async fn start_client_async(mut self) -> Result<MixFetchClient, WasmClientError> {
        console_log!("Starting the mix fetch client");

        let nym_api_endpoints = self.config.base.client.nym_api_urls.clone();

        // TODO: this will have to be re-used for surbs. but this is a problem for another PR.
        let client_store =
            ClientStorage::new_async(&self.config.base.client.id, self.storage_passphrase.take())
                .await?;

        let user_chosen = self.preferred_gateway.clone();
        setup_gateway_from_api(&client_store, user_chosen, &nym_api_endpoints).await?;
        let storage = Self::initialise_storage(&self.config, client_store);

        let mut started_client = BaseClientBuilder::<FakeClient<DirectSigningNyxdClient>, _>::new(
            &self.config.base,
            storage,
            None,
        )
        .start_base()
        .await?;
        let self_address = started_client.address;

        let active_requests = ActiveRequests::default();

        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_reconstructor(client_output, active_requests.clone());

        Ok(MixFetchClient {
            mix_fetch_config: self.config.mix_fetch,
            self_address,
            client_input,
            requests: active_requests,
            _task_manager: started_client.task_manager,
        })
    }
}

#[wasm_bindgen]
impl MixFetchClient {
    pub(crate) async fn new_async(
        config: MixFetchConfig,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Result<MixFetchClient, WasmClientError> {
        MixFetchClientBuilder::new(config, preferred_gateway, storage_passphrase)
            .start_client_async()
            .await
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        config: MixFetchConfig,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Promise {
        future_to_promise(async move {
            Self::new_async(config, preferred_gateway, storage_passphrase)
                .await
                .into_promise_result()
        })
    }

    pub fn self_address(&self) -> String {
        self.self_address.to_string()
    }

    async fn send_socks_connect(
        &self,
        request_id: RequestId,
        target: RemoteAddress,
    ) -> Result<(), MixFetchError> {
        let raw_conn_req = socks5_connect_request(request_id, target, self.self_address);
        let lane = TransmissionLane::ConnectionId(request_id);
        let input = InputMessage::new_regular(
            self.mix_fetch_config.network_requester_address,
            raw_conn_req,
            lane,
            None,
        );

        // the expect here is fine as it implies an unrecoverable failure since one of the client core
        // tasks has terminated
        self.client_input
            .input_sender
            .send(input)
            .await
            .expect("the client has stopped listening for requests");

        Ok(())
    }

    async fn send_socks_data(
        &self,
        request_id: RequestId,
        local_closed: bool,
        message_sequence: u64,
        data: Vec<u8>,
    ) -> Result<(), MixFetchError> {
        let raw_send_req = socks5_data_request(request_id, local_closed, message_sequence, data);
        let lane = TransmissionLane::ConnectionId(request_id);
        let input = InputMessage::new_regular(
            self.mix_fetch_config.network_requester_address,
            raw_send_req,
            lane,
            None,
        );

        // the expect here is fine as it implies an unrecoverable failure since one of the client core
        // tasks has terminated
        self.client_input
            .input_sender
            .send(input)
            .await
            .expect("the client has stopped listening for requests");

        Ok(())
    }

    pub(crate) async fn fetch_async2(
        &self,
        request: web_sys::Request,
    ) -> Result<web_sys::Response, JsValue> {
        let go_fut: JsFuture = goWasmMixFetch2(request).into();

        let timeout = Delay::new(self.mix_fetch_config.request_timeout);

        tokio::select! {
            biased;
            go_res = go_fut => {
                match go_res {
                    Ok(res) => {
                        console_log!("received response to our fetch request: {res:?}");
                        Ok(res.into())
                    },
                    Err(err) => {
                        console_error!("go request failure: {err:?}");
                        Err(err)
                    }
                }
            }
            _ = timeout => {
                console_error!("timed out while waiting for response");
                todo!("timeout")
                // goWasmAbortConnection(request_id.to_string());
                //
                // Err(MixFetchError::Timeout {
                //     id: request_id, timeout: self.mix_fetch_config.request_timeout
                // }.into())
            }
        }
    }

    pub(crate) async fn fetch_async(
        &self,
        request: web_sys::Request,
    ) -> Result<web_sys::Response, JsValue> {
        todo!()
        // let url = web_sys::Request::url(&request);
        // let target = socks5_target(&url)?;
        //
        // let (err_sender, err_receiver) = oneshot::channel();
        // let request_id = self.requests.start_new(err_sender).await;
        // self.send_socks_connect(request_id, target).await?;
        //
        // let go_fut: JsFuture = goWasmMixFetch(request_id.to_string(), request).into();
        //
        // let timeout = Delay::new(self.mix_fetch_config.request_timeout);
        //
        // tokio::select! {
        //     biased;
        //     go_res = go_fut => {
        //         match go_res {
        //             Ok(res) => {
        //                 console_log!("received response to our fetch request: {res:?}");
        //                 self.requests.finish(request_id).await;
        //                 Ok(res.into())
        //             },
        //             Err(err) => {
        //                 console_error!("go request failure: {err:?}");
        //                 self.requests.finish(request_id).await;
        //                 Err(err)
        //             }
        //         }
        //     }
        //     _ = timeout => {
        //         console_error!("timed out while waiting for response");
        //         self.requests.abort(request_id).await;
        //         Err(MixFetchError::Timeout {
        //             id: request_id, timeout: self.mix_fetch_config.request_timeout
        //         }.into())
        //     }
        //     err = err_receiver => {
        //         match err {
        //             Err(_cancelled) => {
        //                 todo!("our error sender was dropped - deal with it")
        //             }
        //             Ok(err) => {
        //                 console_error!("our request has failed: {err}");
        //                 Err(err.into())
        //             }
        //         }
        //     }
        // }
    }

    pub(crate) async fn connect_to_mixnet(
        &self,
        target: String,
    ) -> Result<RequestId, MixFetchError> {
        // console_log!("raw url: {url:?}");
        // let target = socks5_target(&url)?;

        let request_id = self.requests.start_new2().await;
        self.send_socks_connect(request_id, target).await?;

        Ok(request_id)
    }

    pub(crate) async fn forward_request_content(
        &self,
        request_id: RequestId,
        data: Vec<u8>,
    ) -> Result<(), MixFetchError> {
        let seq = self
            .requests
            .get_sending_sequence(request_id)
            .await
            .ok_or(MixFetchError::AbortedRequest { request_id })?;

        self.send_socks_data(request_id, false, seq, data).await
    }

    pub(crate) async fn close_local_socket(
        &self,
        request_id: RequestId,
    ) -> Result<(), MixFetchError> {
        let seq = self
            .requests
            .get_sending_sequence(request_id)
            .await
            .ok_or(MixFetchError::AbortedRequest { request_id })?;

        self.send_socks_data(request_id, true, seq, Vec::new())
            .await
    }
}
