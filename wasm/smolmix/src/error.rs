// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_wasm_utils::wasm_error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    #[error("DNS error: {0}")]
    Dns(String),

    #[error("DNS resolver {endpoint} rate-limited us (HTTP 429)")]
    DnsRateLimited { endpoint: String },

    #[error("DNS resolver {endpoint} returned HTTP {status}")]
    DnsServerError { endpoint: String, status: u16 },

    #[error("DNS resolver {endpoint} returned {rcode}")]
    DnsResponseCode { endpoint: String, rcode: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "fetch")]
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[cfg(feature = "websocket")]
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

wasm_error!(FetchError);

/// Verbose detail for a `FetchError`, for debug logging only.
///
/// `hyper::Error`'s `Display` shows only the category ("connection error"); the
/// real cause (an `io::Error`, an h2 reason, a TLS alert) sits in its `Debug`
/// form and its `source()` chain. This walks both, so a logged error names the
/// cause instead of the bucket. One line, no newlines, so it stays one log line.
///
/// Gated to match the `dns` module (its only caller), which compiles under either
/// `fetch` or `websocket`, not the `dns` feature.
#[cfg(any(feature = "fetch", feature = "websocket"))]
pub(crate) fn detail(e: &FetchError) -> String {
    use std::error::Error as _;
    let mut out = e.to_string();
    if let FetchError::Hyper(h) = e {
        out.push_str(&format!(" [debug: {h:?}]"));
    }
    let mut src = e.source();
    while let Some(s) = src {
        out.push_str(&format!(" <- {s}"));
        src = s.source();
    }
    out
}
