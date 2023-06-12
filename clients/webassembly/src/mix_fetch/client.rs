// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use crate::helpers::{setup_gateway_from_api, setup_reply_surb_storage_backend};
use crate::mix_fetch::config::MixFetchConfig;
use crate::mix_fetch::mix_http_requests::RequestInitWithTypescriptType;
use crate::mix_fetch::request_adapter::WebSysRequestAdapter;
use crate::mix_fetch::request_correlator::ActiveRequests;
use crate::mix_fetch::{Placeholder, Placeholder2};
use crate::storage::traits::FullWasmClientStorage;
use crate::storage::ClientStorage;
use js_sys::Promise;
use nym_bandwidth_controller::wasm_mockups::{Client as FakeClient, DirectSigningNyxdClient};
use nym_client_core::client::base_client::{BaseClientBuilder, ClientOutput};
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_http_requests::error::MixHttpRequestError;
use nym_task::TaskManager;
use nym_validator_client::client::IdentityKey;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, simple_js_error, PromisableResult};
use web_sys::Request;

#[wasm_bindgen]
pub struct MixFetchClient {
    self_address: String,
    placeholder: Placeholder,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _task_manager: TaskManager,
}

#[wasm_bindgen]
pub struct MixFetchClientBuilder {
    config: MixFetchConfig,
    preferred_gateway: Option<IdentityKey>,

    storage_passphrase: Option<String>,
    // on_mix_fetch_message: Option<js_sys::Function>,
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
        Placeholder2::new(client_output, requests).start()
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
        let self_address = started_client.address.to_string();

        let active_requests = ActiveRequests::default();

        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_reconstructor(client_output, active_requests.clone());

        Ok(MixFetchClient {
            self_address,
            placeholder: Placeholder::new(
                self.config.mix_fetch.network_requester_address,
                started_client.address,
                client_input,
                active_requests,
            ),
            _task_manager: started_client.task_manager,
        })
    }
}

#[wasm_bindgen]
impl MixFetchClient {
    async fn _new(
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
            Self::_new(config, preferred_gateway, storage_passphrase)
                .await
                .into_promise_result()
        })
    }

    pub fn self_address(&self) -> String {
        self.self_address.clone()
    }

    pub fn fetch_with_request(&self, input: &Request) -> Promise {
        match WebSysRequestAdapter::new_from_request(input) {
            Ok(req) => self.placeholder.fetch(true, 0u64, req),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }

    pub fn fetch_with_str(&self, input: &str) -> Promise {
        match WebSysRequestAdapter::new_from_string(input) {
            Ok(req) => self.placeholder.fetch(true, 0u64, req),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }

    pub fn fetch_with_request_and_init(
        &self,
        input: &Request,
        init: &RequestInitWithTypescriptType,
    ) -> Promise {
        match WebSysRequestAdapter::new_from_init_or_input(None, Some(input), init) {
            Ok(req) => self.placeholder.fetch(true, 0u64, req),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }

    pub fn fetch_with_str_and_init(
        &self,
        input: String,
        init: &RequestInitWithTypescriptType,
    ) -> Promise {
        match WebSysRequestAdapter::new_from_init_or_input(Some(input), None, init) {
            Ok(req) => self.placeholder.fetch(true, 0u64, req),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }
}

fn mix_http_request_error_to_js_error(err: MixHttpRequestError) -> JsValue {
    simple_js_error(err.to_string())
}
