// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

use nym_wasm_utils::wasm_error;
use thiserror::Error;

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

// Generates `From<FetchError> for JsValue` (wraps in `js_sys::Error`, giving
// DevTools a real Error with stack trace) and `From<FetchError> for Promise`
// (rejecting). Matches the workspace convention used by every other wasm
// crate (mix-fetch, client, node-tester, zknym-lib).
wasm_error!(FetchError);
