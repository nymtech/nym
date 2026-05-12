// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! `mixResolve`: hostname-to-IP resolution over the Nym mixnet tunnel.
//!
//! The thinnest possible WASM export: pulls the global `TUNNEL` out of
//! [`crate::TUNNEL`] and delegates to [`crate::dns::resolve`], which owns
//! the UDP / IPR DNS pipeline. No TCP, no TLS, no HTTP: useful as an
//! isolation diagnostic when `mixFetch` misbehaves.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::error::FetchError;
use crate::{dns, TUNNEL};

/// Resolve a hostname to an IP address through the mixnet tunnel.
///
/// Returns the IP as a string (e.g. `"93.184.216.34"`).
#[wasm_bindgen(js_name = "mixResolve")]
pub fn mix_resolve(hostname: String) -> js_sys::Promise {
    future_to_promise(async move {
        let tunnel = TUNNEL.get().ok_or(FetchError::NotConnected)?;
        let ip = dns::resolve(tunnel, &hostname).await?;
        Ok(JsValue::from_str(&ip.to_string()))
    })
}
