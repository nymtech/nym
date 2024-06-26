// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::active_requests::ActiveRequests;
use crate::config::MixFetchConfig;
use crate::error::MixFetchError;
use crate::go_bridge::goWasmSetMixFetchRequestTimeout;
use crate::request_writer::RequestWriter;
use crate::socks_helpers::{socks5_connect_request, socks5_data_request};
use crate::{config, RequestId};
use futures::SinkExt;
use js_sys::Promise;
use nym_socks5_requests::RemoteAddress;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_client_core::client::base_client::{BaseClientBuilder, ClientInput, ClientOutput};
use wasm_client_core::client::inbound_messages::InputMessage;
use wasm_client_core::helpers::setup_gateway_from_api;
use wasm_client_core::init::types::GatewaySetup;
use wasm_client_core::nym_task::connections::TransmissionLane;
use wasm_client_core::nym_task::TaskManager;
use wasm_client_core::storage::core_client_traits::FullWasmClientStorage;
use wasm_client_core::storage::ClientStorage;
use wasm_client_core::{IdentityKey, QueryReqwestRpcNyxdClient, Recipient};
use wasm_utils::console_log;
use wasm_utils::error::PromisableResult;

#[wasm_bindgen]
pub struct MixFetchClient {
    invalidated: AtomicBool,

    mix_fetch_config: config::MixFetch,

    self_address: Recipient,

    client_input: Arc<RwLock<ClientInput>>,

    requests: ActiveRequests,

    // this has to be guarded by a mutex to be able to disconnect with an immutable reference
    _task_manager: Mutex<TaskManager>,
}

#[wasm_bindgen]
pub struct MixFetchClientBuilder {
    config: MixFetchConfig,
    preferred_gateway: Option<IdentityKey>,
    force_tls: bool,

    storage_passphrase: Option<String>,
}

#[wasm_bindgen]
impl MixFetchClientBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        config: MixFetchConfig,
        force_tls: bool,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Self {
        MixFetchClientBuilder {
            config,
            preferred_gateway,
            force_tls,
            storage_passphrase,
        }
    }

    fn initialise_storage(
        config: &MixFetchConfig,
        base_storage: ClientStorage,
    ) -> FullWasmClientStorage {
        FullWasmClientStorage::new(&config.base, base_storage)
    }

    fn start_reconstructor(client_output: ClientOutput, requests: ActiveRequests) {
        RequestWriter::new(client_output, requests).start()
    }

    // TODO: combine with normal wasm client
    async fn start_client_async(mut self) -> Result<MixFetchClient, MixFetchError> {
        console_log!("Starting the mix fetch client");

        let timeout_ms = self.config.mix_fetch.debug.request_timeout.as_millis();
        if timeout_ms > u32::MAX as u128 {
            return Err(MixFetchError::InvalidTimeoutValue { timeout_ms });
        }
        goWasmSetMixFetchRequestTimeout(timeout_ms as u32);

        let nym_api_endpoints = self.config.base.client.nym_api_urls.clone();

        // TODO: this will have to be re-used for surbs. but this is a problem for another PR.
        let client_store =
            ClientStorage::new_async(&self.config.base.client.id, self.storage_passphrase.take())
                .await?;

        let user_chosen = self.preferred_gateway.clone();
        let init_res = setup_gateway_from_api(
            &client_store,
            self.force_tls,
            user_chosen,
            &nym_api_endpoints,
        )
        .await?;

        let storage = Self::initialise_storage(&self.config, client_store);

        let mut base_builder = BaseClientBuilder::<QueryReqwestRpcNyxdClient, _>::new(
            &self.config.base,
            storage,
            None,
        );

        if let Ok(reuse_setup) = GatewaySetup::try_reuse_connection(init_res) {
            base_builder = base_builder.with_gateway_setup(reuse_setup);
        }
        let mut started_client = base_builder.start_base().await?;

        let self_address = started_client.address;

        let active_requests = ActiveRequests::default();

        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_reconstructor(client_output, active_requests.clone());

        Ok(MixFetchClient {
            invalidated: AtomicBool::new(false),
            mix_fetch_config: self.config.mix_fetch,
            self_address,
            client_input: Arc::new(RwLock::new(client_input)),
            requests: active_requests,
            // this cannot failed as we haven't passed an external task manager
            _task_manager: Mutex::new(started_client.task_handle.try_into_task_manager().unwrap()),
        })
    }
}

#[wasm_bindgen]
impl MixFetchClient {
    pub(crate) async fn new_async(
        config: MixFetchConfig,
        force_tls: bool,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Result<MixFetchClient, MixFetchError> {
        MixFetchClientBuilder::new(config, force_tls, preferred_gateway, storage_passphrase)
            .start_client_async()
            .await
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        config: MixFetchConfig,
        force_tls: bool,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Promise {
        future_to_promise(async move {
            Self::new_async(config, force_tls, preferred_gateway, storage_passphrase)
                .await
                .into_promise_result()
        })
    }

    pub fn active(&self) -> bool {
        !self.invalidated.load(Ordering::Relaxed)
    }

    pub async fn disconnect(&self) -> Result<(), MixFetchError> {
        self.invalidated.store(true, Ordering::Relaxed);

        console_log!("sending shutdown signal");
        let mut shutdown_guard = self._task_manager.lock().await;
        shutdown_guard.signal_shutdown().ok();

        console_log!("waiting for shutdown to complete");
        shutdown_guard.wait_for_shutdown().await;

        self.requests.invalidate_all().await;

        console_log!("done");
        Ok(())
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
            .write()
            .await
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
            .write()
            .await
            .input_sender
            .send(input)
            .await
            .expect("the client has stopped listening for requests");

        Ok(())
    }

    pub(crate) async fn connect_to_mixnet(
        &self,
        target: String,
    ) -> Result<RequestId, MixFetchError> {
        let request_id = self.requests.start_new().await;
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

    pub(crate) async fn disconnect_from_mixnet(
        &self,
        request_id: RequestId,
    ) -> Result<(), MixFetchError> {
        self.close_local_socket(request_id).await?;
        self.requests.finish(request_id).await;
        Ok(())
    }
}
