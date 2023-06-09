// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::client::base_client::ClientInput;
use nym_task::TaskManager;
use std::sync::Arc;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use crate::helpers::parse_recipient;
use crate::mix_fetch::mix_http_requests::RequestInitWithTypescriptType;
use crate::mix_fetch::request_adapter::WebSysRequestAdapter;
use web_sys::Request;

#[wasm_bindgen]
pub struct MixFetchClient {
    client_input: Arc<ClientInput>,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _task_manager: TaskManager,

    mix_fetch_network_requester_address: String,
}

// TODO: deal with it properly after merging with develop (and feature/extract-gateway-config)
#[wasm_bindgen]
pub struct MixFetchClientBuilder {
    config: Config,
    preferred_gateway: Option<IdentityKey>,

    storage_passphrase: Option<String>,
    reply_surb_storage_backend: browser_backend::Backend,

    on_message: js_sys::Function,
    // on_mix_fetch_message: Option<js_sys::Function>,

    // unimplemented:
    bandwidth_controller:
        Option<BandwidthController<FakeClient<DirectSigningNyxdClient>, EphemeralStorage>>,
    disabled_credentials: bool,
}

#[wasm_bindgen]
impl MixFetchClientBuilder {
    //
}

#[wasm_bindgen]
impl MixFetchClient {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub async fn new() -> Promise {
        todo!()
    }
    
    pub fn fetch_with_request(&self, input: &Request) -> Promise {
        let recipient = match parse_recipient(&self.mix_fetch_network_requester_address) {
            Ok(recipient) => recipient,
            Err(err) => return err.into_rejected_promise(),
        };
        match WebSysRequestAdapter::new_from_request(input) {
            Ok(req) => self.client_input.send_mix_fetch_message(
                recipient,
                0u64,
                true,
                0u64,
                req.http_codec_request(),
            ),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }

    pub fn fetch_with_str(&self, input: &str) -> Promise {
        let recipient = match parse_recipient(&self.mix_fetch_network_requester_address) {
            Ok(recipient) => recipient,
            Err(err) => return err.into_rejected_promise(),
        };
        match WebSysRequestAdapter::new_from_string(input) {
            Ok(req) => self.client_input.send_mix_fetch_message(
                recipient,
                0u64,
                true,
                0u64,
                req.http_codec_request(),
            ),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }

    pub fn fetch_with_request_and_init(
        &self,
        input: &Request,
        init: &RequestInitWithTypescriptType,
    ) -> Promise {
        let recipient = match parse_recipient(&self.mix_fetch_network_requester_address) {
            Ok(recipient) => recipient,
            Err(err) => return err.into_rejected_promise(),
        };
        match WebSysRequestAdapter::new_from_init_or_input(None, Some(input), init) {
            Ok(req) => self.client_input.send_mix_fetch_message(
                recipient,
                0u64,
                true,
                0u64,
                req.http_codec_request(),
            ),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }

    pub fn fetch_with_str_and_init(
        &self,
        input: String,
        init: &RequestInitWithTypescriptType,
    ) -> Promise {
        let recipient = match parse_recipient(&self.mix_fetch_network_requester_address) {
            Ok(recipient) => recipient,
            Err(err) => return err.into_rejected_promise(),
        };
        match WebSysRequestAdapter::new_from_init_or_input(Some(input), None, init) {
            Ok(req) => self.client_input.send_mix_fetch_message(
                recipient,
                0u64,
                true,
                0u64,
                req.http_codec_request(),
            ),
            Err(err) => Promise::reject(&mix_http_request_error_to_js_error(err)),
        }
    }
}

fn mix_http_request_error_to_js_error(err: MixHttpRequestError) -> JsValue {
    JsValue::from(JsError::new(&format!("{}", err)))
}
