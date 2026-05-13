// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `mixFetch`: WASM export, delegates to [`crate::fetch::fetch`].

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::error::FetchError;
use crate::{fetch, TUNNEL};

/// Execute an HTTP request through the mixnet tunnel.
#[wasm_bindgen(js_name = "mixFetch")]
pub fn mix_fetch(url: String, init: JsValue) -> js_sys::Promise {
    future_to_promise(async move {
        let tunnel = TUNNEL.get().ok_or(FetchError::NotConnected)?;
        fetch::fetch(tunnel, &url, &init).await.map_err(Into::into)
    })
}
