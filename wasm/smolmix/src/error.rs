// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

use thiserror::Error;

/// Unified error type — DNS, TLS, HTTP, and JS interop.
#[derive(Error, Debug)]
pub enum FetchError {
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    #[error("DNS error: {0}")]
    Dns(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] async_tungstenite::tungstenite::Error),

    #[error("JS interop error: {0}")]
    Js(String),

    #[error("tunnel error: {0}")]
    Tunnel(String),

    #[error("tunnel not connected")]
    NotConnected,

    #[error("operation timed out")]
    Timeout,
}

impl From<FetchError> for wasm_bindgen::JsValue {
    fn from(e: FetchError) -> Self {
        wasm_bindgen::JsValue::from_str(&e.to_string())
    }
}
