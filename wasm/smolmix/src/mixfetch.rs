// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! `mixFetch`: drop-in `fetch()` over the Nym mixnet tunnel.
//!
//! The thinnest possible WASM export: pulls the global `TUNNEL` out of
//! [`crate::TUNNEL`] and delegates to [`crate::fetch::fetch`], which owns
//! the DNS / TCP / TLS / HTTP pipeline.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::error::FetchError;
use crate::{fetch, TUNNEL};

/// Execute an HTTP request through the mixnet tunnel.
///
/// Returns `{ body: Uint8Array, status, statusText, headers }`. The TS
/// layer wraps this in a native `Response`.
#[wasm_bindgen(js_name = "mixFetch")]
pub fn mix_fetch(url: String, init: JsValue) -> js_sys::Promise {
    future_to_promise(async move {
        let tunnel = TUNNEL.get().ok_or(FetchError::NotConnected)?;
        fetch::fetch(tunnel, &url, &init).await.map_err(Into::into)
    })
}
