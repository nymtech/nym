// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixFetchError;
use crate::mix_fetch_client;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::error::{simple_rejected_promise, PromisableResult};

/// Called by go runtime whenever local connection produces any data that has to be sent to the remote.
//
// TODO: currently this requires weird workaround to put it on the global object inside JS...
// need some help from @MS to fix that up.
// this is not expected to be called but a normal user under any circumstances
// (perhaps it should be moved somewhere outside the global object then?
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
        let mix_fetch = mix_fetch_client().map_err(JsValue::from)?;
        mix_fetch.forward_request_content(request_id, data).await?;
        Ok(JsValue::undefined())
    })
}

/// Called by go runtime whenever it establishes new connection
/// (whether the initial one or on any redirection attempt).
//
// TODO: currently this requires weird workaround to put it on the global object inside JS...
// need some help from @MS to fix that up.
// this is not expected to be called but a normal user under any circumstances
// (perhaps it should be moved somewhere outside the global object then?
#[wasm_bindgen]
pub fn start_new_mixnet_connection(target: String) -> Promise {
    future_to_promise(async move {
        // this error should be impossible in normal use
        // (unless, of course, user is messing around, but then it's their fault for this panic)
        let mix_fetch = mix_fetch_client().map_err(JsValue::from)?;
        mix_fetch
            .connect_to_mixnet(target)
            .await
            .map(|request_id| request_id.to_string())
            .into_promise_result()
    })
}

#[wasm_bindgen]
pub fn mix_fetch_initialised() -> Result<bool, MixFetchError> {
    mix_fetch_client()?;
    Ok(true)
}

/// Called by go runtime whenever it's done with a connection
//
// TODO: currently this requires weird workaround to put it on the global object inside JS...
// need some help from @MS to fix that up.
// this is not expected to be called but a normal user under any circumstances
// (perhaps it should be moved somewhere outside the global object then?
#[wasm_bindgen]
pub fn finish_mixnet_connection(stringified_request_id: String) -> Promise {
    let request_id = match stringified_request_id.parse() {
        Ok(id) => id,
        Err(err) => {
            return simple_rejected_promise(format!("failed to parse received request: {err}"))
        }
    };

    future_to_promise(async move {
        // this error should be impossible in normal use
        // (unless, of course, user is messing around, but then it's their fault for this panic)
        let mix_fetch = mix_fetch_client().map_err(JsValue::from)?;
        mix_fetch.disconnect_from_mixnet(request_id).await?;
        Ok(JsValue::undefined())
    })
}

// note the namespace (defined in wasm/main.go)
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __go_rs_bridge__)]
    pub(crate) fn goWasmInjectServerData(raw_connection_id: String, data: Vec<u8>);

    #[wasm_bindgen(js_namespace = __go_rs_bridge__)]
    pub(crate) fn goWasmCloseRemoteSocket(raw_connection_id: String);

    #[wasm_bindgen(js_namespace = __go_rs_bridge__)]
    pub(crate) fn goWasmInjectConnError(raw_connection_id: String, error_msg: String);

    #[wasm_bindgen(js_namespace = __go_rs_bridge__)]
    pub(crate) fn goWasmSetMixFetchRequestTimeout(timeout_ms: u32);
}
