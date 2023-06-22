// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::mix_fetch_client;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::simple_rejected_promise;

// called by go runtime whenever local connection produces any data that has to be sent to the remote
// TODO: currently this requires weird workaround to put it on the global object inside JS...
// need some help from @MS to fix that up.
#[wasm_bindgen]
pub fn send_client_data(stringified_request_id: String, data: Vec<u8>) -> Promise {
    let request_id = match stringified_request_id.parse() {
        Ok(id) => id,
        Err(err) => {
            return simple_rejected_promise(format!("failed to parse received request: {err}"))
        }
    };

    future_to_promise(async move {
        // this error should be impossible in normal use
        // (unless, of course, user is messing around, but then it's their fault for this panic)
        let mix_fetch = mix_fetch_client().expect("mix fetch hasn't been setup");
        mix_fetch.forward_request_content(request_id, data).await?;
        Ok(JsValue::undefined())
    })
}

#[wasm_bindgen]
extern "C" {
    pub(crate) fn goWasmMixFetch(raw_connection_id: String, endpoint: String) -> Promise;

    pub(crate) fn goWasmInjectServerData(raw_connection_id: String, data: Vec<u8>);

    pub(crate) fn goWasmCloseRemoteSocket(raw_connection_id: String);
}
